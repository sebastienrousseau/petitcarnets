//! Choose one item from a list with the arrows and Enter.
//!
//! `cargo run --example picker`

use petitcarnets::style::{Color, Style};
use petitcarnets::{Carnet, Cmd, Event, Key, Model};

struct Picker {
    items: Vec<&'static str>,
    cursor: usize,
    chosen: Option<usize>,
}

impl Model for Picker {
    type Msg = ();

    fn update(&mut self, event: Event<()>) -> Cmd<()> {
        let last = self.items.len().saturating_sub(1);
        match event {
            Event::Key(Key::Up | Key::Char('k')) => self.cursor = self.cursor.saturating_sub(1),
            Event::Key(Key::Down | Key::Char('j')) => self.cursor = (self.cursor + 1).min(last),
            Event::Key(Key::Home) => self.cursor = 0,
            Event::Key(Key::End) => self.cursor = last,
            Event::Key(Key::Enter) => {
                self.chosen = Some(self.cursor);
                return Cmd::quit();
            }
            Event::Key(Key::Char('q') | Key::Esc) => return Cmd::quit(),
            _ => {}
        }
        Cmd::none()
    }

    fn view(&self) -> String {
        let title = Style::new().bold().paint("Choose an allocation method");
        let selected = Style::new().fg(Color::CYAN).bold();
        let rows: Vec<String> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if i == self.cursor {
                    selected.paint(&format!("> {item}"))
                } else {
                    format!("  {item}")
                }
            })
            .collect();
        let help = Style::new()
            .dim()
            .paint("up/down to move, Enter to choose, q to quit");
        format!("\n  {title}\n\n  {}\n\n  {help}", rows.join("\n  "))
    }
}

fn main() -> std::io::Result<()> {
    let items = vec![
        "equal-weight",
        "min-variance",
        "mean-variance",
        "risk-parity",
        "hrp",
    ];
    let picker = Carnet::new(Picker {
        items,
        cursor: 1,
        chosen: None,
    })
    .run()?;
    match picker.chosen {
        Some(i) => println!("You chose {}", picker.items[i]),
        None => println!("Nothing chosen"),
    }
    Ok(())
}
