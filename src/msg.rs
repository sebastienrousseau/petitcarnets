//! What a model receives, and the values around it.

use crate::key::Key;
use std::fmt;
use std::io;

/// A message for [`Model::update`](crate::Model::update): what the
/// terminal reported, or one of the model's own messages.
///
/// Bubble Tea's `tea.Msg`, as an enum: match it where Go uses a type
/// switch. More variants arrive as the runtime grows (mouse, paste,
/// focus), so keep a `_` arm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Msg<M> {
    /// A key was pressed. Bubble Tea's `tea.KeyPressMsg`; compare it with
    /// a name, as in `key == "ctrl+c"`.
    KeyPress(Key),
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
/// with `.into()` or [`View::new`]. Bubble Tea's other `View` fields
/// (cursor, alternate screen, mouse mode, window title) come in later
/// releases.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct View {
    /// The text to draw, one line per row; ANSI styles are allowed.
    pub content: String,
}

impl View {
    /// A view of `content`. Bubble Tea's `tea.NewView`.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    /// Replaces the content. Bubble Tea's `View.SetContent`.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
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
