//! Keys as Bubble Tea v2 reports them: a code, modifiers, the text they
//! type, and how they print and match.

mod code;

pub use code::KeyCode;
use std::fmt::{self, Write as _};
use std::ops::{BitOr, BitOrAssign};

/// Modifier keys and lock states, Bubble Tea's `KeyMod`: combine them
/// with `|` and test them with [`KeyMod::contains`]. The bits are Go's,
/// in Go's order.
///
/// ```
/// use petitcarnets as tea;
/// use tea::{Key, KeyCode, KeyMod};
///
/// let mods = KeyMod::CTRL | KeyMod::SHIFT;
/// assert!(mods.contains(KeyMod::CTRL));
/// assert!(!mods.contains(KeyMod::ALT));
/// assert!(KeyMod::NONE.is_empty());
/// assert!(Key::new(KeyCode::Tab).with_mods(KeyMod::SHIFT) == "shift+tab");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyMod(i64);

impl KeyMod {
    /// No modifiers.
    pub const NONE: Self = Self(0);
    /// Shift.
    pub const SHIFT: Self = Self(1);
    /// Alt (Option on macOS).
    pub const ALT: Self = Self(1 << 1);
    /// Ctrl.
    pub const CTRL: Self = Self(1 << 2);
    /// Meta.
    pub const META: Self = Self(1 << 3);
    /// Hyper (kitty).
    pub const HYPER: Self = Self(1 << 4);
    /// Super: the Windows or Command key (kitty).
    pub const SUPER: Self = Self(1 << 5);
    /// Caps Lock is on (kitty).
    pub const CAPS_LOCK: Self = Self(1 << 6);
    /// Num Lock is on (kitty).
    pub const NUM_LOCK: Self = Self(1 << 7);
    /// Scroll Lock is on (Windows only in Bubble Tea).
    pub const SCROLL_LOCK: Self = Self(1 << 8);

    /// Whether every modifier in `mods` is set. Go's `KeyMod.Contains`.
    #[must_use]
    pub const fn contains(self, mods: Self) -> bool {
        self.0 & mods.0 == mods.0
    }

    /// Whether no modifier is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The modifiers as Go's integer, which a terminal can set to any
    /// value (`-1` for a zero modifier parameter).
    pub(crate) const fn bits(self) -> i64 {
        self.0
    }

    /// Modifiers from Go's integer.
    pub(crate) const fn from_bits(bits: i64) -> Self {
        Self(bits)
    }

    /// These modifiers without those in `mods`.
    pub(crate) const fn without(self, mods: Self) -> Self {
        Self(self.0 & !mods.0)
    }
}

