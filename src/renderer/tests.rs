//! Golden frames: each frame is drawn through the renderer into the
//! test terminal emulator, whose screen must then equal the frame laid
//! out as intended, cell for cell (text, width, style, link).

use super::*;
use crate::testing::Vt;
use crate::view::Cursor;

/// A renderer and the terminal it draws on.
struct Rig {
    r: Renderer,
    vt: Vt,
    size: Size,
}

impl Rig {
    /// Inline, `w` by `h`, autowrap off as the program sets it.
    fn inline(w: u16, h: u16) -> Self {
        let mut vt = Vt::new(w, h);
        vt.feed(b"\x1b[?7l");
        Self {
            r: Renderer::new(Size::new(w, h)),
            vt,
            size: Size::new(w, h),
        }
    }

    /// On the alternate screen.
    fn alt(w: u16, h: u16) -> Self {
        let mut rig = Self::inline(w, h);
        rig.vt.feed(b"\x1b[?1049h");
        rig.r.switch_screen(true);
        rig
    }

    /// Draws `view`; the bytes written.
    fn view(&mut self, view: &View) -> String {
        let mut out = Vec::new();
        self.r.render(view, &mut out).unwrap();
        self.vt.feed(&out);
        assert!(self.vt.unknown().is_empty(), "{:?}", self.vt.unknown());
        String::from_utf8(out).unwrap()
    }

    /// Draws `content`; the bytes written.
    fn draw(&mut self, content: &str) -> String {
        self.view(&View::new(content))
    }

    /// Prints above the frame; the bytes written.
    fn print(&mut self, text: &str) -> String {
        let mut out = Vec::new();
        self.r.insert_above(text, &mut out).unwrap();
        self.vt.feed(&out);
        assert!(self.vt.unknown().is_empty(), "{:?}", self.vt.unknown());
        String::from_utf8(out).unwrap()
    }

    /// Checks the screen from row `top` shows `content` as intended,
    /// and everything else on screen is blank.
    fn expect_at(&self, top: usize, content: &str) {
        let (w, h) = (usize::from(self.size.width), usize::from(self.size.height));
        let rows = if self.r.is_alt() { Some(h) } else { None };
        let mut frame = layout(content, w, rows, WidthMethod::WcWidth);
        let extra = frame.rows.len().saturating_sub(h);
        drop(frame.rows.drain(..extra));
        for y in 0..h {
            let want = y
                .checked_sub(top)
                .and_then(|i| frame.rows.get(i))
                .map_or_else(|| layout::Row::blank(w), Clone::clone);
            for (x, cell) in want.cells.iter().enumerate() {
                let got = self.vt.raw_cell(to16(x), to16(y));
                assert_eq!(got, cell, "cell {x},{y}\nscreen:\n{}", self.vt.text());
            }
        }
    }

    /// Checks the screen shows `content` from the top.
    fn expect(&self, content: &str) {
        self.expect_at(0, content);
    }
}

fn to16(n: usize) -> u16 {
    u16::try_from(n).unwrap()
}

/// Draws `a` then `b` and checks both, returning the bytes for `b`.
fn a_then_b(rig: &mut Rig, a: &str, b: &str) -> String {
    let _ = rig.draw(a);
    rig.expect(a);
    let out = rig.draw(b);
    rig.expect(b);
    out
}

#[test]
fn the_first_frame_draws_every_line() {
    let mut rig = Rig::inline(20, 5);
    let out = rig.draw("hello\nworld");
    rig.expect("hello\nworld");
    assert_eq!(out, "\x1b[?25l\rhello\r\nworld");
}

#[test]
fn an_unchanged_frame_writes_nothing() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("a\nb");
    assert_eq!(rig.draw("a\nb"), "");
    let mut rig = Rig::alt(20, 5);
    let _ = rig.draw("a\nb");
    assert_eq!(rig.draw("a\nb"), "");
}

#[test]
fn one_changed_cell_writes_a_move_and_the_cell() {
    let mut rig = Rig::alt(20, 5);
    let out = a_then_b(&mut rig, "count 1\nxx", "count 2\nxx");
    assert_eq!(out, "\x1b[1;7H2", "a move and one byte");
    let mut rig = Rig::inline(20, 5);
    let out = a_then_b(&mut rig, "count 1\nxx", "count 2\nxx");
    assert_eq!(out, "\x1b[A\x1b[4C2", "relative moves inline");
}

