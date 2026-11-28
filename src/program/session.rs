//! A started program: its model, its channel and its loop, run to the
//! end in one go, or a step at a time by the test harness.

use super::End;
use super::run::Loop;
use crate::Model;
use crate::backend::Backend;
use crate::clock::Clock;
use crate::exec::Event;
use crate::msg::Error;
use std::sync::mpsc::Receiver;
use std::time::Instant;

/// A program between its start and its end.
pub(crate) struct Session<M: Model, C> {
    model: M,
    inbox: Receiver<Event<M::Msg>>,
    state: Loop<M::Msg, C>,
}

impl<M: Model, C: Clock> Session<M, C> {
    /// A session for `model`, with its channel and loop.
    pub(super) const fn new(
        model: M,
        inbox: Receiver<Event<M::Msg>>,
        state: Loop<M::Msg, C>,
    ) -> Self {
        Self {
            model,
            inbox,
            state,
        }
    }

    /// Runs until something ends the program.
    pub(crate) fn run(&mut self, backend: &mut impl Backend) -> Result<End, Error> {
        self.state.run(&mut self.model, &self.inbox, backend)
    }

    /// Updates the model with everything ready and draws if the frame
    /// rate allows; how the program ended, if it did.
    pub(crate) fn step(&mut self, backend: &mut impl Backend) -> Result<Option<End>, Error> {
        self.state.step(&mut self.model, &self.inbox, backend)
    }

    /// Draws a frame the frame rate is holding back, now: for a test,
    /// whose clock stands still until the test moves it.
    pub(crate) fn show(&mut self, backend: &mut impl Backend) -> Result<(), Error> {
        self.state.show(&self.model, backend)
    }

    /// Takes in the events that are ready and fires the timers that are
    /// due, waiting as the clock waits; whether the model has something
    /// to hear.
    pub(crate) fn wake(&mut self) -> bool {
        self.state.wait(&self.inbox);
        self.state.exec.has_ready()
    }

    /// When the loop next has something to do with no event.
    pub(crate) fn next_wake(&self) -> Option<Instant> {
        self.state.next_wake()
    }

    /// The model as it is now.
    pub(crate) const fn model(&self) -> &M {
        &self.model
    }

    /// Turns every mode off; the model, and how the program ended.
    pub(crate) fn close(
        mut self,
        end: Result<End, Error>,
        backend: &mut impl Backend,
    ) -> (M, Result<(), Error>) {
        let cleared = self.state.modes.clear(backend);
        let ended = end.and_then(|end| match end {
            End::Quit => Ok(()),
            End::Interrupt => Err(Error::Interrupted),
            End::Kill => Err(Error::Killed),
            End::Panic(message) => Err(Error::Panic(message)),
        });
        (self.model, ended.and(cleared.map_err(Error::from)))
    }
}
