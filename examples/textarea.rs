//! A text area with the terminal's own cursor; Esc blurs it, any other
//! key focuses it again: Bubble Tea's `textarea` example.
//!
//! `cargo run --example textarea`

use petitcarnets as tea;
use tea::bubbles::textarea::{self, Styles};
use tea::style::height;

pub(crate) struct Model {
    pub(crate) textarea: textarea::Model,
}

pub(crate) fn initial_model() -> Model {
    let mut ti = textarea::Model::new();
    ti.placeholder = "Once upon a time...".into();
    ti.set_virtual_cursor(false);
    ti.set_styles(Styles::new(true));
    let _ = ti.focus();
    Model { textarea: ti }
}

const HEADER: &str = "Tell me a story.\n";

impl tea::Model for Model {
    type Msg = textarea::Msg;

    fn init(&mut self) -> tea::Cmd<textarea::Msg> {
        tea::batch([textarea::blink(), tea::request_background_color()])
    }

    fn update(&mut self, msg: tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
        let mut cmds = Vec::new();
        match &msg {
            tea::Msg::BackgroundColor(c) => self.textarea.set_styles(Styles::new(c.is_dark())),
            tea::Msg::KeyPress(k) if *k == "esc" => {
                if self.textarea.focused() {
                    self.textarea.blur();
                }
            }
            tea::Msg::KeyPress(k) if *k == "ctrl+c" => return tea::quit(),
            tea::Msg::KeyPress(_) if !self.textarea.focused() => cmds.push(self.textarea.focus()),
            _ => {}
        }
        cmds.push(self.textarea.update(&msg));
        tea::batch(cmds)
    }

    fn view(&self) -> tea::View {
        let content = [HEADER, &self.textarea.view(), "\n(ctrl+c to quit)\n"].join("\n");
        let mut v = tea::View::new(content);
        if let Some(mut c) = self.textarea.cursor() {
            c.y += u16::try_from(height(HEADER)).unwrap_or(0);
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