#[test]
fn a_line_inserted_and_deleted() {
    let mut rig = Rig::alt(20, 6);
    let _ = a_then_b(&mut rig, "a\nb\nc\nd", "a\nNEW\nb\nc\nd");
    let _ = rig.draw("a\nb\nc\nd");
    rig.expect("a\nb\nc\nd");
    let mut rig = Rig::inline(20, 6);
    let _ = a_then_b(&mut rig, "a\nb\nc\nd", "a\nNEW\nb\nc\nd");
    let _ = rig.draw("a\nb\nc\nd");
    rig.expect("a\nb\nc\nd");
}

/// Lines `from..to` of a log, as a frame.
fn log(from: usize, to: usize) -> String {
    (from..to)
        .map(|i| format!("log line number {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_scrolled_log_scrolls_the_terminal() {
    let mut rig = Rig::alt(30, 10);
    let _ = rig.draw(&log(0, 10));
    let out = rig.draw(&log(3, 13));
    rig.expect(&log(3, 13));
    assert!(out.starts_with("\x1b[3S"), "{out:?}");
    assert!(out.len() < 80, "{} bytes: {out:?}", out.len());
    let out = rig.draw(&log(1, 11));
    rig.expect(&log(1, 11));
    assert!(out.contains("\x1b[2T"), "{out:?}");
}

#[test]
fn a_scroll_under_a_fixed_header_uses_a_scroll_region() {
    let mut rig = Rig::alt(30, 10);
    let frame = |from| format!("HEADER\n{}\nFOOTER", log(from, from + 8));
    let _ = rig.draw(&frame(0));
    let out = rig.draw(&frame(1));
    rig.expect(&frame(1));
    assert!(out.starts_with("\x1b[2;9r\x1b[S\x1b[r"), "{out:?}");
}

#[test]
fn a_frame_shrinks_and_grows() {
    let mut rig = Rig::inline(20, 6);
    let _ = a_then_b(&mut rig, "1\n2\n3\n4", "1\n2");
    let _ = rig.draw("1\n2\n3\n4\n5");
    rig.expect("1\n2\n3\n4\n5");
    let mut rig = Rig::alt(20, 6);
    let _ = a_then_b(&mut rig, "1\n2\n3\n4", "1\n2");
    let _ = a_then_b(&mut rig, "1\n2", "1\n2\n3\n4\n5");
}

#[test]
fn a_shorter_line_erases_its_old_tail() {
    let mut rig = Rig::alt(30, 3);
    let out = a_then_b(&mut rig, "a long line of text", "a long");
    assert_eq!(out, "\x1b[7G\x1b[K", "erased from where the new line ends");
    let out = a_then_b(&mut rig, "abcdef", "abcd");
    assert_eq!(out, "\x1b[2D  ", "two spaces beat a move and an erase");
}

#[test]
fn wide_characters_at_the_right_edge_are_never_split() {
    let mut rig = Rig::alt(5, 3);
    let _ = rig.draw("abcd字\n字字字");
    rig.expect("abcd\n字字");
    assert_eq!(rig.vt.row_text(0), "abcd");
    let _ = a_then_b(&mut rig, "a字b", "ab字");
    let _ = a_then_b(&mut rig, "字字", "x字");
    let _ = a_then_b(&mut rig, "x字", "字字");
}

#[test]
fn emoji_and_zwj_sequences() {
    let mut rig = Rig::alt(20, 3);
    let _ = a_then_b(&mut rig, "hi 👍 there", "hi 👍🏽 there 🇫🇷");
    let _ = a_then_b(&mut rig, "family 👨\u{200d}👩\u{200d}👧", "family 1️⃣ ok");
}

#[test]
fn combining_marks_stay_with_their_letter() {
    let mut rig = Rig::alt(20, 3);
    let _ = a_then_b(&mut rig, "cafe\u{301}", "cafe\u{301}s");
    let _ = a_then_b(&mut rig, "e\u{301}e\u{300}", "e\u{300}e\u{301}");
    assert_eq!(rig.vt.raw_cell(0, 0).width, 1);
}

#[test]
fn styles_changing_mid_line() {
    let mut rig = Rig::alt(30, 3);
    let a = "plain \x1b[1;31mbold red\x1b[0m plain";
    let b = "plain \x1b[1;32mbold green\x1b[0m \x1b[48;2;1;2;3mbg\x1b[m";
    let out = a_then_b(&mut rig, a, b);
    assert!(out.contains("\x1b[1;32m"), "{out:?}");
    assert_eq!(rig.vt.pen(), cell::Pen::DEFAULT, "the pen is left plain");
    let c = "\x1b[38;5;200;4:3;9mfancy\x1b[24m under off";
    let _ = a_then_b(&mut rig, b, c);
}

#[test]
fn hyperlinks_are_kept_per_cell() {
    let mut rig = Rig::alt(30, 3);
    let a = "see \x1b]8;;https://a.example\x07here\x1b]8;;\x07 now";
    let b = "see \x1b]8;id=1;https://b.example\x1b\\here\x1b]8;;\x1b\\ now";
    let out = a_then_b(&mut rig, a, b);
    assert!(out.contains("\x1b]8;id=1;https://b.example\x07"), "{out:?}");
    assert!(out.ends_with("\x1b]8;;\x07"), "the link is closed: {out:?}");
}

#[test]
fn tabs_expand_to_every_eight_columns() {
    let mut rig = Rig::alt(30, 3);
    let out = a_then_b(&mut rig, "a\tb\n\tc", "ab\tc\nabcdefgh\ti");
    assert!(!out.contains('\t'));
    assert_eq!(rig.vt.raw_cell(8, 0).glyph, cell::Glyph::Char('c'));
    assert_eq!(rig.vt.raw_cell(16, 1).glyph, cell::Glyph::Char('i'));
}

#[test]
fn an_inline_frame_taller_than_the_window_keeps_its_bottom_rows() {
    let mut rig = Rig::inline(20, 4);
    let _ = rig.draw("1\n2\n3\n4\n5\n6");
    rig.expect("3\n4\n5\n6");
    let _ = rig.draw("1\n2\n3\n4\n5\n6\n7");
    rig.expect("4\n5\n6\n7");
}

#[test]
fn the_alternate_screen_cuts_the_frame_to_the_window() {
    let mut rig = Rig::alt(4, 2);
    let _ = rig.draw("abcdef\nghij\nklmn");
    rig.expect("abcd\nghij");
    assert_eq!(rig.vt.text(), "abcd\nghij");
}

#[test]
fn a_resize_redraws_in_full() {
    let mut rig = Rig::alt(20, 5);
    let _ = rig.draw("one\ntwo");
    rig.r.resize(Size::new(10, 3));
    rig.vt.resize(10, 3);
    rig.size = Size::new(10, 3);
    let out = rig.draw("one\ntwo");
    rig.expect("one\ntwo");
    assert!(out.contains("\x1b[2J"), "{out:?}");
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("one\ntwo");
    rig.r.resize(Size::new(10, 5));
    rig.vt.resize(10, 5);
    rig.size = Size::new(10, 5);
    let _ = rig.draw("uno\ndos\ntres");
    rig.expect("uno\ndos\ntres");
}

#[test]
fn println_goes_above_an_inline_frame() {
    let mut rig = Rig::inline(20, 6);
    let _ = rig.draw("frame 1\nstatus");
    let _ = rig.print("installed a\ninstalled b");
    rig.expect("installed a\ninstalled b\nframe 1\nstatus");
    let _ = rig.draw("frame 2\nstatus");
    rig.expect("installed a\ninstalled b\nframe 2\nstatus");
}

#[test]
fn println_at_the_bottom_scrolls_lines_into_the_scrollback() {
    let mut rig = Rig::inline(20, 3);
    let _ = rig.draw("frame");
    for i in 0..4 {
        let _ = rig.print(&format!("line {i}"));
    }
    assert_eq!(rig.vt.scrollback(), ["line 0", "line 1"]);
    rig.expect("line 2\nline 3\nframe");
}

#[test]
fn println_counts_wrapped_rows_with_wide_characters() {
    let mut rig = Rig::inline(5, 8);
    let _ = rig.draw("frame");
    // Three rows on a 5-column screen: "字字" fits, the third wraps and
    // "ab字" fits only "ab" before the next wide one wraps.
    let _ = rig.print("字字字ab字");
    assert_eq!(rig.vt.row_text(0), "字字");
    assert_eq!(rig.vt.row_text(1), "字ab");
    assert_eq!(rig.vt.row_text(2), "字");
    assert_eq!(rig.vt.row_text(3), "frame");
    assert!(!rig.vt.autowrap(), "autowrap is off again");
}

#[test]
fn println_keeps_styles_and_ends_them() {
    let mut rig = Rig::inline(20, 4);
    let _ = rig.draw("frame");
    let out = rig.print("\x1b[1mbold\tx");
    assert!(out.contains("bold    x"), "tabs expanded: {out:?}");
    assert_eq!(rig.vt.pen(), cell::Pen::DEFAULT);
    assert_eq!(rig.vt.raw_cell(0, 0).pen.attrs, cell::attr::BOLD);
}

#[test]
fn println_does_nothing_on_the_alternate_screen() {
    let mut rig = Rig::alt(20, 4);
    let _ = rig.draw("frame");
    assert_eq!(rig.print("lost"), "");
}

#[test]
fn switching_to_the_alternate_screen_and_back() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("inline\nframe");
    rig.vt.feed(b"\x1b[?1049h");
    rig.r.switch_screen(true);
    let _ = rig.draw("full\nscreen");
    rig.expect("full\nscreen");
    rig.vt.feed(b"\x1b[?1049l");
    rig.r.switch_screen(false);
    let _ = rig.draw("inline\nagain");
    rig.expect("inline\nagain");
}

#[test]
fn the_cursor_goes_where_the_view_says() {
    let mut rig = Rig::alt(20, 5);
    let view = View::new("name: \nx").with_cursor(Cursor::new(6, 0));
    let out = rig.view(&view);
    assert_eq!(rig.vt.cursor(), (6, 0));
    assert!(rig.vt.cursor_visible());
    assert!(out.ends_with("\x1b[?25h"), "{out:?}");
    let moved = View::new("name: \nx").with_cursor(Cursor::new(1, 1));
    let out = rig.view(&moved);
    assert_eq!(rig.vt.cursor(), (1, 1));
    assert_eq!(out, "\x1b[?25l\r\n\x1b[C\x1b[?25h", "hidden while it moves");
    let _ = rig.draw("name: \nx");
    assert!(!rig.vt.cursor_visible());
}

#[test]
fn the_cursor_of_a_tall_inline_frame_moves_up_with_its_rows() {
    let mut rig = Rig::inline(20, 3);
    let view = View::new("1\n2\n3\n4\n5").with_cursor(Cursor::new(0, 4));
    let _ = rig.view(&view);
    assert_eq!(rig.vt.cursor(), (0, 2));
}

#[test]
fn synchronized_output_wraps_each_frame() {
    let mut rig = Rig::alt(20, 3);
    rig.r.set_sync(true);
    let out = rig.draw("a");
    assert!(out.starts_with("\x1b[?2026h") && out.ends_with("\x1b[?2026l"));
    assert_eq!(rig.draw("a"), "", "no empty brackets");
    let _ = rig.draw("b");
    assert_eq!(rig.vt.sync_frames(), 2);
}

#[test]
fn a_clear_screen_redraws_from_the_top_left() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("x");
    rig.r.clear_screen();
    let out = rig.draw("x");
    assert!(out.contains("\x1b[H\x1b[2J"), "{out:?}");
    rig.expect("x");
}

