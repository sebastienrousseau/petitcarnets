//! Keys, and turning the bytes a terminal sends into them.

/// A key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// Ctrl and a letter, such as Ctrl-C as `Ctrl('c')`.
    Ctrl(char),
    /// Enter (Return).
    Enter,
    /// Escape.
    Esc,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Shift-Tab.
    BackTab,
    /// The up arrow.
    Up,
    /// The down arrow.
    Down,
    /// The left arrow.
    Left,
    /// The right arrow.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// A function key, F1 to F12.
    F(u8),
}

/// Names as Bubble Tea's `Key.String()` writes them.
const NAMES: [(Key, &str); 16] = [
    (Key::Char(' '), "space"),
    (Key::Enter, "enter"),
    (Key::Esc, "esc"),
    (Key::Backspace, "backspace"),
    (Key::Tab, "tab"),
    (Key::BackTab, "shift+tab"),
    (Key::Up, "up"),
    (Key::Down, "down"),
    (Key::Left, "left"),
    (Key::Right, "right"),
    (Key::Home, "home"),
    (Key::End, "end"),
    (Key::PageUp, "pgup"),
    (Key::PageDown, "pgdown"),
    (Key::Insert, "insert"),
    (Key::Delete, "delete"),
];

impl std::fmt::Display for Key {
    /// The key as Bubble Tea writes it: `"a"`, `"space"`, `"ctrl+c"`,
    /// `"shift+tab"`, `"pgup"`, `"f5"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((_, name)) = NAMES.iter().find(|(k, _)| k == self) {
            return f.write_str(name);
        }
        match self {
            Self::Char(c) => write!(f, "{c}"),
            Self::Ctrl(c) => write!(f, "ctrl+{c}"),
            Self::F(n) => write!(f, "f{n}"),
            _ => Ok(()),
        }
    }
}

impl PartialEq<&str> for Key {
    /// Whether the key is written `name`, so `key == "ctrl+c"` reads as
    /// Go's `msg.String() == "ctrl+c"`. Compares as it writes, without
    /// allocating.
    fn eq(&self, name: &&str) -> bool {
        use std::fmt::Write as _;
        let mut rest = Prefix(Some(name));
        let _ = write!(rest, "{self}");
        rest.0 == Some("")
    }
}

/// What is left of a name after the text written so far; `None` once the
/// text stops matching.
struct Prefix<'a>(Option<&'a str>);

impl std::fmt::Write for Prefix<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0 = self.0.and_then(|rest| rest.strip_prefix(s));
        Ok(())
    }
}

/// The keys in `bytes`. A sequence cut off at the end of the buffer is
/// left unread: the number of bytes consumed is returned with the keys,
/// and the rest should be kept for the next read. A lone Escape at the
/// very end is taken as the Escape key when `complete` says no more bytes
/// are coming.
#[must_use]
pub fn parse(bytes: &[u8], complete: bool) -> (Vec<Key>, usize) {
    let mut keys = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match one(&bytes[at..], complete) {
            Some((key, len)) => {
                keys.extend(key);
                at += len;
            }
            None => break,
        }
    }
    (keys, at)
}

/// The key at the start of `bytes` and its length; `None` if the bytes
/// so far are an incomplete sequence. A recognised but unsupported
/// sequence is consumed without a key.
fn one(bytes: &[u8], complete: bool) -> Option<(Option<Key>, usize)> {
    match bytes[0] {
        0x1b => escape(bytes, complete),
        b'\r' | b'\n' => Some((Some(Key::Enter), 1)),
        b'\t' => Some((Some(Key::Tab), 1)),
        0x7f | 0x08 => Some((Some(Key::Backspace), 1)),
        c @ 0x01..=0x1a => Some((Some(Key::Ctrl(char::from(c - 1 + b'a'))), 1)),
        0x00 | 0x1c..=0x1f => Some((None, 1)),
        _ => utf8(bytes),
    }
}

