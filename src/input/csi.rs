//! Control sequences (`CSI ...`): keys, mouse, focus and the terminal's
//! replies. Go's `parseCsi`.

use super::keys::{CSI_FINALS, final_key, tilde_code};
use super::params::{self, CSI_MAX, Params};
use super::{ESC, Event, byte_code, kitty, mouse, press, press_code};
use crate::backend::Input;
use crate::key::{Key, KeyCode, KeyMod};
use crate::mode::ReportedMode;

/// Go's `UnknownCsiEvent`.
const UNKNOWN: Event = Event::Other("UnknownCsiEvent");

/// A complete control sequence: its bytes, where it ends, its prefix,
/// intermediate and final bytes, and its parameters.
struct Csi<'a> {
    bytes: &'a [u8],
    end: usize,
    prefix: u8,
    intermediate: u8,
    last: u8,
    params: Params,
}

/// What a handler makes of a sequence: an event and its length, or
/// `None` for Go's `UnknownCsiEvent` over the whole sequence.
type Handled = Option<(usize, Event)>;

/// A handler for one kind of sequence.
type Handler = fn(&Csi<'_>) -> Handled;

/// Handlers by prefix, intermediate and final byte, in Go's order.
const HANDLERS: [(u8, u8, u8, Handler); 19] = [
    (b'?', b'$', b'y', |c| mode_report(c, true)),
    (b'?', 0, b'c', |c| {
        Some(other(c, "PrimaryDeviceAttributesEvent"))
    }),
    (b'>', 0, b'c', |c| {
        Some(other(c, "SecondaryDeviceAttributesEvent"))
    }),
    (b'?', 0, b'u', |c| Some(keyboard_flags(c))),
    (b'?', 0, b'R', private_cursor_report),
    (b'<', 0, b'm', sgr_mouse),
    (b'<', 0, b'M', sgr_mouse),
    (b'>', 0, b'm', modify_other_keys_reply),
    (b'?', 0, b'n', colour_scheme),
    (0, 0, b'I', |c| Some((c.end, Event::Input(Input::Focus)))),
    (0, 0, b'O', |c| Some((c.end, Event::Input(Input::Blur)))),
    (0, 0, b'R', cursor_report),
    (0, 0, b'M', |c| Some(x10_mouse(c))),
    (0, b'$', b'y', |c| mode_report(c, false)),
    (0, 0, b'u', kitty_key),
    (0, 0, b'@', tilde),
    (0, 0, b'^', tilde),
    (0, 0, b'~', tilde),
    (0, 0, b't', window_op),
];

/// `CSI ...`, or Alt and `[`. Go's `parseCsi`.
pub(super) fn parse(b: &[u8]) -> (usize, Event) {
    if b.len() == 2 && b[0] == ESC {
        return (2, press_code(byte_code(b[1]), KeyMod::ALT));
    }
    let start = if b[0] == ESC && b.get(1) == Some(&b'[') {
        2
    } else {
        1
    };
    let prefix = params::prefix(b, start);
    let (params, i) = params::scan(b, start, CSI_MAX);
    let (intermediate, i) = params::intermediate(b, i);
    let Some(&last) = b.get(i).filter(|f| (0x40..=0x7E).contains(*f)) else {
        return unfinished(b, i, intermediate);
    };
    let csi = Csi {
        bytes: b,
        end: i + 1,
        prefix,
        intermediate,
        last,
        params,
    };
    dispatch(&csi).unwrap_or((csi.end, UNKNOWN))
}

/// The handler for a sequence, if any, and what it makes of it.
fn dispatch(c: &Csi<'_>) -> Handled {
    let plain = c.prefix == 0 && c.intermediate == 0;
    if plain && b"abcdABCDEFHPQSZ".contains(&c.last) {
        return cursor_key(c);
    }
    let (_, _, _, handler) = HANDLERS
        .iter()
        .find(|h| (h.0, h.1, h.2) == (c.prefix, c.intermediate, c.last))?;
    handler(c)
}

/// A sequence with no final byte: rxvt-unicode's `CSI n $` (Shift and a `~`
/// key), else unknown (cut off, or to wait for).
fn unfinished(b: &[u8], i: usize, intermediate: u8) -> (usize, Event) {
    if intermediate == b'$' && b[i - 1] == b'$' {
        let mut tilde = b[..i - 1].to_vec();
        tilde.push(b'~');
        if let (n, Event::Input(Input::KeyPress(mut key))) = parse(&tilde) {
            key.mods |= KeyMod::SHIFT;
            return (n, press(key));
        }
    }
    (i, Event::Unknown)
}

/// A sequence Go decodes to an event not delivered here.
fn other(c: &Csi<'_>, name: &'static str) -> (usize, Event) {
    (c.end, Event::Other(name))
}

/// A signed parameter as an `i32` coordinate.
fn coordinate(v: i64) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// `CSI [?] mode ; value $ y`: DECRPM.
fn mode_report(c: &Csi<'_>, dec: bool) -> Handled {
    let (mode, ok) = c.params.param(0, -1);
    let mode = u32::try_from(mode).ok().filter(|_| ok)?;
    let (value, ok) = c.params.param(1, 0);
    if !ok {
        return None;
    }
    let mode = if dec {
        ReportedMode::Dec(mode)
    } else {
        ReportedMode::Ansi(mode)
    };
    let value = value.to_le_bytes()[0].into();
    Some((c.end, Event::Input(Input::ModeReport { mode, value })))
}

/// `CSI ? flags u`: the kitty keyboard flags in force.
fn keyboard_flags(c: &Csi<'_>) -> (usize, Event) {
    let flags = coordinate(c.params.param(0, -1).0);
    let report = crate::mode::KeyboardEnhancementsReport { flags };
    (c.end, Event::Input(Input::KeyboardEnhancements(report)))
}

/// The cursor position at row `row`, column `col`, both from 1.
fn position(row: i64, col: i64) -> Event {
    Event::Input(Input::CursorPosition {
        x: coordinate(col - 1),
        y: coordinate(row - 1),
    })
}

/// `CSI ? row ; col R`: an unambiguous cursor position.
fn private_cursor_report(c: &Csi<'_>) -> Handled {
    let row = c.params.param(0, 1).0;
    let (col, ok) = c.params.param(1, 1);
    ok.then(|| (c.end, position(row, col)))
}

/// `CSI row ; col R`: the cursor position, or F3 with modifiers, which
/// is the same sequence on row 1 (then both are reported, as in Go).
fn cursor_report(c: &Csi<'_>) -> Handled {
    let (row, row_ok) = c.params.param(0, 1);
    let (col, col_ok) = c.params.param(1, 1);
    if c.params.len() == 2 && row_ok && col_ok {
        let report = position(row, col);
        if row == 1 && col - 1 <= 15 {
            let f3 = press_code(KeyCode::F(3), KeyMod::from_bits(col - 1));
            return Some((c.end, Event::Multi(vec![f3, report])));
        }
        return Some((c.end, report));
    }
    if c.params.len() != 0 {
        return None;
    }
    cursor_key(c)
}

/// `CSI [1 ; modifier] x`: cursor keys, F1 to F4 and back tab.
fn cursor_key(c: &Csi<'_>) -> Handled {
    let mut key = final_key(&CSI_FINALS, c.last)?;
    let p = &c.params;
    let (id, modifier) = (p.param(0, 1).0, p.param(1, 1).0);
    if (p.len() > 2 && !p.at(1).has_more()) || id != 1 {
        return None;
    }
    if p.len() > 1 && modifier != -1 {
        key.mods |= KeyMod::from_bits(modifier - 1);
    }
    Some((c.end, kitty::extend(p, key)))
}

/// `CSI n [; modifier] ~` and rxvt-unicode's `^` (Ctrl) and `@` (Ctrl and
/// Shift); also bracketed paste marks and modifyOtherKeys keys.
fn tilde(c: &Csi<'_>) -> Handled {
    let p = &c.params;
    if p.len() == 0 {
        return None;
    }
    let n = p.param(0, 0).0;
    if c.last == b'~' && [27, 200, 201].contains(&n) {
        return special_tilde(c, n);
    }
    let mut key = Key::new(tilde_code(n)?);
    let modifier = p.param(1, -1).0;
    if p.len() > 1 && modifier != -1 {
        key.mods |= KeyMod::from_bits(modifier - 1);
    }
    Some((c.end, tilde_suffix(c, key)))
}

/// `CSI 27 ; mods ; code ~` (modifyOtherKeys) and the bracketed paste
/// marks `CSI 200 ~` and `CSI 201 ~`.
fn special_tilde(c: &Csi<'_>, n: i64) -> Handled {
    let event = match n {
        200 => Event::Input(Input::PasteStart),
        201 => Event::Input(Input::PasteEnd),
        _ if c.params.len() == 3 => kitty::modify_other_keys(&c.params),
        _ => return None,
    };
    Some((c.end, event))
}

/// The key after its final byte: `~` may carry a kitty event type,
/// rxvt-unicode's `^` adds Ctrl and `@` Ctrl and Shift.
fn tilde_suffix(c: &Csi<'_>, mut key: Key) -> Event {
    key.mods |= match c.last {
        b'~' => return kitty::extend(&c.params, key),
        b'^' => KeyMod::CTRL,
        _ => KeyMod::CTRL | KeyMod::SHIFT,
    };
    press(key)
}

/// `CSI < button ; x ; y M` or `m`: an SGR mouse event.
fn sgr_mouse(c: &Csi<'_>) -> Handled {
    (c.params.len() == 3).then(|| (c.end, mouse::sgr(c.last, &c.params)))
}

/// `CSI M b x y`: an X10 mouse event, three raw bytes after the `M`.
fn x10_mouse(c: &Csi<'_>) -> (usize, Event) {
    if c.end + 3 > c.bytes.len() {
        return (c.end, UNKNOWN);
    }
    (c.end + 3, mouse::x10(c.bytes))
}

/// `CSI > 4 ; level m`: the modifyOtherKeys level.
fn modify_other_keys_reply(c: &Csi<'_>) -> Handled {
    let (resource, ok) = c.params.param(0, 0);
    let (level, level_ok) = c.params.param(1, -1);
    (ok && resource == 4 && level_ok && level != -1)
        .then_some((c.end, Event::Other("ModifyOtherKeysEvent")))
}

/// `CSI ? 997 ; 1 n` or `2 n`: dark or light colour scheme.
fn colour_scheme(c: &Csi<'_>) -> Handled {
    let report = c.params.param(0, -1).0;
    match (report, c.params.param(1, -1).0) {
        (997, 1) => Some(other(c, "DarkColorSchemeEvent")),
        (997, 2) => Some(other(c, "LightColorSchemeEvent")),
        _ => None,
    }
}

/// `CSI code ... u`: a kitty keyboard protocol key.
fn kitty_key(c: &Csi<'_>) -> Handled {
    (c.params.len() != 0).then(|| (c.end, kitty::parse(&c.params)))
}

/// `CSI op ... t`: window reports and other window operations.
fn window_op(c: &Csi<'_>) -> Handled {
    let (op, ok) = c.params.param(0, 0);
    if !ok {
        return None;
    }
    let event = match (op, c.params.len()) {
        (4, 3) => Event::Other("PixelSizeEvent"),
        (6, 3) => Event::Other("CellSizeEvent"),
        (8, 3) => Event::Other("WindowSizeEvent"),
        (48, 5) => Event::Multi(vec![
            Event::Other("WindowSizeEvent"),
            Event::Other("PixelSizeEvent"),
        ]),
        _ => Event::Other("WindowOpEvent"),
    };
    Some((c.end, event))
}
