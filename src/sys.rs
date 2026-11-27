//! The few libc calls a terminal needs, declared by hand so the crate has
//! no dependencies. This is the only module allowed `unsafe` code.
//!
//! The `termios` structure differs between Linux and macOS (field widths
//! and the size of `c_cc`). It is never read here: it is kept as an
//! opaque, over-sized, 8-byte-aligned buffer that `tcgetattr` fills,
//! `cfmakeraw` edits and `tcsetattr` reads back, so its layout never
//! matters.
#![allow(unsafe_code)] // The FFI boundary: each call's safety is argued where it is made.

use std::ffi::{c_int, c_ulong, c_ushort};
use std::io;

/// Room for any platform's `termios` (60 bytes on Linux, 72 on macOS).
const TERMIOS_WORDS: usize = 32;

/// `TCSAFLUSH`: apply the change after output drains, discarding unread
/// input (2 on Linux and macOS alike).
const TCSAFLUSH: c_int = 2;

/// `TIOCGWINSZ`: read the window size.
#[cfg(target_os = "linux")]
const TIOCGWINSZ: c_ulong = 0x5413;
#[cfg(not(target_os = "linux"))]
const TIOCGWINSZ: c_ulong = 0x4008_7468;

/// A saved terminal mode, as an opaque buffer.
#[derive(Clone, Copy)]
pub(crate) struct Termios([u64; TERMIOS_WORDS]);

impl std::fmt::Debug for Termios {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Termios(..)")
    }
}

/// `struct winsize`: the same four `unsigned short`s everywhere.
#[repr(C)]
#[derive(Default)]
struct Winsize {
    rows: c_ushort,
    cols: c_ushort,
    x_pixels: c_ushort,
    y_pixels: c_ushort,
}

unsafe extern "C" {
    fn tcgetattr(fd: c_int, termios: *mut u64) -> c_int;
    fn tcsetattr(fd: c_int, action: c_int, termios: *const u64) -> c_int;
    fn cfmakeraw(termios: *mut u64);
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn isatty(fd: c_int) -> c_int;
}

/// Standard input's file descriptor.
pub(crate) const STDIN: c_int = 0;

/// Standard output's file descriptor.
pub(crate) const STDOUT: c_int = 1;

/// The terminal's current mode.
pub(crate) fn get_mode(fd: c_int) -> io::Result<Termios> {
    let mut t = Termios([0; TERMIOS_WORDS]);
    // SAFETY: `t.0` is a writable buffer larger than any platform's
    // `termios` and suitably aligned; `tcgetattr` writes at most one.
    let rc = unsafe { tcgetattr(fd, t.0.as_mut_ptr()) };
    if rc == 0 {
        Ok(t)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Sets the terminal's mode.
pub(crate) fn set_mode(fd: c_int, termios: &Termios) -> io::Result<()> {
    // SAFETY: the buffer was filled by `tcgetattr` (and perhaps edited by
    // `cfmakeraw`), so it holds a valid `termios`; it outlives the call.
    let rc = unsafe { tcsetattr(fd, TCSAFLUSH, termios.0.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// `mode` changed to raw: no echo, no line buffering, no signal keys,
/// no output processing; reads return after one byte.
pub(crate) fn make_raw(mode: &Termios) -> Termios {
    let mut raw = *mode;
    // SAFETY: the buffer holds a valid `termios` from `tcgetattr`;
    // `cfmakeraw` only edits its fields in place.
    unsafe { cfmakeraw(raw.0.as_mut_ptr()) };
    raw
}

/// The window's size as `(columns, rows)`.
pub(crate) fn window_size(fd: c_int) -> io::Result<(u16, u16)> {
    let mut size = Winsize::default();
    // SAFETY: `TIOCGWINSZ` writes one `struct winsize`, which `Winsize`
    // matches field for field (`repr(C)`), into a live local.
    let rc = unsafe { ioctl(fd, TIOCGWINSZ, &raw mut size) };
    if rc == 0 && size.cols > 0 && size.rows > 0 {
        Ok((size.cols, size.rows))
    } else {
        Err(io::Error::other("the window size is not available"))
    }
}

/// Whether `fd` is a terminal.
pub(crate) fn is_tty(fd: c_int) -> bool {
    // SAFETY: `isatty` only inspects the descriptor; any value is safe.
    unsafe { isatty(fd) == 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pipe_is_not_a_terminal() {
        // cargo test's stdin is not a terminal in CI; a bad descriptor never is.
        assert!(!is_tty(-1));
        assert!(get_mode(-1).is_err());
        assert!(window_size(-1).is_err());
    }

    #[test]
    fn the_opaque_buffer_holds_any_termios() {
        assert!(size_of::<Termios>() >= 72);
        assert_eq!(align_of::<Termios>(), 8);
    }
}
