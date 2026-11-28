//! Golden tests for the ports of Bubble Tea's examples that use
//! components, in `TestProgram` on a virtual clock. Rewrite the golden
//! files with `PETITCARNETS_UPDATE_GOLDEN=1 cargo test --test bubbles_examples`.

use petitcarnets as tea;
use std::sync::mpsc;
use std::time::Duration;
use tea::testing::{TestProgram, assert_golden};
use tea::{Key, KeyCode};

#[path = "../examples/composable-views.rs"]
mod composable_views;
#[path = "../examples/prevent-quit.rs"]
mod prevent_quit;
#[path = "../examples/realtime.rs"]
mod realtime;
#[path = "../examples/send-msg.rs"]
mod send_msg;
#[path = "../examples/set-terminal-color.rs"]
mod set_terminal_color;
#[path = "../examples/split-editors.rs"]
mod split_editors;

/// Naming each example's `main` keeps it from counting as unused here.
#[test]
fn every_included_example_keeps_its_main() {
    let mains: [fn() -> Result<(), tea::Error>; 6] = [
        composable_views::main,
        prevent_quit::main,
        realtime::main,
        send_msg::main,
        set_terminal_color::main,
        split_editors::main,
    ];
    assert_eq!(mains.len(), 6);
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn check<M: tea::Model>(name: &str, test: &TestProgram<M>) {
    assert_golden(name, &test.screen().snapshot());
}

#[test]
fn send_msg_shows_the_last_five_meals_sent_from_outside() {
    let mut test = TestProgram::new(send_msg::new_model(), 60, 12);
    check("send-msg-start", &test);
    let meals = [("an apple", 120), ("tacos", 999), ("some ramen", 450)];
    for (food, pause) in meals {
        test.send(send_msg::Msg::Result(send_msg::ResultMsg {
            duration: ms(pause),
            food,
        }));
    }
    test.advance(ms(300));
    check("send-msg-eating", &test);
    test.type_text("x");
    assert_golden("send-msg-end", &test.final_screen().snapshot());
}

#[test]
fn realtime_counts_events_from_a_channel() {
    let (tx, rx) = mpsc::channel();
    for _ in 0..3 {
        tx.send(()).unwrap();
    }
    drop(tx);
    let mut test = TestProgram::new(realtime::Model::new(rx), 40, 6);
    test.advance(ms(250));
    check("realtime-three", &test);
    test.type_text("x");
    assert_eq!(test.final_model().responses, 3);
}

#[test]
fn prevent_quit_quits_at_once_without_changes() {
    let mut test = TestProgram::new(prevent_quit::initial_model(), 70, 14);
    check("prevent-quit-start", &test);
    test.press(Key::new(KeyCode::Esc));
    assert_golden("prevent-quit-bye", &test.final_screen().snapshot());
}

#[test]
fn prevent_quit_asks_before_dropping_changes() {
    let mut test = TestProgram::new(prevent_quit::initial_model(), 70, 14);
    test.type_text("Hi");
    test.press(Key::new(KeyCode::Esc));
    check("prevent-quit-ask", &test);
    assert!(!test.is_finished(), "unsaved changes stop the quit");
    test.type_text("n");
    assert!(test.screen().text().contains("Type some important things."));
    test.press(Key::ctrl('s'));
    check("prevent-quit-saved", &test);
    test.press(Key::new(KeyCode::Esc));
    assert!(test.is_finished(), "saved: nothing to ask");
}

#[test]
fn prevent_quit_quits_on_yes() {
    let mut test = TestProgram::new(prevent_quit::initial_model(), 70, 14);
    test.type_text("Hi");
    test.press(Key::new(KeyCode::Esc));
    test.type_text("y");
    let model = test.final_model();
    assert_eq!(model.textarea.value(), "Hi");
    assert!(!model.has_changes);
}

#[test]
fn composable_views_moves_the_focus_and_ticks_both() {
    let model = composable_views::new_model(Duration::from_secs(60));
    let mut test = TestProgram::new(model, 50, 8);
    check("composable-views-start", &test);
    test.advance(Duration::from_secs(2));
    test.press(Key::new(KeyCode::Tab));
    check("composable-views-spinner", &test);
    test.type_text("nnnnnn");
    test.advance(ms(100));
    check("composable-views-globe", &test);
    test.type_text("q");
    assert_eq!(test.final_model().index, 6);
}

#[test]
fn split_editors_adds_moves_between_and_removes_editors() {
    let mut test = TestProgram::new(split_editors::new_model(), 60, 12);
    check("split-editors-start", &test);
    test.type_text("left");
    test.press(Key::new(KeyCode::Tab));
    test.type_text("right");
    test.press(Key::ctrl('n'));
    check("split-editors-three", &test);
    test.press(Key::new(KeyCode::Tab).with_mods(tea::KeyMod::SHIFT));
    test.type_text("!");
    test.press(Key::new(KeyCode::Tab));
    test.press(Key::new(KeyCode::Tab));
    test.press(Key::ctrl('w'));
    check("split-editors-removed", &test);
    test.press(Key::ctrl('w'));
    check("split-editors-one", &test);
    assert!(
        !test.model().keymap.remove.enabled(),
        "one editor is the least"
    );
    test.press(Key::new(KeyCode::Esc));
    let model = test.final_model();
    assert_eq!(model.inputs[0].value(), "left!");
}

#[test]
fn set_terminal_color_sets_the_colours_the_view_asks_for() {
    let mut test = TestProgram::new(set_terminal_color::initial_model(), 60, 10);
    check("set-terminal-color-start", &test);
    test.press(Key::new(KeyCode::Down));
    test.press(Key::new(KeyCode::Enter));
    test.type_text("#1e1e2e");
    test.press(Key::new(KeyCode::Enter));
    test.press(Key::new(KeyCode::Enter));
    test.type_text("#f0f");
    test.press(Key::new(KeyCode::Enter));
    check("set-terminal-color-set", &test);
    let osc = test.screen().osc_log().join(" ");
    assert!(
        osc.contains("11;#1e1e2e") && osc.contains("10;#ff00ff"),
        "{osc}"
    );
    test.press(Key::new(KeyCode::Enter));
    test.type_text("#zz0");
    test.press(Key::new(KeyCode::Enter));
    check("set-terminal-color-error", &test);
    test.type_text("q");
    let model = test.final_model();
    assert_eq!(
        model.bg,
        Some(tea::Rgb {
            r: 0x1e,
            g: 0x1e,
            b: 0x2e
        })
    );
    assert_eq!(
        model.fg,
        Some(tea::Rgb {
            r: 0xff,
            g: 0,
            b: 0xff
        })
    );
}
