//! A text cursor drawn in the view, which can blink: Bubbles' `cursor`
//! package, used by [`textinput`](super::textinput) and
//! [`textarea`](super::textarea).
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::cursor::{self, Mode};
//!
//! let mut c = cursor::Model::new();
//! c.set_char("a");
//! let blink = c.focus();
//! assert!(!blink.is_none(), "a focused cursor starts blinking");
//! assert_eq!(c.mode(), Mode::Blink);
//! assert_eq!(c.view(), "\x1b[7ma\x1b[0m", "shown, in reverse");
//! ```

use super::next_id;
use crate::style::Style;
use crate::{Cmd, Msg as TeaMsg};
use std::fmt;
use std::time::Duration;

/// How long the cursor stays on or off when blinking: Bubbles' 530 ms.
const BLINK_SPEED: Duration = Duration::from_millis(530);

/// How the cursor shows: Bubbles' `cursor.Mode`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::cursor::Mode;
///
/// assert_eq!(Mode::default(), Mode::Blink);
/// assert_eq!(Mode::Hide.to_string(), "hidden");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// On and off, every [`Model::blink_speed`].
    #[default]
    Blink,
    /// Always on.
    Static,
    /// Never shown.
    Hide,
}

impl fmt::Display for Mode {
    /// `"blink"`, `"static"` or `"hidden"`, as Go's `Mode.String`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blink => "blink",
            Self::Static => "static",
            Self::Hide => "hidden",
        })
    }
}

/// The cursor's messages.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::cursor::{self, Msg};
///
/// let mut c = cursor::Model::new();
/// let blink = c.update(&tea::Msg::App(Msg::InitialBlink));
/// assert!(blink.is_none(), "a blurred cursor does not blink");
/// let _ = c.focus();
/// let blink = c.update(&tea::Msg::App(Msg::InitialBlink));
/// assert!(!blink.is_none(), "a focused one schedules its next blink");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    /// Start blinking, from [`blink`]: Bubbles' `initialBlinkMsg`.
    InitialBlink,
    /// Time to blink: Bubbles' `cursor.BlinkMsg`.
    Blink(BlinkMsg),
}

/// Time for one cursor to blink, carrying its id so other cursors leave
/// it alone. Bubbles' `cursor.BlinkMsg`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::cursor::{self, BlinkMsg, Msg};
///
/// struct App(cursor::Model, Option<BlinkMsg>);
///
/// impl tea::Model for App {
///     type Msg = Msg;
///     fn init(&mut self) -> tea::Cmd<Msg> {
///         self.0.focus()
///     }
///     fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
///         if let tea::Msg::App(Msg::Blink(b)) = &msg {
///             self.1 = Some(*b);
///         }
///         self.0.update(&msg)
///     }
///     fn view(&self) -> tea::View {
///         self.0.view().into()
///     }
/// }
///
/// let mut p = tea::testing::TestProgram::new(App(cursor::Model::new(), None), 4, 1);
/// assert!(p.model().1.is_none());
/// p.advance(Duration::from_millis(600));
/// assert!(p.model().1.is_some(), "one blink after 530 ms");
/// assert!(p.model().0.is_blinked, "and the cursor blinked off");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlinkMsg {
    id: u64,
    tag: u64,
}

