//! The components against Bubbles v2 itself: tests/fixtures/bubbles.tsv,
//! written by scripts/bubbles-fixture from the Go packages, holds every
//! default key binding, the spinners, help and paginator views, text
//! input and text area editing sequences, list filter ranks, colour
//! blends, springs, progress bars, viewports, tables and lists. This
//! module reads it and checks the key maps; each component's tests
//! check its own lines.

use crate::key::{Key, KeyCode, KeyMod};
use crate::mouse::{Mouse, MouseButton};
use crate::msg::Msg;

/// The fixture, as Go wrote it.
pub(crate) const FIXTURE: &str = include_str!("../../tests/fixtures/bubbles.tsv");

/// The lines of one kind, split into fields (the kind left out).
pub(crate) fn rows(kind: &str) -> Vec<Vec<String>> {
    let rows: Vec<Vec<String>> = FIXTURE
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            (fields.next() == Some(kind)).then(|| fields.map(unescape).collect())
        })
        .collect();
    assert!(!rows.is_empty(), "no {kind} lines in the fixture");
    rows
}

/// A field with `\\`, `\t` and `\n` read back.
pub(crate) fn unescape(field: &str) -> String {
    let mut out = String::new();
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// A Go field name in snake case: `CharacterForward` is
/// `character_forward`.
pub(crate) fn snake(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() && !out.is_empty() {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// The messages for a token sequence, as the Go generator builds them:
/// `t:text` types (· is a space), `p:text` pastes (¶ is a line break,
/// ⇥ a tab),
/// `w:down` turns the wheel, anything else is a key such as `ctrl+w`.
pub(crate) fn messages<M>(tokens: &str) -> Vec<Msg<M>> {
    let mut out = Vec::new();
    for tok in tokens.split_whitespace() {
        if let Some(text) = tok.strip_prefix("t:") {
            out.extend(
                text.replace('·', " ")
                    .chars()
                    .map(|c| Msg::KeyPress(typed(c))),
            );
        } else if let Some(name) = tok.strip_prefix("w:") {
            out.push(Msg::MouseWheel(wheel(name)));
        } else if let Some(text) = tok.strip_prefix("p:") {
            let text = text.replace('·', " ").replace('¶', "\n").replace('⇥', "\t");
            out.push(Msg::Paste(text));
        } else {
            out.push(Msg::KeyPress(key(tok)));
        }
    }
    out
}

/// A wheel turn: `up`, `down`, `left` or `right`, maybe after `shift+`.
fn wheel(name: &str) -> Mouse {
    let (mods, name) = match name.strip_prefix("shift+") {
        Some(rest) => (KeyMod::SHIFT, rest),
        None => (KeyMod::NONE, name),
    };
    let button = match name {
        "up" => MouseButton::WheelUp,
        "down" => MouseButton::WheelDown,
        "left" => MouseButton::WheelLeft,
        _ => MouseButton::WheelRight,
    };
    Mouse {
        x: 0,
        y: 0,
        button,
        mods,
    }
}

/// The key that types `c`, as the generator makes it: the character as
/// the code, no modifiers.
fn typed(c: char) -> Key {
    let mut k = Key::new(KeyCode::from_rune(i64::from(u32::from(c))));
    k.text = c.to_string();
    k
}

/// A key token.
fn key(tok: &str) -> Key {
    let mut parts: Vec<&str> = tok.split('+').collect();
    let name = parts.pop().unwrap_or_default();
    let mut mods = KeyMod::NONE;
    for p in parts {
        mods |= match p {
            "ctrl" => KeyMod::CTRL,
            "alt" => KeyMod::ALT,
            "shift" => KeyMod::SHIFT,
            other => panic!("unknown modifier {other}"),
        };
    }
    let named = match name {
        "space" => Some(KeyCode::Space),
        "esc" => Some(KeyCode::Esc),
        "backspace" => Some(KeyCode::Backspace),
        "enter" => Some(KeyCode::Enter),
        "tab" => Some(KeyCode::Tab),
        other => KeyCode::from_name(other),
    };
    let mut k = match named {
        Some(code) => Key::new(code),
        None => typed(name.chars().next().unwrap_or(' ')),
    };
    k.mods = mods;
    if !mods.is_empty() || named.is_some() {
        k.text = String::new();
    }
    if named == Some(KeyCode::Space) && mods.is_empty() {
        k.text = " ".into();
    }
    k
}

// Every default binding of every component, against Bubbles v2's.

use crate::bubbles::key::Binding;
use crate::bubbles::{list, paginator, table, textarea, textinput, viewport};

/// A component's key map as (field, binding) pairs.
type Fields<'a> = Vec<(&'static str, &'a Binding)>;

fn textinput_fields(k: &textinput::KeyMap) -> Fields<'_> {
    vec![
        ("character_forward", &k.character_forward),
        ("character_backward", &k.character_backward),
        ("word_forward", &k.word_forward),
        ("word_backward", &k.word_backward),
        ("delete_word_backward", &k.delete_word_backward),
        ("delete_word_forward", &k.delete_word_forward),
        ("delete_after_cursor", &k.delete_after_cursor),
        ("delete_before_cursor", &k.delete_before_cursor),
        ("delete_character_backward", &k.delete_character_backward),
        ("delete_character_forward", &k.delete_character_forward),
        ("line_start", &k.line_start),
        ("line_end", &k.line_end),
        ("paste", &k.paste),
        ("accept_suggestion", &k.accept_suggestion),
        ("next_suggestion", &k.next_suggestion),
        ("prev_suggestion", &k.prev_suggestion),
    ]
}

fn paginator_fields(k: &paginator::KeyMap) -> Fields<'_> {
    vec![("prev_page", &k.prev_page), ("next_page", &k.next_page)]
}

fn textarea_fields(k: &textarea::KeyMap) -> Fields<'_> {
    vec![
        ("character_backward", &k.character_backward),
        ("character_forward", &k.character_forward),
        ("delete_after_cursor", &k.delete_after_cursor),
        ("delete_before_cursor", &k.delete_before_cursor),
        ("delete_character_backward", &k.delete_character_backward),
        ("delete_character_forward", &k.delete_character_forward),
        ("delete_word_backward", &k.delete_word_backward),
        ("delete_word_forward", &k.delete_word_forward),
        ("insert_newline", &k.insert_newline),
        ("line_end", &k.line_end),
        ("line_next", &k.line_next),
        ("line_previous", &k.line_previous),
        ("line_start", &k.line_start),
        ("page_up", &k.page_up),
        ("page_down", &k.page_down),
        ("paste", &k.paste),
        ("word_backward", &k.word_backward),
        ("word_forward", &k.word_forward),
        ("input_begin", &k.input_begin),
        ("input_end", &k.input_end),
        ("uppercase_word_forward", &k.uppercase_word_forward),
        ("lowercase_word_forward", &k.lowercase_word_forward),
        ("capitalize_word_forward", &k.capitalize_word_forward),
        (
            "transpose_character_backward",
            &k.transpose_character_backward,
        ),
        ("select_character_forward", &k.select_character_forward),
        ("select_character_backward", &k.select_character_backward),
        ("select_word_forward", &k.select_word_forward),
        ("select_word_backward", &k.select_word_backward),
        ("select_line_up", &k.select_line_up),
        ("select_line_down", &k.select_line_down),
        ("select_all", &k.select_all),
        ("copy_selection", &k.copy_selection),
    ]
}

