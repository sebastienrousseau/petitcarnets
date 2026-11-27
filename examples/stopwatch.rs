//! A stopwatch counting milliseconds: `s` stops and starts it, `r`
//! resets it. Bubble Tea's `stopwatch` example.
//!
//! `cargo run --example stopwatch`

use petitcarnets as tea;
use std::time::Duration;
use tea::bubbles::help;
use tea::bubbles::key::{self, Binding};
use tea::bubbles::stopwatch;

/// The example's keys.
pub(crate) struct Keys {
    start: Binding,
    stop: Binding,
    reset: Binding,
    quit: Binding,
}

pub(crate) struct Model {
    pub(crate) stopwatch: stopwatch::Model,
    pub(crate) keymap: Keys,
    pub(crate) help: help::Model,
    pub(crate) quitting: bool,
}

pub(crate) fn initial_model() -> Model {
    let mut keymap = Keys {
        start: Binding::new(&["s"]).with_help("s", "start"),
        stop: Binding::new(&["s"]).with_help("s", "stop"),
        reset: Binding::new(&["r"]).with_help("r", "reset"),
        quit: Binding::new(&["ctrl+c", "q"]).with_help("q", "quit"),
    };
    keymap.start.set_enabled(false);
    Model {
        stopwatch: stopwatch::Model::new().with_interval(Duration::from_millis(1)),
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
    type Msg = stopwatch::Msg;

    fn init(&mut self) -> tea::Cmd<stopwatch::Msg> {
        self.stopwatch.init()
    }

    fn update(&mut self, msg: tea::Msg<stopwatch::Msg>) -> tea::Cmd<stopwatch::Msg> {
        if let tea::Msg::KeyPress(k) = &msg {
            if self.keymap.quit.matches(k) {
                self.quitting = true;
                return tea::quit();
            }
            if self.keymap.reset.matches(k) {
                return self.stopwatch.reset();
            }
            if key::matches(k, &[&self.keymap.start, &self.keymap.stop]) {
                let running = self.stopwatch.running();
                self.keymap.stop.set_enabled(!running);
                self.keymap.start.set_enabled(running);
                return self.stopwatch.toggle();
            }
        }
        self.stopwatch.update(&msg)
    }

    fn view(&self) -> tea::View {
        let mut s = format!("{}\n", self.stopwatch.view());
        if !self.quitting {
            s = format!("Elapsed: {s}{}", self.help_view());
        }
        tea::View::new(s)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
