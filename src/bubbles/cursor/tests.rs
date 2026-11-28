use super::*;

fn blink_of(c: &Model) -> TeaMsg<Msg> {
    TeaMsg::App(Msg::Blink(BlinkMsg {
        id: c.id(),
        tag: c.blink_tag,
    }))
}

#[test]
fn a_focused_cursor_blinks_on_its_own_messages_only() {
    let mut c = Model::new();
    c.set_char("x");
    assert!(
        c.update(&TeaMsg::App(Msg::InitialBlink)).is_none(),
        "not focused"
    );
    assert!(!c.focus().is_none());
    assert!(c.focused() && !c.is_blinked);
    let mine = blink_of(&c);
    assert!(!c.update(&mine).is_none());
    assert!(c.is_blinked, "blinked off");
    assert_eq!(c.view(), "x");
    assert!(c.update(&mine).is_none(), "an old blink is ignored");
    let other = Model::new();
    let theirs = TeaMsg::App(Msg::Blink(BlinkMsg {
        id: other.id(),
        tag: c.blink_tag,
    }));
    assert!(
        c.update(&theirs).is_none(),
        "another cursor's blink is ignored"
    );
    assert!(c.is_blinked);
    assert!(!c.update(&TeaMsg::App(Msg::InitialBlink)).is_none());
    if let TeaMsg::App(Msg::Blink(b)) = blink_of(&c) {
        assert_eq!(b.id(), c.id());
    }
}

#[test]
fn modes_and_focus_events() {
    let mut c = Model::default();
    assert!(c.set_mode(Mode::Static).is_none());
    assert!(c.blink().is_none(), "a static cursor does not blink");
    assert!(c.focus().is_none());
    assert!(!c.is_blinked, "shown");
    assert!(c.update(&TeaMsg::Blur).is_none());
    assert!(c.is_blinked && !c.focused());
    let _ = c.update(&TeaMsg::Focus);
    assert!(c.focused());
    assert!(c.set_mode(Mode::Hide).is_none());
    assert!(c.is_blinked);
    assert!(!c.set_mode(Mode::Blink).is_none());
    assert_eq!(
        [Mode::Blink, Mode::Static, Mode::Hide].map(|m| m.to_string()),
        ["blink", "static", "hidden"]
    );
    assert_eq!(Mode::default(), Mode::Blink);
    assert!(c.update(&TeaMsg::KeyPress(crate::Key::char('a'))).is_none());
}
