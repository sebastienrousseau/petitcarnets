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

mod control;

use super::next_id;
use super::timer::go_duration;
use crate::{Cmd, Msg as TeaMsg};
use std::time::{Duration, SystemTime};

/// A stopwatch's messages.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::{Duration, UNIX_EPOCH};
/// use tea::bubbles::stopwatch::{self, Msg};
///
/// let mut s = stopwatch::Model::new();
/// let id = s.id();
/// let start = Msg::StartStop {
///     id,
///     running: true,
///     at: UNIX_EPOCH,
/// };
/// assert!(s.update(&tea::Msg::App(start)).is_none());
/// let at = UNIX_EPOCH + Duration::from_secs(3);
/// assert!(
///     !s.update(&tea::Msg::App(Msg::Tick { id, at, tag: 0 }))
///         .is_none()
/// );
/// assert_eq!(s.view(), "3s");
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::stopwatch;
/// use tea::testing::TestProgram;
/// # struct App(
/// #     stopwatch::Model,
/// #     fn(&stopwatch::Model) -> tea::Cmd<stopwatch::Msg>,
/// # );
/// #
/// # impl tea::Model for App {
/// #     type Msg = stopwatch::Msg;
/// #     fn init(&mut self) -> tea::Cmd<stopwatch::Msg> {
/// #         self.0.init()
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<stopwatch::Msg>) -> tea::Cmd<stopwatch::Msg> {
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
/// let mut test = TestProgram::new(App(stopwatch::Model::new(), stopwatch::Model::stop), 10, 1);
/// test.advance(Duration::from_millis(2500));
/// assert_eq!(test.model().0.view(), "2s");
/// assert!(test.model().0.running());
/// ```
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
    ///
    /// ```
    /// use petitcarnets::bubbles::stopwatch;
    /// use std::time::Duration;
    ///
    /// let s = stopwatch::Model::new();
    /// assert_eq!(s.interval, Duration::from_secs(1));
    /// assert_eq!(s.elapsed(), Duration::ZERO);
    /// assert!(!s.running());
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::{Duration, UNIX_EPOCH};
    /// use tea::bubbles::stopwatch::{self, Msg};
    ///
    /// let mut s = stopwatch::Model::new().with_interval(Duration::from_millis(100));
    /// let id = s.id();
    /// let _ = s.update(&tea::Msg::App(Msg::StartStop {
    ///     id,
    ///     running: true,
    ///     at: UNIX_EPOCH,
    /// }));
    /// let at = UNIX_EPOCH + Duration::from_millis(1250);
    /// let _ = s.update(&tea::Msg::App(Msg::Tick { id, at, tag: 0 }));
    /// assert_eq!(s.view(), "1.2s");
    /// ```
    #[must_use]
    pub const fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// The stopwatch's id.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::UNIX_EPOCH;
    /// use tea::bubbles::stopwatch::{self, Msg};
    ///
    /// let mut s = stopwatch::Model::new();
    /// let other = stopwatch::Model::new();
    /// assert_ne!(s.id(), other.id());
    /// let id = other.id();
    /// let _ = s.update(&tea::Msg::App(Msg::StartStop {
    ///     id,
    ///     running: true,
    ///     at: UNIX_EPOCH,
    /// }));
    /// assert!(!s.running(), "not its message");
    /// ```
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Starts it: Go's `Init`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::stopwatch;
    /// use tea::testing::TestProgram;
    /// # struct App(
    /// #     stopwatch::Model,
    /// #     fn(&stopwatch::Model) -> tea::Cmd<stopwatch::Msg>,
    /// # );
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = stopwatch::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<stopwatch::Msg> {
    /// #         self.0.init()
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<stopwatch::Msg>) -> tea::Cmd<stopwatch::Msg> {
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
    /// // The app starts the stopwatch, then calls `stop` on a key.
    /// let app = App(stopwatch::Model::new(), stopwatch::Model::stop);
    /// let mut test = TestProgram::new(app, 10, 1);
    /// test.advance(Duration::from_secs(3));
    /// assert_eq!(test.model().0.view(), "3s");
    /// ```
    #[must_use]
    pub fn init(&self) -> Cmd<Msg> {
        self.start()
    }

    /// Whether it is running.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::stopwatch;
    /// use tea::testing::TestProgram;
    /// # struct App(
    /// #     stopwatch::Model,
    /// #     fn(&stopwatch::Model) -> tea::Cmd<stopwatch::Msg>,
    /// # );
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = stopwatch::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<stopwatch::Msg> {
    /// #         self.0.init()
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<stopwatch::Msg>) -> tea::Cmd<stopwatch::Msg> {
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
    /// // The app starts the stopwatch, then calls `stop` on a key.
    /// let app = App(stopwatch::Model::new(), stopwatch::Model::stop);
    /// let mut test = TestProgram::new(app, 10, 1);
    /// assert!(test.model().0.running());
    /// test.press(tea::Key::char('s'));
    /// assert!(!test.model().0.running());
    /// ```
    #[must_use]
    pub const fn running(&self) -> bool {
        self.running
    }

    /// Starts, stops, resets and counts on its own messages.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::{Duration, UNIX_EPOCH};
    /// use tea::bubbles::stopwatch::{self, Msg};
    ///
    /// let mut s = stopwatch::Model::new();
    /// let id = s.id();
    /// let _ = s.update(&tea::Msg::App(Msg::StartStop {
    ///     id,
    ///     running: true,
    ///     at: UNIX_EPOCH,
    /// }));
    /// let at = UNIX_EPOCH + Duration::from_secs(5);
    /// let _ = s.update(&tea::Msg::App(Msg::StartStop {
    ///     id,
    ///     running: false,
    ///     at,
    /// }));
    /// assert_eq!(s.elapsed(), Duration::from_secs(5));
    /// let _ = s.update(&tea::Msg::App(Msg::Reset { id, at }));
    /// assert_eq!(s.elapsed(), Duration::ZERO);
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::{Duration, UNIX_EPOCH};
    /// use tea::bubbles::stopwatch::{self, Msg};
    ///
    /// let mut s = stopwatch::Model::new();
    /// let id = s.id();
    /// let _ = s.update(&tea::Msg::App(Msg::StartStop {
    ///     id,
    ///     running: true,
    ///     at: UNIX_EPOCH,
    /// }));
    /// let at = UNIX_EPOCH + Duration::from_millis(1750);
    /// let _ = s.update(&tea::Msg::App(Msg::Tick { id, at, tag: 0 }));
    /// assert_eq!(s.elapsed(), Duration::from_millis(1750));
    /// assert_eq!(s.view(), "1s", "the view is cut to the interval");
    /// ```
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// The time counted, cut to the interval, as Go writes a duration.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::{Duration, UNIX_EPOCH};
    /// use tea::bubbles::stopwatch::{self, Msg};
    ///
    /// let mut s = stopwatch::Model::new();
    /// assert_eq!(s.view(), "0s");
    /// let id = s.id();
    /// let _ = s.update(&tea::Msg::App(Msg::StartStop {
    ///     id,
    ///     running: true,
    ///     at: UNIX_EPOCH,
    /// }));
    /// let at = UNIX_EPOCH + Duration::from_secs(61);
    /// let _ = s.update(&tea::Msg::App(Msg::Tick { id, at, tag: 0 }));
    /// assert_eq!(s.view(), "1m1s");
    /// ```
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
