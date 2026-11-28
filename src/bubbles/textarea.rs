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
mod focus;
mod keymap;
mod nav;
mod select;
mod style;
mod value;
mod view;
mod words;

pub use keymap::KeyMap;
pub use select::Position;
pub use style::{CursorStyle, StyleState, Styles};

use super::cursor::{self, Mode};
use super::{Flags, viewport};
use crate::style::{Border, Style};
use crate::{Cmd, Msg as TeaMsg, batch};
use std::sync::Arc;

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
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let mut area = textarea::Model::new();
/// area.show_line_numbers = false;
/// area.set_prompt_func(2, |info: textarea::PromptInfo| {
///     format!("{} ", info.line_number)
/// });
/// area.set_value("a\nb");
/// let view = tea::style::strip_ansi(&area.view());
/// let lines: Vec<&str> = view.lines().collect();
/// assert!(lines[0].starts_with("0 a") && lines[1].starts_with("1 b"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptInfo {
    /// The screen line, from 0.
    pub line_number: usize,
    /// Whether the text area is focused.
    pub focused: bool,
}

/// Where the cursor is within its soft wrapped line: Bubbles'
/// `textarea.LineInfo`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let mut area = textarea::Model::new();
/// area.show_line_numbers = false;
/// area.set_width(12);
/// area.set_value("hello world");
/// let info = area.line_info();
/// assert_eq!(
///     (info.height, info.row_offset),
///     (2, 1),
///     "on the second of two"
/// );
/// assert_eq!(info.column_offset, 5);
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let mut area = textarea::Model::new();
/// area.show_line_numbers = false;
/// area.placeholder = "Say something".into();
/// let first = tea::style::strip_ansi(&area.view());
/// assert!(first.starts_with("┃ Say something"));
/// ```
#[derive(Clone)]
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
    prompt_fn: Option<Arc<dyn Fn(PromptInfo) -> String + Send + Sync>>,
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let area = textarea::Model::new();
    /// assert_eq!(area.prompt, "┃ ");
    /// assert_eq!(
    ///     (area.width(), area.height()),
    ///     (34, 6),
    ///     "40 wide, less the gutter"
    /// );
    /// assert!(area.show_line_numbers && !area.focused());
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let area = textarea::Model::new();
    /// assert_eq!(area.styles(), textarea::Styles::new(true));
    /// ```
    #[must_use]
    pub const fn styles(&self) -> Styles {
        self.styles
    }

    /// Replaces the styles.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut styles = textarea::Styles::new(false);
    /// styles.focused.prompt = tea::style::Style::new().bold(true);
    /// let mut area = textarea::Model::new();
    /// area.set_styles(styles);
    /// assert_eq!(area.styles(), styles);
    /// ```
    pub fn set_styles(&mut self, styles: Styles) {
        self.styles = styles;
        self.update_cursor_style();
    }

    /// Whether the cursor is drawn in the view (the default) rather than
    /// being the terminal's own, from [`Model::cursor`].
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// assert!(area.virtual_cursor());
    /// area.set_virtual_cursor(false);
    /// assert!(!area.virtual_cursor());
    /// ```
    #[must_use]
    pub const fn virtual_cursor(&self) -> bool {
        self.flags.get(USE_VIRTUAL_CURSOR)
    }

    /// Draws the cursor in the view, or leaves it to the terminal.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let _ = area.focus();
    /// assert_eq!(area.cursor(), None, "drawn in the view");
    /// area.set_virtual_cursor(false);
    /// assert!(area.cursor().is_some(), "the terminal's own");
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let _ = area.focus();
    /// area.set_value("one two");
    /// let _ = area.update(&tea::Msg::KeyPress(tea::Key::ctrl('w')));
    /// assert_eq!(area.value(), "one ", "Ctrl-W deletes a word");
    /// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// use std::time::Duration;
///
/// struct App(textarea::Model);
///
/// impl tea::Model for App {
///     type Msg = textarea::Msg;
///     fn init(&mut self) -> tea::Cmd<textarea::Msg> {
///         textarea::blink()
///     }
///     fn update(&mut self, msg: tea::Msg<textarea::Msg>) -> tea::Cmd<textarea::Msg> {
///         self.0.update(&msg)
///     }
///     fn view(&self) -> tea::View {
///         self.0.view().into()
///     }
/// }
///
/// let mut area = textarea::Model::new();
/// let _ = area.focus();
/// let mut p = tea::testing::TestProgram::new(App(area), 40, 6);
/// let shown = p.model().0.view();
/// p.advance(Duration::from_millis(600));
/// assert_ne!(p.model().0.view(), shown, "the cursor blinked off");
/// ```
#[must_use]
pub fn blink() -> Cmd<Msg> {
    cursor::blink()
}

#[cfg(test)]
mod tests;
