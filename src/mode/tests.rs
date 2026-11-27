//! Mode sequences, and how a change of view turns modes on and off.

use super::*;
use crate::backend::TestBackend;

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn each_mode_has_bubble_teas_bytes() {
    let cases = [
        (TerminalMode::BracketedPaste, "\x1b[?2004h", "\x1b[?2004l"),
        (TerminalMode::ReportFocus, "\x1b[?1004h", "\x1b[?1004l"),
        (
            TerminalMode::MouseCellMotion,
            "\x1b[?1002h\x1b[?1006h",
            "\x1b[?1002l\x1b[?1003l\x1b[?1006l",
        ),
        (
            TerminalMode::MouseAllMotion,
            "\x1b[?1003h\x1b[?1006h",
            "\x1b[?1002l\x1b[?1003l\x1b[?1006l",
        ),
        (
            TerminalMode::KeyboardEnhancements(3),
            "\x1b[>4;2m\x1b[>3u",
            "\x1b[>4m\x1b[<u",
        ),
        (TerminalMode::KeyboardFlags(31), "\x1b[>4;2m\x1b[=31;1u", ""),
    ];
    for (mode, on, off) in cases {
        assert_eq!(text(&mode.sequence(true)), on, "{mode:?} on");
        assert_eq!(text(&mode.sequence(false)), off, "{mode:?} off");
    }
}

#[test]
fn keyboard_flags_always_disambiguate() {
    assert_eq!(KeyboardEnhancements::default().flags(), 1);
    let all = KeyboardEnhancements::new()
        .report_event_types()
        .report_alternate_keys()
        .report_all_keys_as_escape_codes()
        .report_associated_text();
    assert_eq!(all.flags(), 31);
    assert_eq!(
        KeyboardEnhancements::new().report_alternate_keys().flags(),
        5
    );
    let report = KeyboardEnhancementsReport { flags: 3 };
    assert!(report.supports_key_disambiguation() && report.supports_event_types());
    assert!(!report.supports_alternate_keys() && !report.supports_associated_text());
    assert!(!KeyboardEnhancementsReport { flags: -1 }.supports_key_disambiguation());
}

/// The bytes `views`, applied in turn and then cleared, write.
fn applied(views: &[View]) -> Vec<String> {
    let mut backend = TestBackend::new(10, 3);
    let screen = backend.screen();
    let mut modes = Modes::default();
    let mut out = Vec::new();
    let mut seen = 0;
    let mut step = |modes: &mut Modes, backend: &mut TestBackend, view: Option<&View>| {
        match view {
            Some(view) => modes.apply(view, backend).unwrap(),
            None => modes.clear(backend).unwrap(),
        }
        let raw = screen.raw();
        out.push(raw[seen..].to_string());
        seen = raw.len();
    };
    for view in views {
        step(&mut modes, &mut backend, Some(view));
    }
    step(&mut modes, &mut backend, None);
    out
}

#[test]
fn a_view_change_turns_on_and_off_only_what_changed() {
    let plain = View::default();
    let cell = View::default().with_mouse_mode(MouseMode::CellMotion);
    let all = cell
        .clone()
        .with_mouse_mode(MouseMode::AllMotion)
        .with_report_focus(true);
    let steps = applied(&[plain.clone(), plain, cell, all.clone(), all]);
    assert_eq!(steps[0], "\x1b[?2004h\x1b[>4;2m\x1b[>1u");
    assert_eq!(steps[1], "", "same view, nothing written");
    assert_eq!(steps[2], "\x1b[?1002h\x1b[?1006h");
    assert_eq!(
        steps[3],
        "\x1b[?1004h\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1003h\x1b[?1006h"
    );
    assert_eq!(steps[4], "");
    assert_eq!(
        steps[5], "\x1b[>4m\x1b[<u\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1004l\x1b[?2004l",
        "the end turns everything off, newest first"
    );
}

#[test]
fn clearing_before_any_view_writes_nothing() {
    assert_eq!(applied(&[]), [""]);
}

#[test]
fn mode_settings_round_trip() {
    for v in 0..=u8::MAX {
        assert_eq!(u8::from(ModeSetting::from(v)), v);
    }
    assert_eq!(ModeSetting::from(2), ModeSetting::Reset);
}
