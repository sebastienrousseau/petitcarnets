//! A chat room: messages scroll in a viewport above a text area; Enter
//! sends. Bubble Tea's `chat` example.
//!
//! `cargo run --example chat`

use petitcarnets as tea;
use tea::bubbles::{textarea, viewport};
use tea::style::{Color, Style, height};

pub(crate) struct Model {
    pub(crate) viewport: viewport::Model,
    pub(crate) messages: Vec<String>,
    pub(crate) textarea: textarea::Model,
    pub(crate) sender_style: Style,
}

pub(crate) fn initial_model() -> Model {
    let mut ta = textarea::Model::new();
    ta.placeholder = "Send a message...".into();
    ta.set_virtual_cursor(false);
    let _ = ta.focus();
    ta.prompt = "┃ ".into();
    ta.char_limit = 280;
    ta.set_width(30);
    ta.set_height(3);
    let mut s = ta.styles();
    s.focused.cursor_line = Style::new();
    ta.set_styles(s);
    ta.show_line_numbers = false;
    let mut vp = viewport::Model::new(30, 5);
    vp.set_content("Welcome to the chat room!\nType a message and press Enter to send.");
    vp.key_map.left.set_enabled(false);
    vp.key_map.right.set_enabled(false);
    ta.key_map.insert_newline.set_enabled(false);
    Model {
        viewport: vp,
        messages: Vec::new(),
        textarea: ta,
        sender_style: Style::new().fg(Color::Ansi(5)),
    }
}

impl Model {
    /// The messages, wrapped at the viewport's width.
    fn show_messages(&mut self) {
        let width = u16::try_from(self.viewport.width()).unwrap_or(u16::MAX);
        self.viewport
            .set_content(&Style::new().width(width).render(&self.messages.join("\n")));
    }
}

impl tea::Model for Model {
    type Msg = textarea::Msg;

    fn init(&mut self) -> tea::Cmd<textarea::Msg> {
        textarea::blink()
    }

    fn update(&mut self, msg: tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
        match &msg {
            tea::Msg::WindowSize(size) => {
                self.viewport.set_width(usize::from(size.width));
                self.textarea.set_width(usize::from(size.width));
                self.viewport
                    .set_height(usize::from(size.height).saturating_sub(self.textarea.height()));
                if !self.messages.is_empty() {
                    self.show_messages();
                }
                self.viewport.goto_bottom();
                tea::Cmd::none()
            }
            tea::Msg::KeyPress(k) if *k == "ctrl+c" || *k == "esc" => tea::quit(),
            tea::Msg::KeyPress(k) if *k == "enter" => {
                let line = self.sender_style.paint("You: ") + &self.textarea.value();
                self.messages.push(line);
                self.show_messages();
                self.textarea.reset();
                self.viewport.goto_bottom();
                tea::Cmd::none()
            }
            tea::Msg::KeyPress(_) | tea::Msg::App(_) => self.textarea.update(&msg),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let viewport = self.viewport.view();
        let mut v =
            tea::View::new(format!("{viewport}\n{}", self.textarea.view())).with_alt_screen(true);
        if let Some(mut c) = self.textarea.cursor() {
            c.y += u16::try_from(height(&viewport)).unwrap_or(0);
            v = v.with_cursor(c);
        }
        v
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let model = tea::Program::new(initial_model()).run()?;
    println!("{}", model.textarea.value());
    Ok(())
}
