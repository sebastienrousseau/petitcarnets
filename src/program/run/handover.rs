//! Handing the terminal to another program and taking it back: for
//! [`exec`](crate::exec), [`suspend`](crate::suspend), a release and
//! restore, and a stop from outside. Also reading the replies to queries
//! before the terminal goes to anyone else, so they never reach a child
//! or the shell (Bubble Tea issue #1590).

use super::Loop;
use crate::Model;
use crate::backend::{Backend, Input};
use crate::clock::Clock;
use crate::cmd::{Handoff, Handover};
use crate::exec::{Event, Item};
use crate::msg::{Error, Msg};
use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Asks for the primary device attributes: every terminal answers, and
/// answers in order, so once this reply is in, every earlier one is.
/// Bubble Tea's sentinel for the same wait.
const SENTINEL: &[u8] = b"\x1b[c";

/// Replies to queries this young are waited for before the terminal is
/// handed over; Bubble Tea's `queryReplyWindow`.
const REPLY_WINDOW: Duration = Duration::from_secs(1);

/// The longest wait for those replies; Bubble Tea's `queryDrainTimeout`.
const DRAIN_TIMEOUT: Duration = Duration::from_millis(500);

impl<A: Send + 'static, C: Clock> Loop<A, C> {
    /// Hands the terminal over or takes it back, as `handover` says.
    pub(super) fn lifecycle<M: Model<Msg = A>>(
        &mut self,
        handover: Handover<A>,
        model: &M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match handover {
            Handover::Exec(work) => self.exec_child(work, inbox, backend),
            Handover::Suspend => self.suspend(inbox, backend),
            other => self.switch(&other, model, inbox, backend),
        }
    }

    /// A release, a restore, or a continue after a stop from outside;
    /// each does nothing if the terminal is already as it asks.
    fn switch<M: Model<Msg = A>>(
        &mut self,
        handover: &Handover<A>,
        model: &M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match (handover, self.released) {
            (Handover::Release, false) => {
                self.hand_over(inbox, backend)?;
                self.released = true;
            }
            (Handover::Restore, true) => self.restore(model, backend)?,
            (Handover::Continue, false) => {
                self.hand_over(inbox, backend)?;
                self.take_back(backend)?;
                self.exec.push_front(Item::Msg(Msg::Resume));
            }
            _ => {}
        }
        Ok(())
    }

    /// Runs `work` with the terminal handed over; its message is the
    /// next the model sees.
    fn exec_child(
        &mut self,
        work: Handoff<A>,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        if self.released {
            let msg = work(None);
            self.exec.push_front(Item::Msg(Msg::App(msg)));
            return Ok(());
        }
        let failed = self.hand_over(inbox, backend).err();
        let msg = work(failed);
        self.take_back(backend)?;
        self.exec.push_front(Item::Msg(Msg::App(msg)));
        Ok(())
    }

    /// Hands the terminal over, stops until continued, takes it back and
    /// tells the model.
    fn suspend(
        &mut self,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        let handed = self.released || self.hand_over(inbox, backend).is_ok();
        let stopped = if handed { backend.suspend() } else { Ok(()) };
        if !self.released {
            self.take_back(backend)?;
        }
        stopped?;
        self.exec.push_front(Item::Msg(Msg::Resume));
        Ok(())
    }

    /// Takes the terminal back after a release, then writes the output
    /// that waited.
    pub(super) fn restore<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        self.take_back(backend)?;
        self.released = false;
        for output in std::mem::take(&mut self.held_back) {
            self.output(model, output, backend)?;
        }
        Ok(())
    }

    /// Hands the terminal over: replies to queries read first, an inline
    /// frame erased, every mode off, then the backend's release (input
    /// paused, the terminal's own mode back).
    fn hand_over(
        &mut self,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> io::Result<()> {
        self.drain(inbox, backend, true)?;
        self.renderer.release(backend.output())?;
        self.modes.release(backend)?;
        backend.release()
    }

    /// Takes the terminal back: the backend's restore (raw mode, input
    /// read again), every mode on again, and the whole frame drawn at
    /// once. A size that changed meanwhile is sent to the model.
    pub(super) fn take_back(&mut self, backend: &mut impl Backend) -> Result<(), Error> {
        backend.restore()?;
        self.modes.resume(backend)?;
        let size = backend.size()?;
        if size != self.renderer.size() {
            self.exec.push(Item::Msg(Msg::WindowSize(size)));
        }
        self.dirty = true;
        self.urgent = true;
        Ok(())
    }

    /// Reads replies to queries written in the last second, until the
    /// terminal answers a sentinel query or half a second passes. Events
    /// that arrive meanwhile are queued (`keep`) or, at the end, dropped.
    /// Only a real terminal is waited for.
    pub(super) fn drain(
        &mut self,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
        keep: bool,
    ) -> io::Result<()> {
        let Some(asked) = self.queried.take() else {
            return Ok(());
        };
        let now = self.clock.now().at;
        if !self.settings.terminal || now.saturating_duration_since(asked) > REPLY_WINDOW {
            return Ok(());
        }
        let out = backend.output();
        out.write_all(SENTINEL)?;
        out.flush()?;
        let until = now + DRAIN_TIMEOUT;
        loop {
            match self.clock.wait_until(inbox, until) {
                Some(Event::Input(Input::PrimaryDeviceAttributes(_))) | None => return Ok(()),
                Some(event) if keep => self.take(event),
                Some(_) => {}
            }
        }
    }
}
