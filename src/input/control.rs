//! Control characters and text. Go's `parseControl` and `parseUtf8`.

use super::{Event, press, press_code};
use crate::key::{Key, KeyCode, KeyMod};
use crate::unicode::{REPLACEMENT, decode_rune, first_cluster};

/// Control characters with a name of their own, Go's default legacy
/// mapping: NUL is Ctrl and Space, BS is Ctrl and `h`, HT is Tab, CR is
/// Enter, ESC is Escape, DEL is Backspace.
const NAMED: [(u8, KeyCode, KeyMod); 6] = [
    (0x00, KeyCode::Space, KeyMod::CTRL),
    (0x08, KeyCode::Char('h'), KeyMod::CTRL),
    (0x09, KeyCode::Tab, KeyMod::NONE),
    (0x0D, KeyCode::Enter, KeyMod::NONE),
    (0x1B, KeyCode::Esc, KeyMod::NONE),
    (0x7F, KeyCode::Backspace, KeyMod::NONE),
];

/// The key a C0 control character, space or DEL is: a named key, or
/// Ctrl and a letter (`0x01` is Ctrl and `a`) or symbol (`0x1C` is Ctrl
/// and `\`).
pub(super) fn key(b: u8) -> Event {
    if b == b' ' {
        return press(Key::char(' '));
    }
    if let Some((_, code, mods)) = NAMED.iter().find(|n| n.0 == b) {
        return press_code(*code, *mods);
    }
    let offset = if b <= 0x1A { 0x60 } else { 0x40 };
    if b <= 0x1F {
        return press_code(KeyCode::Char(char::from(b + offset)), KeyMod::CTRL);
    }
    Event::Unknown
}

/// Text: one printable ASCII character, or one grapheme cluster, which
/// is a single key with [`KeyCode::Extended`] when it has more than one
/// code point. Bytes that are not UTF-8 are unknown, one at a time.
pub(super) fn text(b: &[u8]) -> (usize, Event) {
    let c = b[0];
    if c <= 0x1F || c == 0x7F {
        return (1, key(c));
    }
    if c < 0x7F {
        return (1, press(Key::char(char::from(c))));
    }
    let (first, _) = decode_rune(b);
    if first == REPLACEMENT {
        return (1, Event::Unknown);
    }
    let n = first_cluster(b);
    let text = String::from_utf8_lossy(&b[..n]).into_owned();
    let code = if text.chars().nth(1).is_some() {
        KeyCode::Extended
    } else {
        KeyCode::from_rune(i64::from(u32::from(first)))
    };
    let mut key = Key::new(code);
    key.text = text;
    (n, press(key))
}
