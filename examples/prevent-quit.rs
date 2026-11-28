//! Asking before quitting with unsaved changes: Bubble Tea's
//! `prevent-quit` example. Go stops its quit message with a
//! `tea.WithFilter` function; petitcarnets has no filter, so the model
//! asks itself before it returns `tea::quit()`, which shows the same
//! screens for the same keys.
//!
//! `cargo run --example prevent-quit`

use petitcarnets as tea;
use tea::bubbles::help;
use tea::bubbles::key::Binding;
use tea::bubbles::textarea;
use tea::style::{Border, Color, Position, Style, join_horizontal};

const CHOICE_STYLE: Style = Style::new().padding_left(1).fg(Color::Palette(241));
const SAVE_TEXT_STYLE: Style = Style::new().fg(Color::Palette(170));
const QUIT_VIEW_STYLE: Style = Style::new()
    .padding(1, 3)
    .border(Border::ROUNDED)
    .border_fg(Color::Palette(170));

pub(crate) struct Keymap {
    save: Binding,
    quit: Binding,
}

pub(crate) struct Model {
    pub(crate) textarea: textarea::Model,
    pub(crate) help: help::Model,
    pub(crate) keymap: Keymap,
    pub(crate) save_text: String,
    pub(crate) has_changes: bool,
    pub(crate) quitting: bool,
}

pub(crate) fn initial_model() -> Model {
    let mut ti = textarea::Model::new();
    ti.placeholder = "Only the best words".into();
    let _ = ti.focus();
    Model {
        textarea: ti,
        help: help::Model::new(),
        keymap: Keymap {
            save: Binding::new(&["ctrl+s"]).with_help("ctrl+s", "save"),
            quit: Binding::new(&["esc", "ctrl+c"]).with_help("esc", "quit"),
        },
        save_text: String::new(),
        has_changes: false,
        quitting: false,
    }
}

impl Model {
    /// Go's filter: a quit with unsaved changes asks first.
    fn quit(&mut self) -> tea::Cmd<textarea::Msg> {
        self.quitting = true;
        if self.has_changes {
            tea::Cmd::none()
        } else {
            tea::quit()
        }
    }

    fn update_text_view(&mut self, msg: &tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
        let mut cmds = Vec::new();
        if let tea::Msg::KeyPress(k) = msg {
            self.save_text.clear();
            if self.keymap.save.matches(k) {
                self.save_text = "Changes saved!".into();
                self.has_changes = false;
            } else if self.keymap.quit.matches(k) {
                return self.quit();
            } else {
                if !k.text.is_empty() {
                    self.has_changes = true;
                }
                if !self.textarea.focused() {
                    cmds.push(self.textarea.focus());
                }
            }
        }
        cmds.push(self.textarea.update(msg));
        tea::batch(cmds)
    }

    fn update_prompt_view(&mut self, msg: &tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
        if let tea::Msg::KeyPress(k) = msg {
            // For simplicity's sake, any key besides "y" is "no".
            if self.keymap.quit.matches(k) || *k == "y" {
                self.has_changes = false;
                return tea::quit();
            }
            self.quitting = false;
        }
        tea::Cmd::none()
    }
}

impl tea::Model for Model {
    type Msg = textarea::Msg;

    fn init(&mut self) -> tea::Cmd<textarea::Msg> {
        textarea::blink()
    }

    fn update(&mut self, msg: tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
        if self.quitting {
            return self.update_prompt_view(&msg);
        }
        self.update_text_view(&msg)
    }

    fn view(&self) -> tea::View {
        if self.quitting {
            if self.has_changes {
                let text = join_horizontal(
                    Position::TOP,
                    &[
                        "You have unsaved changes. Quit without saving?",
                        &CHOICE_STYLE.render("[yN]"),
                    ],
                );
                return tea::View::new(QUIT_VIEW_STYLE.render(&text));
            }
            return tea::View::new("Very important. Thank you.\n");
        }
        let help_view = self
            .help
            .short_help_view(&[self.keymap.save.clone(), self.keymap.quit.clone()]);
        tea::View::new(format!(
            "Type some important things.\n{}\n {}\n {}\n\n",
            self.textarea.view(),
            SAVE_TEXT_STYLE.render(&self.save_text),
            help_view
        ))
    }
}

// `pub(crate)`: `tests/bubbles_examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
