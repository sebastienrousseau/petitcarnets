//! String sequences: OSC (colour replies), DCS (terminal version), and
//! APC, PM and SOS, which are read and dropped. Go's `parseOsc`,
//! `parseDcs`, `parseApc` and `parseStTerminated`.

use super::params::{self, DCS_MAX};
use super::{ESC, Event, PM, SOS, ST, byte_code, press_code};
use crate::backend::Input;
use crate::color::Rgb;
use crate::key::{KeyCode, KeyMod};
use crate::unicode;

/// BEL, which ends an OSC.
const BEL: u8 = 0x07;
/// CAN and SUB, which cancel a string.
const CAN: u8 = 0x18;
/// See [`CAN`].
const SUB: u8 = 0x1A;

/// Alt and the byte after ESC: what an unfinished string sequence turns
/// out to be.
fn alt(b: &[u8]) -> (usize, Event) {
    (2, press_code(byte_code(b[1]), KeyMod::ALT))
}

/// Alt, Shift and the byte after ESC in lower case (`ESC P`, `ESC X`).
fn alt_shift(b: &[u8]) -> (usize, Event) {
    let lower = unicode::to_lower(char::from(b[1]));
    let code = KeyCode::from_rune(i64::from(u32::from(lower)));
    (2, press_code(code, KeyMod::SHIFT | KeyMod::ALT))
}

/// Where a string's data starts: after the 8-bit introducer, or after
/// ESC and `intro`.
fn data_start(b: &[u8], intro: u8) -> usize {
    if b[0] == ESC && b.get(1) == Some(&intro) {
        2
    } else {
        1
    }
}

/// How a string ended, from the byte at `end`: the length taken, and
/// whether the string stands (else it was cancelled or cut short).
enum Ending {
    /// Ended by a terminator; the sequence is this long.
    Done(usize),
    /// Cancelled by CAN or SUB; the sequence is this long.
    Cancelled(usize),
    /// An ESC not followed by `\`.
    StrayEsc(usize),
}

/// How the string whose data ends at `b[end]` ends.
fn ending(b: &[u8], end: usize) -> Ending {
    let i = end + 1;
    match b[end] {
        CAN | SUB => Ending::Cancelled(i),
        ESC if b.get(i) == Some(&b'\\') => Ending::Done(i + 1),
        ESC => Ending::StrayEsc(i),
        _ => Ending::Done(i),
    }
}

/// The first byte from `from` on that is in `set`.
fn find(b: &[u8], from: usize, set: &[u8]) -> Option<usize> {
    (from..b.len()).find(|&j| set.contains(&b[j]))
}

/// `OSC cmd ; data` ended by BEL or ST: colour replies; others are read
/// and dropped.
pub(super) fn osc(b: &[u8]) -> (usize, Event) {
    if b.len() == 2 && b[0] == ESC {
        return alt(b);
    }
    let (cmd, i) = osc_command(b, data_start(b, b']'));
    let (start, i) = if b.get(i) == Some(&b';') {
        (i + 1, i + 1)
    } else {
        (0, i)
    };
    match find(b, i, &[BEL, ESC, ST, CAN, SUB]) {
        Some(end) => osc_end(b, cmd, start, end),
        None => (b.len(), Event::Unknown),
    }
}

/// An OSC whose data runs from `start` to the terminator at `end`.
fn osc_end(b: &[u8], cmd: i64, start: usize, end: usize) -> (usize, Event) {
    match ending(b, end) {
        Ending::StrayEsc(_) if cmd == -1 || (start, end) == (0, 2) => alt(b),
        Ending::Cancelled(n) | Ending::StrayEsc(n) => (n, Event::Ignored),
        Ending::Done(n) if end <= start => (n, Event::Unknown),
        Ending::Done(n) => (n, osc_event(cmd, &b[start..end])),
    }
}

/// The OSC command number at `b[i..]` (-1 if there are no digits), with
/// Go's wrapping arithmetic, and where it ends.
fn osc_command(b: &[u8], mut i: usize) -> (i64, usize) {
    let mut cmd: i64 = -1;
    while let Some(d) = b.get(i).filter(|d| d.is_ascii_digit()) {
        let shifted = if cmd == -1 { 0 } else { cmd.wrapping_mul(10) };
        cmd = shifted.wrapping_add(i64::from(d - b'0'));
        i += 1;
    }
    (cmd, i)
}

