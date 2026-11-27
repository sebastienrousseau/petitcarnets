//! # petitcarnets
//!
//! Terminal user interfaces in the Elm architecture, with no
//! dependencies.
//!
//! A program is a [`Model`]: its state, an [`update`](Model::update) that
//! turns each [`Event`] (a key, a resize, or a message of your own) into
//! the next state and a [`Cmd`], and a [`view`](Model::view) that draws
//! the state as text. A [`Carnet`] runs it: it reads keys, runs commands
//! (work on other threads, timers) and redraws only what changed.
//!
//! ```no_run
//! use petitcarnets::{Carnet, Cmd, Event, Key, Model};
//!
//! struct Counter(i64);
//!
//! impl Model for Counter {
//!     type Msg = ();
//!
//!     fn update(&mut self, event: Event<()>) -> Cmd<()> {
//!         match event {
//!             Event::Key(Key::Up | Key::Char('+')) => self.0 += 1,
//!             Event::Key(Key::Down | Key::Char('-')) => self.0 -= 1,
//!             Event::Key(Key::Char('q')) => return Cmd::quit(),
//!             _ => {},
//!         }
//!         Cmd::none()
//!     }
//!
//!     fn view(&self) -> String {
//!         format!("Count: {}\n\n+/- or arrows to change, q to quit", self.0)
//!     }
//! }
//!
//! let counter = Carnet::new(Counter(0)).run()?;
//! println!("final count: {}", counter.0);
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! *Petits carnets*: little notebooks. Unix (Linux and macOS) for now.

mod backend;
mod carnet;
mod cmd;
mod key;
mod renderer;
pub mod style;
#[cfg(unix)]
mod sys;
#[cfg(unix)]
mod terminal;

pub use backend::{Backend, KeySink};
pub use carnet::Carnet;
pub use cmd::Cmd;
pub use key::{Key, parse as parse_keys};
#[cfg(unix)]
pub use terminal::Terminal;

/// Test helpers: a scripted terminal for running a model headless.
pub mod testing {
    pub use crate::backend::{Screen, TestBackend};
}

/// What a model receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<M> {
    /// A key was pressed.
    Key(Key),
    /// The window is now this size; also sent once at the start.
    Resize {
        /// Columns.
        columns: u16,
        /// Rows.
        rows: u16,
    },
    /// A message of the model's own, from a [`Cmd`].
    Msg(M),
}

/// A program's state and behaviour.
pub trait Model {
    /// The model's own messages, sent by its [`Cmd`]s.
    type Msg: Send + 'static;

    /// A command to run at the start; none by default.
    fn init(&mut self) -> Cmd<Self::Msg> {
        Cmd::none()
    }

    /// The next state for an event, and a command to run.
    fn update(&mut self, event: Event<Self::Msg>) -> Cmd<Self::Msg>;

    /// The state as text, one line per row; ANSI styles are allowed.
    fn view(&self) -> String;
}
