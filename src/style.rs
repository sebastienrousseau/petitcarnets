//! Colour and emphasis, as ANSI escape codes, and the visible width of
//! styled text.

/// A terminal colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// One of the 16 standard colours, 0 to 15 (8 to 15 are the bright ones).
    Ansi(u8),
    /// One of the 256 palette colours.
    Palette(u8),
    /// A 24-bit colour.
    Rgb(u8, u8, u8),
}

impl Color {
    /// Black.
    pub const BLACK: Self = Self::Ansi(0);
    /// Red.
    pub const RED: Self = Self::Ansi(1);
    /// Green.
    pub const GREEN: Self = Self::Ansi(2);
    /// Yellow.
    pub const YELLOW: Self = Self::Ansi(3);
    /// Blue.
    pub const BLUE: Self = Self::Ansi(4);
    /// Magenta.
    pub const MAGENTA: Self = Self::Ansi(5);
    /// Cyan.
    pub const CYAN: Self = Self::Ansi(6);
    /// White.
    pub const WHITE: Self = Self::Ansi(7);
    /// Bright black, the usual grey.
    pub const GREY: Self = Self::Ansi(8);

    /// The SGR parameters for this colour, as foreground or background.
    fn sgr(self, background: bool) -> String {
        match self {
            Self::Ansi(n) => {
                let base = match (background, n < 8) {
                    (false, true) => 30,
                    (false, false) => 90 - 8,
                    (true, true) => 40,
                    (true, false) => 100 - 8,
                };
                (base + u16::from(n.min(15))).to_string()
            }
            Self::Palette(n) => format!("{};5;{n}", if background { 48 } else { 38 }),
            Self::Rgb(r, g, b) => format!("{};2;{r};{g};{b}", if background { 48 } else { 38 }),
        }
    }
}

/// Emphasis attributes, as bits: the SGR code of each is its position
/// in [`ATTRIBUTES`].
const BOLD: u8 = 1;
const DIM: u8 = 1 << 1;
const ITALIC: u8 = 1 << 2;
const UNDERLINE: u8 = 1 << 3;
const REVERSE: u8 = 1 << 4;

/// Each attribute bit and its SGR code, in output order.
const ATTRIBUTES: [(u8, &str); 5] = [
    (BOLD, "1"),
    (DIM, "2"),
    (ITALIC, "3"),
    (UNDERLINE, "4"),
    (REVERSE, "7"),
];

/// A style: colours and emphasis, applied with [`Style::paint`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    attributes: u8,
}

impl Style {
    /// No colour and no emphasis.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            attributes: 0,
        }
    }

    /// With this text colour.
    #[must_use]
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// With this background colour.
    #[must_use]
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Bold.
    #[must_use]
    pub const fn bold(self) -> Self {
        self.with(BOLD)
    }

    /// Dimmed.
    #[must_use]
    pub const fn dim(self) -> Self {
        self.with(DIM)
    }

    /// Italic.
    #[must_use]
    pub const fn italic(self) -> Self {
        self.with(ITALIC)
    }

    /// Underlined.
    #[must_use]
    pub const fn underline(self) -> Self {
        self.with(UNDERLINE)
    }

    /// Foreground and background swapped.
    #[must_use]
    pub const fn reverse(self) -> Self {
        self.with(REVERSE)
    }

    /// With an attribute bit set.
    const fn with(mut self, bit: u8) -> Self {
        self.attributes |= bit;
        self
    }

    /// `text` in this style, reset after it. A plain style returns the
    /// text unchanged.
    #[must_use]
    pub fn paint(&self, text: &str) -> String {
        let codes = self.codes();
        if codes.is_empty() {
            text.to_string()
        } else {
            format!("\x1b[{}m{text}\x1b[0m", codes.join(";"))
        }
    }

    /// The SGR parameters, in a fixed order.
    fn codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = ATTRIBUTES
            .iter()
            .filter(|(bit, _)| self.attributes & bit != 0)
            .map(|(_, c)| (*c).to_string())
            .collect();
        codes.extend(self.fg.map(|c| c.sgr(false)));
        codes.extend(self.bg.map(|c| c.sgr(true)));
        codes
    }
}

pub use crate::unicode::WidthMethod;

use crate::ansi::{Token, spans};
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the running program measures with grapheme widths.
static GRAPHEME_WIDTH: AtomicBool = AtomicBool::new(false);

