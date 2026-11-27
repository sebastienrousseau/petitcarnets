//! A screen cell: a grapheme, its width, its style and its hyperlink,
//! and the SGR codes that read and write styles.

use std::fmt::Write as _;
use std::rc::Rc;

/// A colour in a cell's style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Colour {
    /// The terminal's own colour.
    #[default]
    Default,
    /// One of the 16 standard colours (8 to 15 are the bright ones).
    Basic(u8),
    /// One of the 256 palette colours.
    Indexed(u8),
    /// A 24-bit colour.
    Rgb(u8, u8, u8),
}

/// Attribute bits of a [`Pen`].
pub(crate) mod attr {
    /// Bold (SGR 1).
    pub(crate) const BOLD: u8 = 1;
    /// Faint (SGR 2).
    pub(crate) const FAINT: u8 = 1 << 1;
    /// Italic (SGR 3).
    pub(crate) const ITALIC: u8 = 1 << 2;
    /// Slow blink (SGR 5).
    pub(crate) const BLINK: u8 = 1 << 3;
    /// Rapid blink (SGR 6).
    pub(crate) const RAPID_BLINK: u8 = 1 << 4;
    /// Reverse video (SGR 7).
    pub(crate) const REVERSE: u8 = 1 << 5;
    /// Concealed (SGR 8).
    pub(crate) const CONCEAL: u8 = 1 << 6;
    /// Crossed out (SGR 9).
    pub(crate) const STRIKE: u8 = 1 << 7;
}

/// Each attribute bit, the code that sets it, and the code that clears
/// it.
const ATTRIBUTES: [(u8, u8, u8); 8] = [
    (attr::BOLD, 1, 22),
    (attr::FAINT, 2, 22),
    (attr::ITALIC, 3, 23),
    (attr::BLINK, 5, 25),
    (attr::RAPID_BLINK, 6, 25),
    (attr::REVERSE, 7, 27),
    (attr::CONCEAL, 8, 28),
    (attr::STRIKE, 9, 29),
];

/// A cell's style: colours, attributes and underline. Go's `uv.Style`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Pen {
    /// Text colour.
    pub(crate) fg: Colour,
    /// Background colour.
    pub(crate) bg: Colour,
    /// Underline colour (SGR 58).
    pub(crate) underline_colour: Colour,
    /// Attribute bits, from [`attr`].
    pub(crate) attrs: u8,
    /// Underline style: 0 none, 1 single, 2 double, 3 curly, 4 dotted,
    /// 5 dashed.
    pub(crate) underline: u8,
}

/// A hyperlink: OSC 8's parameters and URL, as `params;url`.
pub(crate) type Link = Option<Rc<str>>;

/// What a cell shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Glyph {
    /// One code point.
    Char(char),
    /// A grapheme cluster of several code points.
    Cluster(Box<str>),
    /// Covered by the wide cell to its left.
    Continuation,
}

/// One cell of the screen.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Cell {
    /// What it shows.
    pub(crate) glyph: Glyph,
    /// How many columns it takes (0 for a continuation).
    pub(crate) width: u8,
    /// Its style.
    pub(crate) pen: Pen,
    /// Its hyperlink, if any.
    pub(crate) link: Link,
}

impl Cell {
    /// A space in the default style.
    pub(crate) const BLANK: Self = Self {
        glyph: Glyph::Char(' '),
        width: 1,
        pen: Pen::DEFAULT,
        link: None,
    };

    /// A cell showing `text`, `width` columns wide.
    pub(crate) fn new(text: &str, width: u8, pen: Pen, link: Link) -> Self {
        let mut chars = text.chars();
        let glyph = match (chars.next(), chars.next()) {
            (Some(c), None) => Glyph::Char(c),
            _ => Glyph::Cluster(text.into()),
        };
        Self {
            glyph,
            width,
            pen,
            link,
        }
    }

    /// The right part of a wide cell, in its style.
    pub(crate) fn continuation(pen: Pen, link: Link) -> Self {
        Self {
            glyph: Glyph::Continuation,
            width: 0,
            pen,
            link,
        }
    }

    /// Whether this is a space in the default style with no link: what
    /// erasing leaves behind.
    pub(crate) fn is_blank(&self) -> bool {
        *self == Self::BLANK
    }

    /// Whether this is covered by a wide cell to its left.
    pub(crate) fn is_continuation(&self) -> bool {
        self.glyph == Glyph::Continuation
    }

