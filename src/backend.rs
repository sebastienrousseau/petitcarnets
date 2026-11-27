//! Where a program reads input and writes frames: the real terminal, or a
//! scripted one for tests.

use crate::color::Rgb;
use crate::input::{ESC_TIMEOUT, Event, Reader};
use crate::key::Key;
use crate::mode::{KeyboardEnhancementsReport, ModeSetting, ReportedMode, TerminalMode};
use crate::mouse::Mouse;
use crate::msg::Size;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// What a backend reports to the running program: what the terminal
/// sent, decoded, and signals. Each terminal variant becomes the
/// [`Msg`](crate::Msg) of the same name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// A key was pressed.
    KeyPress(Key),
    /// A key was released.
    KeyRelease(Key),
    /// A mouse button was pressed.
    MouseClick(Mouse),
    /// A mouse button was released.
    MouseRelease(Mouse),
    /// The mouse wheel turned.
    MouseWheel(Mouse),
    /// The mouse moved.
    MouseMotion(Mouse),
    /// Text was pasted.
    Paste(String),
    /// A paste begins.
    PasteStart,
    /// A paste ended.
    PasteEnd,
    /// The terminal gained focus.
    Focus,
    /// The terminal lost focus.
    Blur,
    /// The cursor's position, from 0.
    CursorPosition {
        /// The column.
        x: i32,
        /// The row.
        y: i32,
    },
    /// The terminal's keyboard enhancements.
    KeyboardEnhancements(KeyboardEnhancementsReport),
    /// The terminal's reply about a mode.
    ModeReport {
        /// The mode.
        mode: ReportedMode,
        /// Its setting.
        value: ModeSetting,
    },
    /// The terminal's name and version.
    TerminalVersion(String),
    /// The terminal's foreground colour.
    ForegroundColor(Rgb),
    /// The terminal's background colour.
    BackgroundColor(Rgb),
    /// The terminal's cursor colour.
    CursorColor(Rgb),
    /// The window changed to this size.
    Resize(Size),
    /// `SIGINT`: stop with [`Error::Interrupted`](crate::Error::Interrupted).
    Interrupt,
    /// `SIGTERM`: stop as if the model had quit.
    Terminate,
}

/// Delivers input to the running program; `false` once it has stopped.
pub type InputSink = Box<dyn Fn(Input) -> bool + Send + 'static>;

/// A terminal a [`Program`](crate::Program) can run in.
pub trait Backend {
    /// Takes over the screen (raw mode, alternate screen, hidden cursor).
    ///
    /// # Errors
    ///
    /// If the terminal cannot be set up.
    fn enter(&mut self) -> io::Result<()>;

    /// Stops input and gives the screen back as it was.
    ///
    /// # Errors
    ///
    /// If the terminal cannot be restored.
    fn leave(&mut self) -> io::Result<()>;

    /// The window's size.
    ///
    /// # Errors
    ///
    /// If the size cannot be read.
    fn size(&mut self) -> io::Result<Size>;

    /// Starts delivering input to `sink`, usually from a thread, until
    /// [`Backend::leave`].
    fn start_input(&mut self, sink: InputSink);

    /// Where frames are written.
    fn output(&mut self) -> &mut dyn Write;

    /// Turns a terminal mode on or off. By default this writes the
    /// mode's [`sequence`](TerminalMode::sequence) to
    /// [`output`](Backend::output); the real terminal also records it,
    /// so it is turned off however the program ends.
    ///
    /// # Errors
    ///
    /// If the sequence cannot be written.
    fn set_mode(&mut self, mode: TerminalMode, on: bool) -> io::Result<()> {
        self.output().write_all(&mode.sequence(on))
    }
}

