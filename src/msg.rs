//! What a model receives, and the values around it.

use crate::color::Rgb;
use crate::key::Key;
use crate::mode::{KeyboardEnhancements, KeyboardEnhancementsReport, ModeSetting, ReportedMode};
use crate::mouse::{Mouse, MouseMode};
use std::fmt;
use std::io;

/// A message for [`Model::update`](crate::Model::update): what the
/// terminal reported, or one of the model's own messages.
///
/// Bubble Tea's `tea.Msg`, as an enum: match it where Go uses a type
/// switch. More variants arrive as the runtime grows, so keep a `_` arm.
/// Sequences the terminal sends that are not listed here are read and
/// dropped; they never arrive as keys.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Msg<M> {
    /// A key was pressed. Bubble Tea's `tea.KeyPressMsg`; compare it with
    /// a name, as in `key == "ctrl+c"`.
    KeyPress(Key),
    /// A key was released: only with
    /// [`KeyboardEnhancements::report_event_types`] on a terminal that
    /// supports it. Bubble Tea's `tea.KeyReleaseMsg`.
    KeyRelease(Key),
    /// A mouse button was pressed. Bubble Tea's `tea.MouseClickMsg`.
    MouseClick(Mouse),
    /// A mouse button was released. Bubble Tea's `tea.MouseReleaseMsg`.
    MouseRelease(Mouse),
    /// The mouse wheel turned. Bubble Tea's `tea.MouseWheelMsg`.
    MouseWheel(Mouse),
    /// The mouse moved. Bubble Tea's `tea.MouseMotionMsg`; write it with
    /// [`Mouse::motion_string`].
    MouseMotion(Mouse),
    /// Text was pasted, all of it at once (bracketed paste). Bubble Tea's
    /// `tea.PasteMsg`.
    Paste(String),
    /// A paste begins; [`Msg::Paste`] follows. Bubble Tea's
    /// `tea.PasteStartMsg`.
    PasteStart,
    /// A paste ended. Bubble Tea's `tea.PasteEndMsg`.
    PasteEnd,
    /// The terminal gained focus (with [`View::report_focus`]). Bubble
    /// Tea's `tea.FocusMsg`.
    Focus,
    /// The terminal lost focus. Bubble Tea's `tea.BlurMsg`.
    Blur,
    /// The terminal's reply giving the cursor's position, from 0.
    /// Bubble Tea's `tea.CursorPositionMsg`.
    CursorPosition {
        /// The column.
        x: i32,
        /// The row.
        y: i32,
    },
    /// The terminal's reply giving its keyboard enhancements. Bubble
    /// Tea's `tea.KeyboardEnhancementsMsg`.
    KeyboardEnhancements(KeyboardEnhancementsReport),
    /// The terminal's reply about a mode. Bubble Tea's `tea.ModeReportMsg`.
    ModeReport {
        /// The mode.
        mode: ReportedMode,
        /// Its setting.
        value: ModeSetting,
    },
    /// The terminal's name and version (`XTVERSION`). Bubble Tea's
    /// `tea.TerminalVersionMsg`.
    TerminalVersion(String),
    /// The terminal's foreground colour. Bubble Tea's
    /// `tea.ForegroundColorMsg`.
    ForegroundColor(Rgb),
    /// The terminal's background colour. Bubble Tea's
    /// `tea.BackgroundColorMsg`.
    BackgroundColor(Rgb),
    /// The terminal's cursor colour. Bubble Tea's `tea.CursorColorMsg`.
    CursorColor(Rgb),
    /// The window's size: sent once at the start and after every resize.
    /// Bubble Tea's `tea.WindowSizeMsg`.
    WindowSize(Size),
    /// A message of the model's own, from a [`Cmd`](crate::Cmd) or a
    /// [`Sender`](crate::Sender).
    App(M),
}

/// A terminal size in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Size {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

impl Size {
    /// A `width` by `height` size.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// What a model shows: Bubble Tea's `tea.View`. Make one from a string
/// with `.into()` or [`View::new`], and set the terminal modes it needs
/// with the `with_` methods. Modes are declared per frame: after each
/// frame the program turns on or off whatever changed, and turns
/// everything off when it ends. Bubble Tea's other `View` fields
/// (cursor, alternate screen, window title) come in later releases.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct View {
    /// The text to draw, one line per row; ANSI styles are allowed.
    pub content: String,
    /// Which mouse events to receive; none by default.
    pub mouse_mode: MouseMode,
    /// Whether to receive [`Msg::Focus`] and [`Msg::Blur`].
    pub report_focus: bool,
    /// Turns bracketed paste off, so pasted text arrives as keys. It is
    /// on by default, as in Bubble Tea.
    pub disable_bracketed_paste_mode: bool,
    /// Keyboard enhancements to ask for. Key disambiguation (the kitty
    /// protocol's first flag) is always asked for, as in Bubble Tea.
    pub keyboard_enhancements: KeyboardEnhancements,
}

impl View {
    /// A view of `content`. Bubble Tea's `tea.NewView`.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    /// Replaces the content. Bubble Tea's `View.SetContent`.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    /// The same view with mouse events of `mode`.
    #[must_use]
    pub fn with_mouse_mode(mut self, mode: MouseMode) -> Self {
        self.mouse_mode = mode;
        self
    }

    /// The same view with focus reports on or off.
    #[must_use]
    pub fn with_report_focus(mut self, on: bool) -> Self {
        self.report_focus = on;
        self
    }

    /// The same view with bracketed paste on (the default) or off.
    #[must_use]
    pub fn with_bracketed_paste(mut self, on: bool) -> Self {
        self.disable_bracketed_paste_mode = !on;
        self
    }

    /// The same view with these keyboard enhancements.
    #[must_use]
    pub fn with_keyboard_enhancements(mut self, enhancements: KeyboardEnhancements) -> Self {
        self.keyboard_enhancements = enhancements;
        self
    }
}

impl From<String> for View {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl From<&str> for View {
    fn from(content: &str) -> Self {
        Self::new(content)
    }
}

/// Why a program stopped without returning its model.
/// Bubble Tea's `ErrProgramKilled`, `ErrInterrupted` and `ErrProgramPanic`.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// [`Sender::kill`](crate::Sender::kill) stopped it, without a final
    /// frame.
    Killed,
    /// `SIGINT` or [`interrupt`](crate::interrupt) stopped it.
    Interrupted,
    /// A command panicked on its thread; this is the panic's message.
    Panic(String),
    /// The terminal failed.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Killed => f.write_str("program was killed"),
            Self::Interrupted => f.write_str("program was interrupted"),
            Self::Panic(message) => write!(f, "a command panicked: {message}"),
            Self::Io(err) => write!(f, "terminal error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_come_from_strings() {
        let mut view: View = "hello".into();
        assert_eq!(view, View::new(String::from("hello")));
        view.set_content("bye");
        assert_eq!(view.content, "bye");
    }

    #[test]
    fn errors_say_what_happened() {
        assert_eq!(Error::Killed.to_string(), "program was killed");
        assert_eq!(Error::Interrupted.to_string(), "program was interrupted");
        assert_eq!(
            Error::Panic("boom".into()).to_string(),
            "a command panicked: boom"
        );
        let io: Error = io::Error::other("gone").into();
        assert_eq!(io.to_string(), "terminal error: gone");
        assert!(std::error::Error::source(&io).is_some());
    }
}
