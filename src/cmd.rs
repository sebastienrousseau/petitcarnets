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
    /// Output for the renderer, in program order.
    Output(Output),
}

/// Output a command asks the renderer for: applied in program order,
/// between frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Output {
    /// Lines to print above an inline frame.
    Print(String),
    /// Bytes to write to the terminal as they are.
    Raw(Vec<u8>),
    /// Clear the screen before the next frame.
    ClearScreen,
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

/// Prints `text` above the program's frame, on lines of its own that
/// stay in the terminal's scrollback: Bubble Tea's `tea.Println`. It is
/// ordered with frames: the frame drawn before it stays below it. On the
/// alternate screen it prints nothing, as in Bubble Tea. Use
/// [`printf!`](crate::printf!) to format.
#[must_use]
pub fn println<M>(text: impl Into<String>) -> Cmd<M> {
    Cmd(Inner::Output(Output::Print(text.into())))
}

/// Writes `bytes` to the terminal as they are, before the next frame:
/// Bubble Tea's `tea.Raw`, for queries and sequences petitcarnets does
/// not send itself. Bytes that move the cursor or change the screen can
/// confuse the renderer.
#[must_use]
pub fn raw<M>(bytes: impl Into<Vec<u8>>) -> Cmd<M> {
    Cmd(Inner::Output(Output::Raw(bytes.into())))
}

/// Clears the screen and draws the next frame from the top left, with
/// its modes written again: Bubble Tea's `tea.ClearScreen`. Never needed
/// for ordinary redraws.
#[must_use]
pub const fn clear_screen<M>() -> Cmd<M> {
    Cmd(Inner::Output(Output::ClearScreen))
}

/// Prints a formatted line above the program's frame: Bubble Tea's
/// `tea.Printf`, a [`println`](crate::println()) of [`format!`]'s result.
///
/// ```
/// use petitcarnets as tea;
///
/// let done: tea::Cmd<()> = tea::printf!("{} packages installed", 3);
/// # let _ = done;
/// ```
#[macro_export]
macro_rules! printf {
    ($($arg:tt)*) => {
        $crate::println(::std::format!($($arg)*))
    };
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
            Inner::Output(output) => output.fmt(f),
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
