//! Many lines of text to edit, soft wrapped at the width by display
//! width, with line numbers, a prompt, a placeholder, limits on
//! characters, lines and size, scrolling, and keyboard selection:
//! Bubbles' `textarea` package.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::textarea;
//!
//! let mut area = textarea::Model::new();
//! let _ = area.focus();
//! area.insert_string("one\ntwo");
//! let _ = area.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Up)));
//! assert_eq!(area.value(), "one\ntwo");
//! assert_eq!((area.line(), area.column()), (0, 3));
//! ```

mod edit;
mod keymap;
mod nav;
mod select;
mod style;
mod view;
mod words;

pub use keymap::KeyMap;
pub use select::Position;
pub use style::{CursorStyle, StyleState, Styles};

use super::cursor::{self, Mode};
use super::{Flags, viewport};
use crate::style::{Border, Style, width};
use crate::{Cmd, Msg as TeaMsg, batch};

/// A text area's messages: its cursor's blinks.
pub type Msg = cursor::Msg;

/// Bubbles' limits and defaults.
const MIN_HEIGHT: usize = 1;
const DEFAULT_HEIGHT: usize = 6;
const DEFAULT_WIDTH: usize = 40;
const DEFAULT_MAX_HEIGHT: usize = 99;
const DEFAULT_MAX_WIDTH: usize = 500;
const MAX_LINES: usize = 10_000;

/// What a prompt function is told: Bubbles' `textarea.PromptInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptInfo {
    /// The screen line, from 0.
    pub line_number: usize,
    /// Whether the text area is focused.
    pub focused: bool,
}

/// Where the cursor is within its soft wrapped line: Bubbles'
/// `textarea.LineInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineInfo {
    /// Characters in the wrapped line the cursor is on.
    pub width: usize,
    /// Cells that wrapped line takes.
    pub char_width: usize,
    /// How many wrapped lines the line makes.
    pub height: usize,
    /// Where that wrapped line starts, in characters.
    pub start_column: usize,
    /// The cursor's character in that wrapped line.
    pub column_offset: usize,
    /// Which wrapped line it is.
    pub row_offset: usize,
    /// The cursor's cell in that wrapped line.
    pub char_offset: usize,
}

/// A [`Model`] setting's bit.
const USE_VIRTUAL_CURSOR: u16 = 1 << 0;
/// A [`Model`] setting's bit.
const FOCUS: u16 = 1 << 1;
/// A [`Model`] setting's bit.
const HAS_SELECTION: u16 = 1 << 2;
/// A [`Model`] setting's bit.
const SELECTING: u16 = 1 << 3;
/// A [`Model`] setting's bit.
const PASTE_PENDING: u16 = 1 << 4;