/// Finishes `rig` and feeds what that writes to its terminal.
fn finish(rig: &mut Rig) -> String {
    let mut out = Vec::new();
    rig.r.finish(&mut out).unwrap();
    rig.vt.feed(&out);
    assert!(rig.vt.unknown().is_empty(), "{:?}", rig.vt.unknown());
    String::from_utf8(out).unwrap()
}

#[test]
fn finishing_inline_keeps_every_row_and_leaves_the_cursor_below() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("done\nlast line");
    let _ = finish(&mut rig);
    rig.expect("done\nlast line");
    assert_eq!(rig.vt.cursor(), (0, 2), "a fresh line below the frame");
    assert!(rig.vt.cursor_visible());
    assert_eq!(finish(&mut rig), "", "only once");
}

#[test]
fn finishing_a_frame_ending_in_a_newline_keeps_its_blank_row() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("done\n");
    let _ = finish(&mut rig);
    assert_eq!(rig.vt.text(), "done");
    assert_eq!(rig.vt.cursor(), (0, 2));
}

#[test]
fn finishing_at_the_bottom_of_the_screen_scrolls_to_a_fresh_line() {
    let mut rig = Rig::inline(20, 3);
    let _ = rig.draw("a\nb\nc");
    let _ = finish(&mut rig);
    assert_eq!(rig.vt.scrollback(), ["a"]);
    assert_eq!(rig.vt.text(), "b\nc");
    assert_eq!(rig.vt.cursor(), (0, 2));
}

