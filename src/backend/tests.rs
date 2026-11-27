use super::*;
use crate::mode::TerminalMode;
use std::sync::mpsc;

#[test]
fn a_backend_hands_over_and_suspends_with_nothing_to_do_by_default() {
    let mut backend = TestBackend::new(10, 3);
    assert!(backend.release().is_ok());
    assert!(backend.restore().is_ok());
    assert!(backend.suspend().is_ok());
    assert!(!backend.is_terminal());
    assert!(backend.clone().terminal().is_terminal());
}

#[test]
fn a_mode_is_written_as_its_sequence_by_default() {
    let mut backend = TestBackend::new(10, 3);
    let screen = backend.screen();
    backend
        .set_mode(TerminalMode::BracketedPaste, true)
        .unwrap();
    backend
        .set_mode(TerminalMode::BracketedPaste, false)
        .unwrap();
    assert_eq!(screen.raw(), "\x1b[?2004h\x1b[?2004l");
    assert_eq!(screen.text(), "");
}

#[test]
fn input_stops_once_the_program_has_stopped() {
    let mut backend = TestBackend::new(10, 3)
        .gap(Duration::ZERO)
        .keys([Key::char('a'), Key::char('b')])
        .bytes("c");
    let (tx, rx) = mpsc::channel();
    backend.start_input(Box::new(move |input| {
        let _ = tx.send(input);
        false
    }));
    assert_eq!(rx.recv().unwrap(), Input::KeyPress(Key::char('a')));
    assert!(rx.recv().is_err(), "nothing after the sink said stop");
}
