//! Debouncing commands: Bubble Tea's `debounce` example.
//!
//! When a key is pressed, the model's tag goes up by one, and after a
//! short delay a tick brings back the tag as it was then. If the tag in
//! the message is still the model's, no key came since, so the debounce
//! is over and the program quits; otherwise another tick is on its way
//! and this one is ignored.
//!
//! `cargo run --example debounce`

use petitcarnets as tea;
use std::time::Duration;

const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

/// The tag a tick brings back.
pub(crate) struct ExitMsg(u64);

#[derive(Default)]
pub(crate) struct Model {
    pub(crate) tag: u64,
}

impl tea::Model for Model {
    type Msg = ExitMsg;

    fn update(&mut self, msg: tea::Msg<ExitMsg>) -> tea::Cmd<ExitMsg> {
        match msg {
            tea::Msg::KeyPress(_) => {
                // Increment the tag on the model...
                self.tag += 1;
                let tag = self.tag;
                // ...and include a copy of it in the message.
                tea::tick(DEBOUNCE_DURATION, move |_| ExitMsg(tag))
            }
            // Only the last tick's tag is still the model's.
            tea::Msg::App(ExitMsg(tag)) if tag == self.tag => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        tea::View::new(format!(
            "Key presses: {}\nTo exit press any key, then wait for one second without pressing anything.",
            self.tag
        ))
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Model::default()).run()?;
    Ok(())
}
