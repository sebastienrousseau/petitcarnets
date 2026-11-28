//! A simple program that handles losing and gaining focus: Bubble Tea's
//! `focus-blur` example. t turns focus reports on and off.
//!
//! `cargo run --example focus-blur`

use petitcarnets as tea;

pub(crate) struct Model {
    pub(crate) focused: bool,
    pub(crate) reporting: bool,
}

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::Focus => self.focused = true,
            tea::Msg::Blur => self.focused = false,
            tea::Msg::KeyPress(k) if k == "t" => self.reporting = !self.reporting,
            tea::Msg::KeyPress(k) if k == "ctrl+c" || k == "q" => return tea::quit(),
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let mut s = String::from("Hi. Focus report is currently ");
        s += if self.reporting {
            "enabled"
        } else {
            "disabled"
        };
        s += ".\n\n";
        if self.reporting {
            s += if self.focused {
                "This program is currently focused!"
            } else {
                "This program is currently blurred!"
            };
        }
        s += "\n\nTo quit sooner press ctrl-c, or t to toggle focus reporting...\n";
        tea::View::new(s).with_report_focus(self.reporting)
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let model = Model {
        focused: true,
        reporting: true,
    };
    let _ = tea::Program::new(model).run()?;
    Ok(())
}
