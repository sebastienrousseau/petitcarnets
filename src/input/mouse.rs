//! Mouse reports: SGR (`CSI < b ; x ; y M`) and X10 (`CSI M b x y`).
//! Go's `parseSGRMouseEvent`, `parseX10MouseEvent` and
//! `parseMouseButton`.

use super::Event;
use super::params::Params;
use crate::backend::Input;
use crate::key::KeyMod;
use crate::mouse::{Mouse, MouseButton};

/// Buttons by their low two bits: plain, wheel (bit 64) and extra (bit
/// 128). Plain 3 is X10's release of an unknown button.
const BUTTONS: [[MouseButton; 4]; 3] = {
    use MouseButton as B;
    [
        [B::Left, B::Middle, B::Right, B::None],
        [B::WheelUp, B::WheelDown, B::WheelLeft, B::WheelRight],
        [B::Backward, B::Forward, B::Button10, B::Button11],
    ]
};

/// What a button byte says.
struct Decoded {
    mods: KeyMod,
    button: MouseButton,
    release: bool,
    motion: bool,
}

/// Decodes a button value: modifiers in bits 4, 8 and 16, motion in 32,
/// the button in the rest.
fn button(b: i64) -> Decoded {
    let mut mods = KeyMod::NONE;
    for (bit, m) in [(8, KeyMod::ALT), (16, KeyMod::CTRL), (4, KeyMod::SHIFT)] {
        if b & bit != 0 {
            mods |= m;
        }
    }
    let row = if b & 128 != 0 {
        2
    } else {
        usize::from(b & 64 != 0)
    };
    let low = usize::try_from(b & 3).unwrap_or(0);
    let button = BUTTONS[row][low];
    Decoded {
        mods,
        button,
        release: row == 0 && low == 3,
        motion: b & 32 != 0 && !button.is_wheel(),
    }
}

/// A coordinate from 1 as a cell from 0, held to `u16`.
fn cell(v: i64) -> u16 {
    u16::try_from((v - 1).max(0)).unwrap_or(u16::MAX)
}

/// `CSI < b ; x ; y M` (press) or `m` (release).
pub(super) fn sgr(last: u8, params: &Params) -> Event {
    let b = button(params.param(0, 0).0);
    let mouse = Mouse {
        x: cell(params.param(1, 1).0),
        y: cell(params.param(2, 1).0),
        button: b.button,
        mods: b.mods,
    };
    let input = if b.button.is_wheel() {
        Input::MouseWheel(mouse)
    } else if b.motion {
        Input::MouseMotion(mouse)
    } else if last == b'm' {
        Input::MouseRelease(mouse)
    } else {
        Input::MouseClick(mouse)
    };
    Event::Input(input)
}

/// `CSI M b x y`, each a byte offset by 32. Go reads the three bytes at
/// offsets 3 to 5 of the sequence, whatever comes before them; a byte
/// past the end reads as 0 here.
pub(super) fn x10(bytes: &[u8]) -> Event {
    let at = |i: usize| i64::from(bytes.get(i).copied().unwrap_or(0));
    let raw = at(3);
    let b = button(if raw >= 32 { raw - 32 } else { raw });
    let mouse = Mouse {
        x: cell(at(4) - 32),
        y: cell(at(5) - 32),
        button: b.button,
        mods: b.mods,
    };
    let input = if b.button.is_wheel() {
        Input::MouseWheel(mouse)
    } else if b.motion {
        Input::MouseMotion(mouse)
    } else if b.release {
        Input::MouseRelease(mouse)
    } else {
        Input::MouseClick(mouse)
    };
    Event::Input(input)
}