fn viewport_fields(k: &viewport::KeyMap) -> Fields<'_> {
    vec![
        ("page_down", &k.page_down),
        ("page_up", &k.page_up),
        ("half_page_up", &k.half_page_up),
        ("half_page_down", &k.half_page_down),
        ("down", &k.down),
        ("up", &k.up),
        ("left", &k.left),
        ("right", &k.right),
    ]
}

fn list_fields(k: &list::KeyMap) -> Fields<'_> {
    vec![
        ("cursor_up", &k.cursor_up),
        ("cursor_down", &k.cursor_down),
        ("next_page", &k.next_page),
        ("prev_page", &k.prev_page),
        ("go_to_start", &k.go_to_start),
        ("go_to_end", &k.go_to_end),
        ("filter", &k.filter),
        ("clear_filter", &k.clear_filter),
        ("cancel_while_filtering", &k.cancel_while_filtering),
        ("accept_while_filtering", &k.accept_while_filtering),
        ("show_full_help", &k.show_full_help),
        ("close_full_help", &k.close_full_help),
        ("quit", &k.quit),
        ("force_quit", &k.force_quit),
    ]
}

fn table_fields(k: &table::KeyMap) -> Fields<'_> {
    vec![
        ("line_up", &k.line_up),
        ("line_down", &k.line_down),
        ("page_up", &k.page_up),
        ("page_down", &k.page_down),
        ("half_page_up", &k.half_page_up),
        ("half_page_down", &k.half_page_down),
        ("goto_top", &k.goto_top),
        ("goto_bottom", &k.goto_bottom),
    ]
}

/// Checks a component's bindings against the fixture's, field by field,
/// and that neither has a field the other lacks.
fn check(component: &str, fields: &Fields<'_>) {
    let want: Vec<Vec<String>> = rows("keymap")
        .into_iter()
        .filter(|r| r[0] == component)
        .collect();
    assert_eq!(want.len(), fields.len(), "{component}: number of bindings");
    for row in want {
        let name = snake(&row[1]);
        let (_, b) = fields
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("{component} has no {name}"));
        let keys: Vec<&str> = row[2].split_whitespace().collect();
        assert_eq!(b.keys(), keys.as_slice(), "{component}.{name} keys");
        assert_eq!(b.help().key, row[3], "{component}.{name} help key");
        assert_eq!(b.help().desc, row[4], "{component}.{name} help");
        assert_eq!(b.enabled(), row[5] == "1", "{component}.{name} enabled");
    }
}

#[test]
fn textinput_bindings_match_bubbles() {
    check(
        "textinput",
        &textinput_fields(&textinput::KeyMap::default()),
    );
}

#[test]
fn paginator_bindings_match_bubbles() {
    check(
        "paginator",
        &paginator_fields(&paginator::KeyMap::default()),
    );
}

#[test]
fn textarea_bindings_match_bubbles() {
    check("textarea", &textarea_fields(&textarea::KeyMap::default()));
}

#[test]
fn viewport_bindings_match_bubbles() {
    check("viewport", &viewport_fields(&viewport::KeyMap::default()));
}

#[test]
fn list_bindings_match_bubbles() {
    check("list", &list_fields(&list::KeyMap::default()));
}

#[test]
fn table_bindings_match_bubbles() {
    check("table", &table_fields(&table::KeyMap::default()));
}

#[test]
fn every_component_with_a_key_map_is_checked() {
    let mut components: Vec<String> = rows("keymap").into_iter().map(|r| r[0].clone()).collect();
    components.dedup();
    assert_eq!(
        components,
        [
            "textinput",
            "textarea",
            "viewport",
            "list",
            "table",
            "paginator"
        ]
    );
}