/// A text area: Bubbles' `textarea.Model`.
#[derive(Debug, Clone)]
pub struct Model {
    /// The last error, if any.
    pub err: Option<String>,
    /// Shown before each line.
    pub prompt: String,
    /// Shown when there is no text.
    pub placeholder: String,
    /// Whether to number lines.
    pub show_line_numbers: bool,
    /// Shown on the lines after the end.
    pub end_of_buffer_character: char,
    /// The keys.
    pub key_map: KeyMap,
    /// The most characters; 0 is no limit.
    pub char_limit: usize,
    /// The most rows, and lines; 0 is no limit.
    pub max_height: usize,
    /// The most columns; 0 is no limit.
    pub max_width: usize,
    /// Whether the height follows the content, between
    /// [`Model::min_height`] and [`Model::max_height`].
    pub dynamic_height: bool,
    /// The least rows with a dynamic height.
    pub min_height: usize,
    /// The most wrapped lines the content may take; 0 is no limit.
    pub max_content_height: usize,
    styles: Styles,
    virtual_cursor: cursor::Model,
    prompt_fn: Option<fn(PromptInfo) -> String>,
    prompt_width: usize,
    width: usize,
    height: usize,
    value: Vec<Vec<char>>,
    col: usize,
    row: usize,
    last_char_offset: usize,
    viewport: viewport::Model,
    anchor: Position,
    head: Position,
    flags: Flags,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// An empty, blurred text area, 40 by 6, with line numbers and the
    /// prompt `"┃ "`: `textarea.New`.
    #[must_use]
    pub fn new() -> Self {
        let mut vp = viewport::Model::new(0, 0);
        vp.key_map = viewport::KeyMap::none();
        let mut m = Self {
            err: None,
            prompt: format!("{} ", Border::THICK.left),
            placeholder: String::new(),
            show_line_numbers: true,
            end_of_buffer_character: ' ',
            key_map: KeyMap::default(),
            char_limit: 0,
            max_height: DEFAULT_MAX_HEIGHT,
            max_width: DEFAULT_MAX_WIDTH,
            dynamic_height: false,
            min_height: 0,
            max_content_height: 0,
            styles: Styles::new(true),
            virtual_cursor: cursor::Model::new(),
            prompt_fn: None,
            prompt_width: 0,
            width: 0,
            height: 0,
            value: vec![Vec::new()],
            col: 0,
            row: 0,
            last_char_offset: 0,
            viewport: vp,
            anchor: Position::default(),
            head: Position::default(),
            flags: Flags(USE_VIRTUAL_CURSOR),
        };
        m.update_cursor_style();
        m.set_height(DEFAULT_HEIGHT);
        m.set_width(DEFAULT_WIDTH);
        m
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

    /// Whether the cursor is drawn in the view (the default) rather than
    /// being the terminal's own, from [`Model::cursor`].
    #[must_use]
    pub const fn virtual_cursor(&self) -> bool {
        self.flags.get(USE_VIRTUAL_CURSOR)
    }

    /// Draws the cursor in the view, or leaves it to the terminal.
    pub fn set_virtual_cursor(&mut self, on: bool) {
        self.flags.set(USE_VIRTUAL_CURSOR, on);
        self.update_cursor_style();
    }

    /// The drawn cursor follows the styles.
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

    /// Replaces the text.
    pub fn set_value(&mut self, text: &str) {
        self.reset();
        self.insert_string(text);
        self.reposition_view();
    }

    /// Inserts `text` at the cursor, as if typed or pasted.
    pub fn insert_string(&mut self, text: &str) {
        self.insert_from_user(&text.chars().collect::<Vec<_>>());
        self.recalculate_height();
    }

    /// Inserts one character at the cursor.
    pub fn insert_rune(&mut self, c: char) {
        self.insert_from_user(&[c]);
        self.recalculate_height();
    }

    /// The text, lines joined with `\n`.
    #[must_use]
    pub fn value(&self) -> String {
        let lines: Vec<String> = self.value.iter().map(|l| l.iter().collect()).collect();
        lines.join("\n")
    }

    /// Characters in the text, a line break counting one.
    #[must_use]
    pub fn length(&self) -> usize {
        self.value.iter().map(Vec::len).sum::<usize>() + self.value.len() - 1
    }

    /// How many lines.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.value.len()
    }

