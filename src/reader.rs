//! The input thread: waits in `poll` on the terminal and on a wake pipe,
//! and turns bytes and signals into [`Input`].
//!
//! The pipe carries one byte per signal (its number), a zero byte to
//! stop, and the pause and resume bytes. Waiting in `poll` rather than in
//! `read` means stopping or pausing never swallows a key meant for
//! whatever runs next, and an idle program sleeps with no timer at all:
//! the only timeout is the 50 ms escape timeout, while a lone ESC or a
//! cut off sequence waits for its end.
//!
//! Paused, the thread acknowledges, then waits on the pipe alone, so the
//! terminal's bytes stay for the program it was handed to; signals that
//! arrive meanwhile are dropped, as Bubble Tea ignores them while it has
//! released the terminal.

use crate::backend::{Input, InputSink};
use crate::input::{Reader, inputs};
use crate::sys::{self, signo};
use crate::terminal::size_of;
use std::ffi::c_int;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use std::time::Instant;

/// The byte that tells the reader to stop.
pub(crate) const STOP: u8 = 0;

/// The byte that tells the reader to stop reading the terminal until
/// [`RESUME`]; it answers on its acknowledgement channel once it has.
pub(crate) const PAUSE: u8 = 0xF0;

/// The byte that tells a paused reader to read the terminal again.
pub(crate) const RESUME: u8 = 0xF1;

/// What the reader does after a wait.
enum Next {
    /// Read on.
    Go,
    /// Pause; the bytes after the pause byte, read with it.
    Pause(Vec<u8>),
    /// Stop.
    Stop,
}

/// Starts reading `input`, and signals from `wake`, into `sink`; the
/// window size comes from `output`. Each pause is acknowledged on `ack`.
pub(crate) fn spawn(
    input: c_int,
    wake: c_int,
    output: c_int,
    sink: InputSink,
    ack: Sender<()>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = Reader::default();
        loop {
            match step(input, wake, output, &sink, &mut reader) {
                Next::Go => {}
                Next::Stop => return,
                Next::Pause(rest) => {
                    if !deliver_all(reader.flush(Instant::now()), &sink) {
                        return;
                    }
                    let _ = ack.send(());
                    if !paused(wake, &rest) {
                        return;
                    }
                }
            }
        }
    })
}

/// Waits for the terminal or the pipe and handles what is ready.
fn step(input: c_int, wake: c_int, output: c_int, sink: &InputSink, reader: &mut Reader) -> Next {
    let timeout = reader
        .deadline()
        .map(|at| at.saturating_duration_since(Instant::now()));
    let Ok([keys, signals]) = sys::wait_readable([input, wake], timeout) else {
        return Next::Stop;
    };
    if signals {
        match on_signals(wake, output, sink) {
            Next::Go => {}
            other => return other,
        }
    }
    let delivered = if keys {
        on_keys(input, reader, sink)
    } else {
        deliver_all(reader.expire(Instant::now()), sink)
    };
    if delivered { Next::Go } else { Next::Stop }
}

/// Waits on the pipe alone, dropping signals, until a resume (`true`)
/// or a stop (`false`); `first` holds bytes already read.
fn paused(wake: c_int, first: &[u8]) -> bool {
    let mut buf = [0u8; 64];
    let mut bytes = first.to_vec();
    loop {
        for byte in bytes.drain(..) {
            match byte {
                RESUME => return true,
                STOP => return false,
                _ => {}
            }
        }
        if sys::wait_readable([wake], None).is_err() {
            return false;
        }
        match sys::read_fd(wake, &mut buf) {
            Ok(0) | Err(_) => return false,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
        }
    }
}

/// Delivers decoded input; `false` once the program has stopped.
fn deliver_all(events: Vec<crate::input::Event>, sink: &InputSink) -> bool {
    inputs(events).all(sink)
}

/// Delivers the signals waiting in the pipe, up to a pause or a stop.
fn on_signals(wake: c_int, output: c_int, sink: &InputSink) -> Next {
    let mut buf = [0u8; 64];
    let n = match sys::read_fd(wake, &mut buf) {
        Ok(0) | Err(_) => return Next::Stop,
        Ok(n) => n,
    };
    for (i, &byte) in buf[..n].iter().enumerate() {
        match byte {
            PAUSE => return Next::Pause(buf[i + 1..n].to_vec()),
            STOP => return Next::Stop,
            _ if !deliver(byte, output, sink) => return Next::Stop,
            _ => {}
        }
    }
    Next::Go
}

/// One signal from the pipe; `false` once the program has stopped.
fn deliver(byte: u8, output: c_int, sink: &InputSink) -> bool {
    match c_int::from(byte) {
        signo::SIGWINCH => sink(Input::Resize(size_of(output))),
        signo::SIGINT => sink(Input::Interrupt),
        signo::SIGTERM => sink(Input::Terminate),
        signo::SIGTSTP => sink(Input::Suspend),
        signo::SIGCONT => sink(Input::Continue),
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
mod tests;
