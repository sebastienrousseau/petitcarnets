//! Commands that ask the terminal something, and the clipboard. Replies
//! arrive as messages whenever the terminal sends them; nothing waits
//! for one, and a terminal that never answers costs nothing but, at the
//! end, a short bounded wait (see [`Program::run`](crate::Program::run)).

use super::{Cmd, Inner, Output};
use std::fmt::Write as _;

/// A query: bytes for the terminal whose reply is read before the
/// terminal is handed back.
fn query<M>(bytes: impl Into<Vec<u8>>) -> Cmd<M> {
    Cmd(Inner::Output(Output::Query(bytes.into())))
}

/// Sends the window's size as [`Msg::WindowSize`](crate::Msg::WindowSize),
/// read from the terminal now. Bubble Tea's `tea.RequestWindowSize`.
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
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_window_size()
///     }
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
/// let test = TestProgram::new(Log(Vec::new()), 20, 1);
/// let size = tea::Msg::WindowSize(tea::Size::new(20, 1));
/// // Once at the start, and once more when asked.
/// assert_eq!(test.model().0.iter().filter(|m| **m == size).count(), 2);
/// ```
#[must_use]
pub const fn request_window_size<M>() -> Cmd<M> {
    Cmd(Inner::Output(Output::WindowSize))
}

/// Asks where the cursor is; the reply is
/// [`Msg::CursorPosition`](crate::Msg::CursorPosition). Bubble Tea's
/// `tea.RequestCursorPosition`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_cursor_position()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b[6n", "\x1b[3;7R");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let position = tea::Msg::CursorPosition { x: 6, y: 2 };
/// assert!(test.model().0.contains(&position));
/// ```
#[must_use]
pub fn request_cursor_position<M>() -> Cmd<M> {
    query(b"\x1b[6n".as_slice())
}

/// Asks for the terminal's background colour; the reply is
/// [`Msg::BackgroundColor`](crate::Msg::BackgroundColor). Bubble Tea's
/// `tea.RequestBackgroundColor`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_background_color()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b]11;?\x07", "\x1b]11;rgb:1a1a/1b1b/2c2c\x07");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let color = tea::Msg::BackgroundColor(tea::Rgb::new(0x1a, 0x1b, 0x2c));
/// assert!(test.model().0.contains(&color));
/// ```
#[must_use]
pub fn request_background_color<M>() -> Cmd<M> {
    query(b"\x1b]11;?\x07".as_slice())
}

/// Asks for the terminal's text colour; the reply is
/// [`Msg::ForegroundColor`](crate::Msg::ForegroundColor). Bubble Tea's
/// `tea.RequestForegroundColor`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_foreground_color()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b]10;?\x07", "\x1b]10;rgb:ffff/ffff/ffff\x07");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let color = tea::Msg::ForegroundColor(tea::Rgb::new(255, 255, 255));
/// assert!(test.model().0.contains(&color));
/// ```
#[must_use]
pub fn request_foreground_color<M>() -> Cmd<M> {
    query(b"\x1b]10;?\x07".as_slice())
}

/// Asks for the cursor's colour; the reply is
/// [`Msg::CursorColor`](crate::Msg::CursorColor). Bubble Tea's
/// `tea.RequestCursorColor`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_cursor_color()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b]12;?\x07", "\x1b]12;rgb:ff00/8000/0000\x07");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let color = tea::Msg::CursorColor(tea::Rgb::new(255, 128, 0));
/// assert!(test.model().0.contains(&color));
/// ```
#[must_use]
pub fn request_cursor_color<M>() -> Cmd<M> {
    query(b"\x1b]12;?\x07".as_slice())
}

/// Asks for the terminal's name and version (`XTVERSION`); the reply is
/// [`Msg::TerminalVersion`](crate::Msg::TerminalVersion). Bubble Tea's
/// `tea.RequestTerminalVersion`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_terminal_version()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b[>q", "\x1bP>|kitty(0.40.0)\x1b\\");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let version = tea::Msg::TerminalVersion("kitty(0.40.0)".into());
/// assert!(test.model().0.contains(&version));
/// ```
#[must_use]
pub fn request_terminal_version<M>() -> Cmd<M> {
    query(b"\x1b[>q".as_slice())
}

