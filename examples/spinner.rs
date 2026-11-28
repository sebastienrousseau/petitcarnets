//! A spinner that ticks while slow work goes on: a two-second wait, then
//! 20! computed on another thread.
//!
//! `cargo run --example spinner`

use petitcarnets as tea;
use std::time::Duration;
use tea::style::{Color, Style};

const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

pub(crate) enum Msg {
    Tick,
    Compute,
    Loaded(u64),
}

pub(crate) struct Loader {
    pub(crate) frame: usize,
    pub(crate) result: Option<u64>,
}

fn tick() -> tea::Cmd<Msg> {
    tea::tick(Duration::from_millis(80), |_| Msg::Tick)
}

impl tea::Model for Loader {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        let slow = tea::tick(Duration::from_secs(2), |_| Msg::Compute);
        tea::batch([tick(), slow])
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::App(Msg::Tick) if self.result.is_none() => {
                self.frame = (self.frame + 1) % FRAMES.len();
                tick()
            }
            tea::Msg::App(Msg::Compute) => tea::Cmd::task(|| Msg::Loaded((1..=20).product())),
            tea::Msg::App(Msg::Loaded(n)) => {
                self.result = Some(n);
                tea::Cmd::none()
            }
            tea::Msg::KeyPress(k) if k == "q" || k == "esc" || k == "ctrl+c" => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let text = match self.result {
            None => format!(
                "\n  {} Computing 20!...",
                Style::new().fg(Color::MAGENTA).paint(FRAMES[self.frame])
            ),
            Some(n) => format!(
                "\n  Done: 20! = {}\n\n  {}",
                Style::new().bold(true).paint(&n.to_string()),
                Style::new().dim(true).paint("q to quit")
            ),
        };
        // Inline, as Bubble Tea's spinner example: the result stays in
        // the scrollback.
        tea::View::new(text)
    }
}

// `pub(crate)`: `tests/golden.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Loader {
        frame: 0,
        result: None,
    })
    .run()?;
    Ok(())
}
