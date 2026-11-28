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

mod control;
mod update;

use super::{next_id, now};
use crate::Cmd;
use std::fmt::Write as _;
use std::time::Duration;

/// Start or stop a timer: Bubbles' `timer.StartStopMsg`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::timer;
/// use tea::testing::TestProgram;
/// # struct App(timer::Model, fn(&timer::Model) -> tea::Cmd<timer::Msg>);
/// #
/// # impl tea::Model for App {
/// #     type Msg = timer::Msg;
/// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
/// #         self.0.init()
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
/// #         match msg {
/// #             tea::Msg::KeyPress(_) => (self.1)(&self.0),
/// #             _ => self.0.update(&msg),
/// #         }
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app starts the timer, then calls `stop` on a key.
/// let t = timer::Model::new(Duration::from_secs(3));
/// let mut test = TestProgram::new(App(t, timer::Model::stop), 10, 1);
/// test.advance(Duration::from_secs(1));
/// test.press(tea::Key::char('s'));
/// test.advance(Duration::from_secs(5));
/// assert!(!test.model().0.running(), "the stop message came");
/// assert_eq!(test.model().0.view(), "2s");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StartStopMsg {
    /// The timer it is for.
    pub id: u64,
    running: bool,
}

/// Time for a timer to count down one interval: Bubbles' `timer.TickMsg`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::timer;
/// use tea::testing::TestProgram;
/// # struct App(timer::Model, fn(&timer::Model) -> tea::Cmd<timer::Msg>);
/// #
/// # impl tea::Model for App {
/// #     type Msg = timer::Msg;
/// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
/// #         self.0.init()
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
/// #         match msg {
/// #             tea::Msg::KeyPress(_) => (self.1)(&self.0),
/// #             _ => self.0.update(&msg),
/// #         }
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app starts the timer, then calls `stop` on a key.
/// let t = timer::Model::new(Duration::from_secs(3));
/// let mut test = TestProgram::new(App(t, timer::Model::stop), 10, 1);
/// assert_eq!(test.model().0.view(), "3s");
/// test.advance(Duration::from_secs(1));
/// assert_eq!(test.model().0.view(), "2s", "a tick came");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TickMsg {
    /// The timer it is for.
    pub id: u64,
    /// Whether the timer had run out when the tick was asked for.
    pub timeout: bool,
    tag: u64,
}

/// A timer ran out: Bubbles' `timer.TimeoutMsg`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::timer;
/// use tea::testing::TestProgram;
/// # struct App(timer::Model, Option<timer::TimeoutMsg>);
/// #
/// # impl tea::Model for App {
/// #     type Msg = timer::Msg;
/// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
/// #         self.0.init()
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
/// #         if let tea::Msg::App(timer::Msg::Timeout(out)) = msg {
/// #             self.1 = Some(out);
/// #             return tea::quit();
/// #         }
/// #         self.0.update(&msg)
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app quits when its timer runs out.
/// let t = timer::Model::new(Duration::from_secs(3));
/// let id = t.id();
/// let mut test = TestProgram::new(App(t, None), 10, 1);
/// assert!(test.wait_finished(Duration::from_secs(5)));
/// assert_eq!(test.elapsed(), Duration::from_secs(3));
/// assert_eq!(test.final_model().1, Some(timer::TimeoutMsg { id }));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeoutMsg {
    /// The timer that ran out.
    pub id: u64,
}

