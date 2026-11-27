//! Views and their parts.

use super::*;

#[test]
fn views_come_from_strings() {
    let mut view: View = "hello".into();
    assert_eq!(view, View::new(String::from("hello")));
    view.set_content("bye");
    assert_eq!(view.content, "bye");
    assert!(!view.alt_screen, "inline by default, as Bubble Tea v2");
}

#[test]
fn builders_set_each_field() {
    let view = View::new("x")
        .with_alt_screen(true)
        .with_cursor(Cursor::new(1, 2))
        .with_window_title("t")
        .with_foreground_color(Rgb::new(1, 2, 3))
        .with_background_color(Rgb::new(4, 5, 6))
        .with_progress_bar(ProgressBar::new(ProgressBarState::Error, 200));
    assert!(view.alt_screen);
    assert_eq!(view.cursor.map(|c| (c.x, c.y)), Some((1, 2)));
    assert_eq!(view.window_title, "t");
    assert_eq!(view.foreground_color, Some(Rgb::new(1, 2, 3)));
    assert_eq!(view.background_color, Some(Rgb::new(4, 5, 6)));
    assert_eq!(view.progress_bar.map(|p| p.value), Some(100));
}

#[test]
fn cursor_styles_are_gos() {
    let c = Cursor::new(0, 0);
    assert_eq!(c.style(), 1, "a blinking block, as NewCursor");
    assert_eq!(c.with_blink(false).style(), 2);
    assert_eq!(c.with_shape(CursorShape::Underline).style(), 3);
    assert_eq!(c.with_shape(CursorShape::Bar).with_blink(false).style(), 6);
    assert_eq!(
        c.with_color(Rgb::new(1, 1, 1)).color,
        Some(Rgb::new(1, 1, 1))
    );
}

#[test]
fn progress_bars_write_osc_9_4() {
    let bar = |state, value| ProgressBar::new(state, value).sequence();
    assert_eq!(bar(ProgressBarState::None, 5), "\x1b]9;4;0\x07");
    assert_eq!(bar(ProgressBarState::Default, 5), "\x1b]9;4;1;5\x07");
    assert_eq!(bar(ProgressBarState::Error, 5), "\x1b]9;4;2;5\x07");
    assert_eq!(bar(ProgressBarState::Indeterminate, 5), "\x1b]9;4;3\x07");
    assert_eq!(bar(ProgressBarState::Warning, 5), "\x1b]9;4;4;5\x07");
}
