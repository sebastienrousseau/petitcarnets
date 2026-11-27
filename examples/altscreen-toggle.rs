//! Switching between the alternate screen and inline drawing with the
//! space bar, as Bubble Tea's `altscreen-toggle` example does.
//!
//! `cargo run --example altscreen-toggle`

use petitcarnets as tea;
use tea::style::{Color, Style};

#[derive(Default)]
pub(crate) struct Toggle {
    alt_screen: bool,
    quitting: bool,
}

impl tea::Model for Toggle {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        let tea::Msg::KeyPress(key) = msg else {
            return tea::Cmd::none();
        };
        match key.to_string().as_str() {
            "q" | "esc" | "ctrl+c" => {
                self.quitting = true;
                return tea::quit();
            }
            "space" => self.alt_screen = !self.alt_screen,
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        if self.quitting {
            return tea::View::new("Bye!\n").with_alt_screen(self.alt_screen);
        }
        let mode = if self.alt_screen {
            " altscreen mode "
        } else {
            " inline mode "
        };
        let keyword = Style::new().fg(Color::Palette(204)).bg(Color::Palette(235));
        let help = Style::new().fg(Color::Palette(241));
        let content = format!(
            "\n\n  You're in {}\n\n\n{}",
            keyword.paint(mode),
            help.paint("  space: switch modes • q: exit\n")
        );
        tea::View::new(content).with_alt_screen(self.alt_screen)
    }
}

// `pub(crate)`: `tests/golden.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Toggle::default()).run()?;
    Ok(())
}
