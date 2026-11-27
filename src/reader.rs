//! The input thread: waits in `poll` on the terminal and on a wake pipe,
//! and turns bytes and signals into [`Input`].
//!
//! The pipe carries one byte per signal (its number) and a zero byte to
//! stop. Waiting in `poll` rather than in `read` means stopping never
//! swallows a key meant for whatever runs next, and an idle program
//! sleeps with no timer at all: the only timeout is the 50 ms escape
//! timeout, while a lone ESC or a cut off sequence waits for its end.

use crate::backend::{Input, InputSink};
use crate::input::{Reader, inputs};
use crate::sys::{self, signo};
use crate::terminal::size_of;
use std::ffi::c_int;
use std::thread::JoinHandle;
use std::time::Instant;

/// The byte that tells the reader to stop.
pub(crate) const STOP: u8 = 0;

/// Starts reading `input`, and signals from `wake`, into `sink`; the
/// window size comes from `output`.
pub(crate) fn spawn(input: c_int, wake: c_int, output: c_int, sink: InputSink) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = Reader::default();
        loop {
            let timeout = reader
                .deadline()
                .map(|at| at.saturating_duration_since(Instant::now()));
            let Ok([keys, signals]) = sys::wait_readable([input, wake], timeout) else {
                return;
            };
            if signals && !on_signals(wake, output, &sink) {
                return;
            }
            let delivered = if keys {
                on_keys(input, &mut reader, &sink)
            } else {
                deliver_all(reader.expire(Instant::now()), &sink)
            };
            if !delivered {
                return;
            }
        }
    })
}

/// Delivers decoded input; `false` once the program has stopped.
fn deliver_all(events: Vec<crate::input::Event>, sink: &InputSink) -> bool {
    inputs(events).all(sink)
}

/// Delivers the signals waiting in the pipe; `false` to stop.
fn on_signals(wake: c_int, output: c_int, sink: &InputSink) -> bool {
    let mut buf = [0u8; 64];
    match sys::read_fd(wake, &mut buf) {
        Ok(0) | Err(_) => false,
        Ok(n) => buf[..n].iter().all(|&b| deliver(b, output, sink)),
    }
}

/// One byte from the pipe; `false` to stop.
fn deliver(byte: u8, output: c_int, sink: &InputSink) -> bool {
    match c_int::from(byte) {
        0 => false,
        signo::SIGWINCH => sink(Input::Resize(size_of(output))),
        signo::SIGINT => sink(Input::Interrupt),
        signo::SIGTERM => sink(Input::Terminate),
        _ => true,
    }
}

/// Reads what the terminal sent and delivers what it decodes to;
/// `false` at end of input or once the program has stopped. A sequence
/// cut off by the end of a read waits for the next, or for the escape
/// timeout.
fn on_keys(input: c_int, reader: &mut Reader, sink: &InputSink) -> bool {
    let mut buf = [0u8; 4096];
    match sys::read_fd(input, &mut buf) {
        Ok(0) | Err(_) => false,
        Ok(n) => deliver_all(reader.feed(&buf[..n], Instant::now()), sink),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{Key, KeyCode};
    use std::sync::mpsc;

    fn collect() -> (InputSink, mpsc::Receiver<Input>) {
        let (tx, rx) = mpsc::channel();
        (Box::new(move |i| tx.send(i).is_ok()), rx)
    }

    #[test]
    fn keys_and_signals_arrive_and_a_zero_byte_stops_the_reader() {
        let keys = sys::Pipe::new().unwrap();
        let wake = sys::Pipe::new().unwrap();
        let (sink, rx) = collect();
        let reader = spawn(keys.read, wake.read, -1, sink);
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
    fn a_lone_escape_arrives_after_the_timeout() {
        let keys = sys::Pipe::new().unwrap();
        let wake = sys::Pipe::new().unwrap();
        let (sink, rx) = collect();
        let reader = spawn(keys.read, wake.read, -1, sink);
        let sent = Instant::now();
        sys::write_all_fd(keys.write, b"\x1b").unwrap();
        assert_eq!(rx.recv().unwrap(), Input::KeyPress(Key::new(KeyCode::Esc)));
        assert!(sent.elapsed() >= crate::input::ESC_TIMEOUT);
        sys::write_all_fd(wake.write, &[STOP]).unwrap();
        reader.join().unwrap();
    }

    #[test]
    fn the_reader_stops_at_end_of_input() {
        let wake = sys::Pipe::new().unwrap();
        let mut keys = sys::Pipe::new().unwrap();
        let (sink, _rx) = collect();
        let reader = spawn(keys.read, wake.read, -1, sink);
        keys.close_write();
        reader.join().unwrap();
    }
}