/// Asks for a terminfo capability by name (`XTGETTCAP`), such as `RGB`,
/// `Tc` or `Ms`; the reply is [`Msg::Capability`](crate::Msg::Capability),
/// and none comes for a capability the terminal does not know. Bubble
/// Tea's `tea.RequestCapability`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::request_capability("RGB")
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1bP+q524742\x1b\\", "\x1bP1+r524742\x1b\\");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// assert!(test.model().0.contains(&tea::Msg::Capability("RGB".into())));
/// let profile = tea::Msg::ColorProfile(tea::ColorProfile::TrueColor);
/// assert!(test.model().0.contains(&profile));
/// ```
#[must_use]
pub fn request_capability<M>(name: &str) -> Cmd<M> {
    let mut bytes = String::from("\x1bP+q");
    for b in name.bytes() {
        let _ = write!(bytes, "{b:02X}");
    }
    bytes.push_str("\x1b\\");
    query(bytes)
}

/// Copies `text` to the system clipboard (OSC 52), where the terminal
/// allows it. Bubble Tea's `tea.SetClipboard`.
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
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::set_clipboard("hi")
///     }
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
/// let test = TestProgram::new(Log(Vec::new()), 20, 1);
/// let output = String::from_utf8_lossy(test.output());
/// assert!(output.contains("\x1b]52;c;aGk=\x07"));
/// ```
#[must_use]
pub fn set_clipboard<M>(text: &str) -> Cmd<M> {
    Cmd(Inner::Output(Output::Raw(osc52(b'c', text))))
}

/// Copies `text` to the primary selection (OSC 52). Bubble Tea's
/// `tea.SetPrimaryClipboard`.
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
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::set_primary_clipboard("hi")
///     }
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
/// let test = TestProgram::new(Log(Vec::new()), 20, 1);
/// let output = String::from_utf8_lossy(test.output());
/// assert!(output.contains("\x1b]52;p;aGk=\x07"));
/// ```
#[must_use]
pub fn set_primary_clipboard<M>(text: &str) -> Cmd<M> {
    Cmd(Inner::Output(Output::Raw(osc52(b'p', text))))
}

/// Asks for the system clipboard's text; the reply, where the terminal
/// allows it, is [`Msg::Clipboard`](crate::Msg::Clipboard). Bubble Tea's
/// `tea.ReadClipboard`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::read_clipboard()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b]52;c;?\x07", "\x1b]52;c;aGk=\x07");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let content = "hi".to_string();
/// let selection = tea::ClipboardSelection::System;
/// let clipboard = tea::Msg::Clipboard { content, selection };
/// assert!(test.model().0.contains(&clipboard));
/// ```
#[must_use]
pub fn read_clipboard<M>() -> Cmd<M> {
    query(b"\x1b]52;c;?\x07".as_slice())
}

/// Asks for the primary selection's text; the reply is
/// [`Msg::Clipboard`](crate::Msg::Clipboard). Bubble Tea's
/// `tea.ReadPrimaryClipboard`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::{TestProgram, TestTerminal};
///
/// /// Keeps every message it gets.
/// struct Log(Vec<tea::Msg<()>>);
///
/// impl tea::Model for Log {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::read_primary_clipboard()
///     }
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
/// let terminal = TestTerminal::new(20, 1).reply("\x1b]52;p;?\x07", "\x1b]52;p;aGk=\x07");
/// let test = TestProgram::with_terminal(tea::Program::new(Log(Vec::new())), terminal);
/// let content = "hi".to_string();
/// let selection = tea::ClipboardSelection::Primary;
/// let clipboard = tea::Msg::Clipboard { content, selection };
/// assert!(test.model().0.contains(&clipboard));
/// ```
#[must_use]
pub fn read_primary_clipboard<M>() -> Cmd<M> {
    query(b"\x1b]52;p;?\x07".as_slice())
}

/// OSC 52 setting `selection` to `text`, as Go's `ansi.SetClipboard`
/// writes it: empty text clears it.
fn osc52(selection: u8, text: &str) -> Vec<u8> {
    let data = crate::base64::encode(text.as_bytes());
    let mut out = format!("\x1b]52;{};", char::from(selection)).into_bytes();
    out.extend_from_slice(data.as_bytes());
    out.push(0x07);
    out
}
