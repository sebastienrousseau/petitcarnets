//! Every kind of spinner, one at a time, changed with the arrows:
//! Bubble Tea's `spinners` example.
//!
//! `cargo run --example spinners`

use petitcarnets as tea;
use tea::bubbles::spinner::{self, Spinner};
use tea::style::{Color, Style};

const SPINNERS: [Spinner; 9] = [
    Spinner::LINE,
    Spinner::DOT,
    Spinner::MINI_DOT,
    Spinner::JUMP,
    Spinner::PULSE,
    Spinner::POINTS,
    Spinner::GLOBE,
    Spinner::MOON,
    Spinner::MONKEY,
];

const TEXT: Style = Style::new().fg(Color::Palette(252));
const SPINNER: Style = Style::new().fg(Color::Palette(69));
const HELP: Style = Style::new().fg(Color::Palette(241));

pub(crate) struct Model {
    pub(crate) index: usize,
    pub(crate) spinner: spinner::Model,
}

pub(crate) fn initial_model() -> Model {
    let mut m = Model {
        index: 0,
        spinner: spinner::Model::new(),
    };
    m.reset_spinner();
    m
}

impl Model {
    fn reset_spinner(&mut self) {
        self.spinner = spinner::Model::new()
            .with_style(SPINNER)
            .with_spinner(SPINNERS[self.index]);
    }
}

impl tea::Model for Model {
    type Msg = spinner::TickMsg;

    fn init(&mut self) -> tea::Cmd<spinner::TickMsg> {
        self.spinner.tick()
    }

    fn update(&mut self, msg: tea::Msg<spinner::TickMsg>) -> tea::Cmd<spinner::TickMsg> {
        match &msg {
            tea::Msg::KeyPress(k) => match k.to_string().as_str() {
                "ctrl+c" | "q" | "esc" => tea::quit(),
                "h" | "left" => {
                    self.index = (self.index + SPINNERS.len() - 1) % SPINNERS.len();
                    self.reset_spinner();
                    self.spinner.tick()
                }
                "l" | "right" => {
                    self.index = (self.index + 1) % SPINNERS.len();
                    self.reset_spinner();
                    self.spinner.tick()
                }
                _ => tea::Cmd::none(),
            },
            tea::Msg::App(_) => self.spinner.update(&msg),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let gap = if self.index == 1 { "" } else { " " };
        let s = format!(
            "\n {}{gap}{}\n\n{}",
            self.spinner.view(),
            TEXT.paint("Spinning..."),
            HELP.paint("h/l, ←/→: change spinner • q: exit\n")
        );
        tea::View::new(s)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
