//! The components in the runtime: the examples that use them, run in
//! `TestProgram` on its virtual clock, their screens compared with
//! `tests/golden/*.golden` (rewrite them with
//! `PETITCARNETS_UPDATE_GOLDEN=1 cargo test --test bubbles`), and
//! `Cmd::map` and `Msg::route` composing children.

use petitcarnets as tea;
use std::time::Duration;
use tea::testing::{TestProgram, assert_golden};
use tea::{Key, KeyCode, KeyMod};

#[path = "../examples/chat.rs"]
mod chat;
#[path = "../examples/help.rs"]
mod help;
#[path = "../examples/list-default.rs"]
mod list_default;
#[path = "../examples/list-simple.rs"]
mod list_simple;
#[path = "../examples/pager.rs"]
mod pager;
#[path = "../examples/paginator.rs"]
mod paginator;
#[path = "../examples/progress-animated.rs"]
mod progress_animated;
#[path = "../examples/progress-static.rs"]
mod progress_static;
#[path = "../examples/spinners.rs"]
mod spinners;
#[path = "../examples/stopwatch.rs"]
mod stopwatch;
#[path = "../examples/table.rs"]
mod table;
#[path = "../examples/textarea.rs"]
mod textarea;
#[path = "../examples/textinput.rs"]
mod textinput;
#[path = "../examples/textinputs.rs"]
mod textinputs;
#[path = "../examples/timer.rs"]
mod timer;

#[path = "bubbles/compose.rs"]
mod compose;
#[path = "bubbles/stress.rs"]
mod stress;

