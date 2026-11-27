//! The smallest program: a number you change with the arrows.
//!
//! `cargo run --example counter`

use petitcarnets as tea;
use tea::style::Style;

pub(crate) struct Counter(pub(crate) i64);

impl tea::Model for Counter {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        let tea::Msg::KeyPress(key) = msg else {
            return tea::Cmd::none();
        };
        match key.to_string().as_str() {
            "up" | "+" | "k" => self.0 += 1,
            "down" | "-" | "j" => self.0 -= 1,
            "q" | "esc" | "ctrl+c" => return tea::quit(),
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let count = Style::new().bold().paint(&self.0.to_string());
        let help = Style::new()
            .dim()
            .paint("up/down or +/- to change, q to quit");
        // Full window, as before views chose their screen.
        tea::View::new(format!("\n  Count: {count}\n\n  {help}")).with_alt_screen(true)
    }
}

// `pub(crate)`: `tests/golden.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let counter = tea::Program::new(Counter(0)).run()?;
    println!("Final count: {}", counter.0);
    Ok(())
}
