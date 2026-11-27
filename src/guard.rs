//! Giving the terminal back however the program ends.
//!
//! The guard saves the terminal's mode before changing it, then keeps
//! the terminal modes that are on (alternate screen, hidden cursor,
//! mouse, bracketed paste and the rest), keyed by mode, in the order they
//! were turned on. Turning one off writes its leave sequence and forgets
//! it; restoring turns the rest off, newest first, and puts the saved
//! mode back. It runs when the guard drops (a normal end, an error,
//! a panic unwinding through `run`) and from a panic hook, so a panic on
//! any thread, or one under `panic = "abort"`, still leaves a usable
//! terminal. Restoring twice is harmless: the second time finds nothing.

use crate::mode::TerminalMode;
use crate::sys::{self, Termios};
use std::ffi::c_int;
use std::io;
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

/// Which mode an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModeKey {
    /// The hidden cursor.
    HiddenCursor,
    /// A [`TerminalMode`], whatever its flags.
    Mode(TerminalMode),
}

/// A mode the guard turns on: its key, and the bytes that turn it on and
/// off.
pub(crate) type Mode = (ModeKey, &'static [u8], &'static [u8]);

/// The cursor, hidden while the program draws.
pub(crate) const HIDDEN_CURSOR: Mode = (ModeKey::HiddenCursor, b"\x1b[?25l", b"\x1b[?25h");

/// The modes that are on, in the order they were turned on, with the
/// bytes that turn each off.
#[derive(Debug, Default)]
pub(crate) struct Active(Vec<(ModeKey, &'static [u8])>);

impl Active {
    /// Records `key` as on; one already on keeps its place.
    pub(crate) fn on(&mut self, key: ModeKey, leave: &'static [u8]) {
        match self.0.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = leave,
            None => self.0.push((key, leave)),
        }
    }

    /// Forgets `key`; the bytes that turn it off, if it was on.
    pub(crate) fn off(&mut self, key: ModeKey) -> Option<&'static [u8]> {
        let i = self.0.iter().position(|(k, _)| *k == key)?;
        Some(self.0.remove(i).1)
    }

    /// The bytes that turn every mode off, newest first.
    pub(crate) fn leave_all(&self) -> Vec<u8> {
        self.0
            .iter()
            .rev()
            .flat_map(|(_, l)| l.iter().copied())
            .collect()
    }
}

/// What restoring needs; shared with the panic hook.
struct Saved {
    input: c_int,
    mode: Termios,
    output: c_int,
    active: Active,
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
    /// Saves `input`'s mode, then puts it in raw mode. Modes turned on
    /// later are written to `output`.
    pub(crate) fn enter(input: c_int, output: c_int) -> io::Result<Self> {
        let mode = sys::get_mode(input)?;
        *saved() = Some(Saved {
            input,
            mode,
            output,
            active: Active::default(),
        });
        install_panic_hook();
        let guard = Self { output };
        sys::set_mode(input, &sys::make_raw(&mode))?;
        Ok(guard)
    }

    /// Turns a mode on with `enter`; restoring writes `leave`. The leave
    /// sequence is recorded first, so a failed write is still undone.
    pub(crate) fn on(&self, key: ModeKey, enter: &[u8], leave: &'static [u8]) -> io::Result<()> {
        if let Some(state) = saved().as_mut() {
            state.active.on(key, leave);
        }
        sys::write_all_fd(self.output, enter)
    }

    /// Turns a mode off now, if it is on.
    pub(crate) fn off(&self, key: ModeKey) -> io::Result<()> {
        let leave = saved().as_mut().and_then(|state| state.active.off(key));
        leave.map_or(Ok(()), |leave| sys::write_all_fd(self.output, leave))
    }

    /// Writes `bytes` that change a mode already on, such as new kitty
    /// keyboard flags.
    pub(crate) fn write(&self, bytes: &[u8]) -> io::Result<()> {
        sys::write_all_fd(self.output, bytes)
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
    let written = sys::write_all_fd(state.output, &state.active.leave_all());
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
mod tests;
