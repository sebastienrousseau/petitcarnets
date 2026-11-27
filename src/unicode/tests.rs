//! Unicode answers checked against Go's.

use super::*;

#[test]
fn printable_as_go_says() {
    for (r, print) in [
        (u32::from('a'), true),
        (0x20, true),
        (0xA0, false),
        (0x1F, false),
        (0x85, false),
        (0xAD, false),
        (0xE09C, false),
        (0x10_FFFF, false),
        (0xFFFD, true),
        (0xD800, false),
        (0x0301, true),
        (0x1F600, true),
    ] {
        assert_eq!(is_print(r), print, "U+{r:04X}");
    }
}

#[test]
fn case_maps_one_to_one() {
    assert_eq!(to_upper('ß'), 'ß');
    assert_eq!(to_upper('é'), 'É');
    assert_eq!(to_upper('ǅ'), 'Ǆ');
    assert_eq!(to_lower('İ'), 'i');
    assert_eq!(to_lower('O'), 'o');
}

#[test]
fn runes_decode_as_go_decodes_them() {
    assert_eq!(decode_rune(b"a"), ('a', 1));
    assert_eq!(decode_rune("é!".as_bytes()), ('é', 2));
    assert_eq!(decode_rune(b"\xc3"), (REPLACEMENT, 1));
    assert_eq!(decode_rune(b"\xed\xa0\x80"), (REPLACEMENT, 1));
    assert_eq!(decode_rune(b"\xff"), (REPLACEMENT, 1));
    assert_eq!(decode_rune("\u{FFFD}".as_bytes()), (REPLACEMENT, 3));
}

#[test]
fn noncharacters_and_format_characters_are_reserved() {
    for r in [
        0xFDD0, 0xFDEF, 0xFFFE, 0xFFFF, 0x1_FFFE, 0x10_FFFF, 0x200B, 0xE0001,
    ] {
        assert!(is_reserved(r), "{r:X}");
    }
    for r in [0x41, 0xFDCF, 0xFDF0, 0xFFFD, 0x1_FFFD, 0x2010] {
        assert!(!is_reserved(r), "{r:X}");
    }
}
