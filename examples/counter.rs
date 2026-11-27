//! The smallest program: a number you change with the arrows.
//!
//! `cargo run --example counter`

use petitcarnets::style::Style;
use petitcarnets::{Carnet, Cmd, Event, Key, Model};

struct Counter(i64);

impl Model for Counter {
    type Msg = ();

    fn update(&mut self, event: Event<()>) -> Cmd<()> {
        match event {
            Event::Key(Key::Up | Key::Char('+' | 'k')) => self.0 += 1,
            Event::Key(Key::Down | Key::Char('-' | 'j')) => self.0 -= 1,
            Event::Key(Key::Char('q') | Key::Esc) => return Cmd::quit(),
            _ => {}
        }
        Cmd::none()
    }

    fn view(&self) -> String {
        let count = Style::new().bold().paint(&self.0.to_string());
        let help = Style::new()
            .dim()
            .paint("up/down or +/- to change, q to quit");
        format!("\n  Count: {count}\n\n  {help}")
    }
}

fn main() -> std::io::Result<()> {
    let counter = Carnet::new(Counter(0)).run()?;
    println!("Final count: {}", counter.0);
    Ok(())
}
