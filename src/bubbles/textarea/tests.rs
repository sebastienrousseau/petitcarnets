use super::*;
use crate::bubbles::tests::{messages, rows};
use crate::style::strip_ansi;

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

#[test]
fn text_area_editing_matches_bubbles() {
    for row in rows("textarea") {
        let n = |i: usize| row[i].parse::<usize>().expect("number");
        let mut m = Model::new();
        m.show_line_numbers = row[3] == "true";
        if n(5) > 0 {
            m.max_height = n(5);
        }
        m.set_width(n(1));
        m.set_height(n(2));
        m.char_limit = n(4);
        m.placeholder.clone_from(&row[6]);
        let _ = m.focus();
        for msg in messages(&row[7]) {
            let _ = m.update(&msg);
        }
        let got = (m.value(), m.line(), m.column(), strip_ansi(&m.view()));
        let want = (row[8].clone(), n(9), n(10), row[11].clone());
        assert_eq!(got, want, "case {}", row[0]);
    }
}

// The text area's own behaviour.

use crate::{ClipboardSelection, Key, KeyCode, KeyMod, Msg as TeaMsg};

fn focused(width: usize, height: usize) -> Model {
    let mut m = Model::default();
    m.set_width(width);
    m.set_height(height);
    let _ = m.focus();
    m
}

fn press(m: &mut Model, k: Key) -> Cmd<Msg> {
    m.update(&TeaMsg::KeyPress(k))
}

#[test]
fn values_lines_and_sizes() {
    let mut m = focused(30, 4);
    m.set_value("one two\nthree");
    assert_eq!((m.line_count(), m.length()), (2, 13));
    m.insert_rune('!');
    m.insert_string("?");
    assert_eq!(m.value(), "one two\nthree!?");
    assert_eq!(m.width(), 30 - 2 - 4);
    assert_eq!(m.scroll_y_offset(), 0);
    assert!((m.scroll_percent() - 1.0).abs() < f64::EPSILON);
    m.max_height = 0;
    m.set_height(500);
    assert_eq!(m.height(), 500, "no limit");
    m.set_cursor_column(1);
    assert_eq!(m.word(), "three!?");
    m.cursor_start();
    assert_eq!(m.word(), "");
    assert!(format!("{m:?}").contains("Model"));
    assert!(m.virtual_cursor());
    m.set_virtual_cursor(false);
    assert!(!m.virtual_cursor());
}

#[test]
fn a_prompt_function_and_a_real_cursor() {
    let mut m = focused(30, 3);
    m.set_prompt_func(4, |info| format!("{}>", info.line_number));
    m.set_width(30);
    m.set_value("ab");
    let view = strip_ansi(&m.view());
    assert!(view.starts_with("  0>  1 ab") && view.contains("\n  1>"));
    assert_eq!(m.cursor(), None, "a drawn cursor");
    let mut s = m.styles();
    s.cursor.color = crate::style::Color::Rgb(1, 2, 3);
    s.cursor.blink = false;
    m.set_styles(s);
    m.set_virtual_cursor(false);
    let c = m.cursor().expect("a real cursor");
    assert_eq!((c.x, c.y, c.color), (10, 0, Some(crate::Rgb::new(1, 2, 3))));
    m.blur();
    assert_eq!(m.cursor(), None);
    assert!(
        press(&mut m, Key::char('x')).is_none(),
        "blurred, keys do nothing"
    );
}

#[test]
fn pastes_from_the_clipboard_and_copies_the_selection() {
    let mut m = focused(30, 3);
    assert!(
        !press(&mut m, Key::ctrl('v')).is_none(),
        "asks for the clipboard"
    );
    let reply = TeaMsg::Clipboard {
        content: "pasted".into(),
        selection: ClipboardSelection::System,
    };
    let _ = m.update(&reply);
    let _ = m.update(&reply);
    assert_eq!(m.value(), "pasted", "only the reply asked for");
    assert!(m.copy_selection::<()>().is_none());
    let _ = press(
        &mut m,
        Key::new(KeyCode::Left).with_mods(KeyMod::SHIFT | KeyMod::CTRL),
    );
    let _ = press(&mut m, Key::new(KeyCode::Left).with_mods(KeyMod::SHIFT));
    let _ = press(&mut m, Key::new(KeyCode::Right).with_mods(KeyMod::SHIFT));
    assert_eq!(m.selected_text(), "asted");
    assert!(
        !press(
            &mut m,
            Key::new(KeyCode::Char('c')).with_mods(KeyMod::CTRL | KeyMod::SHIFT)
        )
        .is_none()
    );
    let _ = press(
        &mut m,
        Key::new(KeyCode::Right).with_mods(KeyMod::SHIFT | KeyMod::ALT),
    );
    let _ = press(&mut m, Key::new(KeyCode::Up).with_mods(KeyMod::SHIFT));
    let _ = press(&mut m, Key::ctrl('g'));
    assert_eq!(
        m.selection(),
        Some((Position::default(), Position { row: 0, col: 6 }))
    );
    let _ = press(&mut m, Key::char('Z'));
    assert_eq!(m.value(), "Z");
}

