//! Running commands on the event loop's thread.
//!
//! Timers live on a deadline heap whose earliest entry sets how long the
//! loop may sleep, so a thousand ticks cost no threads. Only
//! [`Cmd::task`] work runs on a thread of its own. Batches and sequences
//! are bookkeeping: each command started under one reports back when it
//! is done, and a sequence starts its next command only then.

use crate::backend::Input;
use crate::clock::Now;
use crate::cmd::{Cmd, Fire, Handover, Inner, Output};
use crate::msg::Msg;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// What arrives on the event loop's channel.
pub(crate) enum Event<M> {
    /// From the terminal.
    Input(Input),
    /// A task finished with this message.
    Done(Task, M),
    /// A task, by id, panicked with this message.
    Panic(u64, String),
    /// From a [`Sender`](crate::Sender).
    Send(M),
    /// From [`Sender::quit`](crate::Sender::quit).
    Quit,
    /// From [`Sender::kill`](crate::Sender::kill).
    Kill,
}

/// What the loop does next, in order.
#[derive(Debug)]
pub(crate) enum Item<M> {
    /// Update the model.
    Msg(Msg<M>),
    /// Stop with the model.
    Quit,
    /// Stop with [`Error::Interrupted`](crate::Error::Interrupted).
    Interrupt,
    /// Stop with [`Error::Killed`](crate::Error::Killed).
    Kill,
    /// Stop with [`Error::Panic`](crate::Error::Panic).
    Panic(String),
    /// Output for the renderer.
    Output(Output),
    /// Hand the terminal over or take it back.
    Handover(Handover<M>),
}

impl<M> Event<M> {
    /// The id of the task this event comes from, if it comes from one.
    pub(crate) const fn task(&self) -> Option<u64> {
        match self {
            Self::Done(task, _) => Some(task.id),
            Self::Panic(id, _) => Some(*id),
            _ => None,
        }
    }
}

/// A task on its thread: its id, in the order tasks start, and who to
/// tell when it is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Task {
    /// Its id.
    pub(crate) id: u64,
    /// Who to tell.
    pub(crate) parent: Parent,
}

/// Who to tell when a command is done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Parent {
    /// Nobody: it came straight from `init` or `update`.
    Root,
    /// A batch, by id.
    Batch(u64),
    /// A sequence, by id.
    Sequence(u64),
}

/// A pending timer.
struct Timer<M> {
    at: Instant,
    id: u64,
    fire: Fire<M>,
    parent: Parent,
}

impl<M> PartialEq for Timer<M> {
    fn eq(&self, other: &Self) -> bool {
        (self.at, self.id) == (other.at, other.id)
    }
}
impl<M> Eq for Timer<M> {}
impl<M> PartialOrd for Timer<M> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<M> Ord for Timer<M> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.at, self.id).cmp(&(other.at, other.id))
    }
}

/// The running commands of one program.
pub(crate) struct Executor<M> {
    ready: VecDeque<Item<M>>,
    done: VecDeque<Parent>,
    batches: HashMap<u64, (usize, Parent)>,
    sequences: HashMap<u64, (VecDeque<Cmd<M>>, Parent)>,
    timers: BinaryHeap<Reverse<Timer<M>>>,
    next_id: u64,
    /// Tasks running on their threads.
    tasks: usize,
    events: Sender<Event<M>>,
}

