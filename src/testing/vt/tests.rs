use super::Vt;
use crate::renderer::cell::{Cell, Colour, Glyph, Pen};
use crate::unicode::WidthMethod;

/// An empty log.
const NONE: [&str; 0] = [];

/// A terminal fed `bytes`.
fn vt(columns: u16, rows: u16, bytes: &str) -> Vt {
    let mut vt = Vt::new(columns, rows);
    vt.feed(bytes.as_bytes());
    vt
}

/// A pen with only a background colour.
fn on(bg: Colour) -> Pen {
    Pen { bg, ..Pen::DEFAULT }
}

#[test]
fn new_terminal_is_blank_with_defaults() {
    let vt = Vt::new(4, 2).with_width_method(WidthMethod::GraphemeWidth);
    assert_eq!(vt.text(), "");
    assert_eq!(vt.cursor(), (0, 0));
    assert!(vt.cursor_visible() && vt.autowrap() && !vt.alt_screen());
    assert_eq!(vt.cursor_style(), 0);
    assert_eq!(vt.pen(), Pen::DEFAULT);
    assert!(vt.raw_cell(3, 1).is_blank());
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn text_and_carriage_return_line_feed() {
    let vt = vt(10, 3, "ab\r\ncd");
    assert_eq!(vt.text(), "ab\ncd");
    assert_eq!(vt.cursor(), (2, 1));
}

#[test]
fn line_feed_does_not_return() {
    let vt = vt(10, 3, "ab\ncd");
    assert_eq!(vt.text(), "ab\n  cd");
}

#[test]
fn pending_wrap_then_wrap() {
    let mut vt = vt(3, 2, "abc");
    assert_eq!(vt.cursor(), (2, 0));
    vt.feed(b"d");
    assert_eq!(vt.text(), "abc\nd");
    assert_eq!(vt.cursor(), (1, 1));
}

#[test]
fn carriage_return_clears_pending_wrap() {
    let vt = vt(3, 2, "abc\rx");
    assert_eq!(vt.text(), "xbc");
}

#[test]
fn autowrap_off_overwrites_last_column() {
    let vt = vt(3, 2, "\x1b[?7labcde");
    assert!(!vt.autowrap() && !vt.mode(7));
    assert_eq!(vt.text(), "abe");
    assert_eq!(vt.cursor(), (2, 0));
}

#[test]
fn wide_at_right_edge_with_autowrap_wraps() {
    let vt = vt(3, 2, "ab\u{4E2D}");
    assert_eq!(vt.text(), "ab\n\u{4E2D}");
    assert!(vt.raw_cell(1, 1).is_continuation());
    assert_eq!(vt.cursor(), (2, 1));
}

#[test]
fn wide_at_right_edge_without_autowrap_ends_at_last_column() {
    let vt = vt(3, 2, "\x1b[?7lab\u{4E2D}");
    assert_eq!(vt.text(), "a\u{4E2D}");
    assert_eq!(vt.raw_cell(1, 0).width, 2);
    assert!(vt.raw_cell(2, 0).is_continuation());
}

#[test]
fn overwriting_half_a_wide_cell_blanks_the_rest() {
    let vt = vt(6, 1, "\u{4E2D}\u{6587}\x1b[1GX\x1b[4GY");
    assert_eq!(vt.text(), "X  Y");
    assert!(!vt.raw_cell(1, 0).is_continuation());
    assert!(!vt.raw_cell(2, 0).is_continuation());
}

#[test]
fn combining_mark_joins_previous_cell() {
    let mut vt = vt(5, 1, "e");
    vt.feed("\u{301}x".as_bytes());
    assert_eq!(vt.raw_cell(0, 0).glyph, Glyph::Cluster("e\u{301}".into()));
    assert_eq!(vt.row_text(0), "e\u{301}x");
}

#[test]
fn combining_mark_joins_pending_wrap_cell() {
    let mut vt = vt(2, 2, "ab");
    vt.feed("\u{301}".as_bytes());
    assert_eq!(vt.row_text(0), "ab\u{301}");
    assert_eq!(vt.cursor(), (1, 0));
}

#[test]
fn cursor_movement_is_clamped() {
    let mut vt = vt(10, 5, "\x1b[3;4H");
    assert_eq!(vt.cursor(), (3, 2));
    vt.feed(b"\x1b[2A\x1b[5C");
    assert_eq!(vt.cursor(), (8, 0));
    vt.feed(b"\x1b[9B\x1b[20D");
    assert_eq!(vt.cursor(), (0, 4));
    vt.feed(b"\x1b[7G\x1b[2d");
    assert_eq!(vt.cursor(), (6, 1));
    vt.feed(b"\x1b[2E");
    assert_eq!(vt.cursor(), (0, 3));
    vt.feed(b"\x1b[4`\x1b[F");
    assert_eq!(vt.cursor(), (0, 2));
    vt.feed(b"\x1b[99;99f");
    assert_eq!(vt.cursor(), (9, 4));
    vt.feed(b"\x1b[H");
    assert_eq!(vt.cursor(), (0, 0));
}

#[test]
fn erase_in_line() {
    let mut vt = vt(6, 3, "abcdef\r\nabcdef\r\nabcdef");
    vt.feed(b"\x1b[1;3H\x1b[K\x1b[2;3H\x1b[1K\x1b[3;3H\x1b[2K");
    assert_eq!(vt.text(), "ab\n   def");
}

#[test]
fn erase_in_display() {
    let mut vt = vt(3, 3, "abc\r\ndef\r\nghi");
    vt.feed(b"\x1b[2;2H\x1b[J");
    assert_eq!(vt.text(), "abc\nd");
    let mut vt = self::vt(3, 3, "abc\r\ndef\r\nghi\x1b[2;2H\x1b[1J");
    assert_eq!(vt.text(), "\n  f\nghi");
    vt.feed(b"\x1b[2J");
    assert_eq!(vt.text(), "");
}

#[test]
fn erase_display_3_clears_scrollback() {
    let mut vt = vt(3, 2, "a\r\nb\r\nc");
    assert_eq!(vt.scrollback(), ["a"]);
    vt.feed(b"\x1b[3J");
    assert_eq!(vt.scrollback(), NONE);
    assert_eq!(vt.text(), "");
}

#[test]
fn erase_insert_delete_characters() {
    let mut vt = vt(6, 1, "abcdef\x1b[2G\x1b[2X");
    assert_eq!(vt.text(), "a  def");
    vt.feed(b"\x1b[2P");
    assert_eq!(vt.text(), "adef");
    vt.feed(b"\x1b[3@");
    assert_eq!(vt.text(), "a   de");
}

#[test]
fn background_colour_erase() {
    let vt = vt(4, 2, "ab\x1b[41m\x1b[1G\x1b[K\x1b[2;1H\x1b[2X");
    let red = on(Colour::Basic(1));
    assert_eq!(vt.raw_cell(0, 0).pen, red);
    assert_eq!(vt.raw_cell(3, 0).pen, red);
    assert_eq!(vt.raw_cell(1, 1).pen, red);
    assert_eq!(vt.raw_cell(2, 1).pen, Pen::DEFAULT);
    assert!(!vt.raw_cell(0, 0).is_blank());
}

#[test]
fn background_colour_fills_scrolled_in_lines() {
    let vt = vt(2, 2, "\x1b[44m\x1b[S");
    assert_eq!(vt.raw_cell(0, 1).pen, on(Colour::Basic(4)));
}

#[test]
fn sgr_sets_and_resets_pen() {
    let mut vt = vt(5, 1, "\x1b[1;38;2;1;2;3mA\x1b[mB");
    let a = vt.raw_cell(0, 0).pen;
    assert_eq!(a.fg, Colour::Rgb(1, 2, 3));
    assert_eq!(a.attrs, crate::renderer::cell::attr::BOLD);
    assert_eq!(vt.raw_cell(1, 0).pen, Pen::DEFAULT);
    vt.feed(b"\x1b[4:3m");
    assert_eq!(vt.pen().underline, 3);
}

#[test]
fn modify_other_keys_is_not_sgr() {
    let vt = vt(5, 1, "\x1b[1m\x1b[>4;2m\x1b[>4m");
    assert_eq!(vt.pen().attrs, crate::renderer::cell::attr::BOLD);
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn scrollback_from_full_screen_scrolls_only() {
    let mut vt = vt(4, 3, "a\r\nb\r\nc\r\nd");
    assert_eq!(vt.scrollback(), ["a"]);
    assert_eq!(vt.text(), "b\nc\nd");
    vt.feed(b"\x1b[1;2r\x1b[S");
    assert_eq!(vt.text(), "c\n\nd");
    assert_eq!(vt.scrollback(), ["a"]);
}

#[test]
fn scroll_region_with_su_and_sd() {
    let mut vt = vt(3, 4, "a\r\nb\r\nc\r\nd\x1b[2;3r");
    assert_eq!(vt.cursor(), (0, 0));
    vt.feed(b"\x1b[S");
    assert_eq!(vt.text(), "a\nc\n\nd");
    vt.feed(b"\x1b[2T");
    assert_eq!(vt.text(), "a\n\n\nd");
    assert_eq!(vt.scrollback(), NONE);
}

#[test]
fn line_feed_scrolls_inside_region() {
    let vt = vt(3, 4, "a\r\nb\r\nc\r\nd\x1b[2;3r\x1b[3;1H\nx");
    assert_eq!(vt.text(), "a\nc\nx\nd");
    assert_eq!(vt.scrollback(), NONE);
}

#[test]
fn insert_and_delete_lines() {
    let mut vt = vt(3, 4, "a\r\nb\r\nc\r\nd\x1b[2;3H\x1b[L");
    assert_eq!(vt.text(), "a\n\nb\nc");
    assert_eq!(vt.cursor(), (0, 1));
    vt.feed(b"\x1b[2;2H\x1b[M");
    assert_eq!(vt.text(), "a\nb\nc");
    assert_eq!(vt.cursor(), (0, 1));
    vt.feed(b"\x1b[4;1Hd\x1b[1;2r\x1b[4;1H\x1b[L");
    assert_eq!(vt.text(), "a\nb\nc\nd");
    assert_eq!(vt.scrollback(), NONE);
}

#[test]
fn reverse_index_scrolls_down_at_top() {
    let vt = vt(3, 3, "a\r\nb\x1b[H\x1bMx");
    assert_eq!(vt.text(), "x\na\nb");
}

#[test]
fn save_and_restore_cursor() {
    let vt = vt(5, 3, "\x1b[2;3H\x1b[31m\x1b7\x1b[m\x1b[H\x1b8X");
    assert_eq!(vt.cursor(), (3, 1));
    assert_eq!(vt.raw_cell(2, 1).pen.fg, Colour::Basic(1));
}

#[test]
fn alt_screen_round_trip() {
    let mut vt = vt(4, 2, "main\x1b[2;2H\x1b[?1049h");
    assert!(vt.alt_screen() && vt.mode(1049));
    assert_eq!(vt.text(), "");
    vt.feed(b"alt\r\nx\r\nyz");
    assert_eq!(vt.cursor(), (2, 1));
    assert_eq!(vt.scrollback(), NONE);
    vt.feed(b"\x1b[?1049l");
    assert!(!vt.alt_screen());
    assert_eq!(vt.text(), "main");
    assert_eq!(vt.cursor(), (1, 1));
}

#[test]
fn private_modes_are_recorded() {
    let mut vt = vt(4, 2, "\x1b[?2004;1004h\x1b[?25l\x1b[?1002h\x1b[?1006h");
    assert!(vt.mode(2004) && vt.mode(1004) && vt.mode(1002) && vt.mode(1006));
    assert!(!vt.cursor_visible());
    vt.feed(b"\x1b[?1002l\x1b[?2027h");
    assert!(!vt.mode(1002) && vt.mode(2027));
}

#[test]
fn sync_brackets_are_counted() {
    let mut vt = vt(4, 2, "\x1b[?2026h");
    assert!(vt.in_sync());
    vt.feed(b"x\x1b[?2026l\x1b[?2026l\x1b[?2026hy\x1b[?2026l");
    assert!(!vt.in_sync());
    assert_eq!(vt.sync_frames(), 2);
}

#[test]
fn cursor_style_and_queries() {
    let vt = vt(4, 2, "\x1b[5 q\x1b[?2026$p\x1b[>1u\x1b[<u\x1b[=1;1u\x1b[?u");
    assert_eq!(vt.cursor_style(), 5);
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn title_and_title_stack() {
    let mut vt = vt(4, 2, "\x1b]2;one\x07\x1b[22;0t\x1b]0;two\x1b\\");
    assert_eq!(vt.title(), "two");
    vt.feed(b"\x1b[23;0t");
    assert_eq!(vt.title(), "one");
    assert_eq!(vt.osc_log(), ["2;one", "0;two"]);
}

#[test]
fn osc_8_links_cells() {
    let vt = vt(6, 1, "\x1b]8;id=1;http://a\x1b\\ab\x1b]8;;\x1b\\c");
    let link = vt.raw_cell(1, 0).link.as_deref();
    assert_eq!(link, Some("id=1;http://a"));
    assert_eq!(vt.raw_cell(2, 0).link, None);
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn other_osc_are_logged_only() {
    let vt = vt(
        4,
        1,
        "\x1b]11;?\x07\x1b]9;4;1;50\x07\x1b]112\x07\x1b]777;x\x07",
    );
    assert_eq!(vt.osc_log().len(), 4);
    assert_eq!(vt.unknown(), ["OSC 777;x"]);
}

#[test]
fn split_feeds_are_joined() {
    let mut vt = Vt::new(6, 2);
    let bytes = "\x1b[31mé\x1b]2;t\x1b\\中".as_bytes();
    for b in bytes {
        vt.feed(std::slice::from_ref(b));
    }
    assert_eq!(vt.row_text(0), "é中");
    assert_eq!(vt.raw_cell(0, 0).pen.fg, Colour::Basic(1));
    assert_eq!(vt.title(), "t");
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn invalid_utf8_becomes_replacement() {
    let vt = {
        let mut vt = Vt::new(4, 1);
        vt.feed(b"a\xffb");
        vt
    };
    assert_eq!(vt.row_text(0), "a\u{FFFD}b");
}

#[test]
fn tab_and_backspace_work_but_are_logged() {
    let vt = vt(20, 1, "a\tb\x08c");
    assert_eq!(vt.row_text(0), "a       c");
    assert_eq!(vt.unknown(), ["C0 09", "C0 08"]);
}

#[test]
fn unknown_sequences_are_logged() {
    let vt = vt(4, 1, "\x1b[5i\x1b(B\x1bP1$r\x1b\\\x01\x1b[?h");
    assert_eq!(vt.unknown().len(), 5);
    assert_eq!(vt.unknown()[0], "CSI 5i");
}

#[test]
fn bell_is_ignored() {
    let vt = vt(4, 1, "a\x07b");
    assert_eq!(vt.text(), "ab");
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn resize_cuts_and_pads() {
    let mut vt = vt(4, 3, "abcd\r\nefgh\r\nij\x1b[2;3r\x1b[3;3H");
    vt.resize(2, 2);
    assert_eq!(vt.text(), "ab\nef");
    assert_eq!(vt.cursor(), (1, 1));
    vt.resize(3, 3);
    assert_eq!(vt.text(), "ab\nef");
    vt.feed(b"\x1b[3;1H\nz");
    assert_eq!(vt.scrollback(), ["ab"]);
}

#[test]
fn resize_blanks_cut_wide_cell() {
    let mut vt = vt(4, 1, "a\u{4E2D}");
    vt.resize(2, 1);
    assert_eq!(*vt.raw_cell(1, 0), Cell::BLANK);
}

#[test]
fn full_reset() {
    let vt = vt(4, 2, "\x1b[31mab\x1b[?25l\x1b]2;t\x07\x1bc");
    assert_eq!(vt.text(), "");
    assert_eq!(vt.pen(), Pen::DEFAULT);
    assert!(vt.cursor_visible());
    assert_eq!(vt.title(), "");
    assert_eq!(vt.osc_log(), ["2;t"]);
}

#[test]
fn a_cell_shows_its_text_width_style_and_link() {
    let vt = vt(
        6,
        2,
        "\x1b[1;31ma\x1b[m\x1b]8;;https://x.example\x07界\x1b]8;;\x07",
    );
    let a = vt.cell(0, 0).unwrap();
    assert_eq!(
        (a.text(), a.width(), a.style()),
        ("a".into(), 1, "\x1b[1;31m".into())
    );
    assert_eq!(a.link(), None);
    let wide = vt.cell(1, 0).unwrap();
    assert_eq!((wide.text(), wide.width()), ("界".into(), 2));
    assert_eq!(wide.link(), Some("https://x.example"));
    let half = vt.cell(2, 0).unwrap();
    assert!(half.is_continuation() && !wide.is_continuation());
    assert_eq!((half.text(), half.width()), (String::new(), 0));
    let blank = vt.cell(5, 1).unwrap();
    assert_eq!((blank.text(), blank.style()), (" ".into(), String::new()));
    assert!(vt.cell(6, 0).is_none() && vt.cell(0, 2).is_none());
    assert_eq!(vt.row_text(2), "", "off the screen");
    assert_eq!(vt.size(), (6, 2));
}

#[test]
fn kitty_keyboard_flags_are_pushed_set_and_popped() {
    let mut vt = vt(4, 1, "\x1b[>1u");
    assert_eq!(vt.keyboard_flags(), 1);
    vt.feed(b"\x1b[=4;2u");
    assert_eq!(vt.keyboard_flags(), 5, "added");
    vt.feed(b"\x1b[=1;3u");
    assert_eq!(vt.keyboard_flags(), 4, "removed");
    vt.feed(b"\x1b[=3u");
    assert_eq!(vt.keyboard_flags(), 3, "replaced");
    vt.feed(b"\x1b[>8u\x1b[<u");
    assert_eq!(vt.keyboard_flags(), 3, "one popped");
    vt.feed(b"\x1b[<5u");
    assert_eq!(vt.keyboard_flags(), 0, "popped past the bottom");
    vt.feed(b"\x1b[=2u");
    assert_eq!(vt.keyboard_flags(), 2, "set with nothing pushed");
    assert_eq!(vt.unknown(), NONE);
}

#[test]
fn modify_other_keys_levels_are_kept() {
    let mut vt = vt(4, 1, "\x1b[>4;2m");
    assert_eq!(vt.modify_other_keys(), 2);
    vt.feed(b"\x1b[>4m");
    assert_eq!(vt.modify_other_keys(), 0);
    vt.feed(b"\x1b[>5;1m");
    assert_eq!(vt.unknown(), ["CSI >5;1m"]);
}

#[test]
fn a_snapshot_shows_the_alternate_screen_a_hidden_cursor_and_the_title() {
    let vt = vt(4, 2, "\x1b[?1049h\x1b[?25l\x1b[3 q\x1b]2;t\x07ab\r\n");
    assert_eq!(
        vt.snapshot(),
        "size 4x2, alternate screen\ncursor 0,1 hidden, style 3\nmodes 7 1049\n\
         keyboard flags 0, modifyOtherKeys 0\ntitle \"t\"\n----\nab\n"
    );
    assert_eq!(vt.modes().collect::<Vec<_>>(), [7, 1049]);
}
