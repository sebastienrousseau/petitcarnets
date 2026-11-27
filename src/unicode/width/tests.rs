//! Widths and tokens checked against Go's, line by line.

use super::WidthMethod;
use crate::ansi::{Token, spans};

/// What Go's `ansi` package says about each corpus string; the format is
/// described in scripts/width-fixture/main.go.
const FIXTURE: &str = include_str!("../../../tests/fixtures/width.tsv");

/// Bytes from hex.
fn unhex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// The tokens of `s` as `len:width` pairs.
fn pieces(s: &str, method: WidthMethod) -> String {
    spans(s, method)
        .map(|(token, raw)| {
            let width = match token {
                Token::Text(_, width) => width,
                _ => 0,
            };
            format!("{}:{width}", raw.len())
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn widths_and_tokens_match_go() {
    let mut lines = 0;
    for line in FIXTURE.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 5, "malformed fixture line {line:?}");
        let bytes = unhex(fields[0]);
        let s = std::str::from_utf8(&bytes).unwrap();
        let got = [
            WidthMethod::WcWidth.string_width(s).to_string(),
            WidthMethod::GraphemeWidth.string_width(s).to_string(),
            pieces(s, WidthMethod::WcWidth),
            pieces(s, WidthMethod::GraphemeWidth),
        ];
        assert_eq!(got, fields[1..], "for {s:?} ({})", fields[0]);
        lines += 1;
    }
    assert!(lines >= 2000, "only {lines} fixture lines");
}

/// Go ends a string at an 8-bit ST (0x9C) even inside a character, here
/// the middle byte of U+2713 (e2 9c 93), then reads the stray 0x93 as a
/// control. A `str` cannot be cut there, so the whole character goes
/// with the string; the widths and the tokens after it are Go's.
#[test]
fn st_inside_a_character_ends_the_string_after_it() {
    let s = "\u{1b}]0;\u{2713}x";
    let tokens: Vec<_> = spans(s, WidthMethod::WcWidth).collect();
    assert_eq!(
        tokens,
        [
            (Token::Osc("0;"), "\u{1b}]0;\u{2713}"),
            (Token::Text("x", 1), "x")
        ]
    );
    assert_eq!(WidthMethod::WcWidth.string_width(s), 1);
    assert_eq!(WidthMethod::GraphemeWidth.string_width(s), 1);
}
