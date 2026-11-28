//! Components: [Bubbles](https://github.com/charmbracelet/bubbles) v2,
//! with the same names, default key bindings and behaviour, behind the
//! `bubbles` feature (on by default, no dependencies).
//!
//! | Go (`charm.land/bubbles/v2`) | Rust                                   |
//! |------------------------------|----------------------------------------|
//! | `key`                        | [`key`]: bindings and their help       |
//! | `help`                       | [`help`]: short and full help views    |
//! | `cursor`                     | [`cursor`]: a blinking text cursor     |
//! | `spinner`                    | [`spinner`]: every Bubbles spinner     |
//! | `paginator`                  | [`paginator`]: pages, dots or numbers  |
//! | `textinput`                  | [`textinput`]: one line of text        |
//! | `textarea`                   | [`textarea`]: many lines, soft wrapped |
//! | `viewport`                   | [`viewport`]: a scrolling pane         |
//! | `list`                       | [`list`]: items, filtered and paged    |
//! | `table`                      | [`table`]: columns and rows            |
//! | `progress`                   | [`progress`]: a bar, animated by spring |
//! | `timer`                      | [`timer`]: a countdown                 |
//! | `stopwatch`                  | [`stopwatch`]: a count up              |
//!
//! # How components compose
//!
//! A component is a plain struct, made with `new()` (Go's `New()`), with
//! `update(&mut self, msg: &tea::Msg<X>) -> tea::Cmd<X>` and
//! `view(&self) -> String`. Go's `Update(Msg) (Model, Cmd)` returns a
//! changed copy; here the component changes in place, which is what a
//! parent does with the copy anyway, and the message is borrowed, so a
//! parent can hand the same key to several children.
//!
//! `X` is the component's own message type, always a typed enum or
//! struct of its own (such as [`spinner::TickMsg`]), never a
//! `Box<dyn Any>`. A component with no messages of its own (such as
//! [`viewport`] and [`table`]) is generic over `X`, so it takes the
//! parent's message as it is. A parent holds its children's messages in
//! its own enum, maps a child's commands up with
//! [`Cmd::map`](crate::Cmd::map) and routes messages down with
//! [`Msg::route`](crate::Msg::route):
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::{spinner, textinput};
//!
//! enum Msg {
//!     Spinner(spinner::TickMsg),
//!     Input(textinput::Msg),
//! }
//!
//! struct Form {
//!     spinner: spinner::Model,
//!     input: textinput::Model,
//! }
//!
//! impl tea::Model for Form {
//!     type Msg = Msg;
//!
//!     fn init(&mut self) -> tea::Cmd<Msg> {
//!         tea::batch([
//!             self.spinner.tick().map(Msg::Spinner),
//!             self.input.focus().map(Msg::Input),
//!         ])
//!     }
//!
//!     fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
//!         if matches!(&msg, tea::Msg::KeyPress(k) if *k == "ctrl+c") {
//!             return tea::quit();
//!         }
//!         let mut cmds = Vec::new();
//!         if let Some(m) = msg.route(|m| match m {
//!             Msg::Spinner(t) => Some(*t),
//!             Msg::Input(_) => None,
//!         }) {
//!             cmds.push(self.spinner.update(&m).map(Msg::Spinner));
//!         }
//!         if let Some(m) = msg.route(|m| match m {
//!             Msg::Input(i) => Some(*i),
//!             Msg::Spinner(_) => None,
//!         }) {
//!             cmds.push(self.input.update(&m).map(Msg::Input));
//!         }
//!         tea::batch(cmds)
//!     }
//!
//!     fn view(&self) -> tea::View {
//!         format!("{} {}", self.spinner.view(), self.input.view()).into()
//!     }
//! }
//!
//! let form = Form { spinner: spinner::Model::new(), input: textinput::Model::new() };
//! let mut test = tea::testing::TestProgram::new(form, 30, 1);
//! test.type_text("hi");
//! assert!(test.screen().text().ends_with(" > hi"), "the spinner, then the input");
//! ```
//!
//! Components that tick (the spinner, the cursor's blink, the progress
//! bar's animation, the timer and stopwatch) carry an id, as Bubbles
//! does, so two of them in one parent each take only their own ticks.
//!
//! Text is measured with [`style::width`](crate::style::width), which
//! follows the running program's width method; no component splits a
//! grapheme cluster or a wide character.

pub mod cursor;
pub mod help;
pub mod key;
pub mod list;
pub mod paginator;
pub mod progress;
pub mod spinner;
pub mod stopwatch;
pub mod table;
pub mod textarea;
pub mod textinput;
pub mod timer;
pub mod viewport;

mod text;

use crate::Cmd;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// A component's on or off settings, a bit each, so its struct holds
/// one field rather than many booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Flags(u16);

impl Flags {
    /// Whether `bit` is on.
    const fn get(self, bit: u16) -> bool {
        self.0 & bit != 0
    }

    /// Turns `bit` on or off.
    const fn set(&mut self, bit: u16, on: bool) {
        if on {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }
}

/// A stored closure where a component's `Debug` shows it: `..`, as a
/// closure has nothing to show.
struct Opaque;

impl std::fmt::Debug for Opaque {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("..")
    }
}

/// How a component's `Debug` shows an optional closure: `Some(..)` or
/// `None`.
const fn opaque<T>(callback: Option<&T>) -> Option<Opaque> {
    match callback {
        Some(_) => Some(Opaque),
        None => None,
    }
}

/// The last id given to a component; ids start at 1.
static LAST_ID: AtomicU64 = AtomicU64::new(0);

/// A new id for a component that ticks, unique in the process.
fn next_id() -> u64 {
    LAST_ID.fetch_add(1, Ordering::Relaxed) + 1
}

/// A command that sends `msg` now, on the program's clock, without a
/// thread: a Bubbles command that returns a message at once.
fn now<M: Send + 'static>(msg: M) -> Cmd<M> {
    crate::tick(Duration::ZERO, move |_| msg)
}

#[cfg(test)]
mod tests;
