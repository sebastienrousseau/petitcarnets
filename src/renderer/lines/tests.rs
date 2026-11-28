//! The line printer: changed lines only, plain text, no cursor moves.

use super::*;

fn frame(lines: &mut Lines, content: &str) -> String {
    lines.frame(content, WidthMethod::WcWidth, ColorProfile::NoTty)
}

#[test]
fn the_first_frame_prints_every_line_that_is_not_blank() {
    let mut lines = Lines::new(false);
    assert_eq!(frame(&mut lines, "one\n\n  two  \n"), "one\r\n  two\r\n");
}

#[test]
fn later_frames_print_only_the_lines_that_changed() {
    let mut lines = Lines::new(false);
    let _ = frame(&mut lines, "> a\n  b\n  c");
    assert_eq!(frame(&mut lines, "  a\n> b\n  c"), "  a\r\n> b\r\n");
    assert_eq!(
        frame(&mut lines, "  a\n> b\n  c"),
        "",
        "the same frame prints nothing"
    );
    assert_eq!(frame(&mut lines, "  a\n> b\n  c\nmore"), "more\r\n");
    assert_eq!(
        frame(&mut lines, "  a"),
        "",
        "a shorter frame prints nothing"
    );
    assert_eq!(frame(&mut lines, "  a\n> b"), "> b\r\n");
}

#[test]
fn a_forgotten_frame_is_printed_in_full_again() {
    let mut lines = Lines::new(false);
    let _ = frame(&mut lines, "a\nb");
    lines.forget();
    assert_eq!(frame(&mut lines, "a\nb"), "a\r\nb\r\n");
}

#[test]
fn styles_are_left_out_without_colours_and_kept_with_them() {
    let styled = "\x1b[1;31mred\x1b[m and \x1b]8;;http://x\x07link\x1b]8;;\x07";
    let mut plain = Lines::new(false);
    assert_eq!(frame(&mut plain, styled), "red and link\r\n");
    let mut coloured = Lines::new(false);
    let out = coloured.frame(styled, WidthMethod::WcWidth, ColorProfile::Ansi);
    assert!(out.starts_with("\x1b[1;31mred"), "{out:?}");
    assert!(out.ends_with("\r\n"), "{out:?}");
}

#[test]
fn wide_text_tabs_and_carriage_returns_are_laid_out_as_on_screen() {
    let mut lines = Lines::new(false);
    assert_eq!(frame(&mut lines, "字\tx\nabc\rZ"), "字      x\r\nZbc\r\n");
}

#[test]
fn printed_text_keeps_its_blank_lines() {
    let mut lines = Lines::new(false);
    let out = lines.print("a\n\nb", WidthMethod::WcWidth, ColorProfile::NoTty);
    assert_eq!(out, "a\r\n\r\nb\r\n");
}

#[test]
fn the_cursor_is_shown_once_before_the_first_output() {
    let mut lines = Lines::new(true);
    assert_eq!(
        frame(&mut lines, ""),
        "",
        "nothing to print, nothing written"
    );
    assert_eq!(frame(&mut lines, "a"), "\x1b[?25ha\r\n");
    assert_eq!(frame(&mut lines, "b"), "b\r\n");
}
