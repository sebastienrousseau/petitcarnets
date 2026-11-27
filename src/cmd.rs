//! Commands: work an update asks the runtime to do, whose results come
//! back as messages.

use std::fmt;
use std::time::Duration;

/// What an update asks the runtime to do next.
///
/// A command never runs inside `update`: the runtime runs it after, on
/// its own thread for a task or a tick, and sends the result back as a
/// message. `update` stays a pure function of the model and the message.
pub struct Cmd<M>(pub(crate) Inner<M>);

/// The kinds of command.
pub(crate) enum Inner<M> {
    /// Nothing.
    None,
    /// Stop the program after this update.
    Quit,
    /// Several commands, run together.
    Batch(Vec<Cmd<M>>),
    /// Work on another thread; its result is the next message.
    Task(Box<dyn FnOnce() -> M + Send>),
    /// A message after a delay.
    Tick(Duration, Box<dyn FnOnce() -> M + Send>),
}

impl<M> Cmd<M> {
    /// No command.
    #[must_use]
    pub const fn none() -> Self {
        Self(Inner::None)
    }

    /// Stop the program once this update's view is drawn.
    #[must_use]
    pub const fn quit() -> Self {
        Self(Inner::Quit)
    }

    /// Several commands at once.
    #[must_use]
    pub fn batch(cmds: impl IntoIterator<Item = Self>) -> Self {
        Self(Inner::Batch(cmds.into_iter().collect()))
    }

    /// Runs `work` on another thread and sends its result as a message:
    /// for I/O or anything slow, so the interface stays responsive.
    #[must_use]
    pub fn task(work: impl FnOnce() -> M + Send + 'static) -> Self {
        Self(Inner::Task(Box::new(work)))
    }

    /// Sends `message()` after `delay`: for animation, polling or
    /// timeouts. Return another tick from `update` to keep ticking.
    #[must_use]
    pub fn tick(delay: Duration, message: impl FnOnce() -> M + Send + 'static) -> Self {
        Self(Inner::Tick(delay, Box::new(message)))
    }

    /// Whether this is [`Cmd::none`].
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self.0, Inner::None)
    }
}

impl<M> fmt::Debug for Cmd<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Inner::None => f.write_str("Cmd::none"),
            Inner::Quit => f.write_str("Cmd::quit"),
            Inner::Batch(cmds) => f.debug_tuple("Cmd::batch").field(cmds).finish(),
            Inner::Task(_) => f.write_str("Cmd::task(..)"),
            Inner::Tick(d, _) => write!(f, "Cmd::tick({d:?}, ..)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_describe_themselves() {
        assert!(Cmd::<()>::none().is_none());
        assert!(!Cmd::<()>::quit().is_none());
        let b = Cmd::batch([
            Cmd::<u8>::none(),
            Cmd::quit(),
            Cmd::task(|| 1),
            Cmd::tick(Duration::from_millis(5), || 2),
        ]);
        assert_eq!(
            format!("{b:?}"),
            "Cmd::batch([Cmd::none, Cmd::quit, Cmd::task(..), Cmd::tick(5ms, ..)])"
        );
    }
}
