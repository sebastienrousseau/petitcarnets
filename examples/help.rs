//! Key bindings with short and full help, `?` to switch: Bubble Tea's
//! `help` example.
//!
//! `cargo run --example help`

use petitcarnets as tea;
use tea::bubbles::help::{self, KeyMap};
use tea::bubbles::key::Binding;
use tea::style::{Color, Style};

/// The example's keys.
#[derive(Debug, Clone)]
pub(crate) struct Keys {
    up: Binding,
    down: Binding,
    left: Binding,
    right: Binding,
    help: Binding,
    quit: Binding,
}

impl KeyMap for Keys {
    fn short_help(&self) -> Vec<Binding> {
        vec![self.help.clone(), self.quit.clone()]
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        vec![
            vec![
                self.up.clone(),
                self.down.clone(),
                self.left.clone(),
                self.right.clone(),
            ],
            vec![self.help.clone(), self.quit.clone()],
        ]
    }
}

fn keys() -> Keys {
    Keys {
        up: Binding::new(&["up", "k"]).with_help("↑/k", "move up"),
        down: Binding::new(&["down", "j"]).with_help("↓/j", "move down"),
        left: Binding::new(&["left", "h"]).with_help("←/h", "move left"),
        right: Binding::new(&["right", "l"]).with_help("→/l", "move right"),
        help: Binding::new(&["?"]).with_help("?", "toggle help"),
        quit: Binding::new(&["q", "esc", "ctrl+c"]).with_help("q", "quit"),
    }
}

pub(crate) struct Model {
    pub(crate) keys: Keys,
    pub(crate) help: help::Model,
    pub(crate) input_style: Style,
    pub(crate) last_key: &'static str,
    pub(crate) quitting: bool,
}

pub(crate) fn new_model() -> Model {
    Model {
        keys: keys(),
        help: help::Model::new(),
        input_style: Style::new().fg(Color::Rgb(0xFF, 0x75, 0xB7)),
        last_key: "",
        quitting: false,
    }
}

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match &msg {
            tea::Msg::WindowSize(size) => self.help.set_width(usize::from(size.width)),
            tea::Msg::KeyPress(k) => {
                let arrows = [
                    (&self.keys.up, "↑"),
                    (&self.keys.down, "↓"),
                    (&self.keys.left, "←"),
                    (&self.keys.right, "→"),
                ];
                if let Some((_, arrow)) = arrows.iter().find(|(b, _)| b.matches(k)) {
                    self.last_key = arrow;
                } else if self.keys.help.matches(k) {
                    self.help.show_all = !self.help.show_all;
                } else if self.keys.quit.matches(k) {
                    self.quitting = true;
                    return tea::quit();
                }
            }
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        if self.quitting {
            return tea::View::new("Bye!\n");
        }
        let status = if self.last_key.is_empty() {
            "Waiting for input...".to_string()
        } else {
            format!("You chose: {}", self.input_style.paint(self.last_key))
        };
        let help_view = self.help.view(&self.keys);
        let lines = status.matches('\n').count() + help_view.matches('\n').count();
        let height = 8_usize.saturating_sub(lines);
        tea::View::new(format!("{status}{}{help_view}", "\n".repeat(height)))
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(new_model()).run()?;
    Ok(())
}
