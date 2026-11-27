use super::*;
use crate::cmd::{batch, quit, sequence, tick};
use std::sync::mpsc::{self, Receiver};

fn executor() -> (Executor<&'static str>, Receiver<Event<&'static str>>) {
    let (tx, rx) = mpsc::channel();
    (Executor::new(tx), rx)
}

/// Everything ready, written short: app messages by name, `quit`, `interrupt`.
fn drain(ex: &mut Executor<&'static str>) -> Vec<&'static str> {
    std::iter::from_fn(|| ex.pop())
        .map(|item| match item {
            Item::Msg(Msg::App(m)) => m,
            Item::Quit => "quit",
            Item::Interrupt => "interrupt",
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

/// No messages.
const NOTHING: [&str; 0] = [];

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn a_sequence_waits_for_each_command_even_a_faster_one_behind() {
    let (mut ex, _rx) = executor();
    let t0 = Instant::now();
    ex.run(sequence([tick(ms(10), |_| "a"), tick(ms(1), |_| "b")]), t0);
    assert_eq!(ex.timers(), 1, "only the first tick is running");
    ex.fire_due(t0 + ms(5));
    assert_eq!(drain(&mut ex), NOTHING);
    ex.fire_due(t0 + ms(10));
    assert_eq!(drain(&mut ex), ["a"]);
    ex.fire_due(t0 + ms(11));
    assert_eq!(drain(&mut ex), ["b"]);
}

#[test]
fn a_batch_inside_a_sequence_finishes_whole_before_the_next_step() {
    let (mut ex, _rx) = executor();
    let t0 = Instant::now();
    let inner = batch([tick(ms(5), |_| "slow"), tick(ms(1), |_| "fast")]);
    ex.run(sequence([inner, tick(ms(1), |_| "after")]), t0);
    ex.fire_due(t0 + ms(1));
    assert_eq!(drain(&mut ex), ["fast"]);
    ex.fire_due(t0 + ms(5));
    assert_eq!(drain(&mut ex), ["slow"]);
    ex.fire_due(t0 + ms(6));
    assert_eq!(drain(&mut ex), ["after"]);
}

#[test]
fn nested_sequences_run_in_place() {
    let (mut ex, _rx) = executor();
    let t0 = Instant::now();
    let cmd = sequence([
        tick(ms(1), |_| "1"),
        sequence([tick(ms(1), |_| "2"), tick(ms(1), |_| "3")]),
        tick(ms(1), |_| "4"),
    ]);
    ex.run(cmd, t0);
    let mut seen = Vec::new();
    for step in 1..=4 {
        ex.fire_due(t0 + ms(step));
        seen.extend(drain(&mut ex));
    }
    assert_eq!(seen, ["1", "2", "3", "4"]);
}

#[test]
fn quit_in_a_sequence_comes_after_the_messages_before_it() {
    let (mut ex, _rx) = executor();
    let t0 = Instant::now();
    ex.run(sequence([tick(ms(1), |_| "last words"), quit()]), t0);
    assert_eq!(drain(&mut ex), NOTHING, "quit waits for the tick");
    ex.fire_due(t0 + ms(1));
    assert_eq!(drain(&mut ex), ["last words", "quit"]);
}

#[test]
fn a_batch_starts_everything_at_once() {
    let (mut ex, _rx) = executor();
    let t0 = Instant::now();
    ex.run(
        batch([tick(ms(2), |_| "b"), tick(ms(1), |_| "a"), quit()]),
        t0,
    );
    assert_eq!(drain(&mut ex), ["quit"]);
    assert_eq!(ex.timers(), 2);
    ex.fire_due(t0 + ms(2));
    assert_eq!(drain(&mut ex), ["a", "b"], "earliest deadline first");
}

#[test]
fn a_thousand_ticks_are_heap_entries_not_threads() {
    let (mut ex, _rx) = executor();
    let t0 = Instant::now();
    ex.run(batch((0..1000).map(|i| tick(ms(i % 7), |_| "t"))), t0);
    assert_eq!(ex.timers(), 1000);
    assert_eq!(ex.next_deadline(), Some(t0));
    ex.fire_due(t0 + ms(7));
    assert_eq!(drain(&mut ex).len(), 1000);
    assert_eq!(ex.next_deadline(), None);
}

#[test]
fn a_long_sequence_of_instant_commands_does_not_recurse() {
    let (mut ex, _rx) = executor();
    let cmds: Vec<Cmd<&str>> = (0..100_000).map(|_| Cmd::none()).collect();
    ex.run(Cmd(Inner::Sequence(cmds)), Instant::now());
    assert_eq!(drain(&mut ex), NOTHING);
    assert!(ex.sequences.is_empty() && ex.batches.is_empty());
}

#[test]
fn a_task_reports_its_message_and_moves_its_sequence_on() {
    let (mut ex, rx) = executor();
    let t0 = Instant::now();
    ex.run(sequence([Cmd::task(|| "worked"), quit()]), t0);
    match rx.recv().unwrap() {
        Event::Done(parent, msg) => ex.task_done(parent, msg, t0),
        _ => panic!("expected the task's message"),
    }
    assert_eq!(drain(&mut ex), ["worked", "quit"]);
}

#[test]
fn a_panicking_task_reports_the_panic() {
    let (mut ex, rx) = executor();
    ex.run(Cmd::task(|| panic!("boom")), Instant::now());
    match rx.recv().unwrap() {
        Event::Panic(message) => assert_eq!(message, "boom"),
        _ => panic!("expected the panic"),
    }
}

#[test]
fn every_lands_on_the_next_multiple_of_its_period() {
    let at = |ms: u64| UNIX_EPOCH + Duration::from_millis(ms);
    assert_eq!(until_next(Duration::from_secs(1), at(1_250)), ms(750));
    assert_eq!(until_next(Duration::from_secs(1), at(2_000)), ms(1_000));
    assert_eq!(until_next(Duration::ZERO, at(5)), Duration::ZERO);
}

#[test]
fn panic_messages_are_read_from_either_payload() {
    assert_eq!(panic_message(&"static"), "static");
    assert_eq!(panic_message(&String::from("owned")), "owned");
    assert_eq!(panic_message(&42), "a panic with no message");
}
