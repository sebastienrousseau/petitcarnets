//! Time for the event loop: the real clock, or a virtual one that only a
//! test moves.
//!
//! The loop asks its clock what time it is, and waits for events through
//! it. The real clock sleeps in the channel until an event or the next
//! deadline, so an idle program never wakes. The virtual clock never
//! sleeps: it takes the events already there, waits only for tasks still
//! running on their threads, and moves forward when the test says so.

use crate::exec::Event;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The longest a virtual clock waits for a task to finish before it
/// gives up: a task that never ends would otherwise hang the test.
const TASK_TIMEOUT: Duration = Duration::from_secs(30);

/// Where the virtual system clock starts: midnight UTC, 15 November 2023,
/// a whole number of days after the Unix epoch, so [`every`](crate::every)
/// with a period of a second, minute, hour or day lands on a whole one.
const VIRTUAL_EPOCH: Duration = Duration::from_secs(19_676 * 86_400);

/// A moment, read from the monotonic clock and the system clock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Now {
    /// For deadlines.
    pub(crate) at: Instant,
    /// For the time timers report, and for [`every`](crate::every).
    pub(crate) wall: SystemTime,
}

/// What the event loop reads the time from and waits with.
pub(crate) trait Clock {
    /// The time now.
    fn now(&self) -> Now;

    /// Waits until an event arrives or `deadline` passes (forever with
    /// none), then takes every event that is ready. `tasks` tasks are
    /// running on their threads.
    fn wait<A>(
        &self,
        inbox: &Receiver<Event<A>>,
        deadline: Option<Instant>,
        tasks: usize,
    ) -> Vec<Event<A>>;

    /// Waits for one event until `until`; `None` if none came.
    fn wait_until<A>(&self, inbox: &Receiver<Event<A>>, until: Instant) -> Option<Event<A>>;
}

/// The process's own clocks.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Now {
        Now {
            at: Instant::now(),
            wall: SystemTime::now(),
        }
    }

    fn wait<A>(
        &self,
        inbox: &Receiver<Event<A>>,
        deadline: Option<Instant>,
        _tasks: usize,
    ) -> Vec<Event<A>> {
        let first = match deadline {
            None => inbox.recv().ok(),
            Some(at) => inbox
                .recv_timeout(at.saturating_duration_since(Instant::now()))
                .ok(),
        };
        first
            .into_iter()
            .chain(std::iter::from_fn(|| inbox.try_recv().ok()))
            .collect()
    }

    fn wait_until<A>(&self, inbox: &Receiver<Event<A>>, until: Instant) -> Option<Event<A>> {
        inbox
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .ok()
    }
}

/// A clock that stands still until it is moved. Clones share the time.
#[derive(Debug, Clone)]
pub(crate) struct VirtualClock {
    start: Instant,
    elapsed: Rc<Cell<Duration>>,
}

impl VirtualClock {
    /// A clock at its start.
    pub(crate) fn new() -> Self {
        Self {
            start: Instant::now(),
            elapsed: Rc::new(Cell::new(Duration::ZERO)),
        }
    }

    /// How far the clock has moved since its start.
    pub(crate) fn elapsed(&self) -> Duration {
        self.elapsed.get()
    }

    /// Moves the clock on to `at`; a time already past leaves it.
    pub(crate) fn move_to(&self, at: Instant) {
        let to = at.saturating_duration_since(self.start);
        if to > self.elapsed.get() {
            self.elapsed.set(to);
        }
    }
}

impl Clock for VirtualClock {
    fn now(&self) -> Now {
        Now {
            at: self.start + self.elapsed(),
            wall: UNIX_EPOCH + VIRTUAL_EPOCH + self.elapsed(),
        }
    }

    /// Takes the events already there, then waits for every running
    /// task, so their messages arrive in the order the tasks started,
    /// whatever order their threads end in. Time does not move.
    ///
    /// # Panics
    ///
    /// If a task is still running after 30 seconds.
    fn wait<A>(
        &self,
        inbox: &Receiver<Event<A>>,
        _deadline: Option<Instant>,
        tasks: usize,
    ) -> Vec<Event<A>> {
        let mut events: Vec<Event<A>> = std::iter::from_fn(|| inbox.try_recv().ok()).collect();
        let mut done = events.iter().filter(|e| e.task().is_some()).count();
        while done < tasks {
            let Ok(event) = inbox.recv_timeout(TASK_TIMEOUT) else {
                panic!("a task was still running after {TASK_TIMEOUT:?}");
            };
            done += usize::from(event.task().is_some());
            events.push(event);
        }
        events.sort_by_key(Event::task);
        events
    }

    /// Takes an event already there; with none, moves the clock on to
    /// `until`, as if it had waited.
    fn wait_until<A>(&self, inbox: &Receiver<Event<A>>, until: Instant) -> Option<Event<A>> {
        let event = inbox.try_recv().ok();
        if event.is_none() {
            self.move_to(until);
        }
        event
    }
}

#[cfg(test)]
mod tests;
