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
mod keymap;
mod style;
mod suggest;
mod value;
mod view;

pub use keymap::KeyMap;
pub use style::{CursorStyle, StyleState, Styles};

use super::Flags;
use super::cursor::{self, Mode};
use crate::style::Style;
use crate::{Cmd, Msg as TeaMsg, batch};
use std::sync::Arc;

/// A text input's messages: its cursor's blinks.
pub type Msg = cursor::Msg;

/// How the text shows: Bubbles' `textinput.EchoMode`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// let mut input = textinput::Model::new();
/// input.echo_mode = textinput::EchoMode::Password;
/// input.set_value("abc");
/// assert_eq!(input.value(), "abc");
/// assert_eq!(tea::style::strip_ansi(&input.view()), "> *** ");
/// ```
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

/// A check run on every change; its error is kept in [`Model::err`]:
/// Bubbles' `textinput.ValidateFunc`. A closure, so it may capture
/// state; a plain `fn` works too.
///
/// ```
/// use petitcarnets as tea;
/// use std::sync::Arc;
/// use tea::bubbles::textinput;
///
/// let max = 3;
/// let mut input = textinput::Model::new();
/// input.validate = Some(Arc::new(move |s: &str| {
///     if s.len() > max { Err(format!("over {max}")) } else { Ok(()) }
/// }));
/// input.set_value("abcd");
/// assert_eq!(input.err.as_deref(), Some("over 3"));
/// ```
pub type ValidateFn = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// A [`Model`] setting's bit.
const FOCUS: u16 = 1 << 0;
/// A [`Model`] setting's bit.
const USE_VIRTUAL_CURSOR: u16 = 1 << 1;
/// A [`Model`] setting's bit.
const PASTE_PENDING: u16 = 1 << 2;

/// A text input: Bubbles' `textinput.Model`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// let mut input = textinput::Model::new();
/// input.prompt = "$ ".into();
/// input.placeholder = "command".into();
/// assert_eq!(tea::style::strip_ansi(&input.view()), "$ command");
/// ```
#[derive(Clone)]
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

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("err", &self.err)
            .field("prompt", &self.prompt)
            .field("placeholder", &self.placeholder)
            .field("echo_mode", &self.echo_mode)
            .field("echo_character", &self.echo_character)
            .field("char_limit", &self.char_limit)
            .field("key_map", &self.key_map)
            .field("validate", &super::opaque(self.validate.as_ref()))
            .field("show_suggestions", &self.show_suggestions)
            .field("styles", &self.styles)
            .field("width", &self.width)
            .field("value", &self.value)
            .field("pos", &self.pos)
            .field("offset", &self.offset)
            .field("offset_right", &self.offset_right)
            .field("virtual_cursor", &self.virtual_cursor)
            .field("suggestions", &self.suggestions)
            .field("matched", &self.matched)
            .field("current_suggestion", &self.current_suggestion)
            .field("flags", &self.flags)
            .finish()
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// An empty, blurred input with the prompt `"> "`: `textinput.New`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let input = textinput::Model::new();
    /// assert_eq!(input.prompt, "> ");
    /// assert_eq!(input.value(), "");
    /// assert!(!input.focused());
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// assert!(input.virtual_cursor());
    /// input.set_virtual_cursor(false);
    /// assert!(!input.virtual_cursor());
    /// ```
    #[must_use]
    pub const fn virtual_cursor(&self) -> bool {
        self.flags.get(USE_VIRTUAL_CURSOR)
    }

    /// Draws the cursor in the view, or leaves it to the terminal.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// let _ = input.focus();
    /// input.set_value("hi");
    /// assert_eq!(input.cursor(), None, "drawn in the view");
    /// input.set_virtual_cursor(false);
    /// assert_eq!(
    ///     input.cursor().map(|c| c.x),
    ///     Some(4),
    ///     "the terminal's, after \"> hi\""
    /// );
    /// ```
    pub fn set_virtual_cursor(&mut self, on: bool) {
        self.flags.set(USE_VIRTUAL_CURSOR, on);
        self.update_cursor_style();
    }

    /// The styles.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let input = textinput::Model::new();
    /// assert_eq!(input.styles(), textinput::Styles::new(true));
    /// ```
    #[must_use]
    pub const fn styles(&self) -> Styles {
        self.styles
    }

    /// Replaces the styles.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut styles = textinput::Styles::new(false);
    /// styles.focused.prompt = tea::style::Style::new().bold(true);
    /// let mut input = textinput::Model::new();
    /// input.set_styles(styles);
    /// assert_eq!(input.styles(), styles);
    /// ```
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// assert_eq!(input.width(), 0, "no limit");
    /// input.set_width(20);
    /// assert_eq!(input.width(), 20);
    /// ```
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the width.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_width(3);
    /// let _ = input.focus();
    /// input.set_value("abcdef");
    /// assert_eq!(
    ///     tea::style::strip_ansi(&input.view()),
    ///     "> def ",
    ///     "scrolled to the cursor"
    /// );
    /// ```
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// Whether it is focused: only a focused input takes keys.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// assert!(!input.focused());
    /// let _ = input.focus();
    /// assert!(input.focused());
    /// ```
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.flags.get(FOCUS)
    }

    /// Focuses it; the command starts the cursor blinking.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// let blink = input.focus();
    /// assert!(input.focused());
    /// assert!(!blink.is_none(), "the cursor starts blinking");
    /// ```
    pub fn focus(&mut self) -> Cmd<Msg> {
        self.flags.set(FOCUS, true);
        self.virtual_cursor.focus()
    }

    /// Blurs it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// let _ = input.focus();
    /// input.blur();
    /// assert!(!input.focused());
    /// let _ = input.update(&tea::Msg::KeyPress(tea::Key::char('a')));
    /// assert_eq!(input.value(), "", "a blurred input takes no keys");
    /// ```
    pub const fn blur(&mut self) {
        self.flags.set(FOCUS, false);
        self.virtual_cursor.blur();
    }

    /// Empties it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_value("abc");
    /// input.reset();
    /// assert_eq!((input.value().as_str(), input.position()), ("", 0));
    /// ```
    pub fn reset(&mut self) {
        self.value.clear();
        self.set_cursor(0);
    }

    /// Edits on its keys and pastes while focused, and blinks its
    /// cursor. Ctrl-V asks the terminal for the clipboard (OSC 52) and
    /// pastes its reply, [`Msg::Clipboard`](crate::Msg::Clipboard).
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// let _ = input.focus();
    /// let _ = input.update(&tea::Msg::Paste("hello world".into()));
    /// let _ = input.update(&tea::Msg::KeyPress(tea::Key::ctrl('w')));
    /// assert_eq!(input.value(), "hello ", "Ctrl-W deletes a word");
    /// ```
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
        self.validate.as_ref().and_then(|f| f(&text).err())
    }
}

/// A command that starts a focused textinput's cursor blinking: Bubbles'
/// `textinput.Blink`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// use std::time::Duration;
///
/// struct App(textinput::Model);
///
/// impl tea::Model for App {
///     type Msg = textinput::Msg;
///     fn init(&mut self) -> tea::Cmd<textinput::Msg> {
///         textinput::blink()
///     }
///     fn update(&mut self, msg: tea::Msg<textinput::Msg>) -> tea::Cmd<textinput::Msg> {
///         self.0.update(&msg)
///     }
///     fn view(&self) -> tea::View {
///         self.0.view().into()
///     }
/// }
///
/// let mut input = textinput::Model::new();
/// let _ = input.focus();
/// let mut p = tea::testing::TestProgram::new(App(input), 10, 1);
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
