//! Where a program reads keys and writes frames: the real terminal, or a
//! scripted one for tests.

use crate::key::Key;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// Delivers a key to the running program; `false` once it has stopped.
pub type KeySink = Box<dyn Fn(Key) -> bool + Send + 'static>;

/// A terminal a [`Carnet`](crate::Carnet) can run in.
pub trait Backend {
    /// Takes over the screen (raw mode, alternate screen, hidden cursor).
    ///
    /// # Errors
    ///
    /// If the terminal cannot be set up.
    fn enter(&mut self) -> io::Result<()>;

    /// Gives the screen back as it was.
    ///
    /// # Errors
    ///
    /// If the terminal cannot be restored.
    fn leave(&mut self) -> io::Result<()>;

    /// The window's size as `(columns, rows)`.
    ///
    /// # Errors
    ///
    /// If the size cannot be read.
    fn size(&mut self) -> io::Result<(u16, u16)>;

    /// Starts delivering key presses to `sink`, usually from a thread.
    fn start_input(&mut self, sink: KeySink);

    /// Where frames are written.
    fn output(&mut self) -> &mut dyn Write;
}

/// A scripted terminal for testing models: a fixed size, keys sent in
/// order, and everything written captured.
///
/// ```
/// use petitcarnets::testing::TestBackend;
/// use petitcarnets::Key;
///
/// let backend = TestBackend::new(40, 10).keys([Key::Char('q')]);
/// let screen = backend.screen();
/// // ... run a Carnet with `backend`, then read `screen.text()`.
/// # let _ = screen;
/// ```
#[derive(Debug, Clone)]
pub struct TestBackend {
    size: (u16, u16),
    keys: Vec<Key>,
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
    /// A `columns` by `rows` terminal with no keys.
    #[must_use]
    pub fn new(columns: u16, rows: u16) -> Self {
        Self {
            size: (columns, rows),
            keys: Vec::new(),
            gap: Duration::from_millis(5),
            screen: Screen::default(),
        }
    }

    /// The keys to press, in order.
    #[must_use]
    pub fn keys(mut self, keys: impl IntoIterator<Item = Key>) -> Self {
        self.keys = keys.into_iter().collect();
        self
    }

    /// The pause between key presses (5 ms by default), so each update's
    /// commands can run between them.
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

    fn size(&mut self) -> io::Result<(u16, u16)> {
        Ok(self.size)
    }

    fn start_input(&mut self, sink: KeySink) {
        let (keys, gap) = (self.keys.clone(), self.gap);
        drop(std::thread::spawn(move || {
            for key in keys {
                std::thread::sleep(gap);
                if !sink(key) {
                    break;
                }
            }
        }));
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.screen
    }
}
