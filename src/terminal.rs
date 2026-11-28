//! The real terminal. Unix only for now.
//!
//! Input comes from standard input when it is a terminal, else from
//! `/dev/tty`; frames go to standard output when it is a terminal, else
//! to `/dev/tty`. So `my-tui | grep x` or `my-tui < file` still draws on
//! the screen.

use crate::backend::{Backend, InputSink};
use crate::guard::{HIDDEN_CURSOR, ModeKey, TerminalGuard};
use crate::mode::TerminalMode;
use crate::msg::Size;
use crate::reader::{self, PAUSE, RESUME, STOP};
use crate::sys::{self, Pipe, STDIN, STDOUT};
use std::ffi::c_int;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;

/// The longest wait for the reader to say it has paused; it answers at
/// once unless it has already stopped.
const PAUSE_TIMEOUT: Duration = Duration::from_secs(1);

/// The process's terminal.
///
/// ```no_run
/// use petitcarnets as tea;
///
/// struct Hello;
///
/// impl tea::Model for Hello {
///     type Msg = ();
///
///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
///         match msg {
///             tea::Msg::KeyPress(_) => tea::quit(),
///             _ => tea::Cmd::none(),
///         }
///     }
///
///     fn view(&self) -> tea::View {
///         "Hello! Press any key.".into()
///     }
/// }
///
/// // What `Program::run` does: draw on the process's terminal.
/// let _model = tea::Program::new(Hello).run_with(tea::Terminal::new())?;
/// # Ok::<(), tea::Error>(())
/// ```
pub struct Terminal {
    input: c_int,
    output: c_int,
    out: Box<dyn Write + Send>,
    tty: Option<File>,
    running: Option<Running>,
}

/// What exists only between `enter` and `leave`.
struct Running {
    guard: TerminalGuard,
    wake: Pipe,
    reader: Option<(JoinHandle<()>, Receiver<()>)>,
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("input", &self.input)
            .field("output", &self.output)
            .field("running", &self.running.is_some())
            .finish_non_exhaustive()
    }
}