/// A scripted terminal for testing models: a fixed size, input sent in
/// order, and everything written captured. Input is either decoded
/// already ([`TestBackend::keys`], [`TestBackend::inputs`]) or raw bytes
/// that go through the same decoder as a real terminal's
/// ([`TestBackend::bytes`]).
///
/// ```
/// use petitcarnets::testing::TestBackend;
/// use petitcarnets::Key;
///
/// let backend = TestBackend::new(40, 10)
///     .bytes(b"\x1b[A")
///     .keys([Key::char('q')]);
/// let screen = backend.screen();
/// // ... run a Program with `backend`, then read `screen.text()`.
/// # let _ = screen;
/// ```
#[derive(Debug, Clone)]
pub struct TestBackend {
    size: Size,
    steps: Vec<Step>,
    gap: Duration,
    screen: Screen,
}

/// One scripted step.
#[derive(Debug, Clone)]
enum Step {
    Input(Input),
    Bytes(Vec<u8>),
}

/// What a [`TestBackend`] captured; it stays readable after the run.
#[derive(Debug, Clone, Default)]
pub struct Screen(Arc<Mutex<Vec<u8>>>);

impl Screen {
    /// Everything written, as text, escape sequences included.
    #[must_use]
    pub fn raw(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap_or_else(PoisonError::into_inner)).into_owned()
    }

    /// Everything written, without escape sequences.
    #[must_use]
    pub fn text(&self) -> String {
        crate::style::strip_ansi(&self.raw())
    }
}

impl Write for Screen {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TestBackend {
    /// A `columns` by `rows` terminal with no input.
    #[must_use]
    pub fn new(columns: u16, rows: u16) -> Self {
        Self {
            size: Size::new(columns, rows),
            steps: Vec::new(),
            gap: Duration::from_millis(5),
            screen: Screen::default(),
        }
    }

    /// Adds key presses, in order.
    #[must_use]
    pub fn keys(self, keys: impl IntoIterator<Item = Key>) -> Self {
        self.inputs(keys.into_iter().map(Input::KeyPress))
    }

    /// Adds input to send, in order: keys, mouse events, resizes,
    /// signals.
    #[must_use]
    pub fn inputs(mut self, inputs: impl IntoIterator<Item = Input>) -> Self {
        self.steps.extend(inputs.into_iter().map(Step::Input));
        self
    }

    /// Adds bytes as the terminal would send them, in one read. They go
    /// through the real decoder: a sequence cut off at the end waits for
    /// the next `bytes` step, unless the gap reaches the 50 ms escape
    /// timeout or another kind of step comes next, when it is read as it
    /// is.
    #[must_use]
    pub fn bytes(mut self, bytes: impl AsRef<[u8]>) -> Self {
        self.steps.push(Step::Bytes(bytes.as_ref().to_vec()));
        self
    }

    /// The pause before each input (5 ms by default), so each update's
    /// commands can run in between.
    #[must_use]
    pub const fn gap(mut self, gap: Duration) -> Self {
        self.gap = gap;
        self
    }

    /// A handle on what the program writes.
    #[must_use]
    pub fn screen(&self) -> Screen {
        self.screen.clone()
    }
}

/// Sends the input among `events`; `false` once the program has stopped.
fn send(events: Vec<Event>, sink: &InputSink) -> bool {
    crate::input::inputs(events).all(sink)
}

/// Plays `steps` into `sink`, one every `gap`.
fn play(steps: &[Step], gap: Duration, sink: &InputSink) {
    let mut reader = Reader::default();
    for (i, step) in steps.iter().enumerate() {
        std::thread::sleep(gap);
        let delivered = match step {
            Step::Input(input) => sink(input.clone()),
            Step::Bytes(bytes) => {
                let now = Instant::now();
                let more = matches!(steps.get(i + 1), Some(Step::Bytes(_))) && gap < ESC_TIMEOUT;
                send(reader.feed(bytes, now), sink) && (more || send(reader.flush(now), sink))
            }
        };
        if !delivered {
            return;
        }
    }
}

impl Backend for TestBackend {
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
        let (steps, gap) = (self.steps.clone(), self.gap);
        drop(std::thread::spawn(move || play(&steps, gap, &sink)));
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.screen
    }
}
