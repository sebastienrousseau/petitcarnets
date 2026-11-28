//! Answering messages: starting, stopping, ticking and timing out.

use super::{Model, Msg};
use crate::{Cmd, Msg as TeaMsg, batch};

impl Model {
    /// Starts, stops and counts down on its own messages.
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
    /// test.advance(Duration::from_secs(3));
    /// assert!(test.model().0.timedout());
    /// assert_eq!(test.model().0.view(), "0s");
    /// ```
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
}