/// A timer's messages.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::timer;
/// use tea::testing::TestProgram;
/// # struct App(timer::Model, Option<timer::TimeoutMsg>);
/// #
/// # impl tea::Model for App {
/// #     type Msg = timer::Msg;
/// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
/// #         self.0.init()
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
/// #         if let tea::Msg::App(timer::Msg::Timeout(out)) = msg {
/// #             self.1 = Some(out);
/// #             return tea::quit();
/// #         }
/// #         self.0.update(&msg)
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app quits when its timer runs out.
/// let t = timer::Model::new(Duration::from_secs(3));
/// let id = t.id();
/// let mut test = TestProgram::new(App(t, None), 10, 1);
/// assert!(test.wait_finished(Duration::from_secs(5)));
/// assert_eq!(test.elapsed(), Duration::from_secs(3));
/// let out = test.final_model().1.map(timer::Msg::Timeout);
/// assert_eq!(out, Some(timer::Msg::Timeout(timer::TimeoutMsg { id })));
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::timer;
/// use tea::testing::TestProgram;
/// # struct App(timer::Model, fn(&timer::Model) -> tea::Cmd<timer::Msg>);
/// #
/// # impl tea::Model for App {
/// #     type Msg = timer::Msg;
/// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
/// #         self.0.init()
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
/// #         match msg {
/// #             tea::Msg::KeyPress(_) => (self.1)(&self.0),
/// #             _ => self.0.update(&msg),
/// #         }
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app starts the timer, then calls `stop` on a key.
/// let t = timer::Model::new(Duration::from_secs(90));
/// let mut test = TestProgram::new(App(t, timer::Model::stop), 10, 1);
/// assert_eq!(test.model().0.view(), "1m30s");
/// test.advance(Duration::from_secs(31));
/// assert_eq!(test.model().0.view(), "59s");
/// ```
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
    ///
    /// ```
    /// use petitcarnets::bubbles::timer;
    /// use std::time::Duration;
    ///
    /// let t = timer::Model::new(Duration::from_secs(5));
    /// assert_eq!(t.timeout, Duration::from_secs(5));
    /// assert_eq!(t.interval, Duration::from_secs(1));
    /// assert!(t.running());
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::timer;
    /// use tea::testing::TestProgram;
    /// # struct App(timer::Model, fn(&timer::Model) -> tea::Cmd<timer::Msg>);
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = timer::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
    /// #         self.0.init()
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
    /// #         match msg {
    /// #             tea::Msg::KeyPress(_) => (self.1)(&self.0),
    /// #             _ => self.0.update(&msg),
    /// #         }
    /// #     }
    /// #     fn view(&self) -> tea::View {
    /// #         tea::View::new(self.0.view())
    /// #     }
    /// # }
    ///
    /// let t = timer::Model::new(Duration::from_secs(1)).with_interval(Duration::from_millis(100));
    /// let mut test = TestProgram::new(App(t, timer::Model::stop), 10, 1);
    /// test.advance(Duration::from_millis(300));
    /// assert_eq!(test.model().0.view(), "700ms");
    /// ```
    #[must_use]
    pub const fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// The timer's id.
    ///
    /// ```
    /// use petitcarnets::bubbles::timer;
    /// use std::time::Duration;
    ///
    /// let a = timer::Model::new(Duration::from_secs(5));
    /// let b = timer::Model::new(Duration::from_secs(5));
    /// assert_ne!(a.id(), b.id());
    /// ```
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Whether it is counting down: started and not run out.
    ///
    /// ```
    /// use petitcarnets::bubbles::timer;
    /// use std::time::Duration;
    ///
    /// assert!(timer::Model::new(Duration::from_secs(5)).running());
    /// assert!(!timer::Model::new(Duration::ZERO).running(), "it ran out");
    /// ```
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running && !self.timedout()
    }

    /// Whether it has run out.
    ///
    /// ```
    /// use petitcarnets::bubbles::timer;
    /// use std::time::Duration;
    ///
    /// let mut t = timer::Model::new(Duration::from_secs(5));
    /// assert!(!t.timedout());
    /// t.timeout = Duration::ZERO;
    /// assert!(t.timedout());
    /// ```
    #[must_use]
    pub const fn timedout(&self) -> bool {
        self.timeout.is_zero()
    }

    /// The first tick: Go's `Init`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::timer;
    /// use tea::testing::TestProgram;
    /// # struct App(timer::Model, fn(&timer::Model) -> tea::Cmd<timer::Msg>);
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = timer::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<timer::Msg> {
    /// #         self.0.init()
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
    /// #         match msg {
    /// #             tea::Msg::KeyPress(_) => (self.1)(&self.0),
    /// #             _ => self.0.update(&msg),
    /// #         }
    /// #     }
    /// #     fn view(&self) -> tea::View {
    /// #         tea::View::new(self.0.view())
    /// #     }
    /// # }
    ///
    /// // The app starts the timer, then calls `stop` on a key.
    /// let t = timer::Model::new(Duration::from_secs(3));
    /// let mut test = TestProgram::new(App(t, timer::Model::stop), 10, 1);
    /// test.advance(Duration::from_secs(2));
    /// assert_eq!(test.model().0.view(), "1s");
    /// ```
    #[must_use]
    pub fn init(&self) -> Cmd<Msg> {
        self.tick()
    }

    /// The time left, as Go writes a duration (`"4.5s"`, `"1m0s"`).
    ///
    /// ```
    /// use petitcarnets::bubbles::timer;
    /// use std::time::Duration;
    ///
    /// assert_eq!(
    ///     timer::Model::new(Duration::from_millis(4500)).view(),
    ///     "4.5s"
    /// );
    /// assert_eq!(timer::Model::new(Duration::from_secs(60)).view(), "1m0s");
    /// assert_eq!(
    ///     timer::Model::new(Duration::from_secs(3600)).view(),
    ///     "1h0m0s"
    /// );
    /// ```
    #[must_use]
    pub fn view(&self) -> String {
        go_duration(self.timeout)
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
