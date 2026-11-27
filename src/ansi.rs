//! Reading text as a terminal reads it: grapheme clusters with their
//! widths, control characters, and escape sequences, one token at a
//! time, split exactly where Go's `ansi.DecodeSequence` and the
//! ultraviolet cell buffer split them.

use crate::unicode::{WidthMethod, first_width_cluster};

/// Escape.
const ESC: u8 = 0x1B;
/// Bell, which ends an OSC.
const BEL: u8 = 0x07;
/// Cancel and substitute, which abandon a string.
const CAN: u8 = 0x18;
const SUB: u8 = 0x1A;
/// The 8-bit string terminator.
const ST: u8 = 0x9C;

/// One piece of a string, as a terminal reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token<'a> {
    /// A grapheme cluster and the cells it takes (0 for a cluster of
    /// zero width, such as a lone combining mark or a C1 control).
    Text(&'a str, usize),
    /// A C0 control (`0x00` to `0x1F`, but not ESC) or DEL.
    Control(u8),
    /// A control sequence: what lies between `ESC [` and the final byte
    /// (private markers such as `?` and intermediates included), and the
    /// final byte.
    Csi {
        /// Parameters, private markers and intermediates.
        params: &'a str,
        /// The final byte.
        final_byte: u8,
    },
    /// An operating system command's data, after `ESC ]` and before its
    /// terminator (BEL or `ESC \`), or up to where it was cancelled or
    /// cut off, as Go still reads a hyperlink from those.
    Osc(&'a str),
    /// Any other escape sequence or string (DCS, APC, PM, SOS, `ESC x`),
    /// whole, or a control sequence cancelled or cut off.
    Other(&'a str),
}

/// The tokens of a string, in order.
#[derive(Debug, Clone)]
pub(crate) struct Tokens<'a> {
    rest: &'a str,
    method: WidthMethod,
}

/// Splits `s` into tokens, measuring text with `method`.
pub(crate) fn tokens(s: &str, method: WidthMethod) -> Tokens<'_> {
    Tokens { rest: s, method }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let first = *self.rest.as_bytes().first()?;
        let (token, len) = match first {
            ESC => escape(self.rest),
            0x20..=0x7E => self.ascii(),
            0x00..=0x1F | 0x7F => (Token::Control(first), 1),
            _ => {
                let (cluster, width) = first_width_cluster(self.rest, self.method);
                (Token::Text(cluster, width), cluster.len())
            }
        };
        self.rest = &self.rest[len..];
        Some(token)
    }
}

impl<'a> Tokens<'a> {
    /// A printable ASCII byte, which is a cluster on its own unless a
    /// non-ASCII character follows: then the cell buffer measures the
    /// whole cluster, so a combining mark stays with its base.
    fn ascii(&self) -> (Token<'a>, usize) {
        let s = self.rest;
        if s.as_bytes().get(1).is_some_and(|&b| b >= 0xC0) {
            let (cluster, width) = first_width_cluster(s, self.method);
            if cluster.len() > 1 {
                return (Token::Text(cluster, width), cluster.len());
            }
        }
        (Token::Text(&s[..1], 1), 1)
    }
}

/// A sequence that starts with ESC, and its length.
fn escape(s: &str) -> (Token<'_>, usize) {
    let mut len = escape_len(s.as_bytes());
    // An 8-bit terminator can be a continuation byte; Go then stops in
    // the middle of a character, which a `str` cannot, so the character
    // goes with the sequence.
    while !s.is_char_boundary(len) {
        len += 1;
    }
    let raw = &s[..len];
    let token = match raw.as_bytes()[1..] {
        [b'[', .., final_byte @ 0x40..=0x7E] => Token::Csi {
            params: &raw[2..len - 1],
            final_byte,
        },
        [b']', ..] => Token::Osc(osc_data(&raw[2..])),
        _ => Token::Other(raw),
    };
    (token, len)
}

/// The data of an OSC whose text after `ESC ]` is `raw`.
fn osc_data(raw: &str) -> &str {
    if let Some(data) = raw.strip_suffix('\u{7}') {
        return data;
    }
    if let Some(data) = raw.strip_suffix("\u{1B}\\") {
        return data;
    }
    // Ended by an 8-bit terminator inside the last character, or not
    // ended at all.
    match raw.bytes().position(|b| b == ST) {
        Some(at) => &raw[..floor_boundary(raw, at)],
        None => raw,
    }
}

/// The largest character boundary of `s` at or before `at`, as the
/// unstable `str::floor_char_boundary` finds it.
fn floor_boundary(s: &str, at: usize) -> usize {
    (0..=at).rev().find(|&i| s.is_char_boundary(i)).unwrap_or(0)
}

/// The length of the sequence at the start of `b`, which starts with
/// ESC: Go's `decodeSequence` from its escape state. A sequence broken by
/// an unexpected byte ends before it; one cut off runs to the end.
fn escape_len(b: &[u8]) -> usize {
    for (at, &c) in b.iter().enumerate().skip(1) {
        match c {
            b'[' | b'P' => return control_len(b, at + 1),
            b']' | b'X' | b'^' | b'_' => return string_len(b, at + 1),
            0x20..=0x2F => {}
            0x30..=0x7E => return at + 1,
            _ => return at,
        }
    }
    b.len()
}

/// Where the bytes of `b` from `from` stop matching `accept`.
fn skip(b: &[u8], from: usize, accept: fn(u8) -> bool) -> usize {
    b[from..]
        .iter()
        .position(|&c| !accept(c))
        .map_or(b.len(), |i| from + i)
}

/// A private marker: `<`, `=`, `>` or `?`.
fn is_marker(c: u8) -> bool {
    (b'<'..=b'?').contains(&c)
}

/// A parameter byte: a digit, or `;` or `:` between parameters.
fn is_parameter(c: u8) -> bool {
    c.is_ascii_digit() || c == b';' || c == b':'
}

/// An intermediate byte, space to `/`.
fn is_intermediate(c: u8) -> bool {
    (0x20..=0x2F).contains(&c)
}

/// The length of a CSI or DCS whose body starts at `from`: private
/// markers, parameters, intermediates, then a final byte. A DCS (`ESC P`)
/// goes on with its data string.
fn control_len(b: &[u8], from: usize) -> usize {
    let at = skip(b, from, is_marker);
    let at = skip(b, at, is_parameter);
    let at = skip(b, at, is_intermediate);
    match b.get(at) {
        Some(0x40..=0x7E) if b[1] == b'P' => string_len(b, at + 1),
        Some(0x40..=0x7E) => at + 1,
        Some(_) => at,
        None => b.len(),
    }
}

/// The length of a string (OSC, DCS, SOS, PM or APC) whose data starts
/// at `from`: ST ends it, and BEL ends an OSC; CAN, SUB or an ESC not
/// starting ST abandon it, and are not part of it.
fn string_len(b: &[u8], from: usize) -> usize {
    let osc = b[1] == b']';
    for (at, &c) in b.iter().enumerate().skip(from) {
        match c {
            BEL if osc => return at + 1,
            ST => return at + 1,
            ESC if b.get(at + 1) == Some(&b'\\') => return at + 2,
            CAN | SUB | ESC => return at,
            _ => {}
        }
    }
    b.len()
}

/// The tokens of `s`, each with the text it was read from.
pub(crate) fn spans(s: &str, method: WidthMethod) -> impl Iterator<Item = (Token<'_>, &str)> {
    let mut tokens = tokens(s, method);
    std::iter::from_fn(move || {
        let before = tokens.rest;
        let token = tokens.next()?;
        Some((token, &before[..before.len() - tokens.rest.len()]))
    })
}
