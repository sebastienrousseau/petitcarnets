use super::*;
use crate::bubbles::tests::{messages, rows};
use crate::style::strip_ansi;

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

/// Cases where Go is wrong and the port is not: (case, Go's view, ours).
/// Go pads a placeholder with NUL characters, by characters rather than
/// cells, so a wide one overflows the width with NULs after it.
const INPUT_FIXES: [(&str, &str, &str); 1] = [("placeholder-cjk", "> 日本語です\0\0", "> 日本語 ")];

/// Cases where Go's text input works on code points and splits a
/// grapheme cluster (moving into it, deleting part of it, or scrolling
/// half of it out of view), and the port, as Bubbles v2's own text area
/// does, keeps it whole: (case, value, cursor, view).
const GRAPHEME_FIXES: [(&str, &str, usize, &str); 6] = [
    ("accent-left", "cafe\u{301}s", 3, "> cafe\u{301}s"),
    ("accent-right", "cafe\u{301}s", 5, "> cafe\u{301}s"),
    ("accent-delete", "cafs", 3, "> cafs"),
    ("emoji-backspace", "a", 1, "> a "),
    ("zwj-delete", "x", 0, "> x"),
    (
        "accent-scroll",
        "e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}",
        2,
        "> e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}",
    ),
];

#[test]
fn text_input_editing_matches_bubbles() {
    for row in rows("textinput") {
        let mut m = Model::new();
        m.set_width(row[1].parse().expect("width"));
        m.char_limit = row[2].parse().expect("limit");
        m.echo_mode = match row[3].as_str() {
            "1" => EchoMode::Password,
            "2" => EchoMode::None,
            _ => EchoMode::Normal,
        };
        m.placeholder.clone_from(&row[4]);
        if !row[5].is_empty() {
            m.show_suggestions = true;
            m.set_suggestions(&row[5].split(',').collect::<Vec<_>>());
        }
        let _ = m.focus();
        for msg in messages(&row[6]) {
            let _ = m.update(&msg);
        }
        let got = (m.value(), m.position().to_string(), strip_ansi(&m.view()));
        let mut want = (row[7].clone(), row[8].clone(), row[9].clone());
        if let Some((_, go, ours)) = INPUT_FIXES.iter().find(|(c, _, _)| *c == row[0]) {
            assert_eq!(want.2, *go, "Go still does this");
            want.2 = (*ours).to_string();
        }
        if let Some((_, value, pos, view)) = GRAPHEME_FIXES.iter().find(|f| f.0 == row[0]) {
            let ours = ((*value).to_string(), pos.to_string(), (*view).to_string());
            assert_ne!(want, ours, "Go still splits a cluster in {}", row[0]);
            want = ours;
        }
        assert_eq!(got, want, "case {}", row[0]);
    }
}

// The text input's own behaviour.

use crate::{ClipboardSelection, Key, Msg as TeaMsg};

#[test]
fn values_validation_and_suggestions() {
    let mut m = Model {
        validate: Some(|s| {
            if s.contains('!') {
                Err("no shouting".into())
            } else {
                Ok(())
            }
        }),
        ..Model::default()
    };
    m.set_value("hi\tthere!");
    assert_eq!(m.value(), "hi there!");
    assert_eq!(m.err.as_deref(), Some("no shouting"));
    assert!(
        m.update(&TeaMsg::KeyPress(Key::char('x'))).is_none(),
        "blurred"
    );
    assert!(!m.focused());
    m.show_suggestions = true;
    m.set_suggestions(&["hi there!", "hi you"]);
    m.reset();
    let _ = m.focus();
    let _ = m.update(&TeaMsg::KeyPress(Key::char('h')));
    assert_eq!(m.available_suggestions().len(), 2);
    assert_eq!(m.matched_suggestions().len(), 2);
    assert_eq!(
        (m.current_suggestion_index(), m.current_suggestion()),
        (0, "hi there!".into())
    );
    let _ = m.update(&TeaMsg::KeyPress(Key::ctrl('p')));
    assert_eq!(m.current_suggestion_index(), 1);
    m.set_suggestions(&[]);
    assert_eq!(m.current_suggestion(), "");
    assert_eq!(m.width(), 0);
    assert!(format!("{m:?}").contains("Model"));
}

#[test]
fn pastes_from_the_clipboard_and_places_the_real_cursor() {
    let mut m = Model::new();
    m.set_width(5);
    let _ = m.focus();
    assert!(!m.update(&TeaMsg::KeyPress(Key::ctrl('v'))).is_none());
    let reply = TeaMsg::Clipboard {
        content: "abcdefgh".into(),
        selection: ClipboardSelection::System,
    };
    let _ = m.update(&reply);
    let _ = m.update(&reply);
    assert_eq!(m.value(), "abcdefgh");
    assert!(m.virtual_cursor());
    assert_eq!(m.cursor(), None);
    let mut s = Styles::new(false);
    s.cursor.color = Color::Rgb(9, 8, 7);
    s.cursor.blink_speed = Duration::from_millis(100);
    m.set_styles(s);
    m.set_virtual_cursor(false);
    let c = m.cursor().expect("a real cursor");
    assert_eq!((c.x, c.color), (7, Some(crate::Rgb::new(9, 8, 7))));
    m.echo_mode = EchoMode::Password;
    let _ = m.update(&TeaMsg::KeyPress(
        Key::new(crate::KeyCode::Delete).with_mods(crate::KeyMod::ALT),
    ));
    let _ = m.update(&TeaMsg::KeyPress(
        Key::new(crate::KeyCode::Left).with_mods(crate::KeyMod::ALT),
    ));
    let _ = m.update(&TeaMsg::KeyPress(
        Key::new(crate::KeyCode::Right).with_mods(crate::KeyMod::ALT),
    ));
    assert_eq!(m.position(), 8);
    m.blur();
    assert_eq!(m.cursor(), None);
    assert!(!blink().is_none());
}

#[test]
fn thai_clusters_and_emoji_placeholders_stay_whole() {
    // Bubbles' issue #1097: the cursor stopped inside `น้ำ`.
    let mut m = Model::new();
    let _ = m.focus();
    m.set_value("กน้ำ");
    let _ = m.update(&TeaMsg::KeyPress(Key::new(crate::KeyCode::Left)));
    assert_eq!(m.position(), 1, "before the whole cluster");
    let _ = m.update(&TeaMsg::KeyPress(Key::new(crate::KeyCode::Home)));
    let _ = m.update(&TeaMsg::KeyPress(Key::new(crate::KeyCode::Right)));
    let _ = m.update(&TeaMsg::KeyPress(Key::new(crate::KeyCode::Delete)));
    assert_eq!(m.value(), "ก", "Delete takes the whole cluster");
    // Bubbles' issue #573: an emoji placeholder lost all but its first
    // code point under the cursor.
    let mut p = Model::new();
    p.placeholder = "👍🏽 yes".into();
    p.set_width(10);
    assert_eq!(
        strip_ansi(&p.view()),
        "> 👍🏽 yes   ",
        "eleven cells, the emoji four of them"
    );
}
