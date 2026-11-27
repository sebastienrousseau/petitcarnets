//! The real terminal: raw mode, the alternate screen and keys from
//! standard input. Unix only for now.

use crate::backend::{Backend, KeySink};
use crate::key::parse;
use crate::sys::{self, STDIN, STDOUT, Termios};
use std::io::{self, Read, Stdout, Write};
use std::sync::{Mutex, Once, PoisonError};

/// The mode to restore, shared with the panic hook so a panic still gives
/// the terminal back.
static SAVED: Mutex<Option<Termios>> = Mutex::new(None);

/// Installs the panic hook once per process.
static HOOK: Once = Once::new();

/// Leaves the alternate screen, shows the cursor and restores the mode.
const LEAVE: &[u8] = b"\x1b[?25h\x1b[?1049l";

/// Enters the alternate screen and hides the cursor.
const ENTER: &[u8] = b"\x1b[?1049h\x1b[?25l";

/// The process's terminal.
#[derive(Debug)]
pub struct Terminal {
    out: Stdout,
    entered: bool,
}

impl Terminal {
    /// The terminal on standard input and output.
    #[must_use]
    pub fn new() -> Self {
        Self {
            out: io::stdout(),
            entered: false,
        }
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

/// Restores the saved mode, if there is one.
fn restore() {
    if let Some(mode) = SAVED.lock().unwrap_or_else(PoisonError::into_inner).take() {
        let _ = sys::set_mode(STDIN, &mode);
    }
}

impl Backend for Terminal {
    fn enter(&mut self) -> io::Result<()> {
        if !sys::is_tty(STDIN) {
            return Err(io::Error::other("standard input is not a terminal"));
        }
        let mode = sys::get_mode(STDIN)?;
        *SAVED.lock().unwrap_or_else(PoisonError::into_inner) = Some(mode);
        HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let _ = io::stdout().write_all(LEAVE);
                restore();
                previous(info);
            }));
        });
        sys::set_mode(STDIN, &sys::make_raw(&mode))?;
        self.entered = true;
        self.out.write_all(ENTER)?;
        self.out.flush()
    }

    fn leave(&mut self) -> io::Result<()> {
        if !self.entered {
            return Ok(());
        }
        self.entered = false;
        self.out.write_all(LEAVE)?;
        self.out.flush()?;
        restore();
        Ok(())
    }

    fn size(&mut self) -> io::Result<(u16, u16)> {
        Ok(sys::window_size(STDOUT)
            .unwrap_or_else(|_| fallback_size(|name| std::env::var(name).ok())))
    }

    fn start_input(&mut self, sink: KeySink) {
        drop(std::thread::spawn(move || read_keys(&sink)));
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.out
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

/// The size to use when the terminal does not report one (some
/// pseudo-terminals and serial lines report 0 x 0): `COLUMNS` and `LINES`
/// if they are set, else 80 x 24.
fn fallback_size(var: impl Fn(&str) -> Option<String>) -> (u16, u16) {
    let read = |name: &str, default: u16| {
        var(name)
            .and_then(|v| v.trim().parse().ok())
            .filter(|n| *n > 0)
            .unwrap_or(default)
    };
    (read("COLUMNS", 80), read("LINES", 24))
}

/// Reads standard input and sends each key, until it closes or the
/// program stops. Bytes that end mid-sequence wait for the next read; a
/// lone Escape at the end of a read is the Escape key, since a terminal
/// writes a whole sequence at once.
fn read_keys(sink: &KeySink) {
    let mut stdin = io::stdin();
    let mut pending: Vec<u8> = Vec::new();
    let mut buf = [0u8; 1024];
    while let Ok(n) = stdin.read(&mut buf) {
        if n == 0 {
            return;
        }
        pending.extend_from_slice(&buf[..n]);
        let (keys, used) = parse(&pending, true);
        let _ = pending.drain(..used);
        if keys.into_iter().any(|key| !sink(key)) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fallback_size;

    #[test]
    fn an_unreported_size_falls_back_to_the_environment_then_80_by_24() {
        assert_eq!(fallback_size(|_| None), (80, 24));
        let env = |name: &str| match name {
            "COLUMNS" => Some("120".to_string()),
            "LINES" => Some("0".to_string()),
            _ => None,
        };
        assert_eq!(fallback_size(env), (120, 24), "a zero is ignored");
    }
}
