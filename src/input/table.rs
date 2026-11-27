//! Go's legacy key table (`buildKeysTable`, without terminfo), as a
//! lookup instead of a map. The reader looks a whole read up in it before
//! decoding, and an unknown sequence after the escape timeout. It mostly
//! agrees with the decoder, but not always: `CSI 1 ; 5 R` alone is Ctrl
//! and F3 here, where the decoder also reports a cursor position.

use super::keys::{CSI_FINALS, SS3_FINALS, final_key, tilde_code};
use super::{ESC, Event, control};
use crate::backend::Input;
use crate::key::{Key, KeyCode, KeyMod};

/// rxvt-unicode's suffixes on `CSI n`: the plain key, then Shift, Ctrl, and
/// Ctrl with Shift.
const URXVT: [(u8, KeyMod); 4] = [
    (b'~', KeyMod::NONE),
    (b'$', KeyMod::SHIFT),
    (b'^', KeyMod::CTRL),
    (
        b'@',
        KeyMod::from_bits(KeyMod::CTRL.bits() | KeyMod::SHIFT.bits()),
    ),
];

/// The modifyOtherKeys codes the table knows.
const MODIFY_OTHER_KEYS: [(&str, KeyCode); 5] = [
    ("8", KeyCode::Backspace),
    ("9", KeyCode::Tab),
    ("13", KeyCode::Enter),
    ("27", KeyCode::Esc),
    ("127", KeyCode::Backspace),
];

/// The key the table has for exactly `seq`.
pub(super) fn lookup(seq: &[u8]) -> Option<Key> {
    base(seq).or_else(|| alt(seq)).or_else(|| modified(seq))
}

/// The VT100, VT220 and rxvt-unicode keys.
fn base(seq: &[u8]) -> Option<Key> {
    match seq {
        [b @ (0x00..=0x20 | 0x7F)] => match control::key(*b) {
            Event::Input(Input::KeyPress(key)) => Some(key),
            _ => None,
        },
        [ESC, b'[', rest @ ..] => csi(rest),
        [ESC, b'O', g] => final_key(&SS3_FINALS, *g),
        _ => None,
    }
}

/// `CSI x` and `CSI n ~` with rxvt-unicode's suffixes.
fn csi(rest: &[u8]) -> Option<Key> {
    if let [f] = rest {
        return final_key(&CSI_FINALS, *f);
    }
    let (last, digits) = rest.split_last()?;
    let (_, mods) = URXVT.iter().find(|(s, _)| s == last)?;
    Some(Key::new(tilde_key(digits)?).with_mods(*mods))
}

/// ESC before a base key: Alt and that key, without text.
fn alt(seq: &[u8]) -> Option<Key> {
    let mut key = base(seq.strip_prefix(&[ESC])?)?;
    key.text.clear();
    key.mods |= KeyMod::ALT;
    Some(key)
}

/// xterm's modified keys, with a modifier parameter from 2 to 16:
/// `CSI 1 ; m x`, `SS3 m x` (keypad only), `CSI n ; m ~` and
/// `CSI 27 ; m ; code ~`.
fn modified(seq: &[u8]) -> Option<Key> {
    let (&last, body) = seq.split_last()?;
    let (key, m) = match body.strip_prefix(b"\x1bO") {
        Some(m) => (final_key(&SS3_FINALS[15..], last)?, m),
        None => csi_modified(body.strip_prefix(b"\x1b[")?, last)?,
    };
    Some(key.with_mods(modifier(m)?))
}

/// The key and the modifier parameter of `CSI 1 ; m x`, `CSI n ; m ~`
/// or `CSI 27 ; m ; code ~`, given what follows `CSI` and the last byte.
fn csi_modified(body: &[u8], last: u8) -> Option<(Key, &[u8])> {
    let parts: Vec<&[u8]> = body.split(|&b| b == b';').collect();
    match (parts.as_slice(), last) {
        ([b"1", m], f) if f != b'~' => Some((final_key(&CSI_FINALS[4..15], f)?, m)),
        ([b"27", m, code], b'~') => Some((Key::new(other_key(code)?), m)),
        ([n, m], b'~') => Some((Key::new(tilde_key(n)?), m)),
        _ => None,
    }
}

/// The key of a modifyOtherKeys code the table knows.
fn other_key(code: &[u8]) -> Option<KeyCode> {
    let (_, named) = MODIFY_OTHER_KEYS
        .iter()
        .find(|(c, _)| c.as_bytes() == code)?;
    Some(*named)
}

/// The key of a `CSI n ~` number written as the table writes it.
fn tilde_key(digits: &[u8]) -> Option<KeyCode> {
    tilde_code(number(digits)?)
}

/// xterm's modifier parameter, 2 to 16, as modifiers (one less).
fn modifier(m: &[u8]) -> Option<KeyMod> {
    let m = number(m).filter(|m| (2..=16).contains(m))?;
    Some(KeyMod::from_bits(m - 1))
}

/// A decimal number written without leading zeros.
fn number(digits: &[u8]) -> Option<i64> {
    let leading_zero = digits.len() > 1 && digits[0] == b'0';
    if digits.is_empty() || leading_zero || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}
