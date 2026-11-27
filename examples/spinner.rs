//! A spinner that ticks while a slow task runs on another thread.
//!
//! `cargo run --example spinner`

use petitcarnets::style::{Color, Style};
use petitcarnets::{Carnet, Cmd, Event, Key, Model};
use std::time::Duration;

const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

enum Msg {
    Tick,
    Loaded(u64),
}

struct Loader {
    frame: usize,
    result: Option<u64>,
}

fn tick() -> Cmd<Msg> {
    Cmd::tick(Duration::from_millis(80), || Msg::Tick)
}

impl Model for Loader {
    type Msg = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        let slow = Cmd::task(|| {
            std::thread::sleep(Duration::from_secs(2));
            Msg::Loaded((1..=20).product())
        });
        Cmd::batch([tick(), slow])
    }

    fn update(&mut self, event: Event<Msg>) -> Cmd<Msg> {
        match event {
            Event::Msg(Msg::Tick) if self.result.is_none() => {
                self.frame = (self.frame + 1) % FRAMES.len();
                tick()
            }
            Event::Msg(Msg::Loaded(n)) => {
                self.result = Some(n);
                Cmd::none()
            }
            Event::Key(Key::Char('q') | Key::Esc) => Cmd::quit(),
            _ => Cmd::none(),
        }
    }

    fn view(&self) -> String {
        match self.result {
            None => format!(
                "\n  {} Computing 20!...",
                Style::new().fg(Color::MAGENTA).paint(FRAMES[self.frame])
            ),
            Some(n) => format!(
                "\n  Done: 20! = {}\n\n  {}",
                Style::new().bold().paint(&n.to_string()),
                Style::new().dim().paint("q to quit")
            ),
        }
    }
}

fn main() -> std::io::Result<()> {
    let _ = Carnet::new(Loader {
        frame: 0,
        result: None,
    })
    .run()?;
    Ok(())
}
