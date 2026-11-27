//! Base64 as Go writes and reads it.

use super::*;

#[test]
fn encoding_pads_to_groups_of_four() {
    assert_eq!(encode(b""), "");
    assert_eq!(encode(b"h"), "aA==");
    assert_eq!(encode(b"hi"), "aGk=");
    assert_eq!(encode(b"hello"), "aGVsbG8=");
    assert_eq!(encode("é✓".as_bytes()), "w6ninJM=");
}

#[test]
fn decoding_round_trips_and_rejects_what_go_rejects() {
    for text in ["", "a", "hi", "hello", "hello world!"] {
        assert_eq!(
            decode(encode(text.as_bytes()).as_bytes()).unwrap(),
            text.as_bytes()
        );
    }
    assert_eq!(
        decode(b"aGVs\r\nbG8=").unwrap(),
        b"hello",
        "line breaks are skipped"
    );
    for bad in [&b"aGk"[..], b"!!", b"a===", b"aG=k", b"aA==aA==", b"aGk=="] {
        assert!(decode(bad).is_none(), "{bad:?}");
    }
}