#[test]
fn the_mouse_selects_by_cell() {
    let mut m = focused(20, 4);
    m.set_value("hello world\nsecond line");
    m.begin_selection(6, 0);
    m.extend_selection(6 + 5, 1);
    m.end_selection();
    assert_eq!(m.selected_text(), "hello world\nsecon");
    m.extend_selection(1, 1);
    m.begin_selection(8, 0);
    m.end_selection();
    assert!(!m.has_selection(), "an empty selection is dropped");
    assert_eq!(
        m.position_at(0, 9),
        Position { row: 1, col: 11 },
        "past the end"
    );
    assert_eq!(m.position_at(99, 0), Position { row: 0, col: 11 });
}

#[test]
fn dynamic_height_and_content_limits() {
    let mut m = focused(20, 3);
    m.dynamic_height = true;
    m.min_height = 2;
    m.max_height = 4;
    m.set_value("a\nb\nc\nd\ne");
    assert_eq!(m.height(), 4);
    m.reset();
    assert_eq!(m.height(), 2);
    m.max_content_height = 2;
    m.insert_string("one\ntwo\nthree");
    assert_eq!(m.value(), "one\ntwo", "cut to two wrapped lines");
    let _ = press(&mut m, Key::new(KeyCode::Enter));
    assert_eq!(m.line_count(), 2, "no new line over the limit");
    m.insert_string(&"x".repeat(40));
    assert_eq!(
        m.value(),
        "one\ntwo",
        "a line that would wrap over the limit"
    );
}

#[test]
fn words_and_lines_join_at_the_edges() {
    let mut m = focused(30, 4);
    m.set_value("ab cd\nef");
    m.move_to_begin();
    let _ = press(&mut m, Key::new(KeyCode::Delete).with_mods(KeyMod::ALT));
    let _ = press(&mut m, Key::new(KeyCode::Delete).with_mods(KeyMod::ALT));
    assert_eq!(m.value(), "\nef");
    let _ = press(&mut m, Key::new(KeyCode::Delete).with_mods(KeyMod::ALT));
    assert_eq!(m.value(), "ef", "at the end of a line it joins the next");
    let _ = press(&mut m, Key::new(KeyCode::Right).with_mods(KeyMod::ALT));
    let _ = press(&mut m, Key::new(KeyCode::Right).with_mods(KeyMod::ALT));
    let _ = press(&mut m, Key::new(KeyCode::Left).with_mods(KeyMod::ALT));
    assert_eq!(m.column(), 0);
    let _ = press(&mut m, Key::new(KeyCode::PgDown));
    let _ = press(&mut m, Key::new(KeyCode::PgUp));
    m.set_value("a\n\nb");
    m.move_to_begin();
    m.cursor_down();
    let _ = press(&mut m, Key::new(KeyCode::Backspace).with_mods(KeyMod::ALT));
    assert_eq!(m.value(), "a\nb");
    let _ = press(&mut m, Key::ctrl('t'));
    let _ = press(&mut m, Key::new(KeyCode::Char('l')).with_mods(KeyMod::ALT));
    assert_eq!(strip_ansi(&m.view()).lines().count(), 4);
}

#[test]
fn the_cursor_at_the_wrap_edge_and_an_empty_placeholder() {
    let mut m = focused(16, 3);
    m.show_line_numbers = false;
    m.set_width(16);
    m.insert_string("abcdefghijklmn");
    assert!(strip_ansi(&m.view()).starts_with("┃ abcdefghijklmn"));
    let mut p = focused(16, 2);
    p.placeholder = "   ".into();
    assert!(strip_ansi(&p.view()).starts_with("┃   1"));
    assert!(!blink().is_none());
}

#[test]
fn the_prompt_function_is_a_closure_that_keeps_state() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&calls);
    let marker = String::from("│");
    let mut m = focused(20, 2);
    m.show_line_numbers = false;
    m.set_prompt_func(2, move |info| {
        let _ = seen.fetch_add(1, Ordering::Relaxed);
        format!("{}{marker}", info.line_number)
    });
    m.set_width(20);
    m.set_value("ab\ncd");
    let copy = m.clone();
    let view = strip_ansi(&m.view());
    assert!(
        view.starts_with("0│ab") && view.contains("\n1│cd"),
        "{view:?}"
    );
    assert!(calls.load(Ordering::Relaxed) >= 2, "called for each line");
    assert_eq!(strip_ansi(&copy.view()), view, "a clone shares it");
    assert!(format!("{m:?}").contains("prompt_fn: Some(.."));
}
