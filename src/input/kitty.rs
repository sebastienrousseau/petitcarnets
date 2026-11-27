//! The kitty keyboard protocol (`CSI code ... u`, and its event types on
//! legacy keys) and xterm's modifyOtherKeys (`CSI 27 ; mod ; code ~`).
//! Go's `parseKittyKeyboard`, `parseKittyKeyboardExt` and
//! `parseXTermModifyOtherKeys`.

use super::params::{Param, Params};
use super::{Event, press};
use crate::backend::Input;
use crate::key::{Key, KeyCode, KeyMod};
use crate::unicode::{self, REPLACEMENT};

/// Kitty's functional keys from code 57344 to 57363.
const FUNCTIONAL_HEAD: [KeyCode; 20] = {
    use KeyCode as K;
    [
        K::Esc,
        K::Enter,
        K::Tab,
        K::Backspace,
        K::Insert,
        K::Delete,
        K::Left,
        K::Right,
        K::Up,
        K::Down,
        K::PgUp,
        K::PgDown,
        K::Home,
        K::End,
        K::CapsLock,
        K::ScrollLock,
        K::NumLock,
        K::PrintScreen,
        K::Pause,
        K::Menu,
    ]
};

/// Kitty's functional keys from code 57399 (after F1 to F35) to 57454.
const FUNCTIONAL_TAIL: [KeyCode; 56] = {
    use KeyCode as K;
    [
        K::Kp0,
        K::Kp1,
        K::Kp2,
        K::Kp3,
        K::Kp4,
        K::Kp5,
        K::Kp6,
        K::Kp7,
        K::Kp8,
        K::Kp9,
        K::KpDecimal,
        K::KpDivide,
        K::KpMultiply,
        K::KpMinus,
        K::KpPlus,
        K::KpEnter,
        K::KpEqual,
        K::KpSep,
        K::KpLeft,
        K::KpRight,
        K::KpUp,
        K::KpDown,
        K::KpPgUp,
        K::KpPgDown,
        K::KpHome,
        K::KpEnd,
        K::KpInsert,
        K::KpDelete,
        K::KpBegin,
        K::MediaPlay,
        K::MediaPause,
        K::MediaPlayPause,
        K::MediaReverse,
        K::MediaStop,
        K::MediaFastForward,
        K::MediaRewind,
        K::MediaNext,
        K::MediaPrev,
        K::MediaRecord,
        K::LowerVol,
        K::RaiseVol,
        K::Mute,
        K::LeftShift,
        K::LeftCtrl,
        K::LeftAlt,
        K::LeftSuper,
        K::LeftHyper,
        K::LeftMeta,
        K::RightShift,
        K::RightCtrl,
        K::RightAlt,
        K::RightSuper,
        K::RightHyper,
        K::RightMeta,
        K::IsoLevel3Shift,
        K::IsoLevel5Shift,
    ]
};

/// The text a keypad key types, when nothing but Shift or a lock is held.
const KEYPAD_TEXT: [(KeyCode, char); 17] = {
    use KeyCode as K;
    [
        (K::Kp0, '0'),
        (K::Kp1, '1'),
        (K::Kp2, '2'),
        (K::Kp3, '3'),
        (K::Kp4, '4'),
        (K::Kp5, '5'),
        (K::Kp6, '6'),
        (K::Kp7, '7'),
        (K::Kp8, '8'),
        (K::Kp9, '9'),
        (K::KpEqual, '='),
        (K::KpMultiply, '*'),
        (K::KpPlus, '+'),
        (K::KpMinus, '-'),
        (K::KpDecimal, '.'),
        (K::KpDivide, '/'),
        (K::KpSep, ','),
    ]
};

