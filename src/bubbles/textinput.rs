//! One line of text to edit, scrolled sideways when it is wider than
//! its width: Bubbles' `textinput` package, with its key bindings,
//! echo modes (passwords), character limit, suggestions and validation.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::textinput;
//!
//! let mut input = textinput::Model::new();
//! input.placeholder = "Pikachu".into();
//! let _ = input.focus();
//! for c in "Hi".chars() {
//!     let _ = input.update(&tea::Msg::KeyPress(tea::Key::char(c)));
//! }
//! let _ = input.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Left)));
//! assert_eq!(input.value(), "Hi");
//! assert_eq!(input.position(), 1);
//! assert_eq!(tea::style::strip_ansi(&input.view()), "> Hi");
//! ```

pub(super) mod edit;
mod view;

use super::Flags;
use super::cursor::{self, Mode};
use super::key::Binding;
use super::text::sanitize;
use crate::style::{Color, Style};
use crate::{Cmd, CursorShape, Msg as TeaMsg, batch};
use std::time::Duration;

/// A text input's messages: its cursor's blinks.
pub type Msg = cursor::Msg;

/// How the text shows: Bubbles' `textinput.EchoMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EchoMode {
    /// As typed.
    #[default]
    Normal,
    /// A [`Model::echo_character`] a cell, for passwords.
    Password,
    /// Nothing at all.
    None,
}

/// A check run on every change; its error is kept in [`Model::err`].
pub type ValidateFn = fn(&str) -> Result<(), String>;

/// The keys of a text input: Bubbles' `textinput.KeyMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// One character right.
    pub character_forward: Binding,
    /// One character left.
    pub character_backward: Binding,
    /// To the end of the word.
    pub word_forward: Binding,
    /// To the start of the word.
    pub word_backward: Binding,
    /// Delete the word before the cursor.
    pub delete_word_backward: Binding,
    /// Delete the word after the cursor.
    pub delete_word_forward: Binding,
    /// Delete everything after the cursor.
    pub delete_after_cursor: Binding,
    /// Delete everything before the cursor.
    pub delete_before_cursor: Binding,
    /// Delete the character before the cursor.
    pub delete_character_backward: Binding,
    /// Delete the character under the cursor.
    pub delete_character_forward: Binding,
    /// To the start.
    pub line_start: Binding,
    /// To the end.
    pub line_end: Binding,
    /// Paste the clipboard.
    pub paste: Binding,
    /// Take the suggestion.
    pub accept_suggestion: Binding,
    /// The next suggestion.
    pub next_suggestion: Binding,
    /// The suggestion before.
    pub prev_suggestion: Binding,
}

impl Default for KeyMap {
    /// Bubbles v2's `textinput.DefaultKeyMap`.
    fn default() -> Self {
        let b = Binding::new;
        Self {
            character_forward: b(&["right", "ctrl+f"]),
            character_backward: b(&["left", "ctrl+b"]),
            word_forward: b(&["alt+right", "ctrl+right", "alt+f"]),
            word_backward: b(&["alt+left", "ctrl+left", "alt+b"]),
            delete_word_backward: b(&["alt+backspace", "ctrl+w", "ctrl+backspace"]),
            delete_word_forward: b(&["alt+delete", "alt+d", "ctrl+delete"]),
            delete_after_cursor: b(&["ctrl+k"]),
            delete_before_cursor: b(&["ctrl+u"]),
            delete_character_backward: b(&["backspace", "shift+backspace", "ctrl+h"]),
            delete_character_forward: b(&["delete", "ctrl+d"]),
            line_start: b(&["home", "ctrl+a"]),
            line_end: b(&["end", "ctrl+e"]),
            paste: b(&["ctrl+v"]),
            accept_suggestion: b(&["tab"]),
            next_suggestion: b(&["down", "ctrl+n"]),
            prev_suggestion: b(&["up", "ctrl+p"]),
        }
    }
}

/// The styles for one state, focused or blurred: Bubbles'
/// `textinput.StyleState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StyleState {
    /// The text.
    pub text: Style,
    /// The placeholder.
    pub placeholder: Style,
    /// The rest of a suggestion.
    pub suggestion: Style,
    /// The prompt.
    pub prompt: Style,
}

/// How the cursor looks: Bubbles' `textinput.CursorStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorStyle {
    /// Its colour. A real cursor (see [`Model::set_virtual_cursor`])
    /// takes it only if it is [`Color::Rgb`].
    pub color: Color,
    /// The real cursor's shape.
    pub shape: CursorShape,
    /// Whether it blinks.
    pub blink: bool,
    /// How fast it blinks; zero keeps the cursor's default.
    pub blink_speed: Duration,
}