/// Each example's `main`; naming them keeps them from counting as
/// unused here.
#[test]
fn every_included_example_keeps_its_main() {
    let mains: [fn() -> Result<(), tea::Error>; 15] = [
        chat::main,
        help::main,
        list_default::main,
        list_simple::main,
        pager::main,
        paginator::main,
        progress_animated::main,
        progress_static::main,
        spinners::main,
        stopwatch::main,
        table::main,
        textarea::main,
        textinput::main,
        textinputs::main,
        timer::main,
    ];
    assert_eq!(mains.len(), 15);
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Compares the screen now with the golden file `name`.
fn check<M: tea::Model>(name: &str, test: &TestProgram<M>) {
    assert_golden(name, &test.screen().snapshot());
}

fn key(code: KeyCode) -> Key {
    Key::new(code)
}

#[test]
fn textinput_shows_the_placeholder_then_scrolls_what_is_typed() {
    let mut test = TestProgram::new(textinput::initial_model(), 40, 6);
    check("textinput-start", &test);
    test.type_text("Charmander and Bulbasaur");
    test.press(key(KeyCode::Left));
    check("textinput-typed", &test);
    test.press(key(KeyCode::Home));
    test.press(Key::ctrl('k'));
    test.type_text("字字 e\u{301}");
    check("textinput-wide", &test);
    test.press(key(KeyCode::Enter));
    assert_eq!(test.final_model().text_input.value(), "字字 e\u{301}");
}

#[test]
fn textinputs_move_focus_hide_the_password_and_change_cursor_modes() {
    let mut test = TestProgram::new(textinputs::initial_model(), 50, 10);
    check("textinputs-start", &test);
    test.type_text("seb");
    test.press(key(KeyCode::Tab));
    test.type_text("seb@example.com");
    test.press(key(KeyCode::Tab));
    test.type_text("secret");
    check("textinputs-filled", &test);
    test.press(Key::ctrl('r'));
    test.press(key(KeyCode::Down));
    check("textinputs-submit", &test);
    test.press(key(KeyCode::Up).with_mods(KeyMod::NONE));
    test.press(key(KeyCode::Tab).with_mods(KeyMod::SHIFT));
    test.press(key(KeyCode::Esc));
    let m = test.final_model();
    assert_eq!(m.inputs[2].value(), "secret");
    assert_eq!(m.focus_index, 1);
}

#[test]
fn textarea_wraps_numbers_and_scrolls_its_lines() {
    let mut test = TestProgram::new(textarea::initial_model(), 50, 14);
    check("textarea-start", &test);
    test.type_text("Once there was a notebook that wanted to see the sea.");
    test.press(key(KeyCode::Enter));
    test.type_text("It packed a pencil.");
    check("textarea-typed", &test);
    for _ in 0..6 {
        test.press(key(KeyCode::Enter));
    }
    test.type_text("The end");
    check("textarea-scrolled", &test);
    test.press(key(KeyCode::Esc));
    test.press(Key::ctrl('c'));
    assert!(test.final_model().textarea.value().ends_with("The end"));
}

#[test]
fn spinners_tick_on_their_own_and_change_with_the_arrows() {
    let mut test = TestProgram::new(spinners::initial_model(), 40, 6);
    check("spinners-start", &test);
    test.advance(ms(250));
    check("spinners-ticked", &test);
    for _ in 0..6 {
        test.press(Key::char('l'));
    }
    test.advance(ms(250));
    check("spinners-globe", &test);
    test.press(Key::char('h'));
    test.press(Key::char('q'));
    assert_eq!(test.final_model().index, 5);
}

#[test]
fn list_simple_pages_and_chooses() {
    let mut test = TestProgram::new(list_simple::initial_model(), 40, 20);
    check("list-simple-start", &test);
    for _ in 0..5 {
        test.press(Key::char('j'));
    }
    check("list-simple-second-page", &test);
    test.press(key(KeyCode::Enter));
    check("list-simple-chosen", &test);
    assert_eq!(test.final_model().choice, Some("Okonomiyaki"));
}

#[test]
fn list_default_filters_as_you_type() {
    let mut test = TestProgram::new(list_default::initial_model(), 60, 24);
    check("list-default-start", &test);
    test.press(Key::char('/'));
    test.type_text("tea");
    check("list-default-filtering", &test);
    test.press(key(KeyCode::Enter));
    test.press(Key::char('j'));
    check("list-default-filtered", &test);
    test.press(Key::char('?'));
    check("list-default-help", &test);
    test.press(Key::ctrl('c'));
    assert!(test.final_model().list.is_filtered());
}

#[test]
fn pager_scrolls_with_keys_and_the_wheel() {
    let mut test = TestProgram::new(pager::initial_model(), 60, 14);
    check("pager-start", &test);
    test.press(Key::char('j'));
    test.press(key(KeyCode::PgDown));
    check("pager-paged", &test);
    test.bytes(b"\x1b[<65;5;5M");
    check("pager-wheel", &test);
    test.press(Key::char('q'));
    assert_eq!(test.final_model().viewport.y_offset(), 12);
}

#[test]
fn progress_static_fills_a_quarter_a_second() {
    let mut test = TestProgram::new(progress_static::initial_model(), 60, 5);
    check("progress-static-start", &test);
    test.advance(Duration::from_secs(2));
    check("progress-static-half", &test);
    assert!(test.wait_finished(Duration::from_secs(5)));
    assert!((test.final_model().percent - 1.0).abs() < f64::EPSILON);
}

#[test]
fn progress_animated_springs_to_each_quarter() {
    let mut test = TestProgram::new(progress_animated::initial_model(), 60, 5);
    test.advance(Duration::from_millis(1050));
    check("progress-animated-moving", &test);
    test.advance(Duration::from_secs(1));
    check("progress-animated-half", &test);
    test.press(Key::char('x'));
    assert!((test.final_model().progress.percent() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn table_moves_its_selection_and_blurs() {
    let mut test = TestProgram::new(table::initial_model(), 50, 14);
    check("table-start", &test);
    for _ in 0..7 {
        test.press(Key::char('j'));
    }
    check("table-moved", &test);
    test.press(key(KeyCode::Esc));
    test.press(Key::char('j'));
    test.press(key(KeyCode::Esc));
    test.press(Key::char('G'));
    check("table-end", &test);
    test.press(key(KeyCode::Enter));
    test.press(Key::char('q'));
    assert_eq!(test.final_model().table.cursor(), 99);
}

#[test]
fn help_switches_between_short_and_full() {
    let mut test = TestProgram::new(help::new_model(), 60, 10);
    check("help-short", &test);
    test.press(Key::char('?'));
    test.press(key(KeyCode::Left));
    check("help-full", &test);
    test.resize(20, 10);
    check("help-narrow", &test);
    test.press(Key::char('q'));
    assert_eq!(test.final_model().last_key, "←");
}

#[test]
fn timer_counts_down_stops_and_runs_out() {
    let mut test = TestProgram::new(timer::initial_model(), 40, 5);
    check("timer-start", &test);
    test.advance(ms(1500));
    test.press(Key::char('s'));
    check("timer-stopped", &test);
    test.advance(Duration::from_secs(1));
    test.press(Key::char('s'));
    assert!(test.wait_finished(Duration::from_secs(5)));
    assert!(test.final_model().timer.timedout());
}

#[test]
fn stopwatch_counts_up_stops_and_resets() {
    let mut test = TestProgram::new(stopwatch::initial_model(), 40, 5);
    test.advance(ms(1234));
    check("stopwatch-running", &test);
    test.press(Key::char('s'));
    test.advance(Duration::from_secs(1));
    check("stopwatch-stopped", &test);
    test.press(Key::char('r'));
    test.press(Key::char('q'));
    assert_eq!(test.final_model().stopwatch.view(), "0s");
}

#[test]
fn paginator_turns_pages() {
    let mut test = TestProgram::new(paginator::new_model(), 30, 30);
    check("paginator-start", &test);
    test.press(Key::char('l'));
    test.press(key(KeyCode::Right));
    check("paginator-third", &test);
    test.press(Key::char('q'));
    assert_eq!(test.final_model().paginator.page, 2);
}

#[test]
fn chat_sends_messages_above_the_text_area() {
    let mut test = TestProgram::new(chat::initial_model(), 40, 12);
    check("chat-start", &test);
    test.type_text("Hello there, how is the weather where you are today?");
    test.press(key(KeyCode::Enter));
    test.type_text("Hi");
    check("chat-sent", &test);
    test.press(key(KeyCode::Esc));
    assert_eq!(test.final_model().messages.len(), 1);
}