    /// Appends what the cell shows to `out`.
    pub(crate) fn push_text(&self, out: &mut String) {
        match &self.glyph {
            Glyph::Char(c) => out.push(*c),
            Glyph::Cluster(s) => out.push_str(s),
            Glyph::Continuation => {}
        }
    }

    /// Adds `mark`, a zero-width cluster, to what the cell shows.
    pub(crate) fn append(&mut self, mark: &str) {
        let mut text = String::new();
        self.push_text(&mut text);
        text.push_str(mark);
        self.glyph = Glyph::Cluster(text.into());
    }
}

impl Pen {
    /// No colour and no attributes.
    pub(crate) const DEFAULT: Self = Self {
        fg: Colour::Default,
        bg: Colour::Default,
        underline_colour: Colour::Default,
        attrs: 0,
        underline: 0,
    };

    /// Applies an SGR sequence's parameters (`1;31`, `38:2::1:2:3`),
    /// as Go's `uv.ReadStyle` reads them. Unknown codes are ignored.
    pub(crate) fn apply_sgr(&mut self, params: &str) {
        let list: Vec<&str> = params.split(';').collect();
        let mut i = 0;
        while i < list.len() {
            i += self.apply_one(&list, i);
        }
    }

    /// Applies the parameter at `list[i]`; how many parameters it took.
    fn apply_one(&mut self, list: &[&str], i: usize) -> usize {
        let mut parts = list[i].split(':');
        let code = number(parts.next().unwrap_or(""));
        let sub: Vec<&str> = parts.collect();
        match code {
            38 | 48 | 58 => {
                let (colour, used) = read_colour(&sub, &list[i + 1..]);
                self.set_colour(code, colour);
                1 + used
            }
            4 => {
                self.underline = match sub.first().map(|s| number(s)) {
                    Some(n @ 0..=5) => u8::try_from(n).unwrap_or(1),
                    _ => 1,
                };
                1
            }
            _ => {
                self.apply_code(code);
                1
            }
        }
    }

    /// Sets the colour SGR `code` (38, 48 or 58) names, if it was read.
    fn set_colour(&mut self, code: u32, colour: Option<Colour>) {
        let Some(colour) = colour else { return };
        match code {
            38 => self.fg = colour,
            48 => self.bg = colour,
            _ => self.underline_colour = colour,
        }
    }

    /// Applies one plain code.
    fn apply_code(&mut self, code: u32) {
        if let Some((bit, _, _)) = ATTRIBUTES.iter().find(|a| u32::from(a.1) == code) {
            self.attrs |= bit;
            return;
        }
        if let Some(reset) = ATTRIBUTES
            .iter()
            .find(|a| u32::from(a.2) == code)
            .map(|a| a.2)
        {
            for (bit, _, off) in ATTRIBUTES {
                if off == reset {
                    self.attrs &= !bit;
                }
            }
            return;
        }
        self.apply_colour_code(code);
    }

    /// Reset, underline off, and the 16-colour codes.
    fn apply_colour_code(&mut self, code: u32) {
        if let Some((colour, background)) = basic_colour(code) {
            if background {
                self.bg = colour;
            } else {
                self.fg = colour;
            }
            return;
        }
        match code {
            0 => *self = Self::DEFAULT,
            24 => self.underline = 0,
            39 => self.fg = Colour::Default,
            49 => self.bg = Colour::Default,
            59 => self.underline_colour = Colour::Default,
            _ => {}
        }
    }

    /// The SGR sequence that changes the terminal's pen from `self` to
    /// `to`, empty if they are the same.
    pub(crate) fn transition(&self, to: &Self) -> String {
        if self == to {
            return String::new();
        }
        if *to == Self::DEFAULT {
            return "\x1b[m".to_string();
        }
        let mut codes = String::new();
        let from = if self.must_reset(to) {
            codes.push('0');
            Self::DEFAULT
        } else {
            *self
        };
        from.push_changes(to, &mut codes);
        format!("\x1b[{codes}m")
    }

    /// Whether going to `to` turns something off, which a reset does in
    /// fewer bytes than codes for each.
    fn must_reset(&self, to: &Self) -> bool {
        let off = |a: Colour, b: Colour| a != Colour::Default && b == Colour::Default;
        self.attrs & !to.attrs != 0
            || (self.underline != 0 && to.underline == 0)
            || off(self.fg, to.fg)
            || off(self.bg, to.bg)
            || off(self.underline_colour, to.underline_colour)
    }

