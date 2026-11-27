//! Commands: work an update asks the runtime to do, whose results come
//! back as messages.

use std::fmt;
use std::time::{Duration, SystemTime};

/// A message made from the time a timer fired.
pub(crate) type Fire<M> = Box<dyn FnOnce(SystemTime) -> M + Send>;

/// What an update asks the runtime to do next.
///
/// A command never runs inside `update`: the runtime runs it afterwards
/// and sends any result back as a message, so `update` stays a function
/// of the model and the message. Build one with [`Cmd::none`],
/// [`Cmd::task`] or the functions [`quit`](crate::quit),
/// [`interrupt`](crate::interrupt), [`batch`](crate::batch),
/// [`sequence`](crate::sequence), [`tick`](crate::tick) and
/// [`every`](crate::every), named as in Bubble Tea.
pub struct Cmd<M>(pub(crate) Inner<M>);

/// The kinds of command.
pub(crate) enum Inner<M> {
    /// Nothing.
    None,
    /// Stop the program.
    Quit,
    /// Stop the program with [`Error::Interrupted`](crate::Error::Interrupted).
    Interrupt,
    /// Several commands at once; done when all are.
    Batch(Vec<Cmd<M>>),
    /// Several commands one after another; done when the last is.
    Sequence(Vec<Cmd<M>>),
    /// Work on its own thread; its result is the next message.
    Task(Box<dyn FnOnce() -> M + Send>),
    /// A message after a delay.
    Tick(Duration, Fire<M>),
    /// A message at the next multiple of a period on the system clock.
    Every(Duration, Fire<M>),
}

impl<M> Cmd<M> {
    /// No command: Bubble Tea's `nil`.
    #[must_use]
    pub const fn none() -> Self {
        Self(Inner::None)
    }

    /// Runs `work` on its own thread and sends its result as a message:
    /// for I/O or anything slow, so the interface stays responsive. This
    /// is what a Bubble Tea `Cmd` function body does.
    #[must_use]
    pub fn task(work: impl FnOnce() -> M + Send + 'static) -> Self {
        Self(Inner::Task(Box::new(work)))
    }

    /// Whether this is [`Cmd::none`].
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self.0, Inner::None)
    }
}

impl<M> Default for Cmd<M> {
    fn default() -> Self {
        Self::none()
    }
}

/// Stops the program; [`Program::run`](crate::Program::run) returns the
/// model. Bubble Tea's `tea.Quit`.
#[must_use]
pub const fn quit<M>() -> Cmd<M> {
    Cmd(Inner::Quit)
}

/// Stops the program with [`Error::Interrupted`](crate::Error::Interrupted),
/// as `kill -INT` does. Bubble Tea's `tea.Interrupt`.
#[must_use]
pub const fn interrupt<M>() -> Cmd<M> {
    Cmd(Inner::Interrupt)
}

/// Runs `cmds` at the same time, with no order between their messages.
/// Bubble Tea's `tea.Batch`.
#[must_use]
pub fn batch<M>(cmds: impl IntoIterator<Item = Cmd<M>>) -> Cmd<M> {
    collapse(cmds, Inner::Batch)
}

/// Runs `cmds` one after another: each starts once the one before has
/// sent its message (or, for a batch or sequence, all of its messages).
/// Bubble Tea's `tea.Sequence`.
#[must_use]
pub fn sequence<M>(cmds: impl IntoIterator<Item = Cmd<M>>) -> Cmd<M> {
    collapse(cmds, Inner::Sequence)
}

/// Sends `message(now)` once `delay` has passed: for animation, polling
/// or timeouts; return another tick from `update` to keep ticking.
/// Bubble Tea's `tea.Tick`. The timer starts when the command runs (in
/// Go, when `Tick` is called; the difference only shows if a command is
/// built long before it is returned).
#[must_use]
pub fn tick<M>(delay: Duration, message: impl FnOnce(SystemTime) -> M + Send + 'static) -> Cmd<M> {
    Cmd(Inner::Tick(delay, Box::new(message)))
}

/// Sends `message(now)` at the next whole multiple of `period` on the
/// system clock: with a period of one second, on the second. Bubble Tea's
/// `tea.Every`; return it again from `update` to keep going.
#[must_use]
pub fn every<M>(
    period: Duration,
    message: impl FnOnce(SystemTime) -> M + Send + 'static,
) -> Cmd<M> {
    Cmd(Inner::Every(period, Box::new(message)))
}

/// Drops the `none`s; no command left is `none`, one is itself.
fn collapse<M>(
    cmds: impl IntoIterator<Item = Cmd<M>>,
    many: fn(Vec<Cmd<M>>) -> Inner<M>,
) -> Cmd<M> {
    let mut cmds: Vec<Cmd<M>> = cmds.into_iter().filter(|c| !c.is_none()).collect();
    match cmds.len() {
        0 => Cmd::none(),
        1 => cmds.remove(0),
        _ => Cmd(many(cmds)),
    }
}

impl<M> fmt::Debug for Cmd<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Inner::None => f.write_str("none"),
            Inner::Quit => f.write_str("quit"),
            Inner::Interrupt => f.write_str("interrupt"),
            Inner::Batch(cmds) => f.debug_tuple("batch").field(cmds).finish(),
            Inner::Sequence(cmds) => f.debug_tuple("sequence").field(cmds).finish(),
            Inner::Task(_) => f.write_str("task(..)"),
            Inner::Tick(d, _) => write!(f, "tick({d:?}, ..)"),
            Inner::Every(d, _) => write!(f, "every({d:?}, ..)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_describe_themselves() {
        let b = batch([
            Cmd::<u8>::none(),
            quit(),
            Cmd::task(|| 1),
            sequence([interrupt(), tick(Duration::from_millis(5), |_| 2)]),
            every(Duration::from_secs(1), |_| 3),
        ]);
        assert_eq!(
            format!("{b:?}"),
            "batch([quit, task(..), sequence([interrupt, tick(5ms, ..)]), every(1s, ..)])"
        );
    }

    #[test]
    fn batch_and_sequence_drop_nones_like_bubble_tea() {
        assert!(batch::<()>([]).is_none());
        assert!(sequence([Cmd::<()>::none(), Cmd::none()]).is_none());
        assert_eq!(format!("{:?}", batch([Cmd::<()>::none(), quit()])), "quit");
        assert_eq!(
            format!("{:?}", sequence([quit::<()>(), Cmd::none()])),
            "quit"
        );
    }
}
