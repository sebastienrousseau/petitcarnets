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
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::TestProgram;
///
/// struct Notes(String, u16);
///
/// impl tea::Model for Notes {
///     type Msg = ();
///
///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
///         match msg {
///             tea::Msg::KeyPress(key) if key == "ctrl+c" => return tea::quit(),
///             tea::Msg::Paste(text) => self.0.push_str(&text),
///             tea::Msg::WindowSize(size) => self.1 = size.width,
///             _ => {}
///         }
///         tea::Cmd::none()
///     }
/// #
/// #     fn view(&self) -> tea::View {
/// #         tea::View::default()
/// #     }
/// }
///
/// let mut test = TestProgram::new(Notes(String::new(), 0), 20, 2);
/// assert_eq!(test.model().1, 20);
/// test.bytes("\x1b[200~hello\x1b[201~");
/// assert_eq!(test.model().0, "hello");
/// test.press(tea::Key::ctrl('c'));
/// assert!(test.is_finished());
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::ClipboardSelection;
///
/// assert_eq!(ClipboardSelection::from(b'c'), ClipboardSelection::System);
/// assert_eq!(
///     ClipboardSelection::from(b'q'),
///     ClipboardSelection::Other(b'q')
/// );
/// assert_eq!(u8::from(ClipboardSelection::Primary), b'p');
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::TestProgram;
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
///         self.0.push(msg);
///         tea::Cmd::none()
///     }
/// #
/// #     fn view(&self) -> tea::View {
/// #         tea::View::default()
/// #     }
/// }
///
/// let mut test = TestProgram::new(Log(Vec::new()), 20, 1);
/// // The terminal says it shows a dark scheme.
/// test.bytes("\x1b[?997;1n");
/// let dark = tea::Msg::ColorScheme(tea::ColorScheme::Dark);
/// assert_eq!(test.model().0.last(), Some(&dark));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    /// Light text on a dark background.
    Dark,
    /// Dark text on a light background.
    Light,
}

/// A terminal size in cells.
///
/// ```
/// use petitcarnets as tea;
/// use tea::Size;
///
/// let size = Size::new(80, 24);
/// assert_eq!((size.width, size.height), (80, 24));
/// assert_eq!(Size::default(), Size::new(0, 0));
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::TestProgram;
///
/// struct Idle;
///
/// impl tea::Model for Idle {
///     type Msg = ();
///
///     fn update(&mut self, _: tea::Msg<()>) -> tea::Cmd<()> {
///         tea::Cmd::none()
///     }
///
///     fn view(&self) -> tea::View {
///         "idle".into()
///     }
/// }
///
/// let mut test = TestProgram::new(Idle, 20, 1);
/// test.interrupt();
/// let err = test.into_result().err();
/// assert!(matches!(err, Some(tea::Error::Interrupted)));
/// assert_eq!(
///     err.map(|e| e.to_string()).as_deref(),
///     Some("program was interrupted")
/// );
/// ```
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