/// Kitty modifier bits and the modifiers they are (kitty's Super and
/// Meta are Bubble Tea's Super and Meta, whose bits are swapped).
const KITTY_MODS: [(i64, KeyMod); 8] = [
    (1, KeyMod::SHIFT),
    (2, KeyMod::ALT),
    (4, KeyMod::CTRL),
    (8, KeyMod::SUPER),
    (16, KeyMod::HYPER),
    (32, KeyMod::META),
    (64, KeyMod::CAPS_LOCK),
    (128, KeyMod::NUM_LOCK),
];

/// Codes Go's `kittyKeyMap` names outright.
const NAMED: [(i64, KeyCode, KeyMod); 6] = [
    (0, KeyCode::Space, KeyMod::CTRL),
    (8, KeyCode::Backspace, KeyMod::NONE),
    (9, KeyCode::Tab, KeyMod::NONE),
    (13, KeyCode::Enter, KeyMod::NONE),
    (27, KeyCode::Esc, KeyMod::NONE),
    (127, KeyCode::Backspace, KeyMod::NONE),
];

/// The key a kitty code names in Go's `kittyKeyMap`, if any: control
/// characters (as Ctrl and a letter or symbol) and the functional keys.
fn mapped(code: i64) -> Option<Key> {
    if let Some((_, key, mods)) = NAMED.iter().find(|n| n.0 == code) {
        return Some(Key::new(*key).with_mods(*mods));
    }
    if (57344..=57454).contains(&code) {
        return functional(code - 57344).map(Key::new);
    }
    let b = u8::try_from(code).ok().filter(|b| (1..=0x1F).contains(b))?;
    let c = if b <= 0x1A { b + 0x60 } else { b + 0x40 };
    Some(Key::new(KeyCode::Char(char::from(c))).with_mods(KeyMod::CTRL))
}

/// Functional key number `i`, from 0.
fn functional(i: i64) -> Option<KeyCode> {
    let i = usize::try_from(i).ok()?;
    match i {
        20..=54 => u8::try_from(i - 19).ok().map(KeyCode::F),
        0..=19 => FUNCTIONAL_HEAD.get(i).copied(),
        _ => FUNCTIONAL_TAIL.get(i - 55).copied(),
    }
}

/// The code of a code point parameter: a valid character, else the
/// replacement character.
fn code_point(value: i64) -> KeyCode {
    let valid = u32::try_from(value).ok().and_then(char::from_u32);
    KeyCode::from_rune(i64::from(u32::from(valid.unwrap_or(REPLACEMENT))))
}

/// The modifiers of a kitty modifier value (1 more than its bits).
fn kitty_mods(value: i64) -> KeyMod {
    KITTY_MODS
        .iter()
        .filter(|(bit, _)| value & bit != 0)
        .fold(KeyMod::NONE, |mods, (_, m)| mods | *m)
}

/// What a kitty sequence says, while its parameters are read.
struct Kitty {
    key: Key,
    release: bool,
}

impl Kitty {
    /// The key code, and its shifted and base codes.
    fn code(&mut self, sub: usize, p: Param) {
        let value = p.get(1);
        let printable = u32::try_from(value).is_ok_and(unicode::is_print);
        match sub {
            0 => {
                self.key = mapped(value).unwrap_or_else(|| Key::new(code_point(value)));
            }
            1 | 2 if printable => {
                // Go falls through from the base code to the shifted one,
                // so a base code sets both.
                if sub == 2 {
                    self.key.base_code = Some(code_point(value));
                }
                self.key.shifted_code = Some(code_point(value));
            }
            _ => {}
        }
    }

    /// The modifiers and the event type.
    fn state(&mut self, sub: usize, p: Param) {
        let value = p.get(1);
        match (sub, value) {
            (0, m) if m > 1 => {
                self.key.mods = kitty_mods(m - 1);
                if self.key.mods.bits() > KeyMod::SHIFT.bits() {
                    self.key.text.clear();
                }
            }
            (1, 2) => self.key.is_repeat = true,
            (1, 3) => self.release = true,
            _ => {}
        }
    }