/// An escape sequence, or the Escape key.
fn escape(bytes: &[u8], complete: bool) -> Option<(Option<Key>, usize)> {
    match bytes.get(1) {
        None => complete.then_some((Some(Key::Esc), 1)),
        Some(b'[') => csi(bytes),
        Some(b'O') => ss3(bytes),
        Some(_) => Some((Some(Key::Esc), 1)),
    }
}

/// The final byte of an `ESC [ x` sequence and its key.
const CSI_FINALS: [(u8, Key); 7] = [
    (b'A', Key::Up),
    (b'B', Key::Down),
    (b'C', Key::Right),
    (b'D', Key::Left),
    (b'H', Key::Home),
    (b'F', Key::End),
    (b'Z', Key::BackTab),
];

/// The byte after `ESC O` and its key.
const SS3_KEYS: [(u8, Key); 10] = [
    (b'A', Key::Up),
    (b'B', Key::Down),
    (b'C', Key::Right),
    (b'D', Key::Left),
    (b'H', Key::Home),
    (b'F', Key::End),
    (b'P', Key::F(1)),
    (b'Q', Key::F(2)),
    (b'R', Key::F(3)),
    (b'S', Key::F(4)),
];

/// The number in `ESC [ n ~` and its key.
const TILDE_KEYS: [(&str, Key); 18] = [
    ("1", Key::Home),
    ("7", Key::Home),
    ("2", Key::Insert),
    ("3", Key::Delete),
    ("4", Key::End),
    ("8", Key::End),
    ("5", Key::PageUp),
    ("6", Key::PageDown),
    ("11", Key::F(1)),
    ("12", Key::F(2)),
    ("13", Key::F(3)),
    ("14", Key::F(4)),
    ("15", Key::F(5)),
    ("17", Key::F(6)),
    ("18", Key::F(7)),
    ("19", Key::F(8)),
    ("20", Key::F(9)),
    ("21", Key::F(10)),
];

/// `F11` and `F12`, which do not fit the table above's run.
const TILDE_F11_F12: [(&str, Key); 2] = [("23", Key::F(11)), ("24", Key::F(12))];

/// The key paired with `code` in `table`.
fn lookup<C: PartialEq + Copy>(table: &[(C, Key)], code: C) -> Option<Key> {
    table.iter().find(|(c, _)| *c == code).map(|(_, k)| *k)
}

/// `ESC [ ...`: arrows, Home/End and the `~` keys.
fn csi(bytes: &[u8]) -> Option<(Option<Key>, usize)> {
    let end = bytes
        .iter()
        .skip(2)
        .position(|b| (0x40..=0x7e).contains(b))?
        + 2;
    let key = if bytes[end] == b'~' {
        let params = std::str::from_utf8(&bytes[2..end]).unwrap_or("");
        tilde(params.split(';').next().unwrap_or(""))
    } else {
        lookup(&CSI_FINALS, bytes[end])
    };
    Some((key, end + 1))
}

/// `ESC O x`: what some terminals send for arrows, Home/End and F1 to F4.
fn ss3(bytes: &[u8]) -> Option<(Option<Key>, usize)> {
    Some((lookup(&SS3_KEYS, *bytes.get(2)?), 3))
}

/// The key an `ESC [ n ~` sequence names.
fn tilde(n: &str) -> Option<Key> {
    lookup(&TILDE_KEYS, n).or_else(|| lookup(&TILDE_F11_F12, n))
}

