//! Several text inputs, a password among them, Tab to move between
//! them and Ctrl-R to change the cursor's mode: Bubble Tea's
//! `textinputs` example.
//!
//! `cargo run --example textinputs`

use petitcarnets as tea;
use tea::bubbles::cursor::Mode;
use tea::bubbles::textinput::{self, EchoMode};
use tea::style::{Color, Style};

const FOCUSED: Style = Style::new().fg(Color::Palette(205));
const BLURRED: Style = Style::new().fg(Color::Palette(240));
const CURSOR_MODE_HELP: Style = Style::new().fg(Color::Palette(244));

pub(crate) struct Model {
    pub(crate) focus_index: usize,
    pub(crate) inputs: Vec<textinput::Model>,
    pub(crate) cursor_mode: Mode,
    pub(crate) quitting: bool,
}

pub(crate) fn initial_model() -> Model {
    let mut inputs = Vec::new();
    for (i, placeholder) in ["Nickname", "Email", "Password"].into_iter().enumerate() {
        let mut t = textinput::Model::new();
        t.char_limit = 32;
        let mut s = t.styles();
        s.cursor.color = Color::Palette(205);
        s.focused.prompt = FOCUSED;
        s.focused.text = FOCUSED;
        s.blurred.prompt = BLURRED;
        t.set_styles(s);
        t.placeholder = placeholder.into();
        match i {
            0 => {
                let _ = t.focus();
            }
            1 => t.char_limit = 64,
            _ => {
                t.echo_mode = EchoMode::Password;
                t.echo_character = '•';
            }
        }
        inputs.push(t);
    }
    Model {
        focus_index: 0,
        inputs,
        cursor_mode: Mode::Blink,
        quitting: false,
    }
}

impl Model {
    /// The next cursor mode, round again after hidden.
    fn next_mode(&mut self) {
        self.cursor_mode = match self.cursor_mode {
            Mode::Blink => Mode::Static,
            Mode::Static => Mode::Hide,
            Mode::Hide => Mode::Blink,
        };
        for input in &mut self.inputs {
            let mut s = input.styles();
            s.cursor.blink = self.cursor_mode == Mode::Blink;
            input.set_styles(s);
        }
    }

    /// Moves the focus up or down, round the inputs and the button.
    fn move_focus(&mut self, up: bool) -> tea::Cmd<textinput::Msg> {
        let n = self.inputs.len() + 1;
        self.focus_index = if up {
            (self.focus_index + n - 1) % n
        } else {
            (self.focus_index + 1) % n
        };
        let mut cmds = Vec::new();
        for (i, input) in self.inputs.iter_mut().enumerate() {
            if i == self.focus_index {
                cmds.push(input.focus());
            } else {
                input.blur();
            }
        }
        tea::batch(cmds)
    }
}

impl tea::Model for Model {
    type Msg = textinput::Msg;

    fn init(&mut self) -> tea::Cmd<textinput::Msg> {
        textinput::blink()
    }

    fn update(&mut self, msg: tea::Msg<textinput::Msg>) -> tea::Cmd<textinput::Msg> {
        if let tea::Msg::KeyPress(k) = &msg {
            let key = k.to_string();
            match key.as_str() {
                "ctrl+c" | "esc" => {
                    self.quitting = true;
                    return tea::quit();
                }
                "ctrl+r" => {
                    self.next_mode();
                    return tea::Cmd::none();
                }
                "enter" if self.focus_index == self.inputs.len() => return tea::quit(),
                "tab" | "shift+tab" | "enter" | "up" | "down" => {
                    return self.move_focus(key == "up" || key == "shift+tab");
                }
                _ => {}
            }
        }
        tea::batch(
            self.inputs
                .iter_mut()
                .map(|i| i.update(&msg))
                .collect::<Vec<_>>(),
        )
    }

    fn view(&self) -> tea::View {
        let views: Vec<String> = self.inputs.iter().map(textinput::Model::view).collect();
        let mut b = views.join("\n");
        let button = if self.focus_index == self.inputs.len() {
            FOCUSED.paint("[ Submit ]")
        } else {
            format!("[ {} ]", BLURRED.paint("Submit"))
        };
        b.push_str("\n\n");
        b.push_str(&button);
        b.push_str("\n\n");
        b += &BLURRED.paint("cursor mode is ");
        b += &CURSOR_MODE_HELP.paint(&self.cursor_mode.to_string());
        b += &BLURRED.paint(" (ctrl+r to change style)");
        if self.quitting {
            b.push('\n');
        }
        tea::View::new(b)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
