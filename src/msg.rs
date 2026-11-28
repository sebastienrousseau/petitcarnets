//! What a model receives, and the values around it.

use crate::color::{ColorProfile, Rgb};
use crate::key::Key;
use crate::mode::{KeyboardEnhancementsReport, ModeSetting, ReportedMode};
use crate::mouse::Mouse;
use std::fmt;
use std::io;

mod route;

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
    /// [`KeyboardEnhancements::report_event_types`](crate::KeyboardEnhancements::report_event_types) on a terminal that
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
    /// The terminal gained focus (with [`View::report_focus`](crate::View::report_focus)). Bubble
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
    /// The window's size: sent once at the start, after every resize,
    /// after [`request_window_size`](crate::request_window_size), and when
    /// the terminal reports it (`CSI 8 ; rows ; columns t`). Bubble Tea's
    /// `tea.WindowSizeMsg`.
    WindowSize(Size),
    /// The window's size in pixels, when the terminal reports it
    /// (`CSI 4 ; height ; width t`). Ultraviolet's `PixelSizeEvent`,
    /// which Bubble Tea passes on as it is.
    PixelSize(Size),
    /// The size of one cell in pixels, when the terminal reports it
    /// (`CSI 6 ; height ; width t`). Ultraviolet's `CellSizeEvent`.
    CellSize(Size),
    /// The colour profile: sent once at the start, and again when the
    /// terminal says it shows 24-bit colour (see [`Msg::Capability`]).
    /// Bubble Tea's `tea.ColorProfileMsg`.
    ColorProfile(ColorProfile),
    /// The program was suspended and has resumed, or was stopped from
    /// outside and continued; the terminal is set up and drawn again.
    /// Bubble Tea's `tea.ResumeMsg`.
    Resume,
    /// The clipboard's content, after [`read_clipboard`](crate::read_clipboard)
    /// or [`read_primary_clipboard`](crate::read_primary_clipboard).
    /// Bubble Tea's `tea.ClipboardMsg`.
    Clipboard {
        /// The text.
        content: String,
        /// Which clipboard it came from.
        selection: ClipboardSelection,
    },
    /// A terminal capability, after
    /// [`request_capability`](crate::request_capability): `name=value`,
    /// or the name alone for a flag. A reply of `RGB` or `Tc` also
    /// upgrades the colour profile to 24-bit, unless
    /// [`Program::with_color_profile`](crate::Program::with_color_profile)
    /// set it. Bubble Tea's `tea.CapabilityMsg`.
    Capability(String),
    /// The terminal's primary device attributes (DA1, `CSI ? ... c`):
    /// its class and features. Ultraviolet's
    /// `PrimaryDeviceAttributesEvent`, which Bubble Tea passes on as it
    /// is; ask with [`raw`](crate::raw)`("\x1b[c")`.
    PrimaryDeviceAttributes(Vec<i64>),
    /// The terminal's secondary device attributes (DA2, `CSI > ... c`):
    /// its type, version and options. Ultraviolet's
    /// `SecondaryDeviceAttributesEvent`.
    SecondaryDeviceAttributes(Vec<i64>),
    /// The terminal's unit id (DA3, `DCS ! | ... ST`). Ultraviolet's
    /// `TertiaryDeviceAttributesEvent`.
    TertiaryDeviceAttributes(String),
    /// The terminal's colour scheme, dark or light (`CSI ? 997 ; n n`).
    /// Ultraviolet's `DarkColorSchemeEvent` and `LightColorSchemeEvent`.
    ColorScheme(ColorScheme),
    /// A message of the model's own, from a [`Cmd`](crate::Cmd) or a
    /// [`Sender`](crate::Sender).
    App(M),
}

/// Which clipboard a [`Msg::Clipboard`] came from: the OSC 52 selection
/// byte, as Bubble Tea's `ClipboardSelection`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipboardSelection {
    /// The system clipboard (`c`).
    System,
    /// The primary selection (`p`).
    Primary,
    /// Another selection byte, or 0 when the reply named none.
    Other(u8),
}

impl From<u8> for ClipboardSelection {
    fn from(byte: u8) -> Self {
        match byte {
            b'c' => Self::System,
            b'p' => Self::Primary,
            other => Self::Other(other),
        }
    }
}

impl From<ClipboardSelection> for u8 {
    fn from(selection: ClipboardSelection) -> Self {
        match selection {
            ClipboardSelection::System => b'c',
            ClipboardSelection::Primary => b'p',
            ClipboardSelection::Other(byte) => byte,
        }
    }
}

/// A terminal's colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    /// Light text on a dark background.
    Dark,
    /// Dark text on a light background.
    Light,
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