    /// One code point of associated text.
    fn text(&mut self, p: Param) {
        let value = p.get(0);
        if value != 0 {
            let c = u32::try_from(value).ok().and_then(char::from_u32);
            self.key.text.push(c.unwrap_or(REPLACEMENT));
        }
    }
}

/// `CSI code[:shifted[:base]] [; mods[:event]] [; text...] u`.
pub(super) fn parse(params: &Params) -> Event {
    let mut k = Kitty {
        key: Key::new(KeyCode::Char(REPLACEMENT)),
        release: false,
    };
    let (mut index, mut sub) = (0, 0);
    for p in params.iter() {
        match index {
            0 => k.code(sub, p),
            1 => k.state(sub, p),
            2 => k.text(p),
            _ => {}
        }
        sub += 1;
        if !p.has_more() {
            index += 1;
            sub = 0;
        }
    }
    fill_text(&mut k.key);
    if k.release {
        Event::Input(Input::KeyRelease(k.key))
    } else {
        press(k.key)
    }
}

/// The text a key types when the terminal sent none: keypad keys and
/// printable keys, held with nothing but Shift and the locks.
fn fill_text(key: &mut Key) {
    let mods = key.mods.without(KeyMod::NUM_LOCK);
    if !key.text.is_empty() || !types_text(mods) {
        return;
    }
    if let Some((_, c)) = KEYPAD_TEXT.iter().find(|(code, _)| *code == key.code) {
        key.text.push(*c);
    } else if key.code.is_print() {
        key.text.push(typed(key, mods));
    }
}

/// Whether keys held with `mods` type text: none, Shift, Caps Lock, or
/// both.
fn types_text(mods: KeyMod) -> bool {
    let caps = KeyMod::CAPS_LOCK;
    mods.bits() <= KeyMod::SHIFT.bits() || mods == caps || mods == caps | KeyMod::SHIFT
}

/// The character a printable key types with `mods`: itself, the
/// shifted key, or itself in the case Shift or Caps Lock asks for.
fn typed(key: &Key, mods: KeyMod) -> char {
    let c = char::from_u32(key.code.rune()).unwrap_or(REPLACEMENT);
    if mods.is_empty() {
        return c;
    }
    if let Some(shifted) = key.shifted_code {
        return char::from_u32(shifted.rune()).unwrap_or(REPLACEMENT);
    }
    if mods.contains(KeyMod::SHIFT) || mods.contains(KeyMod::CAPS_LOCK) {
        unicode::to_upper(c)
    } else {
        unicode::to_lower(c)
    }
}

/// The event type parameter on a legacy key (`CSI 1 ; mods:event A`):
/// a repeat or a release.
pub(super) fn extend(params: &Params, mut key: Key) -> Event {
    if params.len() > 2 && params.at(1).has_more() {
        match params.at(2).get(1) {
            2 => key.is_repeat = true,
            3 => return Event::Input(Input::KeyRelease(key)),
            _ => {}
        }
    }
    press(key)
}

/// `CSI 27 ; mods ; code ~`: xterm's modifyOtherKeys.
pub(super) fn modify_other_keys(params: &Params) -> Event {
    let mods = KeyMod::from_bits(params.param(1, 1).0 - 1);
    let code = params.param(2, 1).0;
    let named = match code {
        8 | 127 => Some(KeyCode::Backspace),
        9 => Some(KeyCode::Tab),
        13 => Some(KeyCode::Enter),
        27 => Some(KeyCode::Esc),
        _ => None,
    };
    if let Some(named) = named {
        return press(Key::new(named).with_mods(mods));
    }
    let mut key = Key::new(KeyCode::from_rune(code)).with_mods(mods);
    if mods.bits() <= KeyMod::SHIFT.bits() {
        let c = u32::try_from(code).ok().and_then(char::from_u32);
        key.text.push(c.unwrap_or(REPLACEMENT));
    }
    press(key)
}
