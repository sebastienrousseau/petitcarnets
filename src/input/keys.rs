//! The legacy key sequences shared by the decoder and the key table:
//! CSI and SS3 final bytes, and the numbers of `CSI n ~` keys.

use super::{ESC, Event, press};
use crate::key::{Key, KeyCode, KeyMod};

/// A key a final byte names, with the modifier the byte implies.
type Final = (u8, KeyCode, KeyMod);

/// `CSI x`: cursor keys, F1 to F4, and back tab. Lower-case arrows are
/// rxvt-unicode's shifted ones.
pub(super) const CSI_FINALS: [Final; 16] = {
    use KeyCode as K;
    const N: KeyMod = KeyMod::NONE;
    const S: KeyMod = KeyMod::SHIFT;
    [
        (b'a', K::Up, S),
        (b'b', K::Down, S),
        (b'c', K::Right, S),
        (b'd', K::Left, S),
        (b'A', K::Up, N),
        (b'B', K::Down, N),
        (b'C', K::Right, N),
        (b'D', K::Left, N),
        (b'E', K::Begin, N),
        (b'F', K::End, N),
        (b'H', K::Home, N),
        (b'P', K::F(1), N),
        (b'Q', K::F(2), N),
        (b'R', K::F(3), N),
        (b'S', K::F(4), N),
        (b'Z', K::Tab, S),
    ]
};

/// `SS3 x`: cursor keys, F1 to F4 and the keypad. Lower-case arrows are
/// rxvt-unicode's Ctrl ones.
pub(super) const SS3_FINALS: [Final; 33] = {
    use KeyCode as K;
    const N: KeyMod = KeyMod::NONE;
    const C: KeyMod = KeyMod::CTRL;
    [
        (b'a', K::Up, C),
        (b'b', K::Down, C),
        (b'c', K::Right, C),
        (b'd', K::Left, C),
        (b'A', K::Up, N),
        (b'B', K::Down, N),
        (b'C', K::Right, N),
        (b'D', K::Left, N),
        (b'E', K::Begin, N),
        (b'F', K::End, N),
        (b'H', K::Home, N),
        (b'P', K::F(1), N),
        (b'Q', K::F(2), N),
        (b'R', K::F(3), N),
        (b'S', K::F(4), N),
        (b'M', K::KpEnter, N),
        (b'X', K::KpEqual, N),
        (b'j', K::KpMultiply, N),
        (b'k', K::KpPlus, N),
        (b'l', K::KpComma, N),
        (b'm', K::KpMinus, N),
        (b'n', K::KpDecimal, N),
        (b'o', K::KpDivide, N),
        (b'p', K::Kp0, N),
        (b'q', K::Kp1, N),
        (b'r', K::Kp2, N),
        (b's', K::Kp3, N),
        (b't', K::Kp4, N),
        (b'u', K::Kp5, N),
        (b'v', K::Kp6, N),
        (b'w', K::Kp7, N),
        (b'x', K::Kp8, N),
        (b'y', K::Kp9, N),
    ]
};

/// The numbers of `CSI n ~` keys (Find and Select read as Home and End,
/// Go's default).
pub(super) const TILDE: [(i64, KeyCode); 28] = {
    use KeyCode as K;
    [
        (1, K::Home),
        (2, K::Insert),
        (3, K::Delete),
        (4, K::End),
        (5, K::PgUp),
        (6, K::PgDown),
        (7, K::Home),
        (8, K::End),
        (11, K::F(1)),
        (12, K::F(2)),
        (13, K::F(3)),
        (14, K::F(4)),
        (15, K::F(5)),
        (17, K::F(6)),
        (18, K::F(7)),
        (19, K::F(8)),
        (20, K::F(9)),
        (21, K::F(10)),
        (23, K::F(11)),
        (24, K::F(12)),
        (25, K::F(13)),
        (26, K::F(14)),
        (28, K::F(15)),
        (29, K::F(16)),
        (31, K::F(17)),
        (32, K::F(18)),
        (33, K::F(19)),
        (34, K::F(20)),
    ]
};

/// The key `byte` names in `table`.
pub(super) fn final_key(table: &[Final], byte: u8) -> Option<Key> {
    let (_, code, mods) = table.iter().find(|f| f.0 == byte)?;
    Some(Key::new(*code).with_mods(*mods))
}

/// The key of `CSI n ~`.
pub(super) fn tilde_code(n: i64) -> Option<KeyCode> {
    TILDE.iter().find(|t| t.0 == n).map(|t| t.1)
}

/// `SS3 [modifier] x`, or Alt and `O`. Go's `parseSs3`.
pub(super) fn ss3(b: &[u8]) -> (usize, Event) {
    if b.len() == 2 && b[0] == ESC {
        return (
            2,
            press(Key::new(KeyCode::Char('o')).with_mods(KeyMod::SHIFT | KeyMod::ALT)),
        );
    }
    let mut i = if b[0] == ESC && b.get(1) == Some(&b'O') {
        2
    } else {
        1
    };
    let mut modifier: i64 = 0;
    while let Some(d) = b.get(i).filter(|d| d.is_ascii_digit()) {
        modifier = modifier.wrapping_mul(10).wrapping_add(i64::from(d - b'0'));
        i += 1;
    }
    let Some(&gl) = b.get(i).filter(|g| (0x21..=0x7E).contains(*g)) else {
        return (i, Event::Unknown);
    };
    let Some(mut key) = final_key(&SS3_FINALS, gl) else {
        return (i + 1, Event::Other("UnknownSs3Event"));
    };
    if modifier > 0 {
        key.mods |= KeyMod::from_bits(modifier - 1);
    }
    (i + 1, press(key))
}