impl BitOr for KeyMod {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for KeyMod {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// A key press or release: Bubble Tea's `Key`.
///
/// Compare one with a name, as in `key == "ctrl+c"`, which reads as Go's
/// `msg.String() == "ctrl+c"`, or use [`Key::matches`].
///
/// ```
/// use petitcarnets as tea;
/// use tea::{Key, KeyCode};
///
/// let key = Key::ctrl('c');
/// assert!(key == "ctrl+c");
/// assert!(key.matches_any(&["q", "ctrl+c"]));
/// assert!(Key::new(KeyCode::Enter) == "enter");
/// // Shift and `a` types "A", and is written as it types.
/// let shifted = Key::char('A');
/// assert_eq!(shifted.to_string(), "A");
/// assert_eq!(shifted.keystroke(), "shift+a");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    /// The text the key types; empty for keys that type nothing, such as
    /// arrows or most keys held with Ctrl or Alt.
    pub text: String,
    /// The modifiers held.
    pub mods: KeyMod,
    /// Which key.
    pub code: KeyCode,
    /// The shifted key, such as `'A'` for Shift and `a` (kitty and
    /// uppercase letters only).
    pub shifted_code: Option<KeyCode>,
    /// The key at this place on a US PC-101 layout (kitty only).
    pub base_code: Option<KeyCode>,
    /// Whether the key is held down and repeating (kitty only).
    pub is_repeat: bool,
}

/// Modifiers in the order Bubble Tea writes them, with the keys that
/// are that modifier and so do not repeat it.
const MOD_NAMES: [(KeyMod, &str, KeyCode, KeyCode); 6] = [
    (KeyMod::CTRL, "ctrl+", KeyCode::LeftCtrl, KeyCode::RightCtrl),
    (KeyMod::ALT, "alt+", KeyCode::LeftAlt, KeyCode::RightAlt),
    (
        KeyMod::SHIFT,
        "shift+",
        KeyCode::LeftShift,
        KeyCode::RightShift,
    ),
    (KeyMod::META, "meta+", KeyCode::LeftMeta, KeyCode::RightMeta),
    (
        KeyMod::HYPER,
        "hyper+",
        KeyCode::LeftHyper,
        KeyCode::RightHyper,
    ),
    (
        KeyMod::SUPER,
        "super+",
        KeyCode::LeftSuper,
        KeyCode::RightSuper,
    ),
];

/// Modifier names `Key::MatchString` reads.
const MATCH_MODS: [(&str, KeyMod); 9] = [
    ("ctrl", KeyMod::CTRL),
    ("alt", KeyMod::ALT),
    ("shift", KeyMod::SHIFT),
    ("meta", KeyMod::META),
    ("hyper", KeyMod::HYPER),
    ("super", KeyMod::SUPER),
    ("capslock", KeyMod::CAPS_LOCK),
    ("scrolllock", KeyMod::SCROLL_LOCK),
    ("numlock", KeyMod::NUM_LOCK),
];

impl Key {
    /// The key `code` with no modifiers and no text.
    #[must_use]
    pub const fn new(code: KeyCode) -> Self {
        Self {
            text: String::new(),
            mods: KeyMod::NONE,
            code,
            shifted_code: None,
            base_code: None,
            is_repeat: false,
        }
    }

    /// The key that types `c`, as a terminal reports it: `'A'` is Shift
    /// and `a` typing `"A"`, `' '` is Space typing `" "`.
    #[must_use]
    pub fn char(c: char) -> Self {
        let mut key = Self::new(KeyCode::from_rune(i64::from(u32::from(c))));
        key.text = c.to_string();
        if c.is_ascii_uppercase() {
            key.code = KeyCode::Char(c.to_ascii_lowercase());
            key.shifted_code = Some(KeyCode::Char(c));
            key.mods = KeyMod::SHIFT;
        }
        key
    }

    /// Ctrl and `c`, such as `Key::ctrl('c')` for `"ctrl+c"`.
    #[must_use]
    pub const fn ctrl(c: char) -> Self {
        Self::new(KeyCode::Char(c)).with_mods(KeyMod::CTRL)
    }

    /// The same key with `mods` held.
    #[must_use]
    pub const fn with_mods(mut self, mods: KeyMod) -> Self {
        self.mods = mods;
        self
    }

    /// The key as a keystroke, modifiers first: `"ctrl+alt+a"`,
    /// `"shift+tab"`. Unlike [`Display`](fmt::Display), it never gives
    /// the typed text, so Shift and `/` is `"shift+/"`, not `"?"`. Go's
    /// `Key.Keystroke`.
    #[must_use]
    pub fn keystroke(&self) -> String {
        let mut s = String::new();
        let _ = self.write_keystroke(&mut s);
        s
    }

    /// Whether the key matches `name`, such as `"enter"`, `"ctrl+a"` or
    /// `"?"`: by code and exact modifiers, or by the text it types. Go's
    /// `Key.MatchString` with one string.
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        let (mods, code, text) = parse_match(name);
        let by_code = self.mods == mods && code == Some(self.code);
        by_code || (!self.text.is_empty() && self.text == text)
    }

