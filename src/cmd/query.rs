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
#[must_use]
pub const fn request_window_size<M>() -> Cmd<M> {
    Cmd(Inner::Output(Output::WindowSize))
}

/// Asks where the cursor is; the reply is
/// [`Msg::CursorPosition`](crate::Msg::CursorPosition). Bubble Tea's
/// `tea.RequestCursorPosition`.
#[must_use]
pub fn request_cursor_position<M>() -> Cmd<M> {
    query(b"\x1b[6n".as_slice())
}

/// Asks for the terminal's background colour; the reply is
/// [`Msg::BackgroundColor`](crate::Msg::BackgroundColor). Bubble Tea's
/// `tea.RequestBackgroundColor`.
#[must_use]
pub fn request_background_color<M>() -> Cmd<M> {
    query(b"\x1b]11;?\x07".as_slice())
}

/// Asks for the terminal's text colour; the reply is
/// [`Msg::ForegroundColor`](crate::Msg::ForegroundColor). Bubble Tea's
/// `tea.RequestForegroundColor`.
#[must_use]
pub fn request_foreground_color<M>() -> Cmd<M> {
    query(b"\x1b]10;?\x07".as_slice())
}

/// Asks for the cursor's colour; the reply is
/// [`Msg::CursorColor`](crate::Msg::CursorColor). Bubble Tea's
/// `tea.RequestCursorColor`.
#[must_use]
pub fn request_cursor_color<M>() -> Cmd<M> {
    query(b"\x1b]12;?\x07".as_slice())
}

/// Asks for the terminal's name and version (`XTVERSION`); the reply is
/// [`Msg::TerminalVersion`](crate::Msg::TerminalVersion). Bubble Tea's
/// `tea.RequestTerminalVersion`.
#[must_use]
pub fn request_terminal_version<M>() -> Cmd<M> {
    query(b"\x1b[>q".as_slice())
}

/// Asks for a terminfo capability by name (`XTGETTCAP`), such as `RGB`,
/// `Tc` or `Ms`; the reply is [`Msg::Capability`](crate::Msg::Capability),
/// and none comes for a capability the terminal does not know. Bubble
/// Tea's `tea.RequestCapability`.
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
#[must_use]
pub fn set_clipboard<M>(text: &str) -> Cmd<M> {
    Cmd(Inner::Output(Output::Raw(osc52(b'c', text))))
}

/// Copies `text` to the primary selection (OSC 52). Bubble Tea's
/// `tea.SetPrimaryClipboard`.
#[must_use]
pub fn set_primary_clipboard<M>(text: &str) -> Cmd<M> {
    Cmd(Inner::Output(Output::Raw(osc52(b'p', text))))
}

/// Asks for the system clipboard's text; the reply, where the terminal
/// allows it, is [`Msg::Clipboard`](crate::Msg::Clipboard). Bubble Tea's
/// `tea.ReadClipboard`.
#[must_use]
pub fn read_clipboard<M>() -> Cmd<M> {
    query(b"\x1b]52;c;?\x07".as_slice())
}

/// Asks for the primary selection's text; the reply is
/// [`Msg::Clipboard`](crate::Msg::Clipboard). Bubble Tea's
/// `tea.ReadPrimaryClipboard`.
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
