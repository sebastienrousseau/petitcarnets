//! The few libc calls a terminal needs, declared by hand so the crate has
//! no dependencies. This is the only module allowed `unsafe` code.
//!
//! The `termios` structure differs between Linux and macOS (field widths
//! and the size of `c_cc`). It is never read here: it is kept as an
//! opaque, over-sized, 8-byte-aligned buffer that `tcgetattr` fills,
//! `cfmakeraw` edits and `tcsetattr` reads back, so its layout never
//! matters.
//!
//! Signals are installed with `signal(2)`, not `sigaction`, because
//! `struct sigaction` differs between glibc, musl, macOS and FreeBSD
//! while `signal` takes only a number and a function. On all four it
//! keeps the handler installed and restarts interrupted calls. Variadic C
//! functions (`ioctl`, `fcntl`) are declared variadic: Apple Silicon
//! passes variadic arguments differently from fixed ones.
#![allow(unsafe_code)] // The FFI boundary: each call's safety is argued where it is made.

use std::ffi::{c_int, c_short, c_ulong, c_ushort, c_void};
use std::io;
use std::sync::atomic::{AtomicI32, Ordering};

/// Room for any platform's `termios` (60 bytes on Linux, 72 on macOS).
const TERMIOS_WORDS: usize = 32;

/// `TCSAFLUSH`: apply the change after output drains, discarding unread
/// input (2 on Linux and macOS alike).
const TCSAFLUSH: c_int = 2;

/// Values that differ between the supported systems.
#[cfg(target_os = "linux")]
mod os {
    use std::ffi::{c_int, c_ulong};
    /// `TIOCGWINSZ`: read the window size.
    pub(super) const TIOCGWINSZ: c_ulong = 0x5413;
    /// `O_NONBLOCK` (the generic Linux value, as on x86 and Arm).
    pub(super) const O_NONBLOCK: c_int = 0o4000;
    /// `nfds_t`.
    pub(super) type NFds = c_ulong;
}
#[cfg(any(target_os = "macos", target_os = "freebsd"))]
mod os {
    use std::ffi::{c_int, c_uint, c_ulong};
    /// `TIOCGWINSZ`: read the window size.
    pub(super) const TIOCGWINSZ: c_ulong = 0x4008_7468;
    /// `O_NONBLOCK`.
    pub(super) const O_NONBLOCK: c_int = 0x0004;
    /// `nfds_t`.
    pub(super) type NFds = c_uint;
}
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
compile_error!("petitcarnets supports Linux, macOS and FreeBSD for now");

/// Signal numbers. `SIGINT`, `SIGTERM` and `SIGWINCH` are the same on
/// Linux, macOS and FreeBSD; `SIGTSTP` and `SIGCONT` are not, and get a
/// per-OS entry when suspend support needs them.
pub(crate) mod signo {
    use std::ffi::c_int;
    /// Interrupt (Ctrl-C outside raw mode, or `kill -INT`).
    pub(crate) const SIGINT: c_int = 2;
    /// Termination request.
    pub(crate) const SIGTERM: c_int = 15;
    /// The window changed size.
    pub(crate) const SIGWINCH: c_int = 28;
}

/// The signals a running program listens for.
const WATCHED: [c_int; 3] = [signo::SIGINT, signo::SIGTERM, signo::SIGWINCH];

/// `fcntl` commands, the same on every supported system.
const F_GETFL: c_int = 3;
const F_SETFL: c_int = 4;
const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;

/// `poll` event bits, the same on every supported system.
const POLLIN: c_short = 0x01;
const POLLERR: c_short = 0x08;
const POLLHUP: c_short = 0x10;
const POLLNVAL: c_short = 0x20;

/// A signal handler; `None` is `SIG_DFL` (a null pointer).
type Handler = Option<extern "C" fn(c_int)>;

/// `SIG_ERR`, the value `signal` returns on failure.
const SIG_ERR: isize = -1;

/// `EINTR`: a call interrupted by a signal (4 everywhere).
const EINTR: i32 = 4;

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

/// `struct pollfd`: the same layout everywhere.
#[repr(C)]
struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

