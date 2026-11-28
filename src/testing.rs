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
//!
//! ```
//! use petitcarnets as tea;
//! use tea::testing::TestProgram;
//!
//! struct Echo(String);
//!
//! impl tea::Model for Echo {
//!     type Msg = ();
//!
//!     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
//!         match msg {
//!             tea::Msg::KeyPress(key) if key == "enter" => return tea::quit(),
//!             tea::Msg::KeyPress(key) => self.0.push_str(&key.text),
//!             _ => {}
//!         }
//!         tea::Cmd::none()
//!     }
//!
//!     fn view(&self) -> tea::View {
//!         format!("> {}", self.0).into()
//!     }
//! }
//!
//! let mut test = TestProgram::new(Echo(String::new()), 20, 2);
//! test.type_text("hi");
//! assert_eq!(test.screen().text(), "> hi");
//! test.press(tea::Key::new(tea::KeyCode::Enter));
//! assert_eq!(test.final_model().0, "hi");
//! ```

mod golden;
mod program;
mod terminal;
mod vt;

pub use crate::backend::{Screen, TestBackend};
pub use golden::{UPDATE_GOLDEN, assert_golden, escape, golden_path};
pub use program::TestProgram;
pub use terminal::TestTerminal;
pub use vt::{ScreenCell, Vt};
