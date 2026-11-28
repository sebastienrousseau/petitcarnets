//! Starting, stopping, toggling and resetting the stopwatch.

use super::{Model, Msg, tick};
use crate::{Cmd, sequence};
use std::time::Duration;

impl Model {
    /// Starts it, then ticks.
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
    /// // The app starts the stopwatch, then calls `start` again on a key.
    /// let app = App(stopwatch::Model::new(), stopwatch::Model::start);
    /// let mut test = TestProgram::new(app, 10, 1);
    /// assert!(test.model().0.running());
    /// test.advance(Duration::from_secs(2));
    /// assert_eq!(test.model().0.view(), "2s");
    /// ```
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
    /// test.advance(Duration::from_secs(2));
    /// test.press(tea::Key::char('s'));
    /// test.advance(Duration::from_secs(5));
    /// assert!(!test.model().0.running());
    /// assert_eq!(test.model().0.view(), "2s");
    /// ```
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
    /// // The app starts the stopwatch, then calls `toggle` on a key.
    /// let app = App(stopwatch::Model::new(), stopwatch::Model::toggle);
    /// let mut test = TestProgram::new(app, 10, 1);
    /// test.press(tea::Key::char('s'));
    /// assert!(!test.model().0.running());
    /// test.press(tea::Key::char('s'));
    /// assert!(test.model().0.running());
    /// ```
    #[must_use]
    pub fn toggle(&self) -> Cmd<Msg> {
        if self.running {
            self.stop()
        } else {
            self.start()
        }
    }

    /// Sets it back to zero.
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
    /// // The app starts the stopwatch, then calls `reset` on a key.
    /// let app = App(stopwatch::Model::new(), stopwatch::Model::reset);
    /// let mut test = TestProgram::new(app, 10, 1);
    /// test.advance(Duration::from_secs(4));
    /// test.press(tea::Key::char('r'));
    /// assert_eq!(test.model().0.view(), "0s");
    /// test.advance(Duration::from_secs(1));
    /// assert_eq!(test.model().0.view(), "1s", "and still running");
    /// ```
    #[must_use]
    pub fn reset(&self) -> Cmd<Msg> {
        let id = self.id;
        crate::tick(Duration::ZERO, move |at| Msg::Reset { id, at })
    }
}
