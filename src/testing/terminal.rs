//! The terminal a [`TestProgram`](super::TestProgram) runs in: a size,
//! replies scripted for the queries a program writes, and everything
//! written, played into a [`Vt`] as it is written.

use super::Vt;
use crate::backend::{Backend, Input, InputSink};
use crate::clock::{Clock, VirtualClock};
use crate::input::{Reader, inputs};
use crate::msg::Size;
use std::io::{self, Write};
use std::time::Instant;

/// How the terminal a [`TestProgram`](super::TestProgram) runs in is set
/// up: its size (teatest's `WithInitialTermSize`), whether it acts as a
/// real terminal, and the replies it gives to queries.
///
/// ```
/// use petitcarnets::testing::TestTerminal;
///
/// // Answers a cursor position request at once, as a fast terminal would.
/// let terminal = TestTerminal::new(40, 10).reply("\x1b[6n", "\x1b[3;7R");
/// # let _ = terminal;
/// ```
#[derive(Debug, Clone)]
pub struct TestTerminal {
    size: Size,
    terminal: bool,
    replies: Vec<(Vec<u8>, Vec<u8>)>,
}

impl TestTerminal {
    /// A `columns` by `rows` terminal that answers nothing.
    #[must_use]
    pub fn new(columns: u16, rows: u16) -> Self {
        Self {
            size: Size::new(columns, rows),
            terminal: false,
            replies: Vec::new(),
        }
    }

    /// Acts as a real terminal ([`Backend::is_terminal`]): the program
    /// asks it about synchronized output and grapheme widths at the
    /// start, detects its colour profile from the program's environment,
    /// and before it hands the terminal over or ends, waits for replies
    /// to its queries: up to half a second of virtual time, unless a
    /// [`reply`](TestTerminal::reply) to `ESC [ c` ends the wait.
    #[must_use]
    pub const fn terminal(mut self) -> Self {
        self.terminal = true;
        self
    }

    /// Answers with `answer` each time the program writes `query` in one
    /// piece (as every request command does), at once, through the same
    /// decoder as typed bytes.
    #[must_use]
    pub fn reply(mut self, query: impl AsRef<[u8]>, answer: impl AsRef<[u8]>) -> Self {
        self.replies
            .push((query.as_ref().to_vec(), answer.as_ref().to_vec()));
        self
    }

    /// The backend for a program, on `clock`.
    pub(super) fn backend(self, clock: VirtualClock) -> Harness {
        let (columns, rows) = (self.size.width, self.size.height);
        Harness {
            size: self.size,
            terminal: self.terminal,
            wire: Wire {
                bytes: Vec::new(),
                vt: Vt::new(columns, rows),
                replies: self.replies,
                keyboard: Keyboard {
                    sink: None,
                    reader: Reader::default(),
                    clock,
                },
            },
            suspensions: 0,
            released: false,
        }
    }
}

/// The test terminal as a backend.
pub(super) struct Harness {
    size: Size,
    terminal: bool,
    pub(super) wire: Wire,
    /// How many times the program stopped the process.
    pub(super) suspensions: usize,
    /// Whether the terminal is handed over.
    pub(super) released: bool,
}

impl Harness {
    /// A new size for the window and the emulated screen.
    pub(super) fn resize(&mut self, size: Size) {
        self.size = size;
        self.wire.vt.resize(size.width, size.height);
    }
}

/// Where the program writes: kept, played into the emulator, and
/// answered.
pub(super) struct Wire {
    pub(super) bytes: Vec<u8>,
    pub(super) vt: Vt,
    replies: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) keyboard: Keyboard,
}

/// What the terminal sends the program, decoded as a real terminal's
/// bytes are.
pub(super) struct Keyboard {
    sink: Option<InputSink>,
    reader: Reader,
    clock: VirtualClock,
}

impl Keyboard {
    /// Sends `input` to the program, if it is running.
    pub(super) fn send(&self, input: Input) {
        if let Some(sink) = &self.sink {
            let _ = sink(input);
        }
    }

    /// Sends what `bytes` decode to. A sequence cut off at the end waits
    /// for more bytes, or for the escape timeout.
    pub(super) fn feed(&mut self, bytes: &[u8]) {
        let events = self.reader.feed(bytes, self.clock.now().at);
        inputs(events).for_each(|input| self.send(input));
    }

    /// Reads what is held back as it is, if the escape timeout passed.
    pub(super) fn expire(&mut self) {
        let events = self.reader.expire(self.clock.now().at);
        inputs(events).for_each(|input| self.send(input));
    }

    /// When the bytes held back are read as they are, if any are.
    pub(super) const fn deadline(&self) -> Option<Instant> {
        self.reader.deadline()
    }
}

impl Write for Wire {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buf);
        self.vt.feed(buf);
        for (query, answer) in &self.replies {
            for _ in 0..occurrences(buf, query) {
                self.keyboard.feed(answer);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// How many times `needle` occurs in `haystack`.
fn occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

impl Backend for Harness {
    fn enter(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn size(&mut self) -> io::Result<Size> {
        Ok(self.size)
    }

    fn start_input(&mut self, sink: InputSink) {
        self.wire.keyboard.sink = Some(sink);
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.wire
    }

    fn is_terminal(&self) -> bool {
        self.terminal
    }

    fn release(&mut self) -> io::Result<()> {
        self.released = true;
        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        self.released = false;
        Ok(())
    }

    /// Stops nothing: the process is continued at once.
    fn suspend(&mut self) -> io::Result<()> {
        self.suspensions += 1;
        Ok(())
    }
}