#[test]
fn finishing_after_a_shorter_frame_clears_what_a_taller_one_left() {
    let mut rig = Rig::inline(20, 6);
    let _ = rig.draw("1\n2\n3\n4");
    let _ = rig.draw("1\n2");
    rig.vt.feed(b"\x1b7\x1b[4;1Hjunk\x1b8");
    let _ = finish(&mut rig);
    assert_eq!(rig.vt.text(), "1\n2");
    assert_eq!(rig.vt.cursor(), (0, 2));
}

#[test]
fn finishing_an_empty_view_leaves_nothing_behind() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("progress\nbar");
    let _ = rig.draw("");
    let _ = finish(&mut rig);
    assert_eq!(rig.vt.text(), "");
    assert_eq!(rig.vt.cursor(), (0, 0), "where the frame started");
}

#[test]
fn finishing_before_any_frame_writes_nothing() {
    let mut out = Vec::new();
    Renderer::new(Size::new(10, 3)).finish(&mut out).unwrap();
    assert_eq!(out, b"");
}

#[test]
fn an_empty_inline_frame_takes_no_rows() {
    let mut rig = Rig::inline(20, 3);
    let _ = a_then_b(&mut rig, "a\nb", "");
    assert_eq!(rig.vt.text(), "");
}

#[test]
fn a_full_redraw_costs_about_the_text() {
    let mut rig = Rig::alt(40, 10);
    let a = log(0, 10);
    let b: String = (0..10)
        .map(|i| format!("{i}{i}{i} different text on this row"))
        .collect::<Vec<_>>()
        .join("\n");
    let out = a_then_b(&mut rig, &a, &b);
    assert!(out.len() < b.len() + 10 * 8, "{} bytes", out.len());
}