unsafe extern "C" {
    fn tcgetattr(fd: c_int, termios: *mut u64) -> c_int;
    fn tcsetattr(fd: c_int, action: c_int, termios: *const u64) -> c_int;
    fn cfmakeraw(termios: *mut u64);
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn isatty(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn poll(fds: *mut PollFd, nfds: os::NFds, timeout: c_int) -> c_int;
    fn signal(signum: c_int, handler: Handler) -> isize;
}

/// Standard input's file descriptor.
pub(crate) const STDIN: c_int = 0;

/// Standard output's file descriptor.
pub(crate) const STDOUT: c_int = 1;

/// `Ok(value)` when a call succeeded, else the OS error.
fn check(rc: c_int) -> io::Result<c_int> {
    if rc == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(rc)
    }
}

/// The terminal's current mode.
pub(crate) fn get_mode(fd: c_int) -> io::Result<Termios> {
    let mut t = Termios([0; TERMIOS_WORDS]);
    // SAFETY: `t.0` is a writable buffer larger than any platform's
    // `termios` and suitably aligned; `tcgetattr` writes at most one.
    check(unsafe { tcgetattr(fd, t.0.as_mut_ptr()) }).map(drop)?;
    Ok(t)
}

/// Sets the terminal's mode.
pub(crate) fn set_mode(fd: c_int, termios: &Termios) -> io::Result<()> {
    // SAFETY: the buffer was filled by `tcgetattr` (and perhaps edited by
    // `cfmakeraw`), so it holds a valid `termios`; it outlives the call.
    check(unsafe { tcsetattr(fd, TCSAFLUSH, termios.0.as_ptr()) }).map(drop)
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
    let rc = unsafe { ioctl(fd, os::TIOCGWINSZ, &raw mut size) };
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

/// Reads into `buf`, retrying when a signal interrupts the call; 0 means
/// end of input.
pub(crate) fn read_fd(fd: c_int, buf: &mut [u8]) -> io::Result<usize> {
    loop {
        // SAFETY: `buf` is a live, writable buffer of `buf.len()` bytes.
        let n = unsafe { read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        match usize::try_from(n) {
            Ok(n) => return Ok(n),
            Err(_) => interrupted_or(io::Error::last_os_error())?,
        }
    }
}

/// Writes all of `bytes`, retrying when a signal interrupts the call.
pub(crate) fn write_all_fd(fd: c_int, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        // SAFETY: `bytes` is a live buffer of `bytes.len()` bytes.
        let n = unsafe { write(fd, bytes.as_ptr().cast(), bytes.len()) };
        match usize::try_from(n) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => bytes = &bytes[n..],
            Err(_) => interrupted_or(io::Error::last_os_error())?,
        }
    }
    Ok(())
}

/// `Ok` for an interrupted call (to retry), else the error.
fn interrupted_or(err: io::Error) -> io::Result<()> {
    if err.raw_os_error() == Some(EINTR) {
        Ok(())
    } else {
        Err(err)
    }
}

/// Which of `fds` can be read without blocking (or have hung up), waiting
/// until at least one can. Interrupted waits are retried.
pub(crate) fn wait_readable<const N: usize>(fds: [c_int; N]) -> io::Result<[bool; N]> {
    let mut polled = fds.map(|fd| PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    });
    let count = os::NFds::try_from(N).map_err(io::Error::other)?;
    loop {
        // SAFETY: `polled` is a live array of `N` `pollfd`s; `poll` only
        // writes their `revents`.
        let rc = unsafe { poll(polled.as_mut_ptr(), count, -1) };
        if rc >= 0 {
            break;
        }
        interrupted_or(io::Error::last_os_error())?;
    }
    let ready = POLLIN | POLLERR | POLLHUP | POLLNVAL;
    Ok(polled.map(|p| p.revents & ready != 0))
}

/// A pipe whose write end never blocks: a signal handler or another
/// thread wakes a `poll` by writing one byte to it. Both ends close on
/// drop and are not inherited by child processes.
#[derive(Debug)]
pub(crate) struct Pipe {
    /// The end to read and poll.
    pub(crate) read: c_int,
    /// The end to write a byte to.
    pub(crate) write: c_int,
}

