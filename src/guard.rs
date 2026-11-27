//! Giving the terminal back however the program ends.
//!
//! The guard saves the terminal's mode before changing it, then records a
//! stack of terminal modes as they are turned on (alternate screen,
//! hidden cursor). Restoring turns them off in reverse order and puts the
//! saved mode back. It runs when the guard drops (a normal end, an error,
//! a panic unwinding through `run`) and from a panic hook, so a panic on
//! any thread, or one under `panic = "abort"`, still leaves a usable
//! terminal. Restoring twice is harmless: the second time finds nothing.

use crate::sys::{self, Termios};
use std::ffi::c_int;
use std::io;
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

/// The alternate screen, saving and restoring the main one.
pub(crate) const ALT_SCREEN: (&[u8], &[u8]) = (b"\x1b[?1049h", b"\x1b[?1049l");

/// The cursor, hidden while the program draws.
pub(crate) const HIDDEN_CURSOR: (&[u8], &[u8]) = (b"\x1b[?25l", b"\x1b[?25h");

/// What restoring needs; shared with the panic hook.
struct Saved {
    input: c_int,
    mode: Termios,
    output: c_int,
    leave: Vec<&'static [u8]>,
}

static SAVED: Mutex<Option<Saved>> = Mutex::new(None);
static HOOK: Once = Once::new();

fn saved() -> MutexGuard<'static, Option<Saved>> {
    SAVED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Owns the terminal's state while a program runs; dropping it restores
/// the terminal.
#[derive(Debug)]
pub(crate) struct TerminalGuard {
    output: c_int,
}

impl TerminalGuard {
    /// Saves `input`'s mode, then puts it in raw mode. Modes pushed later
    /// are written to `output`.
    pub(crate) fn enter(input: c_int, output: c_int) -> io::Result<Self> {
        let mode = sys::get_mode(input)?;
        *saved() = Some(Saved {
            input,
            mode,
            output,
            leave: Vec::new(),
        });
        install_panic_hook();
        let guard = Self { output };
        sys::set_mode(input, &sys::make_raw(&mode))?;
        Ok(guard)
    }

    /// Turns a mode on with `enter`; restoring writes `leave`. The leave
    /// sequence is recorded first, so a failed write is still undone.
    pub(crate) fn push(&self, (enter, leave): (&[u8], &'static [u8])) -> io::Result<()> {
        if let Some(state) = saved().as_mut() {
            state.leave.push(leave);
        }
        sys::write_all_fd(self.output, enter)
    }

    /// Restores the terminal now and reports whether that worked.
    pub(crate) fn restore(self) -> io::Result<()> {
        let result = restore(saved().take());
        drop(self);
        result
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore(saved().take());
    }
}

/// Turns the recorded modes off, newest first, and puts the mode back.
fn restore(state: Option<Saved>) -> io::Result<()> {
    let Some(state) = state else {
        return Ok(());
    };
    let leave: Vec<u8> = state
        .leave
        .iter()
        .rev()
        .flat_map(|l| l.iter().copied())
        .collect();
    let written = sys::write_all_fd(state.output, &leave);
    let reset = sys::set_mode(state.input, &state.mode);
    written.and(reset)
}

/// Restores the terminal before the panic message prints, then runs the
/// previous hook. `try_lock`, because the panic may have happened while
/// this thread held the lock.
fn install_panic_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Ok(mut state) = SAVED.try_lock() {
                let _ = restore(state.take());
            }
            previous(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_needs_a_terminal() {
        assert!(TerminalGuard::enter(-1, -1).is_err());
    }

    #[test]
    fn restoring_with_nothing_saved_does_nothing() {
        assert!(restore(None).is_ok());
    }
}
