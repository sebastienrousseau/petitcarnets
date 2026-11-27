//! Choose one item from a list with the arrows and Enter.
//!
//! `cargo run --example picker`

use petitcarnets as tea;
use tea::style::{Color, Style};

pub(crate) struct Picker {
    pub(crate) items: Vec<&'static str>,
    pub(crate) cursor: usize,
    pub(crate) chosen: Option<usize>,
}

impl tea::Model for Picker {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        let tea::Msg::KeyPress(key) = msg else {
            return tea::Cmd::none();
        };
        let last = self.items.len().saturating_sub(1);
        match key.to_string().as_str() {
            "up" | "k" => self.cursor = self.cursor.saturating_sub(1),
            "down" | "j" => self.cursor = (self.cursor + 1).min(last),
            "home" => self.cursor = 0,
            "end" => self.cursor = last,
            "enter" => {
                self.chosen = Some(self.cursor);
                return tea::quit();
            }
            "q" | "esc" | "ctrl+c" => return tea::quit(),
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
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
            .paint("up/down to move, enter to choose, q to quit");
        let content = format!("\n  {title}\n\n  {}\n\n  {help}", rows.join("\n  "));
        tea::View::new(content).with_alt_screen(true)
    }
}

// `pub(crate)`: `tests/golden.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let items = vec![
        "equal-weight",
        "min-variance",
        "mean-variance",
        "risk-parity",
        "hrp",
    ];
    let picker = tea::Program::new(Picker {
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
