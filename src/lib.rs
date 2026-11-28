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
//! A program that should also work with a screen reader runs with
//! [`Program::with_accessible`], or in a terminal whose environment sets
//! `ACCESSIBLE`: its frames are then printed as plain lines, with no
//! redraws and no cursor movement.
//!
//! Guides, in the repository's `docs` directory: Bubble Tea's names in
//! Rust (`go-to-rust.md`), moving from the bubbletea-rs crate
//! (`migrating-from-bubbletea-rs.md`), how the crate works
//! (`architecture.md`), the Bubble Tea issues it addresses (`issues.md`),
//! and Bubble Tea's examples ported (`examples.md`).
//!
//! *Petits carnets*: little notebooks. Linux and macOS for now; the
//! FreeBSD support in the code is not yet tested.

#![deny(missing_docs)]

mod ansi;
mod backend;
mod base64;
#[cfg(feature = "bubbles")]
pub mod bubbles;
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
    sequence, set_clipboard, set_primary_clipboard, suspend, task, tick,
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::TestProgram;
///
/// struct Greeter(String);
///
/// impl tea::Model for Greeter {
///     type Msg = ();
///
///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
///         match msg {
///             tea::Msg::KeyPress(k) if k == "enter" => tea::quit(),
///             tea::Msg::KeyPress(k) => {
///                 self.0 += &k.text;
///                 tea::Cmd::none()
///             }
///             _ => tea::Cmd::none(),
///         }
///     }
///
///     fn view(&self) -> tea::View {
///         format!("Hello, {}!", self.0).into()
///     }
/// }
///
/// let mut test = TestProgram::new(Greeter(String::new()), 20, 2);
/// test.type_text("Ada");
/// assert_eq!(test.screen().text(), "Hello, Ada!");
/// ```
pub trait Model {
    /// The model's own messages, from its [`Cmd`]s and [`Sender`]s.
    type Msg: Send + 'static;

    /// A command to run at the start; none by default.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::testing::TestProgram;
    ///
    /// struct Timeout;
    ///
    /// impl tea::Model for Timeout {
    ///     type Msg = ();
    ///     fn init(&mut self) -> tea::Cmd<()> {
    ///         tea::tick(Duration::from_secs(2), |_| ())
    ///     }
    ///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
    ///         match msg {
    ///             tea::Msg::App(()) => tea::quit(),
    ///             _ => tea::Cmd::none(),
    ///         }
    ///     }
    ///     fn view(&self) -> tea::View {
    ///         "waiting".into()
    ///     }
    /// }
    ///
    /// let mut test = TestProgram::new(Timeout, 10, 1);
    /// assert!(test.wait_finished(Duration::from_secs(3)));
    /// assert_eq!(test.elapsed(), Duration::from_secs(2));
    /// ```
    fn init(&mut self) -> Cmd<Self::Msg> {
        Cmd::none()
    }

    /// Changes the state for one message and returns a command to run.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::Model as _;
    ///
    /// struct Clicks(u32);
    ///
    /// impl tea::Model for Clicks {
    ///     type Msg = ();
    ///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
    ///         if let tea::Msg::App(()) = msg {
    ///             self.0 += 1;
    ///         }
    ///         tea::Cmd::none()
    ///     }
    ///     fn view(&self) -> tea::View {
    ///         self.0.to_string().into()
    ///     }
    /// }
    ///
    /// let mut clicks = Clicks(0);
    /// assert!(clicks.update(tea::Msg::App(())).is_none());
    /// assert_eq!(clicks.0, 1);
    /// ```
    fn update(&mut self, msg: Msg<Self::Msg>) -> Cmd<Self::Msg>;

    /// What to show for the current state.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::Model as _;
    ///
    /// struct Fullscreen;
    ///
    /// impl tea::Model for Fullscreen {
    ///     type Msg = ();
    ///     fn update(&mut self, _: tea::Msg<()>) -> tea::Cmd<()> {
    ///         tea::Cmd::none()
    ///     }
    ///     fn view(&self) -> tea::View {
    ///         tea::View::new("full window").with_alt_screen(true)
    ///     }
    /// }
    ///
    /// let view = Fullscreen.view();
    /// assert_eq!(view.content, "full window");
    /// assert!(view.alt_screen);
    /// ```
    fn view(&self) -> View;
}
