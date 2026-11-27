//! The input thread: keys, signals, pausing and stopping.

use super::*;
use crate::key::{Key, KeyCode};
use std::sync::mpsc;

fn collect() -> (InputSink, mpsc::Receiver<Input>) {
    let (tx, rx) = mpsc::channel();
    (Box::new(move |i| tx.send(i).is_ok()), rx)
}

#[test]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
fn keys_and_signals_arrive_and_a_zero_byte_stops_the_reader() {
    let keys = sys::Pipe::new().unwrap();
    let wake = sys::Pipe::new().unwrap();
    let (sink, rx) = collect();
    let reader = spawn(keys.read, wake.read, -1, sink, mpsc::channel().0);
    sys::write_all_fd(keys.write, b"a\x1b[A").unwrap();
    assert_eq!(rx.recv().unwrap(), Input::KeyPress(Key::char('a')));
    assert_eq!(rx.recv().unwrap(), Input::KeyPress(Key::new(KeyCode::Up)));
    let sigint = u8::try_from(signo::SIGINT).unwrap();
    sys::write_all_fd(wake.write, &[sigint]).unwrap();
    assert_eq!(rx.recv().unwrap(), Input::Interrupt);
    let sigwinch = u8::try_from(signo::SIGWINCH).unwrap();
    sys::write_all_fd(wake.write, &[sigwinch, STOP]).unwrap();
    assert!(matches!(rx.recv().unwrap(), Input::Resize(_)));
    reader.join().unwrap();
    assert!(rx.try_recv().is_err(), "nothing after the stop byte");
}

#[test]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
fn a_lone_escape_arrives_after_the_timeout() {
    let keys = sys::Pipe::new().unwrap();
    let wake = sys::Pipe::new().unwrap();
    let (sink, rx) = collect();
    let reader = spawn(keys.read, wake.read, -1, sink, mpsc::channel().0);
    let sent = Instant::now();
    sys::write_all_fd(keys.write, b"\x1b").unwrap();
    assert_eq!(rx.recv().unwrap(), Input::KeyPress(Key::new(KeyCode::Esc)));
    assert!(sent.elapsed() >= crate::input::ESC_TIMEOUT);
    sys::write_all_fd(wake.write, &[STOP]).unwrap();
    reader.join().unwrap();
}

#[test]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
fn the_reader_stops_at_end_of_input() {
    let wake = sys::Pipe::new().unwrap();
    let mut keys = sys::Pipe::new().unwrap();
    let (sink, _rx) = collect();
    let reader = spawn(keys.read, wake.read, -1, sink, mpsc::channel().0);
    keys.close_write();
    reader.join().unwrap();
}

#[test]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
fn a_paused_reader_leaves_the_terminal_alone_until_it_resumes() {
    let keys = sys::Pipe::new().unwrap();
    let wake = sys::Pipe::new().unwrap();
    let (sink, rx) = collect();
    let (ack, acked) = mpsc::channel();
    let reader = spawn(keys.read, wake.read, -1, sink, ack);
    sys::write_all_fd(wake.write, &[PAUSE]).unwrap();
    acked
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("the reader acknowledges a pause");
    sys::write_all_fd(keys.write, b"x").unwrap();
    let sigint = u8::try_from(signo::SIGINT).unwrap();
    sys::write_all_fd(wake.write, &[sigint]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert!(
        rx.try_recv().is_err(),
        "nothing read or delivered while paused"
    );
    assert_eq!(
        sys::wait_readable([keys.read], Some(std::time::Duration::ZERO)).unwrap(),
        [true]
    );
    sys::write_all_fd(wake.write, &[RESUME]).unwrap();
    assert_eq!(
        rx.recv().unwrap(),
        Input::KeyPress(Key::char('x')),
        "the signal was dropped"
    );
    sys::write_all_fd(wake.write, &[PAUSE, STOP]).unwrap();
    acked
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("the reader acknowledges a pause");
    reader.join().unwrap();
}

#[test]
#[cfg_attr(miri, ignore)] // calls libc through src/sys.rs
fn stop_and_continue_signals_arrive_as_input() {
    let keys = sys::Pipe::new().unwrap();
    let wake = sys::Pipe::new().unwrap();
    let (sink, rx) = collect();
    let reader = spawn(keys.read, wake.read, -1, sink, mpsc::channel().0);
    let tstp = u8::try_from(signo::SIGTSTP).unwrap();
    let cont = u8::try_from(signo::SIGCONT).unwrap();
    sys::write_all_fd(wake.write, &[tstp, cont, STOP]).unwrap();
    assert_eq!(rx.recv().unwrap(), Input::Suspend);
    assert_eq!(rx.recv().unwrap(), Input::Continue);
    reader.join().unwrap();
}
