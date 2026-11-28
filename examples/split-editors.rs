//! Several text areas side by side: Bubble Tea's `split-editors`
//! example. Tab and shift+tab move between them, ctrl+n adds one (up to
//! six), ctrl+w removes the last one (down to one), esc quits. Unlike
//! Go's, removing the focused editor moves the focus to the new last
//! one instead of leaving none focused.
//!
//! `cargo run --example split-editors`

use petitcarnets as tea;
use tea::bubbles::help;
use tea::bubbles::key::Binding;
use tea::bubbles::textarea;
use tea::style::{Border, Color, Position, Style, join_horizontal};

const INITIAL_INPUTS: usize = 2;
const MAX_INPUTS: usize = 6;
const MIN_INPUTS: usize = 1;
const HELP_HEIGHT: usize = 5;

const CURSOR_COLOR: Color = Color::Palette(212);
const CURSOR_LINE_STYLE: Style = Style::new().bg(Color::Palette(57)).fg(Color::Palette(230));
const PLACEHOLDER_STYLE: Style = Style::new().fg(Color::Palette(238));
const FOCUSED_PLACEHOLDER_STYLE: Style = Style::new().fg(Color::Palette(99));
const FOCUSED_BORDER_STYLE: Style = Style::new()
    .border(Border::ROUNDED)
    .border_fg(Color::Palette(238));
const BLURRED_BORDER_STYLE: Style = Style::new().border(Border::HIDDEN);
const END_OF_BUFFER_STYLE: Style = Style::new().fg(Color::Palette(235));

pub(crate) struct Keymap {
    pub(crate) next: Binding,
    pub(crate) prev: Binding,
    pub(crate) add: Binding,
    pub(crate) remove: Binding,
    pub(crate) quit: Binding,
}

fn new_textarea() -> textarea::Model {
    let mut t = textarea::Model::new();
    t.prompt = String::new();
    t.placeholder = "Type something".into();
    t.show_line_numbers = true;
    t.set_virtual_cursor(true);
    let mut s = t.styles();
    s.cursor.color = CURSOR_COLOR;
    s.focused.placeholder = FOCUSED_PLACEHOLDER_STYLE;
    s.blurred.placeholder = PLACEHOLDER_STYLE;
    s.focused.cursor_line = CURSOR_LINE_STYLE;
    s.focused.cursor_line_number = CURSOR_LINE_STYLE;
    s.focused.base = FOCUSED_BORDER_STYLE;
    s.blurred.base = BLURRED_BORDER_STYLE;
    s.focused.end_of_buffer = END_OF_BUFFER_STYLE;
    s.blurred.end_of_buffer = END_OF_BUFFER_STYLE;
    t.set_styles(s);
    t.key_map.delete_word_backward.set_enabled(false);
    t.key_map.line_next = Binding::new(&["down"]);
    t.key_map.line_previous = Binding::new(&["up"]);
    t.blur();
    t
}

pub(crate) struct Model {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) keymap: Keymap,
    pub(crate) help: help::Model,
    pub(crate) inputs: Vec<textarea::Model>,
    pub(crate) focus: usize,
}

pub(crate) fn new_model() -> Model {
    let mut m = Model {
        width: 0,
        height: 0,
        inputs: (0..INITIAL_INPUTS).map(|_| new_textarea()).collect(),
        help: help::Model::new(),
        keymap: Keymap {
            next: Binding::new(&["tab"]).with_help("tab", "next"),
            prev: Binding::new(&["shift+tab"]).with_help("shift+tab", "prev"),
            add: Binding::new(&["ctrl+n"]).with_help("ctrl+n", "add an editor"),
            remove: Binding::new(&["ctrl+w"]).with_help("ctrl+w", "remove an editor"),
            quit: Binding::new(&["esc", "ctrl+c"]).with_help("esc", "quit"),
        },
        focus: 0,
    };
    let _ = m.inputs[m.focus].focus();
    m.update_keybindings();
    m
}

impl Model {
    /// Moves the focus to the editor `to`.
    fn move_focus(&mut self, to: usize) -> tea::Cmd<textarea::Msg> {
        self.inputs[self.focus].blur();
        self.focus = to;
        self.inputs[self.focus].focus()
    }

    fn key(&mut self, k: &tea::Key) -> Option<tea::Cmd<textarea::Msg>> {
        let n = self.inputs.len();
        if self.keymap.quit.matches(k) {
            self.inputs.iter_mut().for_each(textarea::Model::blur);
            return None;
        }
        let cmd = if self.keymap.next.matches(k) {
            self.move_focus((self.focus + 1) % n)
        } else if self.keymap.prev.matches(k) {
            self.move_focus((self.focus + n - 1) % n)
        } else if self.keymap.add.matches(k) {
            self.inputs.push(new_textarea());
            tea::Cmd::none()
        } else if self.keymap.remove.matches(k) {
            let _ = self.inputs.pop();
            self.focus = self.focus.min(self.inputs.len() - 1);
            // Go leaves no editor focused when the focused one goes.
            self.inputs[self.focus].focus()
        } else {
            tea::Cmd::none()
        };
        Some(cmd)
    }

    fn size_inputs(&mut self) {
        let n = self.inputs.len();
        for input in &mut self.inputs {
            input.set_width(self.width / n);
            input.set_height(self.height.saturating_sub(HELP_HEIGHT));
        }
    }

    fn update_keybindings(&mut self) {
        self.keymap.add.set_enabled(self.inputs.len() < MAX_INPUTS);
        self.keymap
            .remove
            .set_enabled(self.inputs.len() > MIN_INPUTS);
    }
}

impl tea::Model for Model {
    type Msg = textarea::Msg;

    fn init(&mut self) -> tea::Cmd<textarea::Msg> {
        textarea::blink()
    }

    fn update(&mut self, msg: tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
        let mut cmds = Vec::new();
        match &msg {
            tea::Msg::KeyPress(k) => match self.key(k) {
                Some(cmd) => cmds.push(cmd),
                None => return tea::quit(),
            },
            tea::Msg::WindowSize(size) => {
                self.height = usize::from(size.height);
                self.width = usize::from(size.width);
            }
            _ => {}
        }
        self.update_keybindings();
        self.size_inputs();
        // Update all text areas.
        for input in &mut self.inputs {
            cmds.push(input.update(&msg));
        }
        tea::batch(cmds)
    }

    fn view(&self) -> tea::View {
        let help = self.help.short_help_view(&[
            self.keymap.next.clone(),
            self.keymap.prev.clone(),
            self.keymap.add.clone(),
            self.keymap.remove.clone(),
            self.keymap.quit.clone(),
        ]);
        let views: Vec<String> = self.inputs.iter().map(textarea::Model::view).collect();
        let views: Vec<&str> = views.iter().map(String::as_str).collect();
        let content = join_horizontal(Position::TOP, &views) + "\n\n" + &help;
        tea::View::new(content).with_alt_screen(true)
    }
}

// `pub(crate)`: `tests/bubbles_examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(new_model()).run()?;
    Ok(())
}
