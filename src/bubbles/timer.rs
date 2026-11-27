//! A countdown that ticks at an interval and says when it runs out:
//! Bubbles' `timer` package.
//!
//! ```
//! use petitcarnets::bubbles::timer;
//! use std::time::Duration;
//!
//! let t = timer::Model::new(Duration::from_secs(90));
//! assert_eq!(t.view(), "1m30s");
//! assert!(t.running());
//! ```

use super::{next_id, now};
use crate::{Cmd, Msg as TeaMsg, batch};
use std::fmt::Write as _;
use std::time::Duration;

/// Start or stop a timer: Bubbles' `timer.StartStopMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StartStopMsg {
    /// The timer it is for.
    pub id: u64,
    running: bool,
}

/// Time for a timer to count down one interval: Bubbles' `timer.TickMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TickMsg {
    /// The timer it is for.
    pub id: u64,
    /// Whether the timer had run out when the tick was asked for.
    pub timeout: bool,
    tag: u64,
}

/// A timer ran out: Bubbles' `timer.TimeoutMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeoutMsg {
    /// The timer that ran out.
    pub id: u64,
}

/// A timer's messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    /// Start or stop.
    StartStop(StartStopMsg),
    /// Count down.
    Tick(TickMsg),
    /// Ran out.
    Timeout(TimeoutMsg),
}

/// A countdown: Bubbles' `timer.Model`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The time left. Unlike Go's, it stops at zero rather than going
    /// below it when the interval does not divide it.
    pub timeout: Duration,
    /// How often it ticks.
    pub interval: Duration,
    id: u64,
    tag: u64,
    running: bool,
}

impl Model {
    /// A running timer for `timeout`, ticking every second: `timer.New`.
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            interval: Duration::from_secs(1),
            id: next_id(),
            tag: 0,
            running: true,
        }
    }

    /// Ticking every `interval`: `timer.WithInterval`.
    #[must_use]
    pub const fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// The timer's id.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Whether it is counting down: started and not run out.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running && !self.timedout()
    }

    /// Whether it has run out.
    #[must_use]
    pub const fn timedout(&self) -> bool {
        self.timeout.is_zero()
    }

    /// The first tick: Go's `Init`.
    #[must_use]
    pub fn init(&self) -> Cmd<Msg> {
        self.tick()
    }

    /// Starts, stops and counts down on its own messages.
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        match msg {
            TeaMsg::App(Msg::StartStop(m)) if self.is_mine(m.id) => self.start_or_stop(m.running),
            TeaMsg::App(Msg::Tick(m)) if self.running() && self.is_mine(m.id) => {
                self.on_tick(m.tag)
            }
            _ => Cmd::none(),
        }
    }

    /// Whether a message for timer `id` is this one's: its own, or 0 for
    /// every timer.
    const fn is_mine(&self, id: u64) -> bool {
        id == 0 || id == self.id
    }

    /// Starts or stops; a start ticks again.
    fn start_or_stop(&mut self, running: bool) -> Cmd<Msg> {
        if self.running == running {
            return Cmd::none();
        }
        self.running = running;
        if running { self.tick() } else { Cmd::none() }
    }

    /// Its tick `tag`: counts down an interval, ticks again, and says
    /// when it ran out.
    fn on_tick(&mut self, tag: u64) -> Cmd<Msg> {
        if tag > 0 && tag != self.tag {
            return Cmd::none();
        }
        self.timeout = self.timeout.saturating_sub(self.interval);
        self.tag += 1;
        batch([self.tick(), self.timed_out()])
    }

    /// The time left, as Go writes a duration (`"4.5s"`, `"1m0s"`).
    #[must_use]
    pub fn view(&self) -> String {
        go_duration(self.timeout)
    }

    /// Starts it again.
    #[must_use]
    pub fn start(&self) -> Cmd<Msg> {
        self.start_stop(true)
    }

    /// Stops it.
    #[must_use]
    pub fn stop(&self) -> Cmd<Msg> {
        self.start_stop(false)
    }

    /// Starts it if stopped, stops it if running.
    #[must_use]
    pub fn toggle(&self) -> Cmd<Msg> {
        self.start_stop(!self.running())
    }

    /// The next tick.
    fn tick(&self) -> Cmd<Msg> {
        let msg = TickMsg {
            id: self.id,
            timeout: self.timedout(),
            tag: self.tag,
        };
        crate::tick(self.interval, move |_| Msg::Tick(msg))
    }

    /// The timeout message, once run out.
    fn timed_out(&self) -> Cmd<Msg> {
        if self.timedout() {
            now(Msg::Timeout(TimeoutMsg { id: self.id }))
        } else {
            Cmd::none()
        }
    }

    /// A start or stop message; a timer that ran out does not start.
    fn start_stop(&self, running: bool) -> Cmd<Msg> {
        if running && self.timedout() {
            return Cmd::none();
        }
        now(Msg::StartStop(StartStopMsg {
            id: self.id,
            running,
        }))
    }
}

/// `d` as Go's `time.Duration.String` writes it: `"0s"`, `"1.5ms"`,
/// `"2m3.25s"`, `"1h0m0s"`.
pub(super) fn go_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos == 0 {
        return "0s".into();
    }
    if nanos < 1_000_000_000 {
        let (unit, scale) = match nanos {
            0..1_000 => ("ns", 1),
            1_000..1_000_000 => ("µs", 1_000),
            _ => ("ms", 1_000_000),
        };
        return format!("{}{unit}", decimal(nanos, scale));
    }
    let secs = d.as_secs();
    let (h, m) = (secs / 3600, secs / 60 % 60);
    let mut out = String::new();
    if h > 0 {
        let _ = write!(out, "{h}h");
    }
    if h > 0 || m > 0 {
        let _ = write!(out, "{m}m");
    }
    let rest = u128::from(secs % 60) * 1_000_000_000 + u128::from(d.subsec_nanos());
    out + &decimal(rest, 1_000_000_000) + "s"
}

/// `value / scale` with as many decimals as it needs.
fn decimal(value: u128, scale: u128) -> String {
    let (whole, frac) = (value / scale, value % scale);
    if frac == 0 {
        return whole.to_string();
    }
    let digits = scale.ilog10() as usize;
    let frac = format!("{frac:0digits$}");
    format!("{whole}.{}", frac.trim_end_matches('0'))
}

#[cfg(test)]
mod tests;
