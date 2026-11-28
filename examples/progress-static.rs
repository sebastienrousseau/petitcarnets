//! A progress bar drawn at a percentage, a quarter more each second,
//! in a gradient: Bubble Tea's `progress-static` example.
//!
//! `cargo run --example progress-static`

use petitcarnets as tea;
use std::time::Duration;
use tea::bubbles::progress;
use tea::style::{Color, Style};

const PADDING: usize = 2;
const MAX_WIDTH: usize = 80;
const HELP: Style = Style::new().fg(Color::Rgb(0x62, 0x62, 0x62));

pub(crate) struct Model {
    pub(crate) percent: f64,
    pub(crate) progress: progress::Model,
}

pub(crate) fn initial_model() -> Model {
    let pink = Color::Rgb(0xFF, 0x7C, 0xCB);
    let yellow = Color::Rgb(0xFD, 0xFF, 0x8C);
    Model {
        percent: 0.0,
        progress: progress::Model::new()
            .with_scaled(true)
            .with_colors(&[pink, yellow]),
    }
}

fn tick_cmd() -> tea::Cmd<()> {
    tea::tick(Duration::from_secs(1), |_| ())
}

impl tea::Model for Model {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tick_cmd()
    }

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(_) => tea::quit(),
            tea::Msg::WindowSize(size) => {
                let w = usize::from(size.width).saturating_sub(PADDING * 2 + 4);
                self.progress.set_width(w.min(MAX_WIDTH));
                tea::Cmd::none()
            }
            tea::Msg::App(()) => {
                self.percent += 0.25;
                if self.percent > 1.0 {
                    self.percent = 1.0;
                    return tea::quit();
                }
                tick_cmd()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let pad = " ".repeat(PADDING);
        tea::View::new(format!(
            "\n{pad}{}\n\n{pad}{}",
            self.progress.view_as(self.percent),
            HELP.paint("Press any key to quit")
        ))
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