/// What an OSC with command `cmd` and `data` is.
fn osc_event(cmd: i64, data: &[u8]) -> Event {
    let colour = |make: fn(Rgb) -> Input, name| {
        Rgb::parse(data).map_or(Event::NoColour(name), |c| Event::Input(make(c)))
    };
    match cmd {
        10 => colour(Input::ForegroundColor, "ForegroundColorEvent"),
        11 => colour(Input::BackgroundColor, "BackgroundColorEvent"),
        12 => colour(Input::CursorColor, "CursorColorEvent"),
        52 => Event::Other("ClipboardEvent"),
        7501 if data[0] == b'?' => Event::Other("ProgramStatusSupportEvent"),
        _ => Event::Other("UnknownOscEvent"),
    }
}

/// The part of a DCS before its data.
struct DcsHead {
    prefix: u8,
    intermediate: u8,
    last: u8,
    first_param: i64,
    data: usize,
}

/// Reads a DCS up to its data; on a missing final byte, where reading
/// stopped.
fn dcs_head(b: &[u8]) -> Result<DcsHead, usize> {
    let start = data_start(b, b'P');
    let prefix = params::prefix(b, start);
    let (params, i) = params::scan(b, start, DCS_MAX);
    let (intermediate, i) = params::intermediate(b, i);
    let &last = b.get(i).filter(|f| (0x40..=0x7E).contains(*f)).ok_or(i)?;
    Ok(DcsHead {
        prefix,
        intermediate,
        last,
        first_param: params.param(0, 0).0,
        data: i + 1,
    })
}

/// `DCS [prefix] params [intermediate] final data ST`: the terminal's
/// version (`DCS > | name ST`); others are read and dropped.
pub(super) fn dcs(b: &[u8]) -> (usize, Event) {
    if b.len() == 2 && b[0] == ESC {
        return alt_shift(b);
    }
    let head = match dcs_head(b) {
        Ok(head) => head,
        Err(i) => return (i, Event::Unknown),
    };
    let Some(end) = find(b, head.data, &[ST, ESC]) else {
        return (b.len(), Event::Unknown);
    };
    let st = b[end] == ESC && b.get(end + 1) == Some(&b'\\');
    let n = end + 1 + usize::from(st);
    (n, dcs_event(&head, &b[head.data..end]))
}

/// What a DCS with `head` and `data` is.
fn dcs_event(head: &DcsHead, data: &[u8]) -> Event {
    match (head.prefix, head.intermediate, head.last) {
        (0, b'+', b'r') if head.first_param == 1 => Event::Other("CapabilityEvent"),
        (b'>', 0, b'|') => {
            let name = String::from_utf8_lossy(data).into_owned();
            Event::Input(Input::TerminalVersion(name))
        }
        (0, b'!', b'|') if is_hex(data) => Event::Other("TertiaryDeviceAttributesEvent"),
        _ => Event::Other("UnknownDcsEvent"),
    }
}

/// Whether `data` is whole bytes of hex digits, as Go's
/// `hex.DecodeString` wants.
fn is_hex(data: &[u8]) -> bool {
    data.len() % 2 == 0 && data.iter().all(u8::is_ascii_hexdigit)
}

/// `APC data ST`, read and dropped (kitty graphics replies among them).
pub(super) fn apc(b: &[u8]) -> (usize, Event) {
    st_terminated(b)
}

/// APC, PM or SOS: a string ended by ST, read and dropped.
pub(super) fn st_terminated(b: &[u8]) -> (usize, Event) {
    let intro = if b[0] == ESC { b[1] } else { b[0] };
    let default = || string_key(b, intro);
    if b.len() == 2 && b[0] == ESC {
        return default();
    }
    let start = data_start(b, intro);
    let Some(end) = find(b, start, &[ESC, ST, CAN, SUB]) else {
        return (b.len(), Event::Unknown);
    };
    match ending(b, end) {
        Ending::StrayEsc(_) if start == end => default(),
        Ending::Cancelled(n) | Ending::StrayEsc(n) => (n, Event::Ignored),
        Ending::Done(n) => (n, Event::Other(string_name(intro, &b[start..end]))),
    }
}

/// What an unfinished APC, PM or SOS turns out to be: Alt and Shift and
/// `x` for SOS, Alt and the byte after ESC for the others.
fn string_key(b: &[u8], intro: u8) -> (usize, Event) {
    if intro == b'X' || intro == SOS {
        alt_shift(b)
    } else {
        alt(b)
    }
}

/// Go's name for a string sequence that is read and dropped.
fn string_name(intro: u8, data: &[u8]) -> &'static str {
    match intro {
        PM | b'^' => "UnknownPmEvent",
        SOS | b'X' => "UnknownSosEvent",
        _ if data.first() == Some(&b'G') => "KittyGraphicsEvent",
        _ => "UnknownApcEvent",
    }
}
