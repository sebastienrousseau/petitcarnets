//! Row diffs.

use super::*;
use crate::renderer::layout::layout;
use crate::renderer::writer::Addressing;
use crate::unicode::WidthMethod;

fn diff(a: &str, b: &str) -> String {
    let old = layout(a, 20, Some(2), WidthMethod::WcWidth);
    let new = layout(b, 20, Some(2), WidthMethod::WcWidth);
    let mut w = Writer::new(20, 2, Addressing::Absolute);
    draw(&mut w, &old, &new);
    w.buf
}

#[test]
fn only_changed_cells_are_written() {
    assert_eq!(diff("abc\nxyz", "abc\nxyz"), "");
    assert_eq!(diff("abcdef", "abXdef"), "abX", "two cells beat a move");
    assert_eq!(diff("abcdefgh", "abcdefXh"), "\x1b[6CX");
    assert_eq!(diff("abcdefghij", "Xbcdefghi"), "X\x1b[8C ");
    assert_eq!(diff("abcdefghij", "Xbcde"), "X\x1b[4C\x1b[K");
}

#[test]
fn close_changes_are_one_run() {
    assert_eq!(diff("abcdef", "XbcYef"), "XbcY");
    assert_eq!(diff("abcdefghijkl", "Xbcdefghijk"), "X\x1b[10C ");
}

#[test]
fn wide_cells_are_rewritten_whole() {
    assert_eq!(diff("a字b", "aXYb"), "aXY");
    assert_eq!(diff("aXYb", "a字b"), "a字");
    assert_eq!(diff("字b", "x b"), "x ");
}