#[test]
fn the_grapheme_width_method_changes_cluster_widths() {
    let mut rig = Rig::alt(20, 3);
    rig.vt = Vt::new(20, 3).with_width_method(WidthMethod::GraphemeWidth);
    rig.vt.feed(b"\x1b[?7l\x1b[?1049h");
    rig.r.set_width_method(WidthMethod::GraphemeWidth);
    let _ = rig.draw("ok 👍🏽 ok");
    assert_eq!(rig.vt.row_text(0), "ok 👍🏽 ok");
}

#[test]
fn println_after_the_frame_shrank_goes_above_its_top() {
    let mut rig = Rig::inline(20, 6);
    let _ = rig.draw("one\ntwo\nthree");
    let _ = rig.draw("one");
    let _ = rig.print("printed");
    rig.expect("printed\none");
}

#[test]
fn a_release_erases_an_inline_frame_and_the_next_frame_is_whole() {
    let mut rig = Rig::inline(20, 5);
    let _ = rig.draw("one\ntwo");
    let mut out = Vec::new();
    rig.r.release(&mut out).unwrap();
    let released = String::from_utf8(out).unwrap();
    assert!(released.contains("\r\x1b[J"), "{released:?}");
    rig.vt.feed(released.as_bytes());
    assert_eq!(rig.vt.text(), "", "the frame is gone");
    let again = rig.draw("one\ntwo");
    assert!(again.contains("one") && again.contains("two"), "{again:?}");
}

#[test]
fn a_repaint_draws_an_unchanged_frame_again() {
    let mut rig = Rig::alt(20, 5);
    let _ = rig.draw("same");
    assert_eq!(rig.draw("same"), "");
    rig.r.repaint();
    let again = rig.draw("same");
    assert!(
        again.contains("\x1b[2J") && again.contains("same"),
        "{again:?}"
    );
}

#[test]
fn styles_come_down_to_the_colour_profile_in_frames_and_printed_lines() {
    let mut rig = Rig::inline(20, 5);
    rig.r.set_color_profile(ColorProfile::Ascii);
    let frame = rig.draw("\x1b[1;38;2;1;2;3mx");
    assert!(
        frame.contains("\x1b[1m") && !frame.contains("38;"),
        "{frame:?}"
    );
    let printed = rig.print("\x1b[4;31my");
    assert!(
        printed.contains("\x1b[4m") && !printed.contains("31"),
        "{printed:?}"
    );
}