/// How [`width`] and [`truncate`] measure text: the method the running
/// program chose, [`WidthMethod::WcWidth`] unless the terminal reported
/// that it supports grapheme clusters (mode 2027). One process has one
/// terminal, so the choice is shared by the whole process.
#[must_use]
pub fn width_method() -> WidthMethod {
    if GRAPHEME_WIDTH.load(Ordering::Relaxed) {
        WidthMethod::GraphemeWidth
    } else {
        WidthMethod::WcWidth
    }
}

/// Makes `method` the one [`width`] and [`truncate`] use.
pub(crate) fn set_width_method(method: WidthMethod) {
    GRAPHEME_WIDTH.store(method == WidthMethod::GraphemeWidth, Ordering::Relaxed);
}

/// The number of terminal cells `text` takes, without its escape
/// sequences, measured with [`width_method`]: wide characters (CJK, most
/// emoji) take two.
#[must_use]
pub fn width(text: &str) -> usize {
    width_method().string_width(text)
}

/// `text` without its ANSI escape sequences.
#[must_use]
pub fn strip_ansi(text: &str) -> String {
    spans(text, WidthMethod::WcWidth)
        .filter(|(token, _)| matches!(token, Token::Text(..) | Token::Control(_)))
        .map(|(_, raw)| raw)
        .collect()
}

/// The first `columns` cells of `text`, measured with [`width_method`],
/// keeping its escape sequences, with a reset at the end if any text was
/// cut. A wide character that would cross the limit is left out whole.
#[must_use]
pub fn truncate(text: &str, columns: usize) -> String {
    let method = width_method();
    if method.string_width(text) <= columns {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut room = Some(columns);
    for (token, raw) in spans(text, method) {
        if keep(token, &mut room) {
            out.push_str(raw);
        }
    }
    out + "\x1b[0m"
}

/// Whether [`truncate`] keeps `token`, with `room` columns left (`None`
/// once text has been cut, after which only escape sequences are kept).
fn keep(token: Token<'_>, room: &mut Option<usize>) -> bool {
    match (token, *room) {
        (Token::Text(_, w), Some(left)) if w <= left => {
            *room = Some(left - w);
            true
        }
        (Token::Text(..), _) => {
            *room = None;
            false
        }
        (Token::Control(_), None) => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_style_leaves_text_alone() {
        assert_eq!(Style::new().paint("hi"), "hi");
    }

    #[test]
    fn styles_become_sgr_codes() {
        assert_eq!(
            Style::new().bold().fg(Color::RED).paint("x"),
            "\x1b[1;31mx\x1b[0m"
        );
        assert_eq!(
            Style::new()
                .fg(Color::Ansi(9))
                .bg(Color::Ansi(12))
                .paint("x"),
            "\x1b[91;104mx\x1b[0m"
        );
        assert_eq!(
            Style::new().fg(Color::Palette(208)).paint("x"),
            "\x1b[38;5;208mx\x1b[0m"
        );
        assert_eq!(
            Style::new().bg(Color::Rgb(1, 2, 3)).paint("x"),
            "\x1b[48;2;1;2;3mx\x1b[0m"
        );
        assert_eq!(
            Style::new().dim().italic().underline().reverse().paint("x"),
            "\x1b[2;3;4;7mx\x1b[0m"
        );
    }

    #[test]
    fn width_ignores_escape_sequences() {
        let s = Style::new().bold().fg(Color::GREEN).paint("héllo");
        assert_eq!(width(&s), 5);
        assert_eq!(strip_ansi(&s), "héllo");
    }

    #[test]
    fn width_counts_wide_characters_twice() {
        assert_eq!(width("字a"), 3);
        assert_eq!(width("e\u{301}"), 1);
        assert_eq!(
            truncate("字字", 3),
            "字\x1b[0m",
            "a wide character is not split"
        );
        assert_eq!(width_method(), WidthMethod::WcWidth);
    }

    #[test]
    fn truncation_keeps_styles_and_resets() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 3), "hel\x1b[0m");
        let styled = Style::new().bold().paint("hello");
        assert_eq!(strip_ansi(&truncate(&styled, 2)), "he");
        assert!(truncate(&styled, 2).ends_with("\x1b[0m"));
    }
}