    /// Whether the key matches any of `names`. Go's `Key.MatchString`.
    #[must_use]
    pub fn matches_any(&self, names: &[&str]) -> bool {
        names.iter().any(|name| self.matches(name))
    }

    fn write_keystroke(&self, w: &mut impl fmt::Write) -> fmt::Result {
        for (m, name, left, right) in MOD_NAMES {
            if self.mods.contains(m) && ![left, right].contains(&self.code) {
                w.write_str(name)?;
            }
        }
        self.write_code(w)
    }

    /// The key itself, after the modifiers.
    fn write_code(&self, w: &mut impl fmt::Write) -> fmt::Result {
        if let KeyCode::F(n) = self.code {
            return write!(w, "f{n}");
        }
        if let Some(name) = self.code.name() {
            return w.write_str(name);
        }
        match self.base_code.unwrap_or(self.code) {
            KeyCode::Space => w.write_str("space"),
            KeyCode::Extended => w.write_str(&self.text),
            code => {
                w.write_char(char::from_u32(code.rune()).unwrap_or(crate::unicode::REPLACEMENT))
            }
        }
    }
}

/// What `name` asks for: modifiers, a code (Go's zero rune is `None`),
/// and the text it types.
fn parse_match(name: &str) -> (KeyMod, Option<KeyCode>, String) {
    let (mut mods, mut code, mut text) = (KeyMod::NONE, None, String::new());
    for part in name.split('+') {
        if let Some((_, m)) = MATCH_MODS.iter().find(|(n, _)| *n == part) {
            mods |= *m;
            continue;
        }
        let named = KeyCode::from_name(part);
        code = Some(named.unwrap_or_else(|| code_of(part, &mut text)));
    }
    if text.is_empty() {
        text = typed_text(mods, code);
    }
    (mods, code, text)
}

/// The text a printable `code` types with `mods`, if they are only
/// Shift and Caps Lock (which make it upper case); else nothing.
fn typed_text(mods: KeyMod, code: Option<KeyCode>) -> String {
    let plain = mods.without(KeyMod::SHIFT | KeyMod::CAPS_LOCK).is_empty();
    let Some(c) = code.filter(|c| plain && c.is_print()) else {
        return String::new();
    };
    let c = char::from_u32(c.rune()).unwrap_or(crate::unicode::REPLACEMENT);
    let shifted = mods.contains(KeyMod::SHIFT) || mods.contains(KeyMod::CAPS_LOCK);
    let typed = if shifted {
        crate::unicode::to_upper(c)
    } else {
        c
    };
    typed.to_string()
}

/// The code of a part that names no key: its character, or extended
/// text (setting `text`) when it is not exactly one character.
fn code_of(part: &str, text: &mut String) -> KeyCode {
    let mut chars = part.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return KeyCode::from_rune(i64::from(u32::from(c)));
    }
    *text = part.to_string();
    KeyCode::Extended
}

impl fmt::Display for Key {
    /// The key as Bubble Tea's `Key.String` writes it: the text it types
    /// if any (so Shift and `/` is `"?"`), else the keystroke, such as
    /// `"ctrl+c"`, `"space"` or `"shift+tab"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.text.is_empty() && self.text != " " {
            return f.write_str(&self.text);
        }
        self.write_keystroke(f)
    }
}

impl PartialEq<&str> for Key {
    /// Whether the key is written `name`, so `key == "ctrl+c"` reads as
    /// Go's `msg.String() == "ctrl+c"`. Compares as it writes, without
    /// allocating.
    fn eq(&self, name: &&str) -> bool {
        let mut rest = Prefix(Some(name));
        let _ = write!(rest, "{self}");
        rest.0 == Some("")
    }
}

/// What is left of a name after the text written so far; `None` once the
/// text stops matching.
struct Prefix<'a>(Option<&'a str>);

impl fmt::Write for Prefix<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.and_then(|rest| rest.strip_prefix(s));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
