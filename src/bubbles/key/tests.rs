use super::*;
use crate::{KeyCode, KeyMod};

#[test]
fn a_binding_matches_its_keys_while_on() {
    let mut quit = Binding::new(&["q", "ctrl+c"]).with_help("q", "quit");
    assert!(quit.matches(&Key::char('q')));
    assert!(quit.matches(&Key::ctrl('c')));
    assert!(!quit.matches(&Key::char('x')));
    quit.set_enabled(false);
    assert!(!quit.enabled() && !quit.matches(&Key::char('q')));
    quit.set_enabled(true);
    assert!(quit.enabled());
    quit.set_keys(&["esc"]);
    assert_eq!(quit.keys(), ["esc"]);
    assert!(matches(
        &Key::new(KeyCode::Esc),
        &[&Binding::default(), &quit]
    ));
    quit.unbind();
    assert!(!quit.enabled());
    assert_eq!(quit.help(), &Help::default());
}

#[test]
fn keys_match_as_go_prints_them() {
    let b = Binding::new(&["shift+tab", "?", "space", "alt+<"]);
    assert!(b.matches(&Key::new(KeyCode::Tab).with_mods(KeyMod::SHIFT)));
    let mut question = Key::char('/').with_mods(KeyMod::SHIFT);
    question.text = "?".into();
    assert!(b.matches(&question), "the typed text counts");
    assert!(b.matches(&Key::char(' ')));
    let mut less = Key::char('<').with_mods(KeyMod::ALT);
    less.text = String::new();
    assert!(b.matches(&less));
    assert!(
        !Binding::new(&["a"])
            .with_disabled()
            .matches(&Key::char('a'))
    );
}