    /// Appends the codes that add what `to` has and `self` lacks.
    fn push_changes(&self, to: &Self, codes: &mut String) {
        let mut push = |code: &str| {
            if !codes.is_empty() {
                codes.push(';');
            }
            codes.push_str(code);
        };
        for (bit, on, _) in ATTRIBUTES {
            if to.attrs & bit != 0 && self.attrs & bit == 0 {
                push(&on.to_string());
            }
        }
        if to.underline != self.underline {
            push(&underline_code(to.underline));
        }
        for (code, a, b) in [
            (38, self.fg, to.fg),
            (48, self.bg, to.bg),
            (58, self.underline_colour, to.underline_colour),
        ] {
            if a != b {
                push(&colour_code(code, b));
            }
        }
    }
}

/// One of the 16 colours an SGR code (30 to 37, 40 to 47, 90 to 97 or
/// 100 to 107) sets, and whether it is the background.
fn basic_colour(code: u32) -> Option<(Colour, bool)> {
    let (base, bright, background) = match code {
        30..=37 => (30, 0, false),
        40..=47 => (40, 0, true),
        90..=97 => (90, 8, false),
        100..=107 => (100, 8, true),
        _ => return None,
    };
    let n = u8::try_from(code - base).ok()?;
    Some((Colour::Basic(n + bright), background))
}

/// The code for underline style `style` (1 to 5).
fn underline_code(style: u8) -> String {
    if style == 1 {
        "4".to_string()
    } else {
        format!("4:{style}")
    }
}

/// The code that sets colour `c` for SGR `code` (38, 48 or 58).
fn colour_code(code: u8, c: Colour) -> String {
    let mut out = String::new();
    match (code, c) {
        (38 | 48, Colour::Basic(n)) => {
            let base = if code == 38 { 30 } else { 40 };
            let n = u16::from(n);
            let value = if n < 8 { base + n } else { base + 60 + n - 8 };
            let _ = write!(out, "{value}");
        }
        (_, Colour::Basic(n) | Colour::Indexed(n)) => {
            let _ = write!(out, "{code};5;{n}");
        }
        (_, Colour::Rgb(r, g, b)) => {
            let _ = write!(out, "{code};2;{r};{g};{b}");
        }
        (_, Colour::Default) => {
            let _ = write!(out, "{}", code + 1);
        }
    }
    out
}

/// A parameter's value; empty or unreadable is 0.
fn number(s: &str) -> u32 {
    s.parse().unwrap_or(0)
}

/// A colour after 38, 48 or 58: from the code's own `:` parts (`sub`),
/// or else from the `;` parameters that follow (`rest`). The colour, and
/// how many of `rest` it took.
fn read_colour(sub: &[&str], rest: &[&str]) -> (Option<Colour>, usize) {
    if !sub.is_empty() {
        return (colon_colour(sub), 0);
    }
    let value = |i: usize| rest.get(i).map(|s| number(s));
    match value(0) {
        Some(5) => (value(1).map(indexed), 2),
        Some(2) => match (value(1), value(2), value(3)) {
            (Some(r), Some(g), Some(b)) => (Some(rgb(r, g, b)), 4),
            _ => (None, rest.len().min(4)),
        },
        _ => (None, 0),
    }
}

/// A colour written with colons: `5:n`, `2:r:g:b` or `2:id:r:g:b`.
fn colon_colour(sub: &[&str]) -> Option<Colour> {
    let values: Vec<u32> = sub.iter().map(|s| number(s)).collect();
    match values.as_slice() {
        [5, n, ..] => Some(indexed(*n)),
        [2, _, r, g, b, ..] | [2, r, g, b] => Some(rgb(*r, *g, *b)),
        _ => None,
    }
}

/// A palette colour; out of range values wrap as Go's do.
fn indexed(n: u32) -> Colour {
    Colour::Indexed(n.to_le_bytes()[0])
}

/// A 24-bit colour; out of range channels wrap as Go's do.
fn rgb(r: u32, g: u32, b: u32) -> Colour {
    Colour::Rgb(r.to_le_bytes()[0], g.to_le_bytes()[0], b.to_le_bytes()[0])
}

#[cfg(test)]
mod tests;
