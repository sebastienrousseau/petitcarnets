//! Turning the bytes a terminal sends into input: a port of
//! ultraviolet's `EventDecoder` (the Unix paths, with Bubble Tea's
//! zero-value legacy flags and no terminfo) and of the scanner in its
//! `TerminalReader`, which waits for cut off sequences, flushes them after
//! [`ESC_TIMEOUT`], and gathers bracketed paste.
//!
//! [`decode`] is a pure function from bytes to one event and its length,
//! as Go's `EventDecoder.Decode`; [`Scanner`] and [`Reader`] add what
//! Go's reader adds on top.

mod control;
mod csi;
mod keys;
mod kitty;
mod mouse;
mod params;
mod scanner;
mod strings;
mod table;

use crate::backend::Input;
use crate::key::{Key, KeyCode, KeyMod};
pub(crate) use scanner::{ESC_TIMEOUT, Reader};

/// Escape.
const ESC: u8 = 0x1B;
/// The 8-bit single shift 3.
const SS3: u8 = 0x8F;
/// The 8-bit device control string introducer.
const DCS: u8 = 0x90;
/// The 8-bit start of string.
const SOS: u8 = 0x98;
/// The 8-bit control sequence introducer.
const CSI: u8 = 0x9B;
/// The 8-bit string terminator.
const ST: u8 = 0x9C;
/// The 8-bit operating system command.
const OSC: u8 = 0x9D;
/// The 8-bit privacy message.
const PM: u8 = 0x9E;
/// The 8-bit application program command.
const APC: u8 = 0x9F;

/// What one sequence decodes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    /// Input for the program.
    Input(Input),
    /// Go's `UnknownEvent`: a sequence cut off (wait for more bytes) or
    /// not understood.
    Unknown,
    /// A string sequence cancelled by CAN, SUB or a stray ESC.
    Ignored,
    /// Something Go decodes that is not delivered here, by Go's type
    /// name (device attributes, window reports, unknown CSI and others).
    Other(&'static str),
    /// A colour reply Go could not parse (Go's event with a nil colour),
    /// by Go's type name.
    NoColour(&'static str),
    /// Several events from one sequence.
    Multi(Vec<Event>),
}

/// A key press event.
fn press(key: Key) -> Event {
    Event::Input(Input::KeyPress(key))
}

/// A key press with modifiers.
fn press_code(code: KeyCode, mods: KeyMod) -> Event {
    press(Key::new(code).with_mods(mods))
}

/// The code Go gives the byte `b` taken as a rune.
fn byte_code(b: u8) -> KeyCode {
    KeyCode::from_rune(i64::from(b))
}

/// The input among `events`; the rest is read and dropped.
pub(crate) fn inputs(events: Vec<Event>) -> impl Iterator<Item = Input> {
    events.into_iter().filter_map(|event| match event {
        Event::Input(input) => Some(input),
        _ => None,
    })
}

/// A decoder for one kind of sequence.
type Parser = fn(&[u8]) -> (usize, Event);

/// The 8-bit introducers and their parsers.
const INTRODUCERS: [(u8, Parser); 8] = [
    (ESC, escape),
    (SS3, keys::ss3),
    (DCS, strings::dcs),
    (CSI, csi::parse),
    (OSC, strings::osc),
    (APC, strings::apc),
    (PM, strings::st_terminated),
    (SOS, strings::st_terminated),
];

/// The first event in `buf` and how many bytes it takes: Go's
/// `EventDecoder.Decode`. `buf` must not be empty.
pub(crate) fn decode(buf: &[u8]) -> (usize, Event) {
    if let Some((_, parse)) = INTRODUCERS.iter().find(|(b, _)| *b == buf[0]) {
        return parse(buf);
    }
    match buf[0] {
        b @ (0x00..=0x20 | 0x7F) => (1, control::key(b)),
        b @ 0x80..=0x9F => (
            1,
            press_code(byte_code(b - 0x40), KeyMod::CTRL | KeyMod::ALT),
        ),
        _ => control::text(buf),
    }
}

/// A sequence starting with ESC. A run of ESCs nests in Go (each one an
/// Alt on what follows); it is unrolled here so a long run cannot
/// overflow the stack.
fn escape(buf: &[u8]) -> (usize, Event) {
    let depth = buf.iter().skip(1).take_while(|&&b| b == ESC).count();
    let (mut n, mut event) = escape_once(&buf[depth..]);
    for _ in 0..depth {
        (n, event) = with_alt(n, event);
    }
    (n, event)
}

/// ESC followed by anything but another ESC.
fn escape_once(buf: &[u8]) -> (usize, Event) {
    let Some(&next) = buf.get(1) else {
        return (1, press_code(KeyCode::Esc, KeyMod::NONE));
    };
    match next {
        b'O' => keys::ss3(buf),
        b'P' => strings::dcs(buf),
        b'[' => csi::parse(buf),
        b']' => strings::osc(buf),
        b'_' => strings::apc(buf),
        b'^' | b'X' => strings::st_terminated(buf),
        _ => {
            let (n, event) = decode(&buf[1..]);
            with_alt(n, event)
        }
    }
}

/// ESC before `event`: Alt and the key, without text, if it is a key
/// press; else the Escape key alone.
fn with_alt(n: usize, event: Event) -> (usize, Event) {
    match event {
        Event::Input(Input::KeyPress(mut key)) => {
            key.text.clear();
            key.mods |= KeyMod::ALT;
            (n + 1, press(key))
        }
        _ => (1, press_code(KeyCode::Esc, KeyMod::NONE)),
    }
}

#[cfg(test)]
mod tests;
