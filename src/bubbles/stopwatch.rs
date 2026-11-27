//! A stopwatch that counts up at an interval: Bubbles' `stopwatch`
//! package.
//!
//! Go reads the wall clock in `Update` and `View`; here the time comes
//! from the program's clock, in the messages, so a stopwatch in a
//! [`TestProgram`](crate::testing::TestProgram) follows its virtual
//! clock.
//!
//! ```
//! use petitcarnets::bubbles::stopwatch;
//!
//! let s = stopwatch::Model::new();
//! assert_eq!(s.view(), "0s");
//! assert!(!s.running());
//! ```

use super::next_id;
use super::timer::go_duration;
use crate::{Cmd, Msg as TeaMsg, sequence};
use std::time::{Duration, SystemTime};

/// A stopwatch's messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    /// Start or stop: Bubbles' `stopwatch.StartStopMsg`.
    StartStop {
        /// The stopwatch it is for.
        id: u64,
        /// Start or stop.
        running: bool,
        /// When, on the program's clock.
        at: SystemTime,
    },
    /// Back to zero: Bubbles' `stopwatch.ResetMsg`.
    Reset {
        /// The stopwatch it is for.
        id: u64,
        /// When, on the program's clock.
        at: SystemTime,
    },
    /// Time to count: Bubbles' `stopwatch.TickMsg`.
    Tick {
        /// The stopwatch it is for.
        id: u64,
        /// When, on the program's clock.
        at: SystemTime,
        /// Which tick, so an older one is ignored.
        tag: u64,
    },
}

/// A stopwatch: Bubbles' `stopwatch.Model`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// How often it ticks.
    pub interval: Duration,
    elapsed: Duration,
    start: Option<SystemTime>,
    id: u64,
    tag: u64,
    running: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// A stopped stopwatch at zero, ticking every second once started:
    /// `stopwatch.New`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            interval: Duration::from_secs(1),
            elapsed: Duration::ZERO,
            start: None,
            id: next_id(),
            tag: 0,
            running: false,
        }
    }

    /// Ticking every `interval`: `stopwatch.WithInterval`.
    #[must_use]
    pub const fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// The stopwatch's id.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Starts it: Go's `Init`.
    #[must_use]
    pub fn init(&self) -> Cmd<Msg> {
        self.start()
    }

    /// Starts it, then ticks.
    #[must_use]
    pub fn start(&self) -> Cmd<Msg> {
        let (id, tag) = (self.id, self.tag);
        let start = crate::tick(Duration::ZERO, move |at| Msg::StartStop {
            id,
            running: true,
            at,
        });
        sequence([start, tick(id, tag, self.interval)])
    }

    /// Stops it.
    #[must_use]
    pub fn stop(&self) -> Cmd<Msg> {
        let id = self.id;
        crate::tick(Duration::ZERO, move |at| Msg::StartStop {
            id,
            running: false,
            at,
        })
    }

    /// Starts it if stopped, stops it if running.
    #[must_use]
    pub fn toggle(&self) -> Cmd<Msg> {
        if self.running {
            self.stop()
        } else {
            self.start()
        }
    }

    /// Sets it back to zero.
    #[must_use]
    pub fn reset(&self) -> Cmd<Msg> {
        let id = self.id;
        crate::tick(Duration::ZERO, move |at| Msg::Reset { id, at })
    }

    /// Whether it is running.
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }

    /// Starts, stops, resets and counts on its own messages.
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        match msg {
            TeaMsg::App(Msg::StartStop { id, running, at }) if *id == self.id => {
                self.start_stop(*running, *at);
            }
            TeaMsg::App(Msg::Reset { id, at }) if *id == self.id => {
                self.elapsed = Duration::ZERO;
                self.start = Some(*at);
            }
            TeaMsg::App(Msg::Tick { id, at, tag }) if self.running && *id == self.id => {
                return self.on_tick(*at, *tag);
            }
            _ => {}
        }
        Cmd::none()
    }

    /// Starts or stops at `at`.
    fn start_stop(&mut self, running: bool, at: SystemTime) {
        match (self.running, running) {
            (false, true) => self.start = Some(at),
            (true, false) => self.count(at),
            _ => {}
        }
        self.running = running;
    }

    /// Its tick `tag` at `at`: counts, and asks for the next.
    fn on_tick(&mut self, at: SystemTime, tag: u64) -> Cmd<Msg> {
        if tag > 0 && tag != self.tag {
            return Cmd::none();
        }
        self.count(at);
        self.tag += 1;
        tick(self.id, self.tag, self.interval)
    }

    /// Adds the time since the last count.
    fn count(&mut self, at: SystemTime) {
        let since = self
            .start
            .and_then(|s| at.duration_since(s).ok())
            .unwrap_or_default();
        self.elapsed += since;
        self.start = Some(at);
    }

    /// The time counted, up to the last message.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// The time counted, cut to the interval, as Go writes a duration.
    #[must_use]
    pub fn view(&self) -> String {
        let step = if self.interval.is_zero() {
            Duration::from_secs(1)
        } else {
            self.interval
        };
        let steps = self.elapsed.as_nanos() / step.as_nanos();
        let cut = step.checked_mul(u32::try_from(steps).unwrap_or(u32::MAX));
        go_duration(cut.unwrap_or(self.elapsed))
    }
}

/// The tick `tag` for stopwatch `id`, after `interval`.
fn tick(id: u64, tag: u64, interval: Duration) -> Cmd<Msg> {
    crate::tick(interval, move |at| Msg::Tick { id, at, tag })
}

#[cfg(test)]
mod tests;
