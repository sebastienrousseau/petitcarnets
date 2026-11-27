use super::*;

#[test]
fn a_plain_style_leaves_text_alone() {
    assert_eq!(Style::new().paint("hi"), "hi");
}

#[test]
fn styles_become_sgr_codes() {
    assert_eq!(
        Style::new().bold().fg(Color::RED).paint("x"),
        "\x1b[1;31mx\x1b[0m"
    );
    assert_eq!(
        Style::new()
            .fg(Color::Ansi(9))
            .bg(Color::Ansi(12))
            .paint("x"),
        "\x1b[91;104mx\x1b[0m"
    );
    assert_eq!(
        Style::new().fg(Color::Palette(208)).paint("x"),
        "\x1b[38;5;208mx\x1b[0m"
    );
    assert_eq!(
        Style::new().bg(Color::Rgb(1, 2, 3)).paint("x"),
        "\x1b[48;2;1;2;3mx\x1b[0m"
    );
    assert_eq!(
        Style::new().dim().italic().underline().reverse().paint("x"),
        "\x1b[2;3;4;7mx\x1b[0m"
    );
}

#[test]
fn width_ignores_escape_sequences() {
    let s = Style::new().bold().fg(Color::GREEN).paint("héllo");
    assert_eq!(width(&s), 5);
    assert_eq!(strip_ansi(&s), "héllo");
}

#[test]
fn width_counts_wide_characters_twice() {
    assert_eq!(width("字a"), 3);
    assert_eq!(width("e\u{301}"), 1);
    assert_eq!(
        truncate("字字", 3),
        "字\x1b[0m",
        "a wide character is not split"
    );
    assert_eq!(width_method(), WidthMethod::WcWidth);
}

#[test]
fn truncation_keeps_styles_and_resets() {
    assert_eq!(truncate("hello", 10), "hello");
    assert_eq!(truncate("hello", 3), "hel\x1b[0m");
    let styled = Style::new().bold().paint("hello");
    assert_eq!(strip_ansi(&truncate(&styled, 2)), "he");
    assert!(truncate(&styled, 2).ends_with("\x1b[0m"));
}

#[test]
fn every_builder_sets_what_it_says() {
    let s = Style::new()
        .fg(Color::YELLOW)
        .bg(Color::BLUE)
        .bold()
        .dim()
        .italic()
        .underline()
        .reverse()
        .padding(1, 2)
        .margin(1, 1)
        .border(Border::DOUBLE)
        .border_fg(Color::MAGENTA)
        .width(12)
        .height(6);
    assert_eq!(s.horizontal_frame(), 2 + 2 + 1 + 1 + 1 + 1);
    assert_eq!(s.vertical_frame(), 1 + 1 + 1 + 1 + 1 + 1);
    assert_eq!(s.frame_offset(), (2 + 1 + 1, 1 + 1 + 1));
    assert!(format!("{:?}", s.plain()).contains("Ansi(3)") && s.plain() != s);
    assert_eq!(Style::default(), Style::new());
    let m = Style::new().margin_sides(1, 2, 3, 4).margin_top(5);
    assert_eq!(m.get_margin_top(), 5);
    assert_eq!(Style::new().padding_left(3).get_padding_left(), 3);
    assert_eq!(
        Style::new().padding_sides(0, 4, 0, 0).get_padding_right(),
        4
    );
    assert_eq!(
        Style::new()
            .fg(Color::CYAN)
            .inherit(Style::new().fg(Color::RED).bg(Color::WHITE).bold()),
        Style::new().fg(Color::CYAN).bg(Color::WHITE).bold()
    );
    assert_ne!(Color::GREY, Color::BLACK);
}

#[test]
fn a_block_gets_its_border_padding_margins_and_size() {
    let boxed = Style::new()
        .border(Border::NORMAL)
        .border_sides([true, false, true, true])
        .margin_sides(1, 1, 1, 2)
        .height(4)
        .render("ab\nc");
    let plain = strip_ansi(&boxed);
    assert_eq!(
        plain, "      \n  ┌── \n  │ab \n  │c  \n  └── \n      ",
        "the height holds the border"
    );
    let hidden = Style::new().border(Border::HIDDEN).render("x");
    assert_eq!(hidden, "   \n x \n   ");
    let thick = Style::new()
        .border(Border::THICK)
        .border_sides([false; 4])
        .render("x");
    assert_eq!(thick, "x", "no side, no border");
    let reversed = Style::new()
        .reverse()
        .fg(Color::RED)
        .padding(0, 1)
        .render("x");
    assert_eq!(strip_ansi(&reversed), " x ");
    assert!(
        reversed.starts_with("\x1b[7;31m"),
        "spaces keep the reversed colours"
    );
    let tabs = Style::new().bold().render("a\tb\r\nc");
    assert_eq!(strip_ansi(&tabs), "a    b\nc     ");
}

#[test]
fn blocks_join_cut_wrap_and_measure() {
    assert_eq!(join_horizontal(Position::BOTTOM, &["a\nb", "c"]), "a \nbc");
    assert_eq!(
        join_horizontal(Position::CENTER, &["a\nb\nc", "d"]),
        "a \nbd\nc "
    );
    assert_eq!(join_vertical(Position::CENTER, &["abc", "d"]), "abc\n d ");
    assert_eq!(join_vertical(Position::LEFT, &["x"]), "x");
    assert_eq!(join_horizontal(Position::TOP, &[]), "");
    assert_eq!(block_width("a\nabc\n"), 3);
    assert_eq!(height(""), 1);
    let styled = Style::new().bold().paint("abcdef");
    assert_eq!(strip_ansi(&cut(&styled, 2, 4)), "cd");
    assert!(
        cut(&styled, 2, 4).starts_with("\x1b[1m"),
        "escapes are kept"
    );
    assert_eq!(cut("a\tb", 0, 3), "ab", "controls are dropped");
    assert_eq!(wrap("aaa bbbbbbb c", 4), "aaa\nbbbb\nbbb\nc");
    assert_eq!(wrap("  lead", 10), "  lead");
    assert_eq!(wrap("x", 0), "x");
    assert_eq!(wrap("ab  ", 10), "ab  ");
    assert_eq!(truncate_with("字字字", 4, "…"), "字…\x1b[0m");
}
