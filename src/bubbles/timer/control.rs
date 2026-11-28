//! Starting, stopping and toggling the timer, and the commands that
//! tick it, time it out and start or stop it.

use super::{Model, Msg, StartStopMsg, TickMsg, TimeoutMsg, now};
use crate::Cmd;

impl Model {
    /// Starts it again.
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
    /// // The app stops the timer on a key, or starts it if stopped.
    /// let key = |t: &timer::Model| if t.running() { t.stop() } else { t.start() };
    /// let t = timer::Model::new(Duration::from_secs(3));
    /// let mut test = TestProgram::new(App(t, key), 10, 1);
    /// test.advance(Duration::from_secs(1));
    /// test.press(tea::Key::char('s'));
    /// test.press(tea::Key::char('s'));
    /// assert!(test.model().0.running());
    /// test.advance(Duration::from_secs(1));
    /// assert_eq!(test.model().0.view(), "1s");
    /// ```
    #[must_use]
    pub fn start(&self) -> Cmd<Msg> {
        self.start_stop(true)
    }

    /// Stops it.
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
    /// test.press(tea::Key::char('s'));
    /// test.advance(Duration::from_secs(5));
    /// assert_eq!(test.model().0.view(), "3s");
    /// ```
    #[must_use]
    pub fn stop(&self) -> Cmd<Msg> {
        self.start_stop(false)
    }

    /// Starts it if stopped, stops it if running.
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
    /// // The app starts the timer, then calls `toggle` on a key.
    /// let t = timer::Model::new(Duration::from_secs(3));
    /// let mut test = TestProgram::new(App(t, timer::Model::toggle), 10, 1);
    /// test.press(tea::Key::char('s'));
    /// assert!(!test.model().0.running());
    /// test.press(tea::Key::char('s'));
    /// assert!(test.model().0.running());
    /// ```
    #[must_use]
    pub fn toggle(&self) -> Cmd<Msg> {
        self.start_stop(!self.running())
    }

    /// The next tick.
    pub(super) fn tick(&self) -> Cmd<Msg> {
        let msg = TickMsg {
            id: self.id,
            timeout: self.timedout(),
            tag: self.tag,
        };
        crate::tick(self.interval, move |_| Msg::Tick(msg))
    }

    /// The timeout message, once run out.
    pub(super) fn timed_out(&self) -> Cmd<Msg> {
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