/// A text input's styles: Bubbles' `textinput.Styles`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Styles {
    /// When focused.
    pub focused: StyleState,
    /// When blurred.
    pub blurred: StyleState,
    /// The cursor.
    pub cursor: CursorStyle,
}

impl Styles {
    /// Bubbles' default styles for a dark or light background:
    /// `textinput.DefaultStyles`.
    #[must_use]
    pub const fn new(is_dark: bool) -> Self {
        let subdued = Style::new().fg(Color::Palette(240));
        let prompt = Style::new().fg(Color::Ansi(7));
        let state = StyleState {
            text: Style::new(),
            placeholder: subdued,
            suggestion: subdued,
            prompt,
        };
        let blurred_text = if is_dark {
            Color::Ansi(7)
        } else {
            Color::Palette(245)
        };
        Self {
            focused: state,
            blurred: StyleState {
                text: Style::new().fg(blurred_text),
                ..state
            },
            cursor: CursorStyle {
                color: Color::Ansi(7),
                shape: CursorShape::Block,
                blink: true,
                blink_speed: Duration::ZERO,
            },
        }
    }
}

/// A [`Model`] setting's bit.
const FOCUS: u16 = 1 << 0;
/// A [`Model`] setting's bit.
const USE_VIRTUAL_CURSOR: u16 = 1 << 1;
/// A [`Model`] setting's bit.
const PASTE_PENDING: u16 = 1 << 2;

/// A text input: Bubbles' `textinput.Model`.
#[derive(Debug, Clone)]
pub struct Model {
    /// The last validation error, if any.
    pub err: Option<String>,
    /// Shown before the text.
    pub prompt: String,
    /// Shown when there is no text.
    pub placeholder: String,
    /// How the text shows.
    pub echo_mode: EchoMode,
    /// The character a password shows as.
    pub echo_character: char,
    /// The most characters the text may hold; 0 is no limit.
    pub char_limit: usize,
    /// The keys.
    pub key_map: KeyMap,
    /// A check run on every change.
    pub validate: Option<ValidateFn>,
    /// Whether to offer suggestions.
    pub show_suggestions: bool,
    styles: Styles,
    width: usize,
    value: Vec<char>,
    pos: usize,
    offset: usize,
    offset_right: usize,
    virtual_cursor: cursor::Model,
    suggestions: Vec<Vec<char>>,
    matched: Vec<Vec<char>>,
    current_suggestion: usize,
    flags: Flags,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// An empty, blurred input with the prompt `"> "`: `textinput.New`.
    #[must_use]
    pub fn new() -> Self {
        let mut m = Self {
            err: None,
            prompt: "> ".into(),
            placeholder: String::new(),
            echo_mode: EchoMode::Normal,
            echo_character: '*',
            char_limit: 0,
            key_map: KeyMap::default(),
            validate: None,
            show_suggestions: false,
            styles: Styles::new(true),
            width: 0,
            value: Vec::new(),
            pos: 0,
            offset: 0,
            offset_right: 0,
            virtual_cursor: cursor::Model::new(),
            suggestions: Vec::new(),
            matched: Vec::new(),
            current_suggestion: 0,
            flags: Flags(USE_VIRTUAL_CURSOR),
        };
        m.update_cursor_style();
        m
    }

    /// Whether the cursor is drawn in the view (the default) rather
    /// than being the terminal's own, from [`Model::cursor`].
    #[must_use]
    pub const fn virtual_cursor(&self) -> bool {
        self.flags.get(USE_VIRTUAL_CURSOR)
    }

    /// Draws the cursor in the view, or leaves it to the terminal.
    pub fn set_virtual_cursor(&mut self, on: bool) {
        self.flags.set(USE_VIRTUAL_CURSOR, on);
        self.update_cursor_style();
    }

    /// The styles.
    #[must_use]
    pub const fn styles(&self) -> Styles {
        self.styles
    }

    /// Replaces the styles.
    pub fn set_styles(&mut self, styles: Styles) {
        self.styles = styles;
        self.update_cursor_style();
    }

    /// The cursor drawn in the view follows the styles.
    fn update_cursor_style(&mut self) {
        let c = self.styles.cursor;
        let mode = if !self.flags.get(USE_VIRTUAL_CURSOR) {
            Mode::Hide
        } else if c.blink {
            Mode::Blink
        } else {
            Mode::Static
        };
        if self.flags.get(USE_VIRTUAL_CURSOR) {
            self.virtual_cursor.style = Style::new().fg(c.color);
            if c.blink && !c.blink_speed.is_zero() {
                self.virtual_cursor.blink_speed = c.blink_speed;
            }
        }
        let _ = self.virtual_cursor.set_mode(mode);
    }

