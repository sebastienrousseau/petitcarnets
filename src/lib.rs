//! # petitcarnets
//!
//! Terminal user interfaces in the Elm architecture, with no
//! dependencies: [Bubble Tea](https://github.com/charmbracelet/bubbletea)
//! for Rust, with the same names.
//!
//! A program is a [`Model`]: its state, an [`update`](Model::update) that
//! turns each [`Msg`] (a key, a mouse event, a paste, the window size, or
//! a message of your own) into the next state and a [`Cmd`], and a
//! [`view`](Model::view) that draws the state and says which terminal
//! modes it needs (mouse, focus reports, keyboard enhancements). A
//! [`Program`] runs it: it reads and decodes input as Bubble Tea does,
//! runs commands (work on other threads, timers) and redraws only what
//! changed.
//!
//! Import the crate as `tea`, and the code reads like its Go original:
//!
//! ```no_run
//! use petitcarnets as tea;
//!
//! struct Counter(i64);
//!
//! impl tea::Model for Counter {
//!     type Msg = ();
//!
//!     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
//!         match msg {
//!             tea::Msg::KeyPress(k) if k == "q" || k == "ctrl+c" => return tea::quit(),
//!             tea::Msg::KeyPress(k) if k == "up" || k == "+" => self.0 += 1,
//!             tea::Msg::KeyPress(k) if k == "down" || k == "-" => self.0 -= 1,
//!             _ => {}
//!         }
//!         tea::Cmd::none()
//!     }
//!
//!     fn view(&self) -> tea::View {
//!         format!("Count: {}\n\n+/- or arrows to change, q to quit", self.0).into()
//!     }
//! }
//!
//! let counter = tea::Program::new(Counter(0)).run()?;
//! println!("final count: {}", counter.0);
//! # Ok::<(), tea::Error>(())
//! ```
//!
//! *Petits carnets*: little notebooks. Linux, macOS and FreeBSD for now.

mod ansi;
mod backend;
mod base64;
mod clock;
mod cmd;
mod color;
mod environ;
mod exec;
#[cfg(unix)]
mod guard;
mod input;
mod key;
mod mode;
mod mouse;
mod msg;
mod program;
#[cfg(unix)]
mod reader;
mod renderer;
pub mod style;
#[cfg(unix)]
mod sys;
#[cfg(unix)]
mod terminal;
mod unicode;
mod view;

pub use backend::{Backend, Input, InputSink};
pub use cmd::{
    Cmd, ExecCommand, batch, clear_screen, every, exec, exec_command, interrupt, println, quit,
    raw, read_clipboard, read_primary_clipboard, release_terminal, repaint,
    request_background_color, request_capability, request_cursor_color, request_cursor_position,
    request_foreground_color, request_terminal_version, request_window_size, restore_terminal,
    sequence, set_clipboard, set_primary_clipboard, suspend, tick,
};
pub use color::{ColorProfile, Rgb};
pub use environ::Environment;
pub use key::{Key, KeyCode, KeyMod};
pub use mode::{
    KeyboardEnhancements, KeyboardEnhancementsReport, ModeSetting, ReportedMode, TerminalMode,
};
pub use mouse::{Mouse, MouseButton, MouseMode};
pub use msg::{ClipboardSelection, ColorScheme, Error, Msg, Size};
pub use program::{Program, Sender};
#[cfg(unix)]
pub use terminal::Terminal;
pub use view::{Cursor, CursorShape, ProgressBar, ProgressBarState, View};

pub mod testing;

/// A program's state and behaviour. Bubble Tea's `tea.Model`.
pub trait Model {
    /// The model's own messages, from its [`Cmd`]s and [`Sender`]s.
    type Msg: Send + 'static;

    /// A command to run at the start; none by default.
    fn init(&mut self) -> Cmd<Self::Msg> {
        Cmd::none()
    }

    /// Changes the state for one message and returns a command to run.
    fn update(&mut self, msg: Msg<Self::Msg>) -> Cmd<Self::Msg>;

    /// What to show for the current state.
    fn view(&self) -> View;
}
