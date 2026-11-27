//! Key codes: a character, or one of Bubble Tea's named keys.

use crate::unicode::REPLACEMENT;

/// Which key: a character, or a named key. Bubble Tea's `Key.Code`.
///
/// Go keeps every code in one `rune`, with the named keys above the last
/// Unicode code point; here they are variants. Enter, Tab, Backspace,
/// Escape and Space are named, never [`KeyCode::Char`], as in Go where
/// they are the runes `'\r'`, `'\t'`, DEL, ESC and `' '`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyCode {
    /// A character, such as `'a'` or `'é'`.
    Char(char),
    /// Text of more than one code point, such as an emoji sequence; the
    /// text is in [`Key::text`](crate::Key::text). Go's `KeyExtended`.
    Extended,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Right arrow.
    Right,
    /// Left arrow.
    Left,
    /// Begin (keypad 5 without Num Lock on some terminals).
    Begin,
    /// Find (VT220).
    Find,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Select (VT220).
    Select,
    /// Page Up.
    PgUp,
    /// Page Down.
    PgDown,
    /// Home.
    Home,
    /// End.
    End,
    /// Keypad Enter.
    KpEnter,
    /// Keypad `=`.
    KpEqual,
    /// Keypad `*`.
    KpMultiply,
    /// Keypad `+`.
    KpPlus,
    /// Keypad `,`.
    KpComma,
    /// Keypad `-`.
    KpMinus,
    /// Keypad `.`.
    KpDecimal,
    /// Keypad `/`.
    KpDivide,
    /// Keypad 0.
    Kp0,
    /// Keypad 1.
    Kp1,
    /// Keypad 2.
    Kp2,
    /// Keypad 3.
    Kp3,
    /// Keypad 4.
    Kp4,
    /// Keypad 5.
    Kp5,
    /// Keypad 6.
    Kp6,
    /// Keypad 7.
    Kp7,
    /// Keypad 8.
    Kp8,
    /// Keypad 9.
    Kp9,
    /// Keypad separator (kitty).
    KpSep,
    /// Keypad up (kitty).
    KpUp,
    /// Keypad down (kitty).
    KpDown,
    /// Keypad left (kitty).
    KpLeft,
    /// Keypad right (kitty).
    KpRight,
    /// Keypad Page Up (kitty).
    KpPgUp,
    /// Keypad Page Down (kitty).
    KpPgDown,
    /// Keypad Home (kitty).
    KpHome,
    /// Keypad End (kitty).
    KpEnd,
    /// Keypad Insert (kitty).
    KpInsert,
    /// Keypad Delete (kitty).
    KpDelete,
    /// Keypad Begin (kitty).
    KpBegin,
    /// A function key, `F(1)` to `F(63)`.
    F(u8),
    /// Caps Lock (kitty).
    CapsLock,
    /// Scroll Lock (kitty).
    ScrollLock,
    /// Num Lock (kitty).
    NumLock,
    /// Print Screen (kitty).
    PrintScreen,
    /// Pause (kitty).
    Pause,
    /// Menu (kitty).
    Menu,
    /// Media play (kitty).
    MediaPlay,
    /// Media pause (kitty).
    MediaPause,
    /// Media play or pause (kitty).
    MediaPlayPause,
    /// Media reverse (kitty).
    MediaReverse,
    /// Media stop (kitty).
    MediaStop,
    /// Media fast forward (kitty).
    MediaFastForward,
    /// Media rewind (kitty).
    MediaRewind,
    /// Media next track (kitty).
    MediaNext,
    /// Media previous track (kitty).
    MediaPrev,
    /// Media record (kitty).
    MediaRecord,
    /// Volume down (kitty).
    LowerVol,
    /// Volume up (kitty).
    RaiseVol,
    /// Mute (kitty).
    Mute,
    /// Left Shift (kitty).
    LeftShift,
    /// Left Alt (kitty).
    LeftAlt,
    /// Left Ctrl (kitty).
    LeftCtrl,
    /// Left Super (kitty).
    LeftSuper,
    /// Left Hyper (kitty).
    LeftHyper,
    /// Left Meta (kitty).
    LeftMeta,
    /// Right Shift (kitty).
    RightShift,
    /// Right Alt (kitty).
    RightAlt,
    /// Right Ctrl (kitty).
    RightCtrl,
    /// Right Super (kitty).
    RightSuper,
    /// Right Hyper (kitty).
    RightHyper,
    /// Right Meta (kitty).
    RightMeta,
    /// ISO level 3 shift, `AltGr` (kitty).
    IsoLevel3Shift,
    /// ISO level 5 shift (kitty).
    IsoLevel5Shift,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Enter.
    Enter,
    /// Escape.
    Esc,
    /// Space.
    Space,
}

