//! The decoder against Bubble Tea's own: every line of
//! `tests/fixtures/input.tsv`, written by `scripts/input-fixture` from Go,
//! must come out the same here.

use super::scanner::Scanner;
use super::*;
use crate::mouse::Mouse;
use std::fmt::Write as _;

/// Bytes as lower-case hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Hex back to bytes.
fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

/// A code as the fixture writes it.
fn code(c: KeyCode) -> String {
    match c.rune() {
        r if r >= 0x11_0000 => format!("K{}", r - 0x11_0000),
        r => format!("U+{r:04X}"),
    }
}

fn key(k: &Key) -> String {
    let opt = |c: Option<KeyCode>| c.map_or_else(|| "-".to_string(), code);
    format!(
        "code={} text={} mod={} shifted={} base={} repeat={} str={} ks={}",
        code(k.code),
        hex(k.text.as_bytes()),
        k.mods.bits(),
        opt(k.shifted_code),
        opt(k.base_code),
        k.is_repeat,
        hex(k.to_string().as_bytes()),
        hex(k.keystroke().as_bytes()),
    )
}

fn mouse(kind: &str, m: &Mouse, text: &str) -> String {
    format!(
        "{kind} x={} y={} button={} mod={} str={}",
        m.x,
        m.y,
        m.button.name(),
        m.mods.bits(),
        hex(text.as_bytes())
    )
}

fn colour(kind: &str, c: crate::color::Rgb) -> String {
    format!(
        "{kind} #{:02x}{:02x}{:02x} dark={}",
        c.r,
        c.g,
        c.b,
        c.is_dark()
    )
}

/// An event as the fixture writes it.
fn event(e: &Event) -> String {
    match e {
        Event::Input(input) => input_text(input),
        Event::Unknown => "other UnknownEvent".into(),
        Event::Other(name) => format!("other {name}"),
        Event::NoColour("ForegroundColorEvent") => "fg nil".into(),
        Event::NoColour("BackgroundColorEvent") => "bg nil".into(),
        Event::NoColour(_) => "cursor-colour nil".into(),
        Event::Ignored | Event::Multi(_) => format!("unexpected {e:?}"),
    }
}

fn input_text(input: &Input) -> String {
    match input {
        Input::KeyPress(k) => format!("press {}", key(k)),
        Input::KeyRelease(k) => format!("release {}", key(k)),
        Input::MouseClick(m) => mouse("click", m, &m.to_string()),
        Input::MouseRelease(m) => mouse("mouse-release", m, &m.to_string()),
        Input::MouseWheel(m) => mouse("wheel", m, &m.to_string()),
        Input::MouseMotion(m) => mouse("motion", m, &m.motion_string()),
        Input::Paste(p) => format!("paste {}", hex(p.as_bytes())),
        Input::PasteStart => "paste-start".into(),
        Input::PasteEnd => "paste-end".into(),
        input => reply_text(input),
    }
}

fn reply_text(input: &Input) -> String {
    match input {
        Input::Focus => "focus".into(),
        Input::Blur => "blur".into(),
        Input::CursorPosition { x, y } => format!("cursor x={x} y={y}"),
        Input::KeyboardEnhancements(r) => format!("keyboard flags={}", r.flags),
        Input::ModeReport { mode, value } => {
            let (dec, n) = match mode {
                crate::mode::ReportedMode::Dec(n) => (true, n),
                crate::mode::ReportedMode::Ansi(n) => (false, n),
            };
            format!("mode dec={dec} mode={n} value={}", u8::from(*value))
        }
        Input::TerminalVersion(v) => format!("version {}", hex(v.as_bytes())),
        Input::ForegroundColor(c) => colour("fg", *c),
        Input::BackgroundColor(c) => colour("bg", *c),
        Input::CursorColor(c) => colour("cursor-colour", *c),
        other => format!("unexpected {other:?}"),
    }
}

/// Feeds `chunks` as Go's reader does: one scan per read, then one
/// expired scan at the end of input.
pub(super) fn replay(chunks: &[Vec<u8>]) -> Vec<Event> {
    let mut scanner = Scanner::default();
    let (mut pending, mut events) = (Vec::new(), Vec::new());
    for chunk in chunks {
        pending.extend_from_slice(chunk);
        let used = scanner.scan(&pending, false, &mut events);
        let _ = pending.drain(..used);
    }
    let _ = scanner.scan(&pending, true, &mut events);
    events
}

/// The fixture's lines: input chunks and the expected line.
pub(super) fn fixture() -> Vec<(Vec<Vec<u8>>, String)> {
    let text = include_str!("../../tests/fixtures/input.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|line| {
            let input = line.split('\t').next().unwrap();
            let chunks = input.split(' ').map(unhex).collect();
            (chunks, line.to_string())
        })
        .collect()
}

/// The line the Rust decoder writes for `chunks`.
fn line(chunks: &[Vec<u8>]) -> String {
    let input: Vec<String> = chunks.iter().map(|c| hex(c)).collect();
    let mut fields = vec![input.join(" ")];
    fields.extend(replay(chunks).iter().map(event));
    fields.join("\t")
}

#[test]
fn every_fixture_line_matches_bubble_tea() {
    let lines = fixture();
    assert!(lines.len() >= 500, "{} lines", lines.len());
    let wrong: Vec<String> = lines
        .iter()
        .filter_map(|(chunks, want)| {
            let got = line(chunks);
            (got != *want).then(|| format!("want {want}\n got {got}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} lines differ:\n{}",
        wrong.len(),
        lines.len(),
        wrong.join("\n")
    );
}
