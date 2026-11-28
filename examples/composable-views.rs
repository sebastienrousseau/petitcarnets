//! Two components side by side, one of them focused: Bubble Tea's
//! `composable-views` example. Tab moves the focus between a timer and a
//! spinner; n starts a new timer or shows the next spinner.
//!
//! Lip Gloss centres the boxes' text with `Align(Center, Center)`;
//! petitcarnets' `Style` has no alignment yet, so `center` below does
//! what Lip Gloss does (the odd column and row go right and down).
//!
//! `cargo run --example composable-views`

use petitcarnets as tea;
use std::time::Duration;
use tea::bubbles::{spinner, timer};
use tea::style::{Border, Color, Position, Style, join_horizontal, width};

const DEFAULT_TIME: Duration = Duration::from_secs(60);

const SPINNERS: [spinner::Spinner; 9] = [
    spinner::Spinner::LINE,
    spinner::Spinner::DOT,
    spinner::Spinner::MINI_DOT,
    spinner::Spinner::JUMP,
    spinner::Spinner::PULSE,
    spinner::Spinner::POINTS,
    spinner::Spinner::GLOBE,
    spinner::Spinner::MOON,
    spinner::Spinner::MONKEY,
];

/// The boxes, 15 by 5 with their borders, as Lip Gloss v2 counts them.
const BOX: (u16, u16) = (15, 5);
const MODEL_STYLE: Style = Style::new()
    .width(BOX.0)
    .height(BOX.1)
    .border(Border::HIDDEN);
const FOCUSED_MODEL_STYLE: Style = Style::new()
    .width(BOX.0)
    .height(BOX.1)
    .border(Border::NORMAL)
    .border_fg(Color::Palette(69));
const SPINNER_STYLE: Style = Style::new().fg(Color::Palette(69));
const HELP_STYLE: Style = Style::new().fg(Color::Palette(241));

/// Which component has the focus.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SessionState {
    Timer,
    Spinner,
}

pub(crate) enum Msg {
    Timer(timer::Msg),
    Spinner(spinner::TickMsg),
}

pub(crate) struct MainModel {
    pub(crate) state: SessionState,
    pub(crate) timer: timer::Model,
    pub(crate) spinner: spinner::Model,
    pub(crate) index: usize,
}

pub(crate) fn new_model(timeout: Duration) -> MainModel {
    MainModel {
        state: SessionState::Timer,
        timer: timer::Model::new(timeout),
        spinner: spinner::Model::new(),
        index: 0,
    }
}

impl MainModel {
    fn next(&mut self) {
        self.index = (self.index + 1) % SPINNERS.len();
    }

    fn reset_spinner(&mut self) {
        self.spinner = spinner::Model::new()
            .with_style(SPINNER_STYLE)
            .with_spinner(SPINNERS[self.index]);
    }

    const fn current_focused_model(&self) -> &'static str {
        match self.state {
            SessionState::Timer => "timer",
            SessionState::Spinner => "spinner",
        }
    }

    fn key(&mut self, key: &str) -> tea::Cmd<Msg> {
        match key {
            "ctrl+c" | "q" => tea::quit(),
            "tab" => {
                self.state = match self.state {
                    SessionState::Timer => SessionState::Spinner,
                    SessionState::Spinner => SessionState::Timer,
                };
                tea::Cmd::none()
            }
            "n" if self.state == SessionState::Timer => {
                self.timer = timer::Model::new(DEFAULT_TIME);
                self.timer.init().map(Msg::Timer)
            }
            "n" => {
                self.next();
                self.reset_spinner();
                self.spinner.tick().map(Msg::Spinner)
            }
            _ => tea::Cmd::none(),
        }
    }
}

/// `text` centred in a box `columns` by `rows`, as Lip Gloss aligns it.
fn center(text: &str, columns: usize, rows: usize) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let top = rows.saturating_sub(lines.len()) / 2;
    let mut out = vec![String::new(); top];
    for line in lines {
        let gap = columns.saturating_sub(width(line));
        out.push(format!(
            "{}{line}{}",
            " ".repeat(gap / 2),
            " ".repeat(gap - gap / 2)
        ));
    }
    out.resize(rows.max(out.len()), String::new());
    out.join("\n")
}

/// `content` in a box drawn with `style`.
fn boxed(style: Style, content: &str) -> String {
    let (columns, rows) = (usize::from(BOX.0) - 2, usize::from(BOX.1) - 2);
    style.render(&center(content, columns, rows))
}

impl tea::Model for MainModel {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        // Start the timer and the spinner when the program starts.
        tea::batch([
            self.timer.init().map(Msg::Timer),
            self.spinner.tick().map(Msg::Spinner),
        ])
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(k) => self.key(&k.to_string()),
            tea::Msg::App(Msg::Spinner(tick)) => {
                self.spinner.update(&tea::Msg::App(tick)).map(Msg::Spinner)
            }
            tea::Msg::App(Msg::Timer(tick)) => {
                self.timer.update(&tea::Msg::App(tick)).map(Msg::Timer)
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let timer = format!("{:>4}", self.timer.view());
        let (timer_style, spinner_style) = match self.state {
            SessionState::Timer => (FOCUSED_MODEL_STYLE, MODEL_STYLE),
            SessionState::Spinner => (MODEL_STYLE, FOCUSED_MODEL_STYLE),
        };
        let mut s = join_horizontal(
            Position::TOP,
            &[
                &boxed(timer_style, &timer),
                &boxed(spinner_style, &self.spinner.view()),
            ],
        );
        s += &HELP_STYLE.render(&format!(
            "\ntab: focus next • n: new {} • q: exit\n",
            self.current_focused_model()
        ));
        tea::View::new(s)
    }
}

// `pub(crate)`: `tests/bubbles_examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(new_model(DEFAULT_TIME)).run()?;
    Ok(())
}