impl Terminal {
    /// The process's terminal, found when the program starts.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: STDIN,
            output: STDOUT,
            out: Box::new(io::stdout()),
            tty: None,
            running: None,
        }
    }

    /// Uses `/dev/tty` for whichever of standard input and output is not
    /// a terminal.
    fn find_terminal(&mut self) -> io::Result<()> {
        let (stdin_tty, stdout_tty) = (sys::is_tty(STDIN), sys::is_tty(STDOUT));
        if stdin_tty && stdout_tty {
            return Ok(());
        }
        let tty = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(|e| io::Error::new(e.kind(), format!("no terminal to draw on: {e}")))?;
        if !stdin_tty {
            self.input = tty.as_raw_fd();
        }
        if !stdout_tty {
            self.output = tty.as_raw_fd();
            self.out = Box::new(tty.try_clone()?);
        }
        self.tty = Some(tty);
        Ok(())
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for Terminal {
    fn enter(&mut self) -> io::Result<()> {
        self.find_terminal()?;
        let guard = TerminalGuard::enter(self.input, self.output)?;
        let (key, enter, leave) = HIDDEN_CURSOR;
        guard.on(key, enter, leave)?;
        let wake = Pipe::new()?;
        if let Err(err) = sys::watch_signals(wake.write) {
            sys::unwatch_signals();
            return Err(err);
        }
        self.running = Some(Running {
            guard,
            wake,
            reader: None,
        });
        Ok(())
    }

    /// Stops the reader, restores the terminal, and only then gives
    /// signals their default handling back, so a second Ctrl-C or
    /// `SIGTERM` during shutdown cannot kill the process with the
    /// terminal still raw.
    fn leave(&mut self) -> io::Result<()> {
        let Some(Running {
            guard,
            wake,
            reader,
        }) = self.running.take()
        else {
            return Ok(());
        };
        let stopped = sys::write_all_fd(wake.write, &[STOP]);
        if let Some((reader, _)) = reader {
            let _ = reader.join();
        }
        let flushed = self.out.flush();
        let restored = guard.restore();
        sys::unwatch_signals();
        stopped.and(flushed).and(restored)
    }

    fn size(&mut self) -> io::Result<Size> {
        Ok(size_of(self.output))
    }

    fn is_terminal(&self) -> bool {
        true
    }

    fn start_input(&mut self, sink: InputSink) {
        if let Some(running) = self.running.as_mut() {
            let (ack, acks) = mpsc::channel();
            let reader = reader::spawn(self.input, running.wake.read, self.output, sink, ack);
            running.reader = Some((reader, acks));
        }
    }

    /// Pauses the reader once it says so (unread bytes stay for the next
    /// program), shows the cursor, gives `SIGTSTP` its default handling
    /// and puts the saved mode back, at once.
    fn release(&mut self) -> io::Result<()> {
        self.out.flush()?;
        let Some(running) = self.running.as_ref() else {
            return Ok(());
        };
        if let Some((_, acks)) = running.reader.as_ref() {
            sys::write_all_fd(running.wake.write, &[PAUSE])?;
            let _ = acks.recv_timeout(PAUSE_TIMEOUT);
        }
        running.guard.off(ModeKey::HiddenCursor)?;
        sys::default_stop(true)?;
        running.guard.cook()
    }

    /// Raw mode and the hidden cursor again, `SIGTSTP` watched again,
    /// and the reader resumed.
    fn restore(&mut self) -> io::Result<()> {
        let Some(running) = self.running.as_ref() else {
            return Ok(());
        };
        running.guard.raw()?;
        sys::default_stop(false)?;
        let (key, enter, leave) = HIDDEN_CURSOR;
        running.guard.on(key, enter, leave)?;
        if running.reader.is_some() {
            sys::write_all_fd(running.wake.write, &[RESUME])?;
        }
        Ok(())
    }

    /// Stops the process group, as a shell's Ctrl-Z does, until it is
    /// continued.
    fn suspend(&mut self) -> io::Result<()> {
        sys::stop_process_group()
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.out
    }

    /// Writes the mode straight to the terminal, after any frame still
    /// buffered, and records it in the guard, so it is turned off on
    /// every way out.
    fn set_mode(&mut self, mode: TerminalMode, on: bool) -> io::Result<()> {
        self.out.flush()?;
        let Some(running) = self.running.as_ref() else {
            return Ok(());
        };
        let key = ModeKey::Mode(mode.kind());
        match (mode, on) {
            (TerminalMode::KeyboardFlags(_), true) => running.guard.write(&mode.sequence(true)),
            (TerminalMode::KeyboardFlags(_), false) => Ok(()),
            (_, true) => running.guard.on(key, &mode.sequence(true), mode.disable()),
            (_, false) => running.guard.off(key),
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

/// The size of the terminal on `fd`, falling back when it does not report
/// one (some pseudo-terminals and serial lines report 0 x 0).
pub(crate) fn size_of(fd: c_int) -> Size {
    sys::window_size(fd).map_or_else(
        |_| fallback_size(|name| std::env::var(name).ok()),
        |(w, h)| Size::new(w, h),
    )
}

/// `COLUMNS` and `LINES` if they are set, else 80 x 24.
fn fallback_size(var: impl Fn(&str) -> Option<String>) -> Size {
    let read = |name: &str, default: u16| {
        var(name)
            .and_then(|v| v.trim().parse().ok())
            .filter(|n| *n > 0)
            .unwrap_or(default)
    };
    Size::new(read("COLUMNS", 80), read("LINES", 24))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unreported_size_falls_back_to_the_environment_then_80_by_24() {
        assert_eq!(fallback_size(|_| None), Size::new(80, 24));
        let env = |name: &str| match name {
            "COLUMNS" => Some("120".to_string()),
            "LINES" => Some("0".to_string()),
            _ => None,
        };
        assert_eq!(fallback_size(env), Size::new(120, 24), "a zero is ignored");
    }

    #[test]
    fn leaving_a_terminal_never_entered_does_nothing() {
        assert!(Terminal::new().leave().is_ok());
    }
}
