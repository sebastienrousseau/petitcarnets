//! Running a series of commands in order: Bubble Tea's `sequence`
//! example. `tea::sequence` runs its commands one after another;
//! `tea::batch` runs them at the same time, in no order. Each line is
//! printed after a pause, as Go's `SleepPrintln` does.
//!
//! `cargo run --example sequence`

use petitcarnets as tea;
use std::time::Duration;

pub(crate) struct Model;

/// Prints `s` after `ms` milliseconds: a tick, then the print.
fn sleep_println(s: &str, ms: u64) -> tea::Cmd<()> {
    tea::sequence([
        tea::tick(Duration::from_millis(ms), |_| ()),
        tea::println(s),
    ])
}

impl tea::Model for Model {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tea::sequence([
            tea::batch([
                tea::sequence([sleep_println("1-1-1", 1000), sleep_println("1-1-2", 1000)]),
                tea::batch([sleep_println("1-2-1", 1500), sleep_println("1-2-2", 1250)]),
            ]),
            tea::println("2"),
            tea::sequence([
                tea::batch([sleep_println("3-1-1", 500), sleep_println("3-1-2", 1000)]),
                tea::sequence([sleep_println("3-2-1", 750), sleep_println("3-2-2", 500)]),
            ]),
            tea::quit(),
        ])
    }

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(_) => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        tea::View::new("")
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Model).run()?;
    Ok(())
}
