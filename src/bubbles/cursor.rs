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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Msg {
    /// Start blinking, from [`blink`]: Bubbles' `initialBlinkMsg`.
    InitialBlink,
    /// Time to blink: Bubbles' `cursor.BlinkMsg`.
    Blink(BlinkMsg),
}

/// Time for one cursor to blink, carrying its id so other cursors leave
/// it alone. Bubbles' `cursor.BlinkMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlinkMsg {
    id: u64,
    tag: u64,
}

impl BlinkMsg {
    /// The id of the cursor it is for.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }
}

/// A cursor: Bubbles' `cursor.Model`.
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
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Blinks on its own blink messages, starts on [`Msg::InitialBlink`],
    /// and follows the terminal's focus.
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
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Changes the mode; blinking starts with the returned command.
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
    pub const fn blur(&mut self) {
        self.focus = false;
        self.is_blinked = true;
    }

    /// Whether it is focused.
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focus
    }

    /// The character under the cursor.
    pub fn set_char(&mut self, c: &str) {
        c.clone_into(&mut self.char);
    }

    /// The character under the cursor, reversed when the cursor shows.
    #[must_use]
    pub fn view(&self) -> String {
        if self.is_blinked {
            self.text_style.paint(&self.char)
        } else {
            self.style.reverse().paint(&self.char)
        }
    }
}

/// A command that starts a focused cursor blinking: Bubbles'
/// `cursor.Blink`.
#[must_use]
pub fn blink() -> Cmd<Msg> {
    super::now(Msg::InitialBlink)
}

#[cfg(test)]
mod tests;
