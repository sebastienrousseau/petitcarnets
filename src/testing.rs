//! Testing models headless.
//!
//! [`TestProgram`] runs a model in the real runtime on a virtual clock,
//! in an emulated terminal: send it keys, bytes, messages, resizes and
//! signals, move time on, and read the screen a terminal would show
//! ([`Vt`]), the bytes written, or the model. Every run is the same:
//! timers, the frame rate and the escape timeout follow the virtual
//! clock, and task results arrive in the order the tasks started. It is
//! the counterpart of Bubble Tea's teatest, with
//! [`assert_golden`] for golden files.
//!
//! [`TestBackend`] is the simpler, older way: a [`Backend`](crate::Backend)
//! that plays scripted input on the real clock, for
//! [`Program::run_with`](crate::Program::run_with).

mod golden;
mod program;
mod terminal;
mod vt;

pub use crate::backend::{Screen, TestBackend};
pub use golden::{UPDATE_GOLDEN, assert_golden, escape, golden_path};
pub use program::TestProgram;
pub use terminal::TestTerminal;
pub use vt::{ScreenCell, Vt};