/// A named key: its code, how Bubble Tea writes it, and the name
/// `Key::MatchString` reads for it.
type Named = (KeyCode, &'static str, &'static str);

/// Go's named keys before the function keys, in Go's order: the first is
/// `KeyExtended + 1`.
const BEFORE_F: [Named; 43] = {
    use KeyCode as K;
    [
        (K::Up, "up", "up"),
        (K::Down, "down", "down"),
        (K::Right, "right", "right"),
        (K::Left, "left", "left"),
        (K::Begin, "begin", "begin"),
        (K::Find, "find", "find"),
        (K::Insert, "insert", "insert"),
        (K::Delete, "delete", "delete"),
        (K::Select, "select", "select"),
        (K::PgUp, "pgup", "pgup"),
        (K::PgDown, "pgdown", "pgdown"),
        (K::Home, "home", "home"),
        (K::End, "end", "end"),
        (K::KpEnter, "enter", "kpenter"),
        (K::KpEqual, "equal", "kpequal"),
        (K::KpMultiply, "mul", "kpmul"),
        (K::KpPlus, "plus", "kpplus"),
        (K::KpComma, "comma", "kpcomma"),
        (K::KpMinus, "minus", "kpminus"),
        (K::KpDecimal, "period", "kpperiod"),
        (K::KpDivide, "div", "kpdiv"),
        (K::Kp0, "0", "kp0"),
        (K::Kp1, "1", "kp1"),
        (K::Kp2, "2", "kp2"),
        (K::Kp3, "3", "kp3"),
        (K::Kp4, "4", "kp4"),
        (K::Kp5, "5", "kp5"),
        (K::Kp6, "6", "kp6"),
        (K::Kp7, "7", "kp7"),
        (K::Kp8, "8", "kp8"),
        (K::Kp9, "9", "kp9"),
        (K::KpSep, "sep", "kpsep"),
        (K::KpUp, "up", "kpup"),
        (K::KpDown, "down", "kpdown"),
        (K::KpLeft, "left", "kpleft"),
        (K::KpRight, "right", "kpright"),
        (K::KpPgUp, "pgup", "kppgup"),
        (K::KpPgDown, "pgdown", "kppgdown"),
        (K::KpHome, "home", "kphome"),
        (K::KpEnd, "end", "kpend"),
        (K::KpInsert, "insert", "kpinsert"),
        (K::KpDelete, "delete", "kpdelete"),
        (K::KpBegin, "begin", "kpbegin"),
    ]
};

/// Go's named keys after `KeyF63`, in Go's order.
const AFTER_F: [Named; 33] = {
    use KeyCode as K;
    [
        (K::CapsLock, "capslock", "capslock"),
        (K::ScrollLock, "scrolllock", "scrolllock"),
        (K::NumLock, "numlock", "numlock"),
        (K::PrintScreen, "printscreen", "printscreen"),
        (K::Pause, "pause", "pause"),
        (K::Menu, "menu", "menu"),
        (K::MediaPlay, "mediaplay", "mediaplay"),
        (K::MediaPause, "mediapause", "mediapause"),
        (K::MediaPlayPause, "mediaplaypause", "mediaplaypause"),
        (K::MediaReverse, "mediareverse", "mediareverse"),
        (K::MediaStop, "mediastop", "mediastop"),
        (K::MediaFastForward, "mediafastforward", "mediafastforward"),
        (K::MediaRewind, "mediarewind", "mediarewind"),
        (K::MediaNext, "medianext", "medianext"),
        (K::MediaPrev, "mediaprev", "mediaprev"),
        (K::MediaRecord, "mediarecord", "mediarecord"),
        (K::LowerVol, "lowervol", "lowervol"),
        (K::RaiseVol, "raisevol", "raisevol"),
        (K::Mute, "mute", "mute"),
        (K::LeftShift, "leftshift", "leftshift"),
        (K::LeftAlt, "leftalt", "leftalt"),
        (K::LeftCtrl, "leftctrl", "leftctrl"),
        (K::LeftSuper, "leftsuper", "leftsuper"),
        (K::LeftHyper, "lefthyper", "lefthyper"),
        (K::LeftMeta, "leftmeta", "leftmeta"),
        (K::RightShift, "rightshift", "rightshift"),
        (K::RightAlt, "rightalt", "rightalt"),
        (K::RightCtrl, "rightctrl", "rightctrl"),
        (K::RightSuper, "rightsuper", "rightsuper"),
        (K::RightHyper, "righthyper", "righthyper"),
        (K::RightMeta, "rightmeta", "rightmeta"),
        (K::IsoLevel3Shift, "isolevel3shift", "isolevel3shift"),
        (K::IsoLevel5Shift, "isolevel5shift", "isolevel5shift"),
    ]
};

