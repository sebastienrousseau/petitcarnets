//! Sending messages to a program from outside it, with
//! `Program::sender`: Bubble Tea's `send-msg` example (Go's
//! `Program.Send`). A thread pretends to eat food at random intervals
//! and sends each meal to the program.
//!
//! `cargo run --example send-msg`

use petitcarnets as tea;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tea::bubbles::spinner;
use tea::style::{Color, Style};

const SPINNER_STYLE: Style = Style::new().fg(Color::Palette(63));
const HELP_STYLE: Style = Style::new().fg(Color::Palette(241)).margin(1, 0);
const DOT_STYLE: Style = Style::new().fg(Color::Palette(241));
const APP_STYLE: Style = Style::new().margin_sides(1, 2, 0, 2);

const FOOD: [&str; 13] = [
    "an apple",
    "a pear",
    "a gherkin",
    "a party gherkin",
    "a kohlrabi",
    "some spaghetti",
    "tacos",
    "a currywurst",
    "some curry",
    "a sandwich",
    "some peanut butter",
    "some cashews",
    "some ramen",
];

/// A meal: what was eaten, and how long it took.
#[derive(Clone, Default)]
pub(crate) struct ResultMsg {
    pub(crate) duration: Duration,
    pub(crate) food: &'static str,
}

impl ResultMsg {
    fn view(&self) -> String {
        if self.duration.is_zero() {
            return DOT_STYLE.render(&".".repeat(30));
        }
        let ms = format!("{}ms", self.duration.as_millis());
        format!("🍔 Ate {} {}", self.food, DOT_STYLE.render(&ms))
    }
}

pub(crate) enum Msg {
    Result(ResultMsg),
    Spinner(spinner::TickMsg),
}

pub(crate) struct Model {
    pub(crate) spinner: spinner::Model,
    pub(crate) results: Vec<ResultMsg>,
    pub(crate) quitting: bool,
}

pub(crate) fn new_model() -> Model {
    const NUM_LAST_RESULTS: usize = 5;
    Model {
        spinner: spinner::Model::new().with_style(SPINNER_STYLE),
        results: vec![ResultMsg::default(); NUM_LAST_RESULTS],
        quitting: false,
    }
}

impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        self.spinner.tick().map(Msg::Spinner)
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(_) => {
                self.quitting = true;
                tea::quit()
            }
            tea::Msg::App(Msg::Result(result)) => {
                let _ = self.results.remove(0);
                self.results.push(result);
                tea::Cmd::none()
            }
            tea::Msg::App(Msg::Spinner(tick)) => {
                self.spinner.update(&tea::Msg::App(tick)).map(Msg::Spinner)
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let mut b = String::new();
        if self.quitting {
            b += "That's all for today!";
        } else {
            b += &self.spinner.view();
            b += " Eating food...";
        }
        b += "\n\n";
        for res in &self.results {
            b += &res.view();
            b += "\n";
        }
        if self.quitting {
            b += "\n";
        } else {
            b += &HELP_STYLE.render("Press any key to exit");
        }
        tea::View::new(APP_STYLE.render(&b))
    }
}

/// A small random number generator (xorshift), seeded from the clock:
/// the example needs no more than this.
fn random(state: &mut u64, below: u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state % below
}

// `pub(crate)`: `tests/bubbles_examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let program = tea::Program::new(new_model());
    let sender = program.sender();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut seed = u64::from(nanos.subsec_nanos()) | 1;
    // Simulate activity.
    let _ = std::thread::spawn(move || {
        loop {
            let pause = Duration::from_millis(random(&mut seed, 899) + 100);
            std::thread::sleep(pause);
            let food = FOOD[usize::try_from(random(&mut seed, 13)).unwrap_or(0)];
            sender.send(Msg::Result(ResultMsg {
                duration: pause,
                food,
            }));
        }
    });
    let _ = program.run()?;
    Ok(())
}
