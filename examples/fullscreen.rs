//! A simple program that opens the alternate screen buffer, counts down
//! from 5 and then exits: Bubble Tea's `fullscreen` example.
//!
//! `cargo run --example fullscreen`

use petitcarnets as tea;
use std::time::Duration;

/// Seconds left.
pub(crate) struct Model(pub(crate) i64);

fn tick() -> tea::Cmd<()> {
    tea::tick(Duration::from_secs(1), |_| ())
}

impl tea::Model for Model {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tick()
    }

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k.matches_any(&["q", "esc", "ctrl+c"]) => tea::quit(),
            tea::Msg::App(()) => {
                self.0 -= 1;
                if self.0 <= 0 {
                    return tea::quit();
                }
                tick()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let text = format!(
            "\n\n     Hi. This program will exit in {} seconds...",
            self.0
        );
        tea::View::new(text).with_alt_screen(true)
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Model(5)).run()?;
    Ok(())
}
