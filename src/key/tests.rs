//! Keys print and match as Bubble Tea's documentation says.

use super::*;

fn key(code: KeyCode, mods: KeyMod, text: &str) -> Key {
    let mut k = Key::new(code).with_mods(mods);
    k.text = text.to_string();
    k
}

#[test]
fn modifiers_print_in_bubble_teas_order() {
    let all = KeyMod::SUPER | KeyMod::HYPER | KeyMod::META | KeyMod::SHIFT | KeyMod::ALT;
    let k = key(KeyCode::Char('a'), all | KeyMod::CTRL, "");
    assert_eq!(k.keystroke(), "ctrl+alt+shift+meta+hyper+super+a");
    assert_eq!(k.to_string(), "ctrl+alt+shift+meta+hyper+super+a");
    let locks = KeyMod::CAPS_LOCK | KeyMod::NUM_LOCK | KeyMod::SCROLL_LOCK;
    assert_eq!(
        key(KeyCode::Up, locks, "").to_string(),
        "up",
        "locks never print"
    );
}

#[test]
fn text_wins_over_the_keystroke() {
    let question = key(KeyCode::Char('/'), KeyMod::SHIFT, "?");
    assert_eq!(question.to_string(), "?");
    assert_eq!(question.keystroke(), "shift+/");
    let upper = Key::char('A');
    assert_eq!(upper.to_string(), "A");
    assert_eq!(upper.keystroke(), "shift+a");
}

#[test]
fn space_is_written_space() {
    let space = Key::char(' ');
    assert_eq!(space.code, KeyCode::Space);
    assert_eq!(space.text, " ");
    assert_eq!(space.to_string(), "space");
    assert_eq!(space, "space");
    assert!(space.matches("space") && space.matches(" "));
    let ctrl_space = Key::new(KeyCode::Space).with_mods(KeyMod::CTRL);
    assert_eq!(ctrl_space.to_string(), "ctrl+space");
}

#[test]
fn named_keys_print_as_go_names_them() {
    let cases = [
        (KeyCode::Enter, "enter"),
        (KeyCode::Esc, "esc"),
        (KeyCode::Backspace, "backspace"),
        (KeyCode::Tab, "tab"),
        (KeyCode::PgUp, "pgup"),
        (KeyCode::KpEnter, "enter"),
        (KeyCode::KpMultiply, "mul"),
        (KeyCode::KpDecimal, "period"),
        (KeyCode::Kp7, "7"),
        (KeyCode::F(5), "f5"),
        (KeyCode::F(63), "f63"),
        (KeyCode::MediaPlayPause, "mediaplaypause"),
        (KeyCode::IsoLevel3Shift, "isolevel3shift"),
    ];
    for (code, name) in cases {
        assert_eq!(Key::new(code).to_string(), name, "{code:?}");
    }
    let shift_tab = Key::new(KeyCode::Tab).with_mods(KeyMod::SHIFT);
    assert_eq!(shift_tab.to_string(), "shift+tab");
}

#[test]
fn a_modifier_key_does_not_repeat_its_own_modifier() {
    let left_ctrl = Key::new(KeyCode::LeftCtrl).with_mods(KeyMod::CTRL | KeyMod::SHIFT);
    assert_eq!(left_ctrl.keystroke(), "shift+leftctrl");
    let right_super = Key::new(KeyCode::RightSuper).with_mods(KeyMod::SUPER);
    assert_eq!(right_super.keystroke(), "rightsuper");
}

#[test]
fn the_base_key_names_keys_on_other_layouts() {
    let mut cyrillic = key(KeyCode::Char('с'), KeyMod::CTRL, "");
    cyrillic.base_code = Some(KeyCode::Char('c'));
    assert_eq!(cyrillic.to_string(), "ctrl+c");
    let emoji = key(KeyCode::Extended, KeyMod::NONE, "👍🏽");
    assert_eq!(emoji.to_string(), "👍🏽");
    assert_eq!(emoji.keystroke(), "👍🏽");
}

#[test]
fn matching_reads_names_modifiers_and_text() {
    let ctrl_c = Key::ctrl('c');
    assert!(ctrl_c.matches("ctrl+c"));
    assert!(!ctrl_c.matches("c") && !ctrl_c.matches("ctrl+shift+c"));
    let upper = Key::char('A');
    assert!(upper.matches("A") && upper.matches("shift+a"));
    assert!(!upper.matches("a"));
    assert!(Key::new(KeyCode::KpEnter).matches("kpenter"));
    assert!(!Key::new(KeyCode::KpEnter).matches("enter"));
    let esc = Key::new(KeyCode::Esc);
    assert!(esc.matches("escape") && esc.matches("esc"));
    assert!(Key::new(KeyCode::F(12)).matches("f12"));
    assert!(!Key::new(KeyCode::F(1)).matches("f01"));
    assert!(Key::char('é').matches("é"));
    let family = key(KeyCode::Extended, KeyMod::NONE, "👨\u{200d}👩");
    assert!(family.matches("👨\u{200d}👩"));
    assert!(upper.matches_any(&["q", "A"]) && !upper.matches_any(&["q", "b"]));
    assert!(
        Key::new(KeyCode::Up)
            .with_mods(KeyMod::ALT)
            .matches("alt+up")
    );
}

#[test]
fn comparing_with_a_name_compares_the_display() {
    assert_eq!(Key::char('q'), "q");
    assert_eq!(Key::ctrl('c'), "ctrl+c");
    assert_ne!(Key::char('q'), "ctrl+q");
    assert_ne!(Key::char('q'), "qq");
    assert_ne!(Key::char('q'), "");
    assert_eq!(Key::new(KeyCode::Down), "down");
}

#[test]
fn every_named_code_round_trips_through_gos_rune() {
    for r in 0x11_0000..0x11_0000 + 140 {
        assert_eq!(KeyCode::from_rune(r).rune(), u32::try_from(r).unwrap());
    }
    for (code, r) in [
        (KeyCode::Enter, 13),
        (KeyCode::Tab, 9),
        (KeyCode::Esc, 27),
        (KeyCode::Space, 32),
        (KeyCode::Backspace, 127),
    ] {
        assert_eq!(KeyCode::from_rune(i64::from(r)), code);
        assert_eq!(code.rune(), r);
    }
    assert_eq!(KeyCode::from_rune(-1), KeyCode::Char('\u{FFFD}'));
    assert_eq!(KeyCode::from_rune(0xD800), KeyCode::Char('\u{FFFD}'));
}

#[test]
fn modifiers_combine_and_contain() {
    let m = KeyMod::CTRL | KeyMod::ALT;
    assert!(m.contains(KeyMod::CTRL) && m.contains(KeyMod::ALT | KeyMod::CTRL));
    assert!(!m.contains(KeyMod::ALT | KeyMod::CTRL | KeyMod::SHIFT));
    assert!(KeyMod::NONE.is_empty() && !m.is_empty());
    let mut n = KeyMod::NONE;
    n |= KeyMod::META;
    assert_eq!(n, KeyMod::META);
}
