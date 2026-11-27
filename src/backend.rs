//! Where a program reads input and writes frames: the real terminal, or a
//! scripted one for tests.

use crate::key::Key;
use crate::msg::Size;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// What a backend reports to the running program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// A key was pressed.
    Key(Key),
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
}

/// A scripted terminal for testing models: a fixed size, input sent in
/// order, and everything written captured.
///
/// ```
/// use petitcarnets::testing::TestBackend;
/// use petitcarnets::Key;
///
/// let backend = TestBackend::new(40, 10).keys([Key::Char('q')]);
/// let screen = backend.screen();
/// // ... run a Program with `backend`, then read `screen.text()`.
/// # let _ = screen;
/// ```
#[derive(Debug, Clone)]
pub struct TestBackend {
    size: Size,
    inputs: Vec<Input>,
    gap: Duration,
    screen: Screen,
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
            inputs: Vec::new(),
            gap: Duration::from_millis(5),
            screen: Screen::default(),
        }
    }

    /// The keys to press, in order.
    #[must_use]
    pub fn keys(self, keys: impl IntoIterator<Item = Key>) -> Self {
        self.inputs(keys.into_iter().map(Input::Key))
    }

    /// The input to send, in order: keys, resizes and signals.
    #[must_use]
    pub fn inputs(mut self, inputs: impl IntoIterator<Item = Input>) -> Self {
        self.inputs = inputs.into_iter().collect();
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
        let (inputs, gap) = (self.inputs.clone(), self.gap);
        drop(std::thread::spawn(move || {
            for input in inputs {
                std::thread::sleep(gap);
                if !sink(input) {
                    break;
                }
            }
        }));
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.screen
    }
}
