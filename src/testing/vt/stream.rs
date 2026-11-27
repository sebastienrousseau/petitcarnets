//! Cutting fed bytes into what can be read now and what must wait for
//! the next feed: a code point or an escape sequence cut in two.

use crate::ansi;
use crate::unicode::WidthMethod;

/// Escape.
const ESC: u8 = 0x1B;
/// Bell, which may end an OSC.
const BEL: u8 = 0x07;
/// What an invalid byte sequence reads as.
const REPLACEMENT: char = '\u{FFFD}';

/// `bytes` as text, each invalid sequence turned into U+FFFD, and the
/// bytes of a code point cut off at the end, kept for later.
pub(super) fn split_utf8(bytes: &[u8]) -> (String, Vec<u8>) {
    let mut text = String::new();
    let mut held = Vec::new();
    let mut read = 0;
    for chunk in bytes.utf8_chunks() {
        text.push_str(chunk.valid());
        let bad = chunk.invalid();
        read += chunk.valid().len() + bad.len();
        if bad.is_empty() {
            continue;
        }
        if read == bytes.len() && is_prefix(bad) {
            held = bad.to_vec();
        } else {
            text.push(REPLACEMENT);
        }
    }
    (text, held)
}

/// Whether `bad` is the start of a valid code point, cut short.
fn is_prefix(bad: &[u8]) -> bool {
    std::str::from_utf8(bad).is_err_and(|e| e.error_len().is_none())
}

/// How much of `text` can be read now: all of it but a trailing run of
/// escape sequences that have not ended yet.
pub(super) fn complete_len(text: &str, method: WidthMethod) -> usize {
    let mut cut = None;
    let mut at = 0;
    for (_, span) in ansi::spans(text, method) {
        if is_complete(span.as_bytes()) {
            cut = None;
        } else if cut.is_none() {
            cut = Some(at);
        }
        at += span.len();
    }
    cut.unwrap_or(text.len())
}

/// Whether the token read from `b` is whole.
fn is_complete(b: &[u8]) -> bool {
    match (b.first(), b.last()) {
        (Some(&ESC), Some(&last)) => escape_complete(b, last),
        _ => true,
    }
}

/// Whether the escape sequence or string read from `b`, ending in
/// `last`, is whole.
fn escape_complete(b: &[u8], last: u8) -> bool {
    let ended = b.len() > 3 && b.ends_with(b"\x1b\\");
    match b.get(1) {
        None => false,
        Some(b'[') => b.len() > 2 && (0x40..=0x7E).contains(&last),
        Some(b']') => last == BEL || ended,
        Some(b'P' | b'_' | b'^' | b'X') => ended,
        Some(_) => !(0x20..=0x2F).contains(&last),
    }
}
