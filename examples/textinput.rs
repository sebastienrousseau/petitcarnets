//! One text input, with the terminal's own cursor: Bubble Tea's
//! `textinput` example.
//!
//! `cargo run --example textinput`

use petitcarnets as tea;
use tea::bubbles::textinput;
use tea::style::{Position, height, join_vertical};

pub(crate) struct Model {
    pub(crate) text_input: textinput::Model,
    pub(crate) quitting: bool,
}

pub(crate) fn initial_model() -> Model {
    let mut ti = textinput::Model::new();
    ti.placeholder = "Pikachu".into();
    ti.set_virtual_cursor(false);
    let _ = ti.focus();
    ti.char_limit = 156;
    ti.set_width(20);
    Model {
        text_input: ti,
        quitting: false,
    }
}

impl tea::Model for Model {
    type Msg = textinput::Msg;

    fn init(&mut self) -> tea::Cmd<textinput::Msg> {
        textinput::blink()
    }

    fn update(&mut self, msg: tea::Msg<textinput::Msg>) -> tea::Cmd<textinput::Msg> {
        if let tea::Msg::KeyPress(k) = &msg {
            if *k == "enter" || *k == "ctrl+c" || *k == "esc" {
                self.quitting = true;
                return tea::quit();
            }
        }
        self.text_input.update(&msg)
    }

    fn view(&self) -> tea::View {
        let header = "What’s your favorite Pokémon?\n";
        let mut s = join_vertical(
            Position::TOP,
            &[header, &self.text_input.view(), "\n(esc to quit)"],
        );
        if self.quitting {
            s.push('\n');
        }
        let mut v = tea::View::new(s);
        if let Some(mut c) = self.text_input.cursor() {
            c.y += u16::try_from(height(header)).unwrap_or(0);
            v = v.with_cursor(c);
        }
        v
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
