//! The guard's bookkeeping, without a terminal.

use super::*;

#[test]
fn entering_needs_a_terminal() {
    assert!(TerminalGuard::enter(-1, -1).is_err());
}

#[test]
fn restoring_with_nothing_saved_does_nothing() {
    assert!(restore(None).is_ok());
}

#[test]
fn modes_turn_off_newest_first_and_toggled_ones_drop_out() {
    let paste = ModeKey::Mode(TerminalMode::BracketedPaste);
    let mouse = ModeKey::Mode(TerminalMode::MouseCellMotion);
    let focus = ModeKey::Mode(TerminalMode::ReportFocus);
    let mut active = Active::default();
    active.on(ModeKey::Mode(TerminalMode::AltScreen), b"<alt>");
    active.on(ModeKey::HiddenCursor, b"<cursor>");
    active.on(paste, b"<paste>");
    active.on(mouse, b"<mouse>");
    active.on(focus, b"<focus>");
    assert_eq!(active.off(mouse), Some(&b"<mouse>"[..]));
    assert_eq!(active.off(mouse), None, "already off");
    active.on(paste, b"<paste2>");
    active.on(mouse, b"<mouse>");
    assert_eq!(
        active.leave_all(),
        b"<mouse><focus><paste2><cursor><alt>",
        "newest first; turning paste on again keeps its place"
    );
}
