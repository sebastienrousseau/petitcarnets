//! A pseudo-terminal for tests, opened with the POSIX calls, so the same
//! on Linux, macOS and FreeBSD. Part of `sys`, the one module allowed
//! `unsafe`; compiled for tests only.

use super::check;
use std::ffi::c_int;
use std::io;

/// A new pseudo-terminal, for tests: its master end, and its slave end,
/// which a program sees as its terminal. POSIX calls, so the same on
/// Linux, macOS and FreeBSD.
pub(crate) fn open_pty() -> io::Result<(std::fs::File, std::fs::File)> {
    use std::ffi::{CStr, c_char};
    use std::os::fd::FromRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    unsafe extern "C" {
        fn posix_openpt(flags: c_int) -> c_int;
        fn grantpt(fd: c_int) -> c_int;
        fn unlockpt(fd: c_int) -> c_int;
        fn ptsname(fd: c_int) -> *const c_char;
    }
    /// `O_RDWR`, the same everywhere.
    const O_RDWR: c_int = 2;
    /// `O_NOCTTY`: do not become the process's controlling terminal.
    #[cfg(target_os = "linux")]
    const O_NOCTTY: c_int = 0o400;
    #[cfg(target_os = "macos")]
    const O_NOCTTY: c_int = 0x2_0000;
    #[cfg(target_os = "freebsd")]
    const O_NOCTTY: c_int = 0x8000;
    // SAFETY: takes flags only.
    let fd = check(unsafe { posix_openpt(O_RDWR | O_NOCTTY) })?;
    // SAFETY: `fd` was just opened and nothing else owns it.
    let master = unsafe { std::fs::File::from_raw_fd(fd) };
    // SAFETY: `fd` is an open pseudo-terminal master.
    check(unsafe { grantpt(fd) }).map(drop)?;
    // SAFETY: as above.
    check(unsafe { unlockpt(fd) }).map(drop)?;
    // SAFETY: as above; the name is in a static buffer, copied at once
    // (only one test opens a pseudo-terminal, so no call races it).
    let name = unsafe { ptsname(fd) };
    if name.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `ptsname` returned a non-null, NUL-terminated string.
    let path = unsafe { CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned();
    let slave = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_NOCTTY)
        .open(path)?;
    Ok((master, slave))
}
