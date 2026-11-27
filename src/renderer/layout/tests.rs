//! Laying out view text in cells.

use super::*;

fn text(row: &Row) -> String {
    let mut s = String::new();
    for cell in &row.cells {
        cell.push_text(&mut s);
    }
    s
}

#[test]
fn lines_split_as_go_splits_a_view() {
    let none: [&str; 0] = [];
    assert_eq!(lines(""), none);
    assert_eq!(lines("a\r\nb\n"), ["a", "b", ""]);
}

#[test]
fn rows_are_as_wide_as_the_window_and_cut_there() {
    let frame = layout("abcdef\nxy", 4, None, WidthMethod::WcWidth);
    assert_eq!(frame.rows.len(), 2);
    assert_eq!(text(&frame.rows[0]), "abcd");
    assert_eq!(text(&frame.rows[1]), "xy  ");
    assert_eq!(frame.rows[1].content_end(), 2);
    let alt = layout("a\nb\nc", 4, Some(2), WidthMethod::WcWidth);
    assert_eq!(alt.rows.len(), 2);
    let tall = layout("a", 4, Some(3), WidthMethod::WcWidth);
    assert_eq!(tall.rows.len(), 3);
}

#[test]
fn wide_characters_take_two_cells_and_are_never_split() {
    let frame = layout("a字字", 4, None, WidthMethod::WcWidth);
    let cells = &frame.rows[0].cells;
    assert_eq!(cells[1].width, 2);
    assert!(cells[2].is_continuation());
    assert!(cells[3].is_blank(), "the second one does not fit");
}

#[test]
fn tabs_carriage_returns_and_marks() {
    let frame = layout("a\tb\nabc\rX\ne\u{301}", 12, None, WidthMethod::WcWidth);
    assert_eq!(text(&frame.rows[0]), "a       b   ");
    assert_eq!(text(&frame.rows[1]).trim_end(), "Xbc");
    assert_eq!(text(&frame.rows[2]).trim_end(), "e\u{301}");
}

#[test]
fn styles_and_links_carry_across_lines() {
    let frame = layout(
        "\x1b[1m\x1b]8;;http://x\x07a\nb\x1b[m\x1b]8;;\x07c\x1b[?25l",
        4,
        None,
        WidthMethod::WcWidth,
    );
    let b = &frame.rows[1].cells[0];
    assert_eq!(b.pen.attrs, super::super::cell::attr::BOLD);
    assert_eq!(b.link.as_deref(), Some(";http://x"));
    let c = &frame.rows[1].cells[1];
    assert!(c.link.is_none() && c.pen == Pen::DEFAULT);
    assert_eq!(
        text(&frame.rows[1]).trim_end(),
        "bc",
        "other sequences dropped"
    );
}

#[test]
fn equal_rows_hash_equal() {
    let a = layout("same\nsame\nother", 8, None, WidthMethod::WcWidth);
    assert_eq!(a.rows[0].hash, a.rows[1].hash);
    assert_ne!(a.rows[0].hash, a.rows[2].hash);
}
