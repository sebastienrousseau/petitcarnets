//! Accessible output (Bubble Tea's issue #780): frames printed as plain
//! lines for screen readers, with no redraws and no cursor movement.
//! The bytes written are compared with `tests/golden/accessible-*.golden`.

use petitcarnets as tea;
use std::time::Duration;
use tea::testing::{TestProgram, TestTerminal, assert_golden, escape};
use tea::{Environment, Key, KeyCode};

#[path = "../examples/counter.rs"]
mod counter;

/// Names the example's `main`, so it is not unused here.
#[test]
fn the_included_example_keeps_its_main() {
    let main: fn() -> Result<(), tea::Error> = counter::main;
    let _ = main;
}

/// The bytes a program writes for its frames that would move the
/// cursor, redraw, or switch screens.
const REDRAWS: [&str; 9] = [
    "\x1b[?1049", // the alternate screen
    "\x1b[?2026", // synchronized output
    "\x1b[?7l",   // autowrap off
    "\x1b[?25l",  // a hidden cursor
    "\x1b[H",     // home
    "\x1b[2J",    // clear
    "\x1b[K",     // erase in line
    "\x1b[J",     // erase below
    "\x1b[1A",    // up
];

/// Fails if `out` holds any sequence that redraws.
fn assert_no_redraws(out: &[u8]) {
    let text = String::from_utf8_lossy(out);
    for seq in REDRAWS {
        assert!(!text.contains(seq), "{seq:?} in {text:?}");
    }
    let moves = text.match_indices("\x1b[").filter(|(i, _)| {
        let rest = &text[i + 2..];
        let end = rest.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(0);
        matches!(
            rest.as_bytes().get(end),
            Some(b'A' | b'B' | b'C' | b'D' | b'G' | b'H')
        ) && rest[..end].bytes().all(|b| b.is_ascii_digit() || b == b';')
    });
    assert_eq!(moves.count(), 0, "cursor moves in {text:?}");
}

fn accessible(program: tea::Program<counter::Counter>) -> TestProgram<counter::Counter> {
    TestProgram::with_terminal(program, TestTerminal::new(40, 6))
}

#[test]
fn the_counter_prints_each_changed_line_once() {
    let program = tea::Program::new(counter::Counter(0)).with_accessible(true);
    let mut test = accessible(program);
    test.press(Key::new(KeyCode::Up));
    test.type_text("+-");
    test.press(Key::new(KeyCode::Down));
    test.type_text("q");
    assert!(test.wait_finished(Duration::from_secs(1)));
    assert_golden("accessible-counter-output", &escape(test.final_output()));
    assert_no_redraws(test.final_output());
    assert_eq!(test.final_model().0, 0);
}

#[test]
fn the_screen_holds_every_frame_one_after_another() {
    let program = tea::Program::new(counter::Counter(0)).with_accessible(true);
    let mut test = accessible(program);
    test.type_text("++");
    assert_golden("accessible-counter-screen", &test.screen().snapshot());
    assert!(
        !test.screen().alt_screen(),
        "the view's alternate screen is not used"
    );
}

/// A model that prints a line, then keeps a frame with styles and a
/// cursor.
struct Printer;

impl tea::Model for Printer {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tea::println("printed\n\nabove")
    }

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k == "q" => tea::quit(),
            tea::Msg::ColorProfile(profile) => tea::printf!("profile {profile}"),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let red = tea::style::Style::new()
            .fg(tea::style::Color::RED)
            .bold(true)
            .paint("red");
        tea::View::new(format!("a {red} word   \nnext"))
            .with_cursor(tea::Cursor::new(1, 1))
            .with_alt_screen(true)
    }
}

#[test]
fn printed_lines_and_frames_are_plain_text_in_program_order() {
    let terminal = TestTerminal::new(30, 8).terminal();
    let program = tea::Program::new(Printer)
        .with_accessible(true)
        .with_environment(Environment::default());
    let mut test = TestProgram::with_terminal(program, terminal);
    test.type_text("q");
    assert!(test.wait_finished(Duration::from_secs(1)));
    assert_golden("accessible-printer-output", &escape(test.final_output()));
    assert_no_redraws(test.final_output());
}

#[test]
fn a_profile_the_program_sets_keeps_its_colours() {
    let program = tea::Program::new(Printer)
        .with_accessible(true)
        .with_color_profile(tea::ColorProfile::Ansi);
    let mut test = TestProgram::with_terminal(program, TestTerminal::new(30, 8));
    test.type_text("q");
    let out = String::from_utf8_lossy(test.final_output()).into_owned();
    assert!(out.contains("profile ANSI\r\n"), "{out:?}");
    assert!(out.contains("a \x1b[1;31mred\x1b[m word\r\n"), "{out:?}");
}

/// Whether a program in `terminal` with `environment` printed lines.
fn prints_lines(terminal: TestTerminal, environment: &[(&str, &str)], set: Option<bool>) -> bool {
    let mut program = tea::Program::new(counter::Counter(0))
        .with_environment(environment.iter().copied().collect());
    if let Some(on) = set {
        program = program.with_accessible(on);
    }
    let mut test = TestProgram::with_terminal(program, terminal);
    test.type_text("q");
    !String::from_utf8_lossy(test.final_output()).contains("\x1b[?1049h")
}

#[test]
fn a_real_terminal_with_accessible_set_prints_lines() {
    let real = || TestTerminal::new(40, 6).terminal();
    assert!(prints_lines(real(), &[("ACCESSIBLE", "1")], None));
    assert!(prints_lines(real(), &[("ACCESSIBLE", "yes")], None));
    assert!(
        !prints_lines(real(), &[("ACCESSIBLE", "")], None),
        "empty is unset"
    );
    assert!(!prints_lines(real(), &[], None));
    assert!(
        !prints_lines(real(), &[("ACCESSIBLE", "1")], Some(false)),
        "the program decides"
    );
    assert!(prints_lines(real(), &[], Some(true)));
}

#[test]
fn the_environment_alone_does_not_change_a_test_terminal() {
    let fake = TestTerminal::new(40, 6);
    assert!(!prints_lines(fake, &[("ACCESSIBLE", "1")], None));
}

#[test]
fn a_real_terminal_is_not_queried_and_its_cursor_is_shown() {
    let terminal = TestTerminal::new(40, 6).terminal();
    let program = tea::Program::new(counter::Counter(0))
        .with_accessible(true)
        .with_environment(Environment::default());
    let mut test = TestProgram::with_terminal(program, terminal);
    test.type_text("q");
    let out = String::from_utf8_lossy(test.final_output()).into_owned();
    assert!(!out.contains("$p"), "no mode queries: {out:?}");
    assert!(out.contains("\x1b[?25h  Count: 0\r\n"), "{out:?}");
    assert!(test.final_screen().cursor_visible());
}

#[test]
fn after_a_suspend_the_whole_frame_is_printed_again() {
    let program = tea::Program::new(counter::Counter(0)).with_accessible(true);
    let mut test = accessible(program);
    test.type_text("+");
    test.suspend();
    assert_eq!(test.suspensions(), 1);
    test.type_text("q");
    let out = String::from_utf8_lossy(test.final_output()).into_owned();
    assert_eq!(out.matches("  Count: 1\r\n").count(), 2, "{out:?}");
    assert_eq!(out.matches("up/down").count(), 2, "{out:?}");
}
