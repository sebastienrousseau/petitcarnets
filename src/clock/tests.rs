use super::*;
use crate::exec::{Parent, Task};
use std::sync::mpsc;

/// A task's message, by id.
fn done(id: u64) -> Event<u64> {
    Event::Done(
        Task {
            id,
            parent: Parent::Root,
        },
        id,
    )
}

/// The ids of task events, and the rest as 0.
fn ids(events: &[Event<u64>]) -> Vec<u64> {
    events.iter().map(|e| e.task().unwrap_or(0)).collect()
}

#[test]
fn a_virtual_clock_stands_still_until_moved_and_never_goes_back() {
    let clock = VirtualClock::new();
    let start = clock.now();
    assert_eq!(clock.now().at, start.at);
    clock.move_to(start.at + Duration::from_millis(30));
    assert_eq!(clock.elapsed(), Duration::from_millis(30));
    clock.move_to(start.at);
    assert_eq!(clock.elapsed(), Duration::from_millis(30), "no going back");
    let shared = clock.clone();
    shared.move_to(start.at + Duration::from_secs(1));
    assert_eq!(clock.elapsed(), Duration::from_secs(1), "clones share time");
    assert_eq!(clock.now().wall, start.wall + Duration::from_secs(1));
}

#[test]
fn the_virtual_wall_clock_starts_on_a_whole_day() {
    let wall = VirtualClock::new().now().wall;
    let since = wall.duration_since(UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(since % 86_400, 0);
    assert!(since > 0);
}

#[test]
fn a_virtual_wait_takes_what_is_there_and_never_blocks_without_tasks() {
    let (tx, rx) = mpsc::channel();
    let clock = VirtualClock::new();
    assert!(clock.wait(&rx, None, 0).is_empty());
    tx.send(Event::Send(7)).unwrap();
    tx.send(Event::Quit).unwrap();
    assert_eq!(clock.wait(&rx, None, 0).len(), 2);
    assert_eq!(clock.elapsed(), Duration::ZERO);
}

#[test]
fn a_virtual_wait_waits_for_every_task_and_orders_them_by_start() {
    let (tx, rx) = mpsc::channel();
    let clock = VirtualClock::new();
    tx.send(done(3)).unwrap();
    tx.send(Event::Send(0)).unwrap();
    let late = tx.clone();
    let worker = std::thread::spawn(move || {
        late.send(Event::Panic(2, "late".into())).unwrap();
        late.send(done(1)).unwrap();
    });
    let events = clock.wait(&rx, None, 3);
    worker.join().unwrap();
    assert_eq!(ids(&events), [0, 1, 2, 3], "other events first, then by id");
}

#[test]
fn a_virtual_wait_until_moves_the_clock_only_when_nothing_came() {
    let (tx, rx) = mpsc::channel::<Event<u64>>();
    let clock = VirtualClock::new();
    let until = clock.now().at + Duration::from_millis(500);
    tx.send(Event::Quit).unwrap();
    assert!(clock.wait_until(&rx, until).is_some());
    assert_eq!(clock.elapsed(), Duration::ZERO);
    assert!(clock.wait_until(&rx, until).is_none());
    assert_eq!(clock.elapsed(), Duration::from_millis(500));
}

#[test]
fn the_real_clock_waits_for_the_deadline_or_the_first_event() {
    let (tx, rx) = mpsc::channel::<Event<u64>>();
    let now = RealClock.now();
    assert!(
        RealClock.wait(&rx, Some(now.at), 0).is_empty(),
        "a past deadline"
    );
    assert!(RealClock.wait_until(&rx, now.at).is_none());
    tx.send(Event::Quit).unwrap();
    tx.send(Event::Kill).unwrap();
    assert_eq!(RealClock.wait(&rx, None, 0).len(), 2, "all that is ready");
    tx.send(Event::Quit).unwrap();
    assert!(RealClock.wait_until(&rx, now.at).is_some());
}
