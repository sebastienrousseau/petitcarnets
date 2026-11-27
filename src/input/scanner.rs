//! What ultraviolet's `TerminalReader` adds to the decoder: looking a
//! whole read up in the key table, waiting for cut off sequences,
//! flushing them once [`ESC_TIMEOUT`] passes with no new bytes, and
//! gathering a bracketed paste into one message.

use super::{ESC, Event, decode, table};
use crate::backend::Input;
use crate::key::Key;
use std::time::{Duration, Instant};

/// How long a lone ESC or a cut off sequence waits for more bytes before
/// it is read as it is. Go's `DefaultEscTimeout`.
pub(crate) const ESC_TIMEOUT: Duration = Duration::from_millis(50);

/// What the scan does after one event.
enum Flow {
    /// Go on after the event's bytes.
    Next,
    /// Stop before them and wait for more input.
    Wait,
    /// Stop after them.
    Stop,
}

/// Go's `eventScanner`: decodes a buffer, holding back what may be cut
/// off, and keeps a bracketed paste open across buffers.
#[derive(Debug, Default)]
pub(crate) struct Scanner {
    paste: Option<Vec<u8>>,
}

impl Scanner {
    /// Decodes events from `buf` into `events` and returns how many bytes
    /// were used; the rest should be given again with more bytes, or
    /// with `expired` once the escape timeout has passed. Go's
    /// `scanEvents`.
    pub(crate) fn scan(&mut self, buf: &[u8], expired: bool, events: &mut Vec<Event>) -> usize {
        if let Some(key) = whole_read_key(buf) {
            events.push(Event::Input(Input::KeyPress(key)));
            return buf.len();
        }
        let mut total = 0;
        while total < buf.len() {
            let (n, flow) = self.step(&buf[total..], expired, events);
            match flow {
                Flow::Next => total += n,
                Flow::Wait => return total,
                Flow::Stop => return total + n,
            }
        }
        total
    }

    /// Decodes the event at the start of `rest`: its length, and what to
    /// do next.
    fn step(&mut self, rest: &[u8], expired: bool, events: &mut Vec<Event>) -> (usize, Flow) {
        let (n, event) = decode(rest);
        let short_esc = rest[0] == ESC && n <= 2 && !expired;
        let seq = &rest[..n];
        let flow = if self.paste.is_some() && event != Event::Input(Input::PasteEnd) {
            self.pasting(seq, event, short_esc, expired)
        } else if event == Event::Unknown {
            unknown(seq, expired, events)
        } else {
            self.deliver(event, short_esc, events)
        };
        (n, flow)
    }

    /// An event inside a paste: text and unread key bytes join the
    /// paste; the rest is dropped.
    fn pasting(&mut self, seq: &[u8], event: Event, short_esc: bool, expired: bool) -> Flow {
        let Some(paste) = self.paste.as_mut() else {
            return Flow::Next;
        };
        match event {
            Event::Input(Input::KeyPress(key)) if !key.text.is_empty() => {
                paste.extend_from_slice(key.text.as_bytes());
            }
            Event::Input(Input::KeyPress(_)) if short_esc => return Flow::Wait,
            Event::Input(Input::KeyPress(_)) => paste.extend_from_slice(seq),
            Event::Unknown if !expired => return Flow::Wait,
            _ => {}
        }
        Flow::Next
    }

    /// An event outside a paste, other than an unknown one.
    fn deliver(&mut self, event: Event, short_esc: bool, events: &mut Vec<Event>) -> Flow {
        match event {
            Event::Ignored => return Flow::Next,
            Event::Input(Input::PasteStart) => self.paste = Some(Vec::new()),
            Event::Input(Input::PasteEnd) => {
                let content = paste_text(&self.paste.take().unwrap_or_default());
                events.push(Event::Input(Input::Paste(content)));
            }
            _ => {}
        }
        if short_esc {
            return Flow::Wait;
        }
        match event {
            Event::Multi(all) => events.extend(all),
            event => events.push(event),
        }
        Flow::Next
    }
}

/// The key the table has for a whole read, if it starts with ESC and is
/// longer than two bytes.
fn whole_read_key(buf: &[u8]) -> Option<Key> {
    (buf.len() > 2 && buf[0] == ESC)
        .then(|| table::lookup(buf))
        .flatten()
}

/// An unknown sequence: wait for more bytes until the timeout, then
/// look it up in the key table, else pass it on (to be dropped).
fn unknown(seq: &[u8], expired: bool, events: &mut Vec<Event>) -> Flow {
    if !expired {
        return Flow::Wait;
    }
    if let Some(key) = table::lookup(seq) {
        events.push(Event::Input(Input::KeyPress(key)));
        return Flow::Stop;
    }
    events.push(Event::Unknown);
    Flow::Next
}

/// A paste's bytes as text, as Go decodes them: invalid bytes and
/// replacement characters are dropped.
fn paste_text(bytes: &[u8]) -> String {
    bytes
        .utf8_chunks()
        .flat_map(|chunk| chunk.valid().chars())
        .filter(|&c| c != crate::unicode::REPLACEMENT)
        .collect()
}

/// The reader's buffer and clock: bytes in, input out, and when to read
/// what is left as it is. Go's `StreamEvents` loop, without threads, so
/// it can be driven with any clock.
#[derive(Debug, Default)]
pub(crate) struct Reader {
    scanner: Scanner,
    pending: Vec<u8>,
    deadline: Option<Instant>,
}

impl Reader {
    /// Takes bytes just read at `now`; returns what they complete.
    pub(crate) fn feed(&mut self, bytes: &[u8], now: Instant) -> Vec<Event> {
        self.pending.extend_from_slice(bytes);
        self.scan(false, now)
    }

    /// When the bytes held back should be read as they are, if any are.
    pub(crate) const fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// Reads what is held back as it is, if the deadline has passed by
    /// `now`.
    pub(crate) fn expire(&mut self, now: Instant) -> Vec<Event> {
        match self.deadline {
            Some(at) if now >= at => self.scan(true, now),
            _ => Vec::new(),
        }
    }

    /// Reads what is held back as it is, now: no more bytes are coming.
    pub(crate) fn flush(&mut self, now: Instant) -> Vec<Event> {
        self.scan(true, now)
    }

    fn scan(&mut self, expired: bool, now: Instant) -> Vec<Event> {
        let mut events = Vec::new();
        let used = self.scanner.scan(&self.pending, expired, &mut events);
        let _ = self.pending.drain(..used);
        self.deadline = (!self.pending.is_empty()).then(|| now + ESC_TIMEOUT);
        events
    }
}

#[cfg(test)]
mod tests;
