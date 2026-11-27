//! The guard's bookkeeping, without a terminal.

use super::*;

#[test]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
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

/// Every byte the master end has to read now.
#[cfg(target_os = "linux")]
fn drain(master: c_int) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 256];
    while sys::wait_readable([master], Some(std::time::Duration::ZERO)).is_ok_and(|[r]| r) {
        match sys::read_fd(master, &mut buf) {
            Ok(n) if n > 0 => out.extend_from_slice(&buf[..n]),
            _ => break,
        }
    }
    out
}

/// One test, as the guard's state is the process's: raw mode, modes on
/// and off, handing over and back, and restoring, on a real
/// pseudo-terminal.
#[test]
#[cfg(target_os = "linux")]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
fn a_guard_drives_a_real_terminal_and_gives_it_back() {
    use std::os::fd::AsRawFd;
    let (master, slave) = sys::open_pty().unwrap();
    let (m, s) = (master.as_raw_fd(), slave.as_raw_fd());
    let before = sys::get_mode(s).unwrap();
    // A panic in another test would run the guard's hook and restore
    // this terminal mid-test: the hook is installed, then set aside
    // until the end.
    install_panic_hook();
    let hook = std::panic::take_hook();
    let guard = TerminalGuard::enter(s, s).unwrap();
    let raw = sys::get_mode(s).unwrap();
    assert_ne!(raw, before, "raw mode is on");
    assert_eq!(raw, sys::make_raw(&before));
    let paste = ModeKey::Mode(TerminalMode::BracketedPaste);
    guard.on(paste, b"<paste on>", b"<paste off>").unwrap();
    guard
        .on(HIDDEN_CURSOR.0, HIDDEN_CURSOR.1, HIDDEN_CURSOR.2)
        .unwrap();
    guard.write(b"<flags>").unwrap();
    guard.off(HIDDEN_CURSOR.0).unwrap();
    guard.off(HIDDEN_CURSOR.0).unwrap();
    assert_eq!(drain(m), b"<paste on>\x1b[?25l<flags>\x1b[?25h");
    guard.cook().unwrap();
    assert_eq!(
        sys::get_mode(s).unwrap(),
        before,
        "the saved mode for a child"
    );
    guard.raw().unwrap();
    assert_eq!(sys::get_mode(s).unwrap(), raw, "and raw again");
    guard.restore().unwrap();
    assert_eq!(drain(m), b"<paste off>", "what is still on is turned off");
    assert_eq!(sys::get_mode(s).unwrap(), before, "the mode is back");
    let again = TerminalGuard::enter(s, s).unwrap();
    again.on(paste, b"", b"<off on drop>").unwrap();
    drop(again);
    assert_eq!(drain(m), b"<off on drop>");
    assert_eq!(sys::get_mode(s).unwrap(), before);
    let late = TerminalGuard {
        input: s,
        output: s,
    };
    assert!(
        late.cook().is_ok() && late.raw().is_ok(),
        "no guard, nothing to do"
    );
    assert_eq!(sys::get_mode(s).unwrap(), before);
    std::panic::set_hook(hook);
}
