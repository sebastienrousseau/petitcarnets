//! A five second countdown, every millisecond: `s` stops and starts
//! it, `r` resets it. Bubble Tea's `timer` example.
//!
//! `cargo run --example timer`

use petitcarnets as tea;
use std::time::Duration;
use tea::bubbles::help;
use tea::bubbles::key::{self, Binding};
use tea::bubbles::timer;

const TIMEOUT: Duration = Duration::from_secs(5);

/// The example's keys.
pub(crate) struct Keys {
    start: Binding,
    stop: Binding,
    reset: Binding,
    quit: Binding,
}

pub(crate) struct Model {
    pub(crate) timer: timer::Model,
    pub(crate) keymap: Keys,
    pub(crate) help: help::Model,
    pub(crate) quitting: bool,
}

pub(crate) fn initial_model() -> Model {
    let mut keymap = Keys {
        start: Binding::new(&["s"]).with_help("s", "start"),
        stop: Binding::new(&["s"]).with_help("s", "stop"),
        reset: Binding::new(&["r"]).with_help("r", "reset"),
        quit: Binding::new(&["q", "ctrl+c"]).with_help("q", "quit"),
    };
    keymap.start.set_enabled(false);
    Model {
        timer: timer::Model::new(TIMEOUT).with_interval(Duration::from_millis(1)),
        keymap,
        help: help::Model::new(),
        quitting: false,
    }
}

impl Model {
    fn help_view(&self) -> String {
        let k = &self.keymap;
        let bindings = [
            k.start.clone(),
            k.stop.clone(),
            k.reset.clone(),
            k.quit.clone(),
        ];
        format!("\n{}", self.help.short_help_view(&bindings))
    }
}

impl tea::Model for Model {
    type Msg = timer::Msg;

    fn init(&mut self) -> tea::Cmd<timer::Msg> {
        self.timer.init()
    }

    fn update(&mut self, msg: tea::Msg<timer::Msg>) -> tea::Cmd<timer::Msg> {
        match &msg {
            tea::Msg::App(timer::Msg::Tick(_)) => self.timer.update(&msg),
            tea::Msg::App(timer::Msg::StartStop(_)) => {
                let cmd = self.timer.update(&msg);
                self.keymap.stop.set_enabled(self.timer.running());
                self.keymap.start.set_enabled(!self.timer.running());
                cmd
            }
            tea::Msg::App(timer::Msg::Timeout(_)) => {
                self.quitting = true;
                tea::quit()
            }
            tea::Msg::KeyPress(k) if self.keymap.quit.matches(k) => {
                self.quitting = true;
                tea::quit()
            }
            tea::Msg::KeyPress(k) if self.keymap.reset.matches(k) => {
                self.timer.timeout = TIMEOUT;
                tea::Cmd::none()
            }
            tea::Msg::KeyPress(k) if key::matches(k, &[&self.keymap.start, &self.keymap.stop]) => {
                self.timer.toggle()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let mut s = if self.timer.timedout() {
            "All done!".to_string()
        } else {
            self.timer.view()
        };
        s.push('\n');
        if !self.quitting {
            s = format!("Exiting in {s}{}", self.help_view());
        }
        tea::View::new(s)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
