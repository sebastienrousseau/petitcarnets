//! A progress bar that springs to a quarter more each second: Bubble
//! Tea's `progress-animated` example.
//!
//! `cargo run --example progress-animated`

use petitcarnets as tea;
use std::time::Duration;
use tea::bubbles::progress;
use tea::style::{Color, Style};

const PADDING: usize = 2;
const MAX_WIDTH: usize = 80;
const HELP: Style = Style::new().fg(Color::Rgb(0x62, 0x62, 0x62));

pub(crate) enum Msg {
    Tick,
    Frame(progress::FrameMsg),
}

pub(crate) struct Model {
    pub(crate) progress: progress::Model,
}

pub(crate) fn initial_model() -> Model {
    Model {
        progress: progress::Model::new().with_default_blend(),
    }
}

fn tick_cmd() -> tea::Cmd<Msg> {
    tea::tick(Duration::from_secs(1), |_| Msg::Tick)
}

impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        tick_cmd()
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(_) => tea::quit(),
            tea::Msg::WindowSize(size) => {
                let w = usize::from(size.width).saturating_sub(PADDING * 2 + 4);
                self.progress.set_width(w.min(MAX_WIDTH));
                tea::Cmd::none()
            }
            tea::Msg::App(Msg::Tick) => {
                if (self.progress.percent() - 1.0).abs() < f64::EPSILON {
                    return tea::quit();
                }
                let cmd = self.progress.incr_percent(0.25).map(Msg::Frame);
                tea::batch([tick_cmd(), cmd])
            }
            tea::Msg::App(Msg::Frame(f)) => self.progress.update(&tea::Msg::App(f)).map(Msg::Frame),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let pad = " ".repeat(PADDING);
        tea::View::new(format!(
            "\n{pad}{}\n\n{pad}{}",
            self.progress.view(),
            HELP.paint("Press any key to quit")
        ))
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
