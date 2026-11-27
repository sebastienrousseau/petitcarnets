//! The few Unicode questions input decoding asks, answered as Go's
//! `unicode` package and `rivo/uniseg` answer them, so keys come out as
//! Bubble Tea's do.

mod grapheme;

pub(crate) use grapheme::first_cluster;

/// The replacement character, Go's `utf8.RuneError`.
pub(crate) const REPLACEMENT: char = '\u{FFFD}';

/// Ranges of the Unicode `Cf` (format) category, which Go's
/// `unicode.IsPrint` rejects.
const FORMAT: [(u32, u32); 19] = [
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE007F),
];

/// Whether Go's `unicode.IsPrint` holds for the code point `r`: a
/// letter, mark, number, punctuation or symbol, or the ASCII space.
/// Unassigned code points cannot be told apart without the full tables,
/// so they count as printable unless they are noncharacters.
pub(crate) fn is_print(r: u32) -> bool {
    let Some(c) = char::from_u32(r) else {
        return false;
    };
    c == ' ' || !(c.is_control() || c.is_whitespace() || is_private_use(r) || is_reserved(r))
}

/// Private use areas.
fn is_private_use(r: u32) -> bool {
    (0xE000..=0xF8FF).contains(&r) || r >= 0xF_0000
}

/// Format characters and noncharacters.
fn is_reserved(r: u32) -> bool {
    let noncharacter = (0xFDD0..=0xFDEF).contains(&r) || r & 0xFFFE == 0xFFFE;
    noncharacter || FORMAT.iter().any(|&(a, b)| (a..=b).contains(&r))
}

/// Go's `unicode.ToUpper`: the simple (one to one) mapping.
pub(crate) fn to_upper(c: char) -> char {
    single(c.to_uppercase()).unwrap_or(c)
}

/// Go's `unicode.ToLower`: the simple (one to one) mapping.
pub(crate) fn to_lower(c: char) -> char {
    if c == '\u{130}' {
        return 'i';
    }
    single(c.to_lowercase()).unwrap_or(c)
}

/// The only item of `items`, if there is exactly one.
fn single(mut items: impl Iterator<Item = char>) -> Option<char> {
    let first = items.next()?;
    items.next().is_none().then_some(first)
}

/// The code point at the start of `bytes` and its length, as Go's
/// `utf8.DecodeRune` reads it: an invalid or cut off sequence is the
/// replacement character, one byte long. `bytes` must not be empty.
pub(crate) fn decode_rune(bytes: &[u8]) -> (char, usize) {
    let len = match bytes[0] {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return (REPLACEMENT, 1),
    };
    bytes
        .get(..len)
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(|s| s.chars().next())
        .map_or((REPLACEMENT, 1), |c| (c, len))
}

#[cfg(test)]
mod tests;