impl Pipe {
    /// A new pipe.
    pub(crate) fn new() -> io::Result<Self> {
        let mut fds = [-1; 2];
        // SAFETY: `fds` has room for the two descriptors `pipe` writes.
        check(unsafe { pipe(fds.as_mut_ptr()) }).map(drop)?;
        let pipe = Self {
            read: fds[0],
            write: fds[1],
        };
        for fd in fds {
            // SAFETY: `fd` is a descriptor this pipe owns.
            check(unsafe { fcntl(fd, F_SETFD, FD_CLOEXEC) }).map(drop)?;
        }
        // SAFETY: as above; `F_GETFL` takes no argument.
        let flags = check(unsafe { fcntl(pipe.write, F_GETFL) })?;
        // SAFETY: as above; `F_SETFL` takes the new flags.
        check(unsafe { fcntl(pipe.write, F_SETFL, flags | os::O_NONBLOCK) }).map(drop)?;
        Ok(pipe)
    }

    /// Closes the write end, so the read end sees end of input.
    #[cfg(test)]
    pub(crate) fn close_write(&mut self) {
        close_fd(&mut self.write);
    }
}

/// Closes `fd` once and marks it closed with -1.
fn close_fd(fd: &mut c_int) {
    if *fd >= 0 {
        // SAFETY: the caller owns `fd`; it is marked closed right after,
        // so it is never closed twice.
        let _ = unsafe { close(*fd) };
        *fd = -1;
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        close_fd(&mut self.read);
        close_fd(&mut self.write);
    }
}

/// Where the signal handler writes: a pipe's write end, or -1.
static SIGNAL_FD: AtomicI32 = AtomicI32::new(-1);

/// The signal handler: writes the signal's number, as one byte, to
/// [`SIGNAL_FD`]. `write` is async-signal-safe, and the pipe never
/// blocks; if it is full, the byte is dropped, which loses nothing a
/// pending byte does not already say.
extern "C" fn on_signal(signum: c_int) {
    let fd = SIGNAL_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        let byte = u8::try_from(signum).unwrap_or(0);
        // SAFETY: `byte` is a live local; `fd` is a pipe's write end that
        // stays open while it is in `SIGNAL_FD`.
        let _ = unsafe { write(fd, (&raw const byte).cast(), 1) };
    }
}

/// Sends `SIGINT`, `SIGTERM` and `SIGWINCH` to `fd`, one byte each.
pub(crate) fn watch_signals(fd: c_int) -> io::Result<()> {
    SIGNAL_FD.store(fd, Ordering::Relaxed);
    for signum in WATCHED {
        // SAFETY: `on_signal` is an `extern "C" fn(c_int)` that only does
        // async-signal-safe work.
        let previous = unsafe { signal(signum, Some(on_signal)) };
        if previous == SIG_ERR {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Gives the watched signals their default handling back.
pub(crate) fn unwatch_signals() {
    for signum in WATCHED {
        // SAFETY: restoring the default disposition is always valid.
        let _ = unsafe { signal(signum, None) };
    }
    SIGNAL_FD.store(-1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bad_descriptor_is_not_a_terminal() {
        assert!(!is_tty(-1));
        assert!(get_mode(-1).is_err());
        assert!(window_size(-1).is_err());
    }

    #[test]
    fn the_opaque_buffer_holds_any_termios() {
        assert!(size_of::<Termios>() >= 72);
        assert_eq!(align_of::<Termios>(), 8);
    }

    #[test]
    fn a_pipe_carries_bytes_and_wakes_a_wait() {
        let pipe = Pipe::new().unwrap();
        write_all_fd(pipe.write, b"hi").unwrap();
        assert_eq!(wait_readable([pipe.read]).unwrap(), [true]);
        let mut buf = [0u8; 8];
        assert_eq!(read_fd(pipe.read, &mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], b"hi");
    }

    #[test]
    fn the_write_end_never_blocks() {
        let pipe = Pipe::new().unwrap();
        let chunk = [0u8; 4096];
        let full = (0..1024).find_map(|_| write_all_fd(pipe.write, &chunk).err());
        let err = full.expect("a pipe fills up long before 4 MiB");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }
}
