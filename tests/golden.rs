//! Golden tests: the examples' models run in `TestProgram`, their screens
//! (text, cursor, modes, keyboard flags and title) compared with
//! `tests/golden/*.golden`. Rewrite them with
//! `PETITCARNETS_UPDATE_GOLDEN=1 cargo test --test golden`.

use petitcarnets as tea;
use std::time::Duration;
use tea::testing::{TestProgram, TestTerminal, assert_golden, escape};
use tea::{Key, KeyCode};

#[path = "../examples/altscreen-toggle.rs"]
mod altscreen_toggle;
#[path = "../examples/counter.rs"]
mod counter;
#[path = "../examples/cursor-style.rs"]
mod cursor_style;
#[path = "../examples/keyboard-enhancements.rs"]
mod keyboard_enhancements;
#[path = "../examples/lifecycle.rs"]
mod lifecycle;
#[path = "../examples/mouse.rs"]
mod mouse;
#[path = "../examples/package-manager.rs"]
mod package_manager;
#[path = "../examples/picker.rs"]
mod picker;
#[path = "../examples/query-term.rs"]
mod query_term;
#[path = "../examples/set-window-title.rs"]
mod set_window_title;
#[path = "../examples/spinner.rs"]
mod spinner;

/// Each example's `main` has the signature its run needs; naming them
/// here also keeps them from counting as unused in this crate.
#[test]
fn every_included_example_keeps_its_main() {
    let mains: [fn() -> Result<(), tea::Error>; 10] = [
        altscreen_toggle::main,
        counter::main,
        cursor_style::main,
        keyboard_enhancements::main,
        mouse::main,
        package_manager::main,
        picker::main,
        query_term::main,
        set_window_title::main,
        spinner::main,
    ];
    let lifecycle: fn() -> std::process::ExitCode = lifecycle::main;
    assert_eq!(mains.len() + [lifecycle].len(), 11);
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Compares the screen now with the golden file `name`.
fn check<M: tea::Model>(name: &str, test: &TestProgram<M>) {
    assert_golden(name, &test.screen().snapshot());
}

#[test]
fn counter_counts_on_the_alternate_screen() {
    let mut test = TestProgram::new(counter::Counter(0), 40, 6);
    check("counter-start", &test);
    test.press(Key::new(KeyCode::Up));
    test.type_text("++k-");
    check("counter-three", &test);
    test.press(Key::char('q'));
    check("counter-end", &test);
    assert_eq!(test.final_model().0, 3);
}

#[test]
fn counter_writes_the_same_bytes_every_run() {
    let mut test = TestProgram::new(counter::Counter(0), 40, 6);
    test.type_text("+q");
    assert_golden("counter-output", &escape(test.final_output()));
}

/// The picker's list, its cursor on the second item.
fn picker() -> picker::Picker {
    picker::Picker {
        items: vec!["equal-weight", "min-variance", "hrp"],
        cursor: 1,
        chosen: None,
    }
}

#[test]
fn picker_moves_and_chooses() {
    let mut test = TestProgram::new(picker(), 50, 10);
    check("picker-start", &test);
    test.press(Key::new(KeyCode::Down));
    test.press(Key::new(KeyCode::Down));
    check("picker-last", &test);
    test.press(Key::new(KeyCode::Home));
    test.press(Key::new(KeyCode::Enter));
    let model = test.final_model();
    assert_eq!(model.chosen.map(|i| model.items[i]), Some("equal-weight"));
}

#[test]
fn picker_quits_without_choosing_and_leaves_the_screen_as_it_was() {
    let mut test = TestProgram::new(picker(), 50, 10);
    test.press(Key::new(KeyCode::Esc));
    assert_golden("picker-quit", &test.final_screen().snapshot());
    assert_eq!(test.final_model().chosen, None);
}

#[test]
fn spinner_spins_on_virtual_ticks_then_shows_the_result() {
    let start = spinner::Loader {
        frame: 0,
        result: None,
    };
    let mut test = TestProgram::new(start, 40, 6);
    test.advance(ms(240));
    check("spinner-third-tick", &test);
    assert!(test.wait_for(ms(2000), |t| t.screen().text().contains("Done")));
    assert_eq!(
        test.elapsed(),
        ms(2000) + ms(1000) / 60,
        "one frame interval late"
    );
    check("spinner-done", &test);
    assert_eq!(test.model().result, Some(2_432_902_008_176_640_000));
    assert_eq!(test.model().frame, 25 % 8, "25 ticks in two seconds");
}

#[test]
fn spinner_writes_the_same_bytes_every_run() {
    let start = spinner::Loader {
        frame: 0,
        result: None,
    };
    let mut test = TestProgram::new(start, 40, 6);
    test.advance(ms(2100));
    test.press(Key::char('q'));
    assert_golden("spinner-output", &escape(test.final_output()));
}

#[test]
fn package_manager_prints_above_its_progress_line() {
    let installer = package_manager::Installer {
        index: 0,
        frame: 0,
        width: 80,
        delay: ms(100),
        done: false,
    };
    let mut test = TestProgram::new(installer, 70, 16);
    test.advance(ms(500));
    check("package-manager-progress", &test);
    assert!(test.wait_finished(Duration::from_secs(5)));
    assert_golden("package-manager-end", &test.final_screen().snapshot());
    assert!(test.final_model().done);
}

#[test]
fn package_manager_writes_the_same_bytes_every_run() {
    let installer = package_manager::Installer {
        index: 9,
        frame: 0,
        width: 80,
        delay: ms(100),
        done: false,
    };
    let mut test = TestProgram::new(installer, 70, 6);
    assert!(test.wait_finished(Duration::from_secs(2)));
    assert_golden("package-manager-output", &escape(test.final_output()));
}

#[test]
fn altscreen_toggle_switches_screens() {
    let mut test = TestProgram::new(altscreen_toggle::Toggle::default(), 40, 8);
    check("altscreen-toggle-inline", &test);
    test.press(Key::new(KeyCode::Space));
    check("altscreen-toggle-alt", &test);
    test.press(Key::new(KeyCode::Space));
    test.press(Key::char('q'));
    check("altscreen-toggle-bye", &test);
}

#[test]
fn mouse_shows_clicks_motion_wheel_and_focus() {
    let mut test = TestProgram::new(mouse::Mice::default(), 50, 10);
    test.bytes(b"\x1b[<0;10;5M\x1b[<0;10;5m");
    test.bytes(b"\x1b[<35;12;6M\x1b[<64;3;3M");
    test.bytes(b"\x1b[I\x1b[O");
    check("mouse-events", &test);
    test.press(Key::char('q'));
    assert_golden("mouse-end", &test.final_screen().snapshot());
    assert_eq!(
        test.final_model().last_click.as_deref(),
        Some("(X: 9, Y: 4) left")
    );
}

#[test]
fn keyboard_enhancements_show_presses_and_releases() {
    let terminal = TestTerminal::new(80, 8);
    let program = tea::Program::new(keyboard_enhancements::Keys::default());
    let mut test = TestProgram::with_terminal(program, terminal);
    test.bytes(b"\x1b[?3u");
    test.bytes(b"\x1b[97u\x1b[97;1:3u\x1b[97;2u\x1b[97;1:2u");
    check("keyboard-enhancements", &test);
    test.press(Key::new(KeyCode::Esc));
    assert_eq!(test.final_model().events.len(), 4);
}

#[test]
fn query_term_prints_each_reply() {
    let terminal = TestTerminal::new(70, 12)
        .reply("\x1b]11;?\x07", "\x1b]11;rgb:1111/2222/3333\x1b\\")
        .reply("\x1b]10;?\x07", "\x1b]10;#eeeeee\x07")
        .reply("\x1b[6n", "\x1b[5;7R")
        .reply("\x1bP+q524742\x1b\\", "\x1bP1+r524742\x1b\\")
        .reply("\x1b[c", "\x1b[?62;22c");
    let program = tea::Program::new(query_term::Query);
    let mut test = TestProgram::with_terminal(program, terminal);
    test.type_text("bfprd");
    check("query-term", &test);
    test.press(Key::char('q'));
    assert!(test.error().is_none());
}

#[test]
fn cursor_style_moves_through_the_shapes() {
    let shapes = cursor_style::Shapes {
        shape: tea::CursorShape::Block,
        blink: true,
    };
    let mut test = TestProgram::new(shapes, 70, 4);
    check("cursor-style-start", &test);
    test.press(Key::new(KeyCode::Right));
    check("cursor-style-underline", &test);
    test.press(Key::new(KeyCode::Right));
    test.press(Key::new(KeyCode::Left));
    test.press(Key::char('h'));
    check("cursor-style-bar", &test);
}

#[test]
fn set_window_title_sets_then_gives_the_title_back() {
    let mut test = TestProgram::new(set_window_title::Titled, 80, 4);
    check("set-window-title", &test);
    test.press(Key::char('x'));
    assert_golden("set-window-title-end", &test.final_screen().snapshot());
}

#[test]
fn lifecycle_keeps_pastes_and_never_shows_a_process_id() {
    let mut test = TestProgram::new(lifecycle::Lifecycle::default(), 60, 8);
    test.bytes(b"\x1b[200~hello\x1b[201~");
    let pid = format!("pid {}", std::process::id());
    let snapshot = test.screen().snapshot().replace(&pid, "pid <pid>");
    assert_golden("lifecycle", &snapshot);
    test.press(Key::char('q'));
    assert_eq!(test.final_model().pastes, ["hello"]);
}
