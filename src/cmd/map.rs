//! Turning a child's commands into its parent's: Elm's `Cmd.map`.

use super::{Cmd, Handover, Inner};
use std::sync::Arc;

/// A shared function from a child's message to its parent's.
type Wrap<M, N> = Arc<dyn Fn(M) -> N + Send + Sync>;

impl<M: Send + 'static> Cmd<M> {
    /// The same command, with every message it sends passed through
    /// `wrap`: Elm's `Cmd.map`, for composing components. A child's
    /// update returns a `Cmd` of its own messages; the parent maps it
    /// into its own, usually with an enum variant.
    ///
    /// Quit, interrupt, output and terminal handovers stay as they are;
    /// a batch or sequence maps each of its commands, in the same
    /// order; a task, tick or `exec` sends `wrap` of what it would have
    /// sent.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    ///
    /// #[derive(Debug, PartialEq)]
    /// enum Child {
    ///     Tick,
    /// }
    ///
    /// #[derive(Debug, PartialEq)]
    /// enum Parent {
    ///     Child(Child),
    /// }
    ///
    /// let child: tea::Cmd<Child> = tea::tick(Duration::from_secs(1), |_| Child::Tick);
    /// let parent: tea::Cmd<Parent> = child.map(Parent::Child);
    /// assert_eq!(format!("{parent:?}"), "tick(1s, ..)");
    /// ```
    #[must_use]
    pub fn map<N: Send + 'static>(self, wrap: impl Fn(M) -> N + Send + Sync + 'static) -> Cmd<N> {
        let wrap: Wrap<M, N> = Arc::new(wrap);
        map_with(self, &wrap)
    }
}

/// [`Cmd::map`] with the function shared, so a batch's commands can
/// each hold it.
fn map_with<M: Send + 'static, N: Send + 'static>(cmd: Cmd<M>, wrap: &Wrap<M, N>) -> Cmd<N> {
    let inner = match cmd.0 {
        Inner::None => Inner::None,
        Inner::Quit => Inner::Quit,
        Inner::Interrupt => Inner::Interrupt,
        Inner::Batch(cmds) => Inner::Batch(map_all(cmds, wrap)),
        Inner::Sequence(cmds) => Inner::Sequence(map_all(cmds, wrap)),
        Inner::Output(output) => Inner::Output(output),
        Inner::Handover(handover) => Inner::Handover(map_handover(handover, wrap)),
        // Listed, not a catch-all: a new kind of command must be mapped.
        work @ (Inner::Task(_) | Inner::Tick(..) | Inner::Every(..)) => map_work(work, wrap),
    };
    Cmd(inner)
}

/// Each of `cmds` mapped.
fn map_all<M: Send + 'static, N: Send + 'static>(
    cmds: Vec<Cmd<M>>,
    wrap: &Wrap<M, N>,
) -> Vec<Cmd<N>> {
    cmds.into_iter().map(|c| map_with(c, wrap)).collect()
}

/// A task or timer, sending `wrap` of its message.
fn map_work<M: Send + 'static, N: Send + 'static>(work: Inner<M>, wrap: &Wrap<M, N>) -> Inner<N> {
    let wrap = Arc::clone(wrap);
    match work {
        Inner::Task(work) => Inner::Task(Box::new(move || wrap(work()))),
        Inner::Tick(delay, fire) => Inner::Tick(delay, Box::new(move |now| wrap(fire(now)))),
        Inner::Every(period, fire) => Inner::Every(period, Box::new(move |now| wrap(fire(now)))),
        _ => unreachable!("map_with sends only tasks and timers here"),
    }
}

/// A handover, its `exec` callback mapped.
fn map_handover<M: Send + 'static, N: Send + 'static>(
    handover: Handover<M>,
    wrap: &Wrap<M, N>,
) -> Handover<N> {
    match handover {
        Handover::Exec(done) => {
            let wrap = Arc::clone(wrap);
            Handover::Exec(Box::new(move |failed| wrap(done(failed))))
        }
        Handover::Suspend => Handover::Suspend,
        Handover::Release => Handover::Release,
        Handover::Restore => Handover::Restore,
        Handover::Continue => Handover::Continue,
    }
}