    /// The width the text shows in, in cells; 0 is no limit.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the width.
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// Replaces the text, cleaned (tabs and line breaks become spaces)
    /// and cut to the limit.
    pub fn set_value(&mut self, text: &str) {
        let chars = sanitize(&text.chars().collect::<Vec<_>>(), " ", " ");
        self.err = self.check(&chars);
        self.set_value_internal(chars);
        self.update_suggestions();
    }

    /// The text.
    #[must_use]
    pub fn value(&self) -> String {
        self.value.iter().collect()
    }

    /// The cursor's position, in characters from the start.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Moves the cursor to `pos`, within the text.
    pub fn set_cursor(&mut self, pos: usize) {
        self.pos = pos.min(self.value.len());
        self.handle_overflow();
    }

    /// Moves the cursor to the start.
    pub fn cursor_start(&mut self) {
        self.set_cursor(0);
    }

    /// Moves the cursor to the end.
    pub fn cursor_end(&mut self) {
        self.set_cursor(self.value.len());
    }

    /// Whether it is focused: only a focused input takes keys.
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.flags.get(FOCUS)
    }

    /// Focuses it; the command starts the cursor blinking.
    pub fn focus(&mut self) -> Cmd<Msg> {
        self.flags.set(FOCUS, true);
        self.virtual_cursor.focus()
    }

    /// Blurs it.
    pub const fn blur(&mut self) {
        self.flags.set(FOCUS, false);
        self.virtual_cursor.blur();
    }

    /// Empties it.
    pub fn reset(&mut self) {
        self.value.clear();
        self.set_cursor(0);
    }

    /// Sets the suggestions, matched by prefix as the text is typed.
    pub fn set_suggestions(&mut self, suggestions: &[&str]) {
        self.suggestions = suggestions.iter().map(|s| s.chars().collect()).collect();
        self.update_suggestions();
    }

    /// Every suggestion.
    #[must_use]
    pub fn available_suggestions(&self) -> Vec<String> {
        self.suggestions
            .iter()
            .map(|s| s.iter().collect())
            .collect()
    }

    /// The suggestions that match the text.
    #[must_use]
    pub fn matched_suggestions(&self) -> Vec<String> {
        self.matched.iter().map(|s| s.iter().collect()).collect()
    }

    /// Which matched suggestion is offered.
    #[must_use]
    pub const fn current_suggestion_index(&self) -> usize {
        self.current_suggestion
    }

    /// The matched suggestion offered, or nothing.
    #[must_use]
    pub fn current_suggestion(&self) -> String {
        self.matched
            .get(self.current_suggestion)
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    }

    /// Edits on its keys and pastes while focused, and blinks its
    /// cursor. Ctrl-V asks the terminal for the clipboard (OSC 52) and
    /// pastes its reply, [`Msg::Clipboard`](crate::Msg::Clipboard).
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        if !self.flags.get(FOCUS) {
            return Cmd::none();
        }
        let old = self.pos;
        let mut cmds = Vec::new();
        match msg {
            TeaMsg::KeyPress(key) => cmds.push(self.press(key)),
            TeaMsg::Paste(text) => self.insert_from_user(text),
            TeaMsg::Clipboard { content, .. } if self.flags.get(PASTE_PENDING) => {
                self.flags.set(PASTE_PENDING, false);
                self.insert_from_user(content);
            }
            _ => {}
        }
        if self.flags.get(USE_VIRTUAL_CURSOR) {
            cmds.push(self.virtual_cursor.update(msg));
            if old != self.pos && self.virtual_cursor.mode() == Mode::Blink {
                self.virtual_cursor.is_blinked = false;
                cmds.push(self.virtual_cursor.blink());
            }
        }
        self.handle_overflow();
        batch(cmds)
    }

    /// The text cleaned and validated, as a change from the keyboard.
    fn check(&self, chars: &[char]) -> Option<String> {
        let text: String = chars.iter().collect();
        self.validate.and_then(|f| f(&text).err())
    }
}

/// A command that starts a focused textinput's cursor blinking: Bubbles'
/// `textinput.Blink`.
#[must_use]
pub fn blink() -> Cmd<Msg> {
    cursor::blink()
}

#[cfg(test)]
mod tests;
