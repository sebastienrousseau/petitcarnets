use super::*;
use std::time::UNIX_EPOCH;

#[test]
fn every_bubbles_spinner_with_its_speed() {
    let all = [
        (Spinner::LINE, 4, 100),
        (Spinner::DOT, 8, 100),
        (Spinner::MINI_DOT, 10, 83),
        (Spinner::JUMP, 7, 100),
        (Spinner::PULSE, 4, 125),
        (Spinner::POINTS, 4, 142),
        (Spinner::GLOBE, 3, 250),
        (Spinner::MOON, 8, 125),
        (Spinner::MONKEY, 3, 333),
        (Spinner::METER, 7, 142),
        (Spinner::HAMBURGER, 4, 333),
        (Spinner::ELLIPSIS, 4, 333),
    ];
    for (s, frames, ms) in all {
        assert_eq!(s.frames.len(), frames);
        assert_eq!(s.fps.as_millis(), ms);
    }
}

#[test]
fn a_spinner_takes_only_its_own_latest_tick() {
    let mut a = Model::new().with_style(Style::new());
    let b = Model::default().with_spinner(Spinner::PULSE);
    assert_ne!(a.id(), b.id());
    assert_eq!(a.view(), "|");
    assert!(a.update(&TeaMsg::App(b.tick_msg(UNIX_EPOCH))).is_none());
    assert_eq!(a.view(), "|");
    let first = a.tick_msg(UNIX_EPOCH);
    assert!(!a.update(&TeaMsg::App(first)).is_none());
    assert_eq!(a.view(), "/");
    let second = a.tick_msg(UNIX_EPOCH);
    let _ = a.update(&TeaMsg::App(second));
    assert!(a.update(&TeaMsg::App(second)).is_none(), "an old tick");
    assert_eq!(a.view(), "-");
    for _ in 0..2 {
        let _ = a.update(&TeaMsg::App(a.tick_msg(UNIX_EPOCH)));
    }
    assert_eq!(a.view(), "|", "round again");
    assert!(
        !a.update(&TeaMsg::App(TickMsg::default())).is_none(),
        "id 0 is everyone's"
    );
    assert!(a.update(&TeaMsg::Focus).is_none());
    a.spinner = Spinner {
        frames: &[],
        ..Spinner::LINE
    };
    assert_eq!(a.view(), "(error)");
    assert!(!a.tick().is_none());
}

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

use crate::bubbles::tests::rows;

#[test]
fn every_spinner_matches_bubbles() {
    let ours = [
        ("LINE", Spinner::LINE),
        ("DOT", Spinner::DOT),
        ("MINI_DOT", Spinner::MINI_DOT),
        ("JUMP", Spinner::JUMP),
        ("PULSE", Spinner::PULSE),
        ("POINTS", Spinner::POINTS),
        ("GLOBE", Spinner::GLOBE),
        ("MOON", Spinner::MOON),
        ("MONKEY", Spinner::MONKEY),
        ("METER", Spinner::METER),
        ("HAMBURGER", Spinner::HAMBURGER),
        ("ELLIPSIS", Spinner::ELLIPSIS),
    ];
    let want = rows("spinner");
    assert_eq!(want.len(), ours.len());
    for row in want {
        let (_, s) = ours.iter().find(|(n, _)| *n == row[0]).expect("spinner");
        assert_eq!(s.fps.as_nanos().to_string(), row[1], "{}", row[0]);
        assert_eq!(s.frames.join("\u{1f}"), row[2], "{}", row[0]);
    }
}
