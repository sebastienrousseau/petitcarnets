//! Colour conversion and profile detection against Go's colorprofile:
//! every line of `tests/fixtures/color.tsv`, written by
//! `scripts/color-fixture` from Go, must come out the same here.

use super::convert::{to_16, to_256};
use super::*;
use crate::environ::Environment;
use crate::renderer::cell::{Colour, Pen, attr};

/// The fixture's lines, split at tabs, comments left out.
fn fixture() -> Vec<Vec<&'static str>> {
    include_str!("../../tests/fixtures/color.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect())
        .collect()
}

/// `#rrggbb` as a colour.
fn hex(s: &str) -> Rgb {
    let v = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap();
    Rgb::new(v(1), v(3), v(5))
}

/// The environment `KEY=VALUE KEY=VALUE` describes.
fn env(vars: &str) -> Environment {
    vars.split(' ')
        .filter(|kv| !kv.is_empty())
        .map(|kv| kv.split_once('=').unwrap())
        .collect()
}

/// What the Rust port makes of one fixture line, written as the fixture
/// writes it.
fn line(fields: &[&str]) -> String {
    match fields[0] {
        "rgb" => {
            let c = hex(fields[1]);
            let to = |p: ColorProfile| match p.colour(Colour::Rgb(c.r, c.g, c.b)) {
                Colour::Indexed(n) | Colour::Basic(n) => n.to_string(),
                other => format!("{other:?}"),
            };
            format!(
                "rgb\t{}\t{}\t{}",
                fields[1],
                to(ColorProfile::Ansi256),
                to(ColorProfile::Ansi)
            )
        }
        "idx" => {
            let n: u8 = fields[1].parse().unwrap();
            let Colour::Basic(to) = ColorProfile::Ansi.colour(Colour::Indexed(n)) else {
                return "not a basic colour".into();
            };
            format!("idx\t{n}\t{to}")
        }
        _ => {
            let profile = ColorProfile::detect_with(fields[1] == "1", &env(fields[2]), || None);
            format!("env\t{}\t{}\t{profile}", fields[1], fields[2])
        }
    }
}

#[test]
fn every_fixture_line_matches_go() {
    let lines = fixture();
    let count = |kind: &str| lines.iter().filter(|l| l[0] == kind).count();
    assert!(count("rgb") >= 1000, "{} colours", count("rgb"));
    assert!(count("env") >= 50, "{} environments", count("env"));
    let wrong: Vec<String> = lines
        .iter()
        .filter_map(|fields| {
            let want = fields.join("\t");
            let got = line(fields);
            (got != want).then(|| format!("want {want}\n got {got}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} lines differ:\n{}",
        wrong.len(),
        lines.len(),
        wrong.join("\n")
    );
}

#[test]
fn exact_cube_and_grey_colours_keep_their_index() {
    assert_eq!(to_256(Rgb::new(0x5f, 0x87, 0xaf)), 16 + 36 + 12 + 3);
    assert_eq!(to_256(Rgb::new(0, 0, 0)), 16);
    assert_eq!(to_256(Rgb::new(0xff, 0xff, 0xff)), 231);
    assert_eq!(to_16(196), 9, "bright red");
}

#[test]
fn ascii_drops_colours_but_keeps_attributes_and_no_tty_drops_both() {
    let pen = Pen {
        fg: Colour::Rgb(1, 2, 3),
        bg: Colour::Basic(4),
        underline_colour: Colour::Indexed(200),
        attrs: attr::BOLD | attr::ITALIC,
        underline: 1,
    };
    let ascii = ColorProfile::Ascii.pen(pen);
    assert_eq!(
        (ascii.fg, ascii.bg, ascii.underline_colour),
        (Colour::Default, Colour::Default, Colour::Default)
    );
    assert_eq!((ascii.attrs, ascii.underline), (pen.attrs, 1));
    assert_eq!(ColorProfile::NoTty.pen(pen), Pen::DEFAULT);
    assert_eq!(ColorProfile::TrueColor.pen(pen), pen);
    assert!(ColorProfile::NoTty.link(&Some("x".into())).is_none());
}

#[test]
fn terminal_colours_come_down_to_a_palette_colour_or_go() {
    let c = Rgb::new(0x12, 0x34, 0x56);
    assert_eq!(ColorProfile::TrueColor.rgb(c), Some(c));
    assert_eq!(ColorProfile::Ansi256.rgb(c), Some(Rgb::new(0, 0x5f, 0x5f)));
    assert_eq!(ColorProfile::Ascii.rgb(c), None);
    assert_eq!(ColorProfile::NoTty.rgb(c), None);
}

#[test]
fn tmux_info_upgrades_inside_tmux_only() {
    let tmux = env("TERM=screen-256color TMUX=/tmp/t,1,0");
    let rgb = || Some("  1: RGB: (flag) true\n".to_string());
    assert_eq!(
        ColorProfile::detect_with(true, &tmux, rgb),
        ColorProfile::TrueColor
    );
    assert_eq!(
        ColorProfile::detect_with(true, &tmux, || None),
        ColorProfile::Ansi256
    );
    let plain = env("TERM=xterm");
    assert_eq!(
        ColorProfile::detect_with(true, &plain, rgb),
        ColorProfile::Ansi
    );
}

#[test]
fn profiles_are_ordered_and_named_as_in_go() {
    assert!(ColorProfile::NoTty < ColorProfile::Ascii);
    assert!(ColorProfile::Ansi256 < ColorProfile::TrueColor);
    assert_eq!(ColorProfile::Ansi256.to_string(), "ANSI256");
}

#[test]
fn a_channel_with_anything_but_hex_digits_is_zero() {
    assert_eq!(channel(b""), 0);
    assert_eq!(channel(b"g"), 0);
    assert_eq!(channel(b"1g"), 0);
    assert_eq!(channel(b"g1"), 0);
    assert_ne!(channel(b"1f"), 0);
}
