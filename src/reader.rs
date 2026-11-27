//! The input thread: waits in `poll` on the terminal and on a wake pipe,
//! and turns bytes and signals into [`Input`].
//!
//! The pipe carries one byte per signal (its number) and a zero byte to
//! stop. Waiting in `poll` rather than in `read` means stopping never
//! swallows a key meant for whatever runs next, and an idle program
//! sleeps with no timer at all.

use crate::backend::{Input, InputSink};
use crate::key::parse;
use crate::sys::{self, signo};
use crate::terminal::size_of;
use std::ffi::c_int;
use std::thread::JoinHandle;

/// The byte that tells the reader to stop.
pub(crate) const STOP: u8 = 0;

/// Starts reading `input`, and signals from `wake`, into `sink`; the
/// window size comes from `output`.
pub(crate) fn spawn(input: c_int, wake: c_int, output: c_int, sink: InputSink) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut pending = Vec::new();
        while let Ok([keys, signals]) = sys::wait_readable([input, wake]) {
            if signals && !on_signals(wake, output, &sink) {
                return;
            }
            if keys && !on_keys(input, &mut pending, &sink) {
                return;
            }
        }
    })
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

/// Reads what the terminal sent and delivers its keys; `false` at end of
/// input or once the program has stopped. A sequence cut off by the end
/// of a read waits for the next; a lone Escape is the Escape key, since a
/// terminal writes a whole sequence at once.
fn on_keys(input: c_int, pending: &mut Vec<u8>, sink: &InputSink) -> bool {
    let mut buf = [0u8; 1024];
    let n = match sys::read_fd(input, &mut buf) {
        Ok(0) | Err(_) => return false,
        Ok(n) => n,
    };
    pending.extend_from_slice(&buf[..n]);
    let (keys, used) = parse(pending, true);
    let _ = pending.drain(..used);
    keys.into_iter().all(|key| sink(Input::Key(key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::Key;
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
        assert_eq!(rx.recv().unwrap(), Input::Key(Key::Char('a')));
        assert_eq!(rx.recv().unwrap(), Input::Key(Key::Up));
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
    fn the_reader_stops_at_end_of_input() {
        let wake = sys::Pipe::new().unwrap();
        let mut keys = sys::Pipe::new().unwrap();
        let (sink, _rx) = collect();
        let reader = spawn(keys.read, wake.read, -1, sink);
        keys.close_write();
        reader.join().unwrap();
    }
}