/// One UTF-8 character; `None` if it is cut off.
fn utf8(bytes: &[u8]) -> Option<(Option<Key>, usize)> {
    let len = match bytes[0] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => return Some((None, 1)),
    };
    let char_bytes = bytes.get(..len)?;
    let key = std::str::from_utf8(char_bytes)
        .ok()
        .and_then(|s| s.chars().next())
        .map(Key::Char);
    Some((key, len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bytes: &[u8]) -> Vec<Key> {
        parse(bytes, true).0
    }

    #[test]
    fn plain_characters_and_unicode() {
        assert_eq!(
            keys(b"aZ9"),
            vec![Key::Char('a'), Key::Char('Z'), Key::Char('9')]
        );
        assert_eq!(
            keys("é€🦀".as_bytes()),
            vec![Key::Char('é'), Key::Char('€'), Key::Char('🦀')]
        );
    }

    #[test]
    fn control_keys() {
        assert_eq!(
            keys(b"\x03\r\t\x7f"),
            vec![Key::Ctrl('c'), Key::Enter, Key::Tab, Key::Backspace]
        );
        assert_eq!(keys(b"\x01\x1a"), vec![Key::Ctrl('a'), Key::Ctrl('z')]);
    }

    #[test]
    fn arrows_both_ways() {
        assert_eq!(
            keys(b"\x1b[A\x1b[B\x1b[C\x1b[D"),
            vec![Key::Up, Key::Down, Key::Right, Key::Left]
        );
        assert_eq!(keys(b"\x1bOA\x1bOD"), vec![Key::Up, Key::Left]);
    }

    #[test]
    fn tilde_keys_and_function_keys() {
        assert_eq!(
            keys(b"\x1b[3~\x1b[5~\x1b[6~\x1b[2~"),
            vec![Key::Delete, Key::PageUp, Key::PageDown, Key::Insert]
        );
        assert_eq!(
            keys(b"\x1bOP\x1b[15~\x1b[24~"),
            vec![Key::F(1), Key::F(5), Key::F(12)]
        );
        assert_eq!(
            keys(b"\x1b[1~\x1b[4~\x1b[H\x1b[F\x1b[Z"),
            vec![Key::Home, Key::End, Key::Home, Key::End, Key::BackTab]
        );
        // Modifiers after a semicolon do not change the key.
        assert_eq!(keys(b"\x1b[3;5~"), vec![Key::Delete]);
    }

    #[test]
    fn a_cut_off_sequence_waits_for_more_bytes() {
        assert_eq!(parse(b"a\x1b[", false), (vec![Key::Char('a')], 1));
        assert_eq!(parse(&"é".as_bytes()[..1], false), (vec![], 0));
        assert_eq!(parse(b"\x1b", false), (vec![], 0));
    }

    #[test]
    fn a_lone_escape_is_the_escape_key_when_nothing_follows() {
        assert_eq!(keys(b"\x1b"), vec![Key::Esc]);
        assert_eq!(keys(b"\x1bx"), vec![Key::Esc, Key::Char('x')]);
    }

    #[test]
    fn keys_are_written_as_bubble_tea_writes_them() {
        let cases = [
            (Key::Char('a'), "a"),
            (Key::Char('?'), "?"),
            (Key::Char(' '), "space"),
            (Key::Ctrl('c'), "ctrl+c"),
            (Key::Enter, "enter"),
            (Key::Esc, "esc"),
            (Key::Backspace, "backspace"),
            (Key::Tab, "tab"),
            (Key::BackTab, "shift+tab"),
            (Key::Up, "up"),
            (Key::PageUp, "pgup"),
            (Key::PageDown, "pgdown"),
            (Key::Delete, "delete"),
            (Key::F(5), "f5"),
        ];
        for (key, name) in cases {
            assert_eq!(key.to_string(), name);
            assert!(key == name, "{key:?} == {name:?}");
        }
        assert!(Key::Char('q') != "ctrl+q");
    }

    #[test]
    fn unknown_sequences_are_consumed_without_a_key() {
        assert_eq!(parse(b"\x1b[99~a", true), (vec![Key::Char('a')], 6));
        assert_eq!(keys(b"\x1b[M"), Vec::<Key>::new());
        assert_eq!(keys(&[0xff, b'a']), vec![Key::Char('a')]);
    }
}
