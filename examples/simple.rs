//! A simple program that counts down from 5 and then exits: Bubble
//! Tea's `simple` example. Go's command sleeps for a second on its own
//! goroutine; here a `tick` waits the second, so the program's tests run
//! on a virtual clock.
//!
//! `cargo run --example simple`

use petitcarnets as tea;
use std::time::Duration;

/// Seconds left. A model can be any type; this one is a number.
pub(crate) struct Model(pub(crate) i64);

/// The countdown's tick: one second has passed.
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
            tea::Msg::KeyPress(k) => match k.to_string().as_str() {
                "ctrl+c" | "q" => tea::quit(),
                // Ctrl-Z suspends before it gets here, unless the program
                // runs `without_suspend`; Go's program asks for it here.
                "ctrl+z" => tea::suspend(),
                _ => tea::Cmd::none(),
            },
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
        tea::View::new(format!(
            "Hi. This program will exit in {} seconds.\n\nTo quit sooner press ctrl-c, or press ctrl-z to suspend...\n",
            self.0
        ))
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Model(5)).run()?;
    Ok(())
}