/// The keys Go names by their control character: code, rune, name.
const CONTROL: [(KeyCode, u32, &str); 5] = [
    (KeyCode::Backspace, 0x7F, "backspace"),
    (KeyCode::Tab, 0x09, "tab"),
    (KeyCode::Enter, 0x0D, "enter"),
    (KeyCode::Esc, 0x1B, "esc"),
    (KeyCode::Space, 0x20, "space"),
];

/// Go's `KeyExtended`, one past the last Unicode code point.
const EXTENDED: u32 = 0x11_0000;
/// Go's `KeyF1`.
const F1: u32 = EXTENDED + 1 + 43;
/// One past Go's `KeyF63`.
const AFTER_F63: u32 = F1 + 63;

impl KeyCode {
    /// The code Go gives `rune`: a named key for Go's special values,
    /// else the character. A value that is no character (Go keeps it
    /// as is) becomes the replacement character.
    pub(crate) fn from_rune(rune: i64) -> Self {
        let Ok(r) = u32::try_from(rune) else {
            return Self::Char(REPLACEMENT);
        };
        if let Some((code, _, _)) = CONTROL.iter().find(|c| c.1 == r) {
            return *code;
        }
        if r == EXTENDED {
            return Self::Extended;
        }
        if (F1..AFTER_F63).contains(&r) {
            return Self::F(u8::try_from(r - F1 + 1).unwrap_or(0));
        }
        let before = r.checked_sub(EXTENDED + 1).and_then(|i| at(&BEFORE_F, i));
        let after = r.checked_sub(AFTER_F63).and_then(|i| at(&AFTER_F, i));
        before
            .or(after)
            .unwrap_or_else(|| Self::Char(char::from_u32(r).unwrap_or(REPLACEMENT)))
    }

    /// The `rune` Go uses for this code.
    pub(crate) fn rune(self) -> u32 {
        match self {
            Self::Char(c) => u32::from(c),
            Self::Extended => EXTENDED,
            Self::F(n) => F1 + u32::from(n).saturating_sub(1),
            named => named_rune(named),
        }
    }

    /// The name Bubble Tea writes for a named key; `None` for a
    /// character, extended text and function keys.
    pub(crate) fn name(self) -> Option<&'static str> {
        CONTROL
            .iter()
            .find(|c| c.0 == self)
            .map(|c| c.2)
            .or_else(|| named(self).map(|n| n.1))
    }

    /// The key `Key::MatchString` reads for `name`, such as `"kpenter"`
    /// or `"f5"`.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        if name == "escape" {
            return Some(Self::Esc);
        }
        let control = CONTROL.iter().find(|c| c.2 == name).map(|c| c.0);
        let named = BEFORE_F.iter().chain(&AFTER_F).find(|n| n.2 == name);
        control
            .or(named.map(|n| n.0))
            .or_else(|| function_key(name))
    }

    /// Whether Go's `unicode.IsPrint` holds for this code's rune.
    pub(crate) fn is_print(self) -> bool {
        crate::unicode::is_print(self.rune())
    }
}

/// The `rune` Go uses for a named key.
fn named_rune(code: KeyCode) -> u32 {
    let control = CONTROL.iter().find(|c| c.0 == code).map(|c| c.1);
    let before = || position(&BEFORE_F, code).map(|i| EXTENDED + 1 + i);
    let after = || position(&AFTER_F, code).map(|i| AFTER_F63 + i);
    control.or_else(before).or_else(after).unwrap_or(EXTENDED)
}

/// The code at `index` in `table`.
fn at(table: &[Named], index: u32) -> Option<KeyCode> {
    table.get(usize::try_from(index).ok()?).map(|n| n.0)
}

/// Where `code` is in `table`.
fn position(table: &[Named], code: KeyCode) -> Option<u32> {
    let i = table.iter().position(|n| n.0 == code)?;
    u32::try_from(i).ok()
}

/// The table row for a named key.
fn named(code: KeyCode) -> Option<&'static Named> {
    BEFORE_F.iter().chain(&AFTER_F).find(|n| n.0 == code)
}

/// `"f1"` to `"f63"`, written without leading zeros.
fn function_key(name: &str) -> Option<KeyCode> {
    let digits = name.strip_prefix('f')?;
    if digits.starts_with('0') {
        return None;
    }
    let n: u8 = digits.parse().ok()?;
    (1..=63).contains(&n).then_some(KeyCode::F(n))
}