    /// The cursor's line, from 0.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.row
    }

    /// The cursor's column, in characters.
    #[must_use]
    pub const fn column(&self) -> usize {
        self.col
    }

    /// The first screen line showing.
    #[must_use]
    pub const fn scroll_y_offset(&self) -> usize {
        self.viewport.y_offset()
    }

    /// How far down it has scrolled, from 0.0 to 1.0.
    #[must_use]
    pub fn scroll_percent(&self) -> f64 {
        self.viewport.scroll_percent()
    }

    /// Whether it is focused: only a focused text area takes keys.
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
        self.value = vec![Vec::new()];
        self.col = 0;
        self.row = 0;
        self.viewport.goto_top();
        self.set_cursor_column(0);
        self.recalculate_height();
        self.clear_selection();
    }

    /// The text's width, in cells, without prompt and line numbers.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the whole width, prompt, line numbers and frame included,
    /// within [`Model::max_width`].
    pub fn set_width(&mut self, columns: usize) {
        if self.prompt_fn.is_none() {
            self.prompt_width = width(&self.prompt);
        }
        let outer = self.active_style().base.horizontal_frame();
        let mut inner = self.prompt_width;
        if self.show_line_numbers {
            inner += digits(self.max_height) + 2;
        }
        let least = inner + outer + 1;
        let mut input = columns.max(least);
        if self.max_width > 0 {
            input = input.min(self.max_width.max(least));
        }
        self.viewport.set_width(input - outer);
        self.width = input - outer - inner;
        self.recalculate_height();
    }

    /// A prompt drawn by `prompt` for each line, `columns` wide.
    pub fn set_prompt_func(&mut self, columns: usize, prompt: fn(PromptInfo) -> String) {
        self.prompt_fn = Some(prompt);
        self.prompt_width = columns;
    }

    /// The height, in rows.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Sets the height, within [`Model::max_height`].
    pub fn set_height(&mut self, rows: usize) {
        self.height = if self.max_height > 0 {
            rows.clamp(MIN_HEIGHT, self.max_height)
        } else {
            rows.max(MIN_HEIGHT)
        };
        self.viewport.set_height(self.height);
        self.reposition_view();
    }

    /// The styles for the state, focused or blurred.
    const fn active_style(&self) -> &StyleState {
        if self.flags.get(FOCUS) {
            &self.styles.focused
        } else {
            &self.styles.blurred
        }
    }

    /// Edits on its keys and pastes while focused, scrolls, and blinks
    /// its cursor. Ctrl-V asks the terminal for the clipboard (OSC 52)
    /// and pastes its reply; Ctrl-Shift-C copies the selection with OSC 52.
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        if !self.flags.get(FOCUS) {
            self.virtual_cursor.blur();
            return Cmd::none();
        }
        let old = (self.cursor_line_number(), self.col);
        let first = match self.handle(msg) {
            edit::Step::Done => Cmd::none(),
            edit::Step::Run(cmd) => cmd,
            edit::Step::Stop => return Cmd::none(),
        };
        self.recalculate_height();
        self.viewport.set_content(&self.render_lines());
        let cmds = [
            first,
            self.viewport.update(msg),
            self.blink_on_move(msg, old),
        ];
        self.reposition_view();
        batch(cmds)
    }

    /// Edits for a key or a paste.
    fn handle(&mut self, msg: &TeaMsg<Msg>) -> edit::Step {
        match msg {
            TeaMsg::Paste(text) => self.paste(text),
            TeaMsg::KeyPress(key) => return self.press(key),
            TeaMsg::Clipboard { content, .. } if self.flags.get(PASTE_PENDING) => {
                self.flags.set(PASTE_PENDING, false);
                self.paste(content);
            }
            _ => {}
        }
        edit::Step::Done
    }

    /// The drawn cursor's blink, shown again and started over if the
    /// cursor moved from `old` (its screen line and column), as in Go.
    fn blink_on_move(&mut self, msg: &TeaMsg<Msg>, old: (usize, usize)) -> Cmd<Msg> {
        if !self.flags.get(USE_VIRTUAL_CURSOR) {
            return Cmd::none();
        }
        let blink = self.virtual_cursor.update(msg);
        let moved = (self.cursor_line_number(), self.col) != old;
        if !moved || self.virtual_cursor.mode() != Mode::Blink {
            return blink;
        }
        self.virtual_cursor.is_blinked = false;
        self.virtual_cursor.blink()
    }

    /// Pasted text replaces the selection.
    fn paste(&mut self, text: &str) {
        self.delete_selection();
        self.insert_from_user(&text.chars().collect::<Vec<_>>());
    }
}

/// The decimal digits of `n`.
fn digits(n: usize) -> usize {
    n.to_string().len()
}

/// A command that starts a focused textarea's cursor blinking: Bubbles'
/// `textarea.Blink`.
#[must_use]
pub fn blink() -> Cmd<Msg> {
    cursor::blink()
}

#[cfg(test)]
mod tests;