impl BlinkMsg {
    /// The id of the cursor it is for.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::cursor::{self, BlinkMsg, Msg};
    /// # struct App(cursor::Model, Option<BlinkMsg>);
    /// # impl tea::Model for App {
    /// #     type Msg = Msg;
    /// #     fn init(&mut self) -> tea::Cmd<Msg> {
    /// #         self.0.focus()
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
    /// #         if let tea::Msg::App(Msg::Blink(b)) = &msg {
    /// #             self.1 = Some(*b);
    /// #         }
    /// #         self.0.update(&msg)
    /// #     }
    /// #     fn view(&self) -> tea::View {
    /// #         self.0.view().into()
    /// #     }
    /// # }
    ///
    /// let mut p = tea::testing::TestProgram::new(App(cursor::Model::new(), None), 4, 1);
    /// p.advance(Duration::from_millis(600));
    /// let app = p.model();
    /// assert_eq!(
    ///     app.1.map(|b| b.id()),
    ///     Some(app.0.id()),
    ///     "the blink is this cursor's"
    /// );
    /// ```
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

/// A cursor: Bubbles' `cursor.Model`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::cursor;
///
/// let mut c = cursor::Model::new();
/// c.set_char("x");
/// assert_eq!(c.view(), "x", "blurred, the character shows plain");
/// let _ = c.focus();
/// assert_eq!(c.view(), "\x1b[7mx\x1b[0m", "focused, it shows reversed");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The style of the cursor when shown; it is drawn reversed.
    pub style: Style,
    /// The style of the character under the cursor when it is hidden.
    pub text_style: Style,
    /// How long it stays on or off when blinking.
    pub blink_speed: Duration,
    /// Whether it is off now (blinked away), showing the text under it.
    pub is_blinked: bool,
    char: String,
    id: u64,
    focus: bool,
    blink_tag: u64,
    mode: Mode,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// A blinking cursor, not focused: `cursor.New`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let c = cursor::Model::new();
    /// assert_eq!(c.mode(), cursor::Mode::Blink);
    /// assert!(!c.focused());
    /// assert_eq!(c.blink_speed.as_millis(), 530);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            style: Style::new(),
            text_style: Style::new(),
            blink_speed: BLINK_SPEED,
            is_blinked: true,
            char: String::new(),
            id: next_id(),
            focus: false,
            blink_tag: 0,
            mode: Mode::Blink,
        }
    }

    /// The cursor's id, which its blink messages carry.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let (a, b) = (cursor::Model::new(), cursor::Model::new());
    /// assert_ne!(a.id(), b.id(), "each cursor has its own");
    /// ```
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Blinks on its own blink messages, starts on [`Msg::InitialBlink`],
    /// and follows the terminal's focus.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let mut c = cursor::Model::new();
    /// let _ = c.update(&tea::Msg::Focus);
    /// assert!(c.focused(), "the terminal took focus");
    /// let _ = c.update(&tea::Msg::Blur);
    /// assert!(!c.focused());
    /// ```
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        match msg {
            TeaMsg::App(Msg::InitialBlink) if self.blinking() => self.blink(),
            TeaMsg::App(Msg::Blink(b)) if self.blinking() && self.is_mine(*b) => {
                self.is_blinked = !self.is_blinked;
                self.blink()
            }
            TeaMsg::Focus => self.focus(),
            TeaMsg::Blur => {
                self.blur();
                Cmd::none()
            }
            _ => Cmd::none(),
        }
    }

    /// Whether the cursor blinks now: in blink mode and focused.
    fn blinking(&self) -> bool {
        self.mode == Mode::Blink && self.focus
    }

    /// Whether `msg` is this cursor's latest blink.
    const fn is_mine(&self, msg: BlinkMsg) -> bool {
        msg.id == self.id && msg.tag == self.blink_tag
    }

    /// The mode.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor::{self, Mode};
    ///
    /// let mut c = cursor::Model::new();
    /// assert_eq!(c.mode(), Mode::Blink);
    /// let _ = c.set_mode(Mode::Static);
    /// assert_eq!(c.mode(), Mode::Static);
    /// ```
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Changes the mode; blinking starts with the returned command.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor::{self, Mode};
    ///
    /// let mut c = cursor::Model::new();
    /// assert!(c.set_mode(Mode::Static).is_none());
    /// assert!(!c.set_mode(Mode::Blink).is_none(), "blinking starts");
    /// ```
    pub fn set_mode(&mut self, mode: Mode) -> Cmd<Msg> {
        self.mode = mode;
        self.is_blinked = mode == Mode::Hide || !self.focus;
        if mode == Mode::Blink {
            blink()
        } else {
            Cmd::none()
        }
    }

    /// The next blink, after [`Model::blink_speed`]; any earlier one is
    /// ignored when it comes.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor::{self, Mode};
    ///
    /// let mut c = cursor::Model::new();
    /// assert!(!c.blink().is_none());
    /// let _ = c.set_mode(Mode::Static);
    /// assert!(c.blink().is_none(), "only a blinking cursor blinks");
    /// ```
    pub fn blink(&mut self) -> Cmd<Msg> {
        if self.mode != Mode::Blink {
            return Cmd::none();
        }
        self.blink_tag += 1;
        let msg = BlinkMsg {
            id: self.id,
            tag: self.blink_tag,
        };
        crate::tick(self.blink_speed, move |_| Msg::Blink(msg))
    }

    /// Focuses the cursor, showing it, and starts it blinking.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let mut c = cursor::Model::new();
    /// c.set_char("a");
    /// let blink = c.focus();
    /// assert!(c.focused() && !blink.is_none());
    /// assert_eq!(c.view(), "\x1b[7ma\x1b[0m");
    /// ```
    pub fn focus(&mut self) -> Cmd<Msg> {
        self.focus = true;
        self.is_blinked = self.mode == Mode::Hide;
        if self.mode == Mode::Blink {
            self.blink()
        } else {
            Cmd::none()
        }
    }

    /// Blurs the cursor, hiding it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let mut c = cursor::Model::new();
    /// c.set_char("a");
    /// let _ = c.focus();
    /// c.blur();
    /// assert!(!c.focused());
    /// assert_eq!(c.view(), "a", "hidden, the character shows");
    /// ```
    pub const fn blur(&mut self) {
        self.focus = false;
        self.is_blinked = true;
    }

    /// Whether it is focused.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let mut c = cursor::Model::new();
    /// assert!(!c.focused());
    /// let _ = c.focus();
    /// assert!(c.focused());
    /// ```
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focus
    }

    /// The character under the cursor.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor;
    ///
    /// let mut c = cursor::Model::new();
    /// c.set_char("é");
    /// assert_eq!(c.view(), "é");
    /// ```
    pub fn set_char(&mut self, c: &str) {
        c.clone_into(&mut self.char);
    }

    /// The character under the cursor, reversed when the cursor shows.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::cursor::{self, Mode};
    ///
    /// let mut c = cursor::Model::new();
    /// let _ = c.set_mode(Mode::Static);
    /// c.set_char("z");
    /// assert_eq!(c.view(), "z");
    /// let _ = c.focus();
    /// assert_eq!(c.view(), "\x1b[7mz\x1b[0m");
    /// ```
    #[must_use]
    pub fn view(&self) -> String {
        if self.is_blinked {
            self.text_style.paint(&self.char)
        } else {
            self.style.reverse(true).paint(&self.char)
        }
    }
}

/// A command that starts a focused cursor blinking: Bubbles'
/// `cursor.Blink`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::cursor::{self, Msg};
///
/// struct App(cursor::Model);
///
/// impl tea::Model for App {
///     type Msg = Msg;
///     fn init(&mut self) -> tea::Cmd<Msg> {
///         cursor::blink()
///     }
///     fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
///         self.0.update(&msg)
///     }
///     fn view(&self) -> tea::View {
///         self.0.view().into()
///     }
/// }
///
/// let mut c = cursor::Model::new();
/// let _ = c.focus();
/// let mut p = tea::testing::TestProgram::new(App(c), 4, 1);
/// assert!(!p.model().0.is_blinked, "shown");
/// p.advance(Duration::from_millis(600));
/// assert!(p.model().0.is_blinked, "blinking, so off after 530 ms");
/// ```
#[must_use]
pub fn blink() -> Cmd<Msg> {
    super::now(Msg::InitialBlink)
}

#[cfg(test)]
mod tests;