impl<M: Send + 'static> Executor<M> {
    /// An executor whose tasks report on `events`.
    pub(crate) fn new(events: Sender<Event<M>>) -> Self {
        Self {
            ready: VecDeque::new(),
            done: VecDeque::new(),
            batches: HashMap::new(),
            sequences: HashMap::new(),
            timers: BinaryHeap::new(),
            next_id: 0,
            tasks: 0,
            events,
        }
    }

    /// The next thing for the loop to do, in order.
    pub(crate) fn pop(&mut self) -> Option<Item<M>> {
        self.ready.pop_front()
    }

    /// Whether anything is waiting for the loop.
    pub(crate) fn has_ready(&self) -> bool {
        !self.ready.is_empty()
    }

    /// How many tasks are running on their threads.
    pub(crate) const fn tasks(&self) -> usize {
        self.tasks
    }

    /// Queues an item behind those already waiting.
    pub(crate) fn push(&mut self, item: Item<M>) {
        self.ready.push_back(item);
    }

    /// Queues an item ahead of those already waiting.
    pub(crate) fn push_front(&mut self, item: Item<M>) {
        self.ready.push_front(item);
    }

    /// Starts `cmd` from `init` or `update`.
    pub(crate) fn run(&mut self, cmd: Cmd<M>, now: Now) {
        self.start(cmd, Parent::Root, now);
        self.settle(now);
    }

    /// `task` finished with `msg`.
    pub(crate) fn task_done(&mut self, task: Task, msg: M, now: Now) {
        self.tasks = self.tasks.saturating_sub(1);
        self.finish(task.parent, Item::Msg(Msg::App(msg)));
        self.settle(now);
    }

    /// A task panicked: it is no longer running.
    pub(crate) fn task_panicked(&mut self) {
        self.tasks = self.tasks.saturating_sub(1);
    }

    /// When the earliest timer is due, if there is one.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.timers.peek().map(|Reverse(t)| t.at)
    }

    /// How many timers are waiting.
    #[cfg(test)]
    pub(crate) fn timers(&self) -> usize {
        self.timers.len()
    }

    /// Fires every timer due by `now`, earliest first.
    pub(crate) fn fire_due(&mut self, now: Now) {
        while self.next_deadline().is_some_and(|at| at <= now.at) {
            if let Some(Reverse(timer)) = self.timers.pop() {
                let msg = (timer.fire)(now.wall);
                self.finish(timer.parent, Item::Msg(Msg::App(msg)));
            }
        }
        self.settle(now);
    }

    /// Starts one command under `parent`.
    fn start(&mut self, cmd: Cmd<M>, parent: Parent, now: Now) {
        match cmd.0 {
            Inner::Batch(cmds) => self.start_batch(cmds, parent, now),
            Inner::Sequence(cmds) => self.start_sequence(cmds, parent),
            Inner::Task(work) => self.spawn(work, parent),
            Inner::Tick(delay, fire) => self.add_timer(now.at + delay, fire, parent),
            Inner::Every(period, fire) => {
                self.add_timer(now.at + until_next(period, now.wall), fire, parent);
            }
            other => match queued(other) {
                Some(item) => self.finish(parent, item),
                None => self.done.push_back(parent),
            },
        }
    }

    /// Queues `item`, then reports `parent`'s child done.
    fn finish(&mut self, parent: Parent, item: Item<M>) {
        self.ready.push_back(item);
        self.done.push_back(parent);
    }

    fn start_batch(&mut self, cmds: Vec<Cmd<M>>, parent: Parent, now: Now) {
        if cmds.is_empty() {
            self.done.push_back(parent);
            return;
        }
        let id = self.id();
        let _ = self.batches.insert(id, (cmds.len(), parent));
        for cmd in cmds {
            self.start(cmd, Parent::Batch(id), now);
        }
    }

    /// Registers the sequence and reports it "done" once, which starts
    /// its first command.
    fn start_sequence(&mut self, cmds: Vec<Cmd<M>>, parent: Parent) {
        let id = self.id();
        let _ = self.sequences.insert(id, (cmds.into(), parent));
        self.done.push_back(Parent::Sequence(id));
    }

    fn spawn(&mut self, work: Box<dyn FnOnce() -> M + Send>, parent: Parent) {
        let task = Task {
            id: self.id(),
            parent,
        };
        self.tasks += 1;
        let events = self.events.clone();
        drop(std::thread::spawn(move || {
            let event = match catch_unwind(AssertUnwindSafe(work)) {
                Ok(msg) => Event::Done(task, msg),
                Err(panic) => Event::Panic(task.id, panic_message(panic.as_ref())),
            };
            let _ = events.send(event);
        }));
    }

    fn add_timer(&mut self, at: Instant, fire: Fire<M>, parent: Parent) {
        let id = self.id();
        self.timers.push(Reverse(Timer {
            at,
            id,
            fire,
            parent,
        }));
    }

    /// Moves batches and sequences on until nothing more finishes now.
    fn settle(&mut self, now: Now) {
        while let Some(parent) = self.done.pop_front() {
            match parent {
                Parent::Root => {}
                Parent::Batch(id) => self.batch_child_done(id),
                Parent::Sequence(id) => self.sequence_step(id, now),
            }
        }
    }

    fn batch_child_done(&mut self, id: u64) {
        let Some((left, parent)) = self.batches.get_mut(&id) else {
            return;
        };
        *left -= 1;
        if *left == 0 {
            let parent = *parent;
            let _ = self.batches.remove(&id);
            self.done.push_back(parent);
        }
    }

    /// Starts the sequence's next command, or reports it done.
    fn sequence_step(&mut self, id: u64, now: Now) {
        let Some((cmds, parent)) = self.sequences.get_mut(&id) else {
            return;
        };
        if let Some(next) = cmds.pop_front() {
            self.start(next, Parent::Sequence(id), now);
        } else {
            let parent = *parent;
            let _ = self.sequences.remove(&id);
            self.done.push_back(parent);
        }
    }

    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

/// What a command that only queues something queues: nothing for
/// [`Cmd::none`].
fn queued<M>(inner: Inner<M>) -> Option<Item<M>> {
    match inner {
        Inner::Quit => Some(Item::Quit),
        Inner::Interrupt => Some(Item::Interrupt),
        Inner::Output(output) => Some(Item::Output(output)),
        Inner::Handover(handover) => Some(Item::Handover(handover)),
        _ => None,
    }
}

/// How long from `now` until the next whole multiple of `period` since
/// the Unix epoch; a zero period is due at once.
fn until_next(period: Duration, now: SystemTime) -> Duration {
    let since = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let period_ns = period.as_nanos();
    if period_ns == 0 {
        return Duration::ZERO;
    }
    let left = period_ns - since.as_nanos() % period_ns;
    Duration::from_nanos(u64::try_from(left).unwrap_or(u64::MAX))
}

/// The text of a panic's payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with no message".to_string())
}

#[cfg(test)]
mod tests;
