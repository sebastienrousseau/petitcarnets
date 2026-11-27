//! Cell styles read and written as SGR codes.

use super::*;

fn read(params: &str) -> Pen {
    let mut pen = Pen::DEFAULT;
    pen.apply_sgr(params);
    pen
}

#[test]
fn sgr_codes_are_read_as_go_reads_them() {
    let pen = read("1;3;4:3;9;31;48;5;200;58:2::1:2:3");
    assert_eq!(pen.attrs, attr::BOLD | attr::ITALIC | attr::STRIKE);
    assert_eq!(pen.underline, 3);
    assert_eq!(pen.fg, Colour::Basic(1));
    assert_eq!(pen.bg, Colour::Indexed(200));
    assert_eq!(pen.underline_colour, Colour::Rgb(1, 2, 3));
    assert_eq!(read("38;2;10;20;30;97").fg, Colour::Basic(15));
    assert_eq!(read("38;2;10;20;30").fg, Colour::Rgb(10, 20, 30));
    assert_eq!(read("38:2:1:2:3").fg, Colour::Rgb(1, 2, 3));
    assert_eq!(read("1;22").attrs, 0, "22 clears bold and faint");
    assert_eq!(read("1;31;0"), Pen::DEFAULT);
    assert_eq!(read(""), Pen::DEFAULT, "an empty SGR resets");
    assert_eq!(read("4;24").underline, 0);
    assert_eq!(read("5;6;25").attrs, 0);
}

#[test]
fn transitions_write_only_what_changes_and_read_back() {
    let cases = [
        (Pen::DEFAULT, read("1;31")),
        (read("1"), read("1;31")),
        (read("1;31"), read("31")),
        (read("31;44"), Pen::DEFAULT),
        (read("4:2;38;5;9"), read("4:5;38;2;1;2;3;58;5;7")),
        (read("92;103"), read("92;103;7;8")),
    ];
    for (from, to) in cases {
        let sgr = from.transition(&to);
        let params = sgr
            .strip_prefix("\x1b[")
            .and_then(|s| s.strip_suffix('m'))
            .unwrap();
        let mut pen = from;
        pen.apply_sgr(params);
        assert_eq!(pen, to, "{sgr:?}");
    }
    assert_eq!(read("1").transition(&read("1")), "");
    assert_eq!(Pen::DEFAULT.transition(&read("1;31")), "\x1b[1;31m");
    assert_eq!(read("1;31").transition(&read("31")), "\x1b[0;31m");
}

#[test]
fn cells_hold_one_char_or_a_cluster() {
    let one = Cell::new("a", 1, Pen::DEFAULT, None);
    assert_eq!(one.glyph, Glyph::Char('a'));
    let mut cluster = Cell::new("e", 1, Pen::DEFAULT, None);
    cluster.append("\u{301}");
    let mut text = String::new();
    cluster.push_text(&mut text);
    assert_eq!(text, "e\u{301}");
    assert!(Cell::BLANK.is_blank() && !one.is_blank());
    assert!(Cell::continuation(Pen::DEFAULT, None).is_continuation());
}
