//! Lines printed above an inline frame.

use super::*;

#[test]
fn wrapped_rows_count_wide_characters_as_the_terminal_wraps_them() {
    let rows = |text: &str, width| encode(text, width, WidthMethod::WcWidth).1;
    assert_eq!(rows("", 5), 1);
    assert_eq!(rows("abcde", 5), 1);
    assert_eq!(rows("abcdef", 5), 2);
    assert_eq!(rows("字字字", 3), 3, "Go's (6 - 1) / 3 + 1 says 2");
    assert_eq!(rows("a\nb", 5), 2);
}

#[test]
fn lines_are_erased_styled_and_ended() {
    let (bytes, _) = encode("\x1b[1mhi", 10, WidthMethod::WcWidth);
    assert_eq!(bytes, "\x1b[?7h\x1b[K\x1b[1mhi\x1b[m\r\n\x1b[?7l");
}

#[test]
fn room_is_made_as_bubble_tea_makes_it() {
    assert_eq!(make_room(0, 3, 2), "\r\x1b[2B\n\n\x1b[4A\x1b[2L");
    assert_eq!(make_room(0, 1, 1), "\r\n\x1b[1A\x1b[1L");
    assert_eq!(
        make_room(2, 1, 1),
        "\r\x1b[2A\n\x1b[1A\x1b[1L",
        "below a frame that shrank"
    );
}
