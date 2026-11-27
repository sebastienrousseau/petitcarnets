//! Colour and emphasis, as ANSI escape codes, the visible width of
//! styled text, and the few layout tools the components need: padding,
//! borders, margins, joining blocks side by side or one above another,
//! wrapping, truncating and cutting by display width.

mod border;
mod layout;
mod render;

pub use border::Border;
pub use layout::{
    Position, block_width, cut, height, join_horizontal, join_vertical, truncate_with, wrap,
};
#[cfg(feature = "bubbles")]
pub(crate) use layout::{round_clamped, to_f64};

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

/// A style: colours and emphasis, applied to a line with
/// [`Style::paint`], and the box around a block of text (padding,
/// border, margins, a width and a height), applied with
/// [`Style::render`]: the small part of Lip Gloss's `Style` that the
/// [`bubbles`](crate::bubbles) components use.
///
/// ```
/// use petitcarnets::style::{Border, Color, Style};
///
/// let label = Style::new().fg(Color::Palette(212)).padding(0, 1);
/// assert_eq!(petitcarnets::style::strip_ansi(&label.render("hi")), " hi ");
/// let boxed = Style::new().border(Border::ROUNDED).render("hi");
/// assert_eq!(boxed, "╭──╮\n│hi│\n╰──╯");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    attributes: u8,
    frame: Frame,
}

/// The box parts of a style: padding and margins (top, right, bottom,
/// left), the border and its sides, and the size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Frame {
    padding: [u16; 4],
    margin: [u16; 4],
    border: Option<Border>,
    /// Which sides have a border, top, right, bottom, left as bits 0 to
    /// 3, and bit 4 once any side was set (else a border has all four).
    sides: u8,
    border_fg: Option<Color>,
    width: u16,
    height: u16,
}

/// Set once [`Style::border_sides`] chose the sides.
const SIDES_SET: u8 = 1 << 4;

impl Style {
    /// No colour, no emphasis and no box.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            attributes: 0,
            frame: Frame {
                padding: [0; 4],
                margin: [0; 4],
                border: None,
                sides: 0,
                border_fg: None,
                width: 0,
                height: 0,
            },
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

    /// This style with the colours and emphasis of `other` that this one
    /// does not set: Lip Gloss's `Inherit`. The box is not inherited.
    #[must_use]
    pub const fn inherit(mut self, other: Self) -> Self {
        if self.fg.is_none() {
            self.fg = other.fg;
        }
        if self.bg.is_none() {
            self.bg = other.bg;
        }
        self.attributes |= other.attributes;
        self
    }

    /// The same colours and emphasis, without padding, border, margins
    /// or size.
    #[must_use]
    pub const fn plain(mut self) -> Self {
        self.frame = Self::new().frame;
        self
    }

    /// Padding: `vertical` blank lines above and below, `horizontal`
    /// columns left and right, in the background colour. Lip Gloss's
    /// `Padding(vertical, horizontal)`.
    #[must_use]
    pub const fn padding(mut self, vertical: u16, horizontal: u16) -> Self {
        self.frame.padding = [vertical, horizontal, vertical, horizontal];
        self
    }

    /// Padding on each side: top, right, bottom, left, as CSS orders them.
    #[must_use]
    pub const fn padding_sides(mut self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        self.frame.padding = [top, right, bottom, left];
        self
    }

    /// Padding on the left only changed.
    #[must_use]
    pub const fn padding_left(mut self, columns: u16) -> Self {
        self.frame.padding[3] = columns;
        self
    }

    /// Margins: blank space outside the border, `vertical` lines above
    /// and below and `horizontal` columns left and right.
    #[must_use]
    pub const fn margin(mut self, vertical: u16, horizontal: u16) -> Self {
        self.frame.margin = [vertical, horizontal, vertical, horizontal];
        self
    }

    /// Margins on each side: top, right, bottom, left.
    #[must_use]
    pub const fn margin_sides(mut self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        self.frame.margin = [top, right, bottom, left];
        self
    }

    /// The top margin only changed.
    #[must_use]
    pub const fn margin_top(mut self, rows: u16) -> Self {
        self.frame.margin[0] = rows;
        self
    }

    /// The top margin's rows.
    #[must_use]
    pub fn get_margin_top(&self) -> usize {
        usize::from(self.frame.margin[0])
    }

    /// A border on every side, unless [`Style::border_sides`] picks some.
    #[must_use]
    pub const fn border(mut self, border: Border) -> Self {
        self.frame.border = Some(border);
        self
    }

    /// Which sides have the border: top, right, bottom, left, as Lip
    /// Gloss's `Border(border, top, right, bottom, left)` sets them.
    #[must_use]
    pub fn border_sides(mut self, [top, right, bottom, left]: [bool; 4]) -> Self {
        let bit = |on: bool, n: u8| u8::from(on) << n;
        self.frame.sides = SIDES_SET | bit(top, 0) | bit(right, 1) | bit(bottom, 2) | bit(left, 3);
        self
    }

    /// The border's colour.
    #[must_use]
    pub const fn border_fg(mut self, color: Color) -> Self {
        self.frame.border_fg = Some(color);
        self
    }

    /// At least `columns` wide, border included; longer lines are
    /// wrapped at the width less the padding, as Lip Gloss's `Width`.
    /// 0 is no width.
    #[must_use]
    pub const fn width(mut self, columns: u16) -> Self {
        self.frame.width = columns;
        self
    }

    /// At least `rows` high, border included; blank lines are added
    /// below. 0 is no height.
    #[must_use]
    pub const fn height(mut self, rows: u16) -> Self {
        self.frame.height = rows;
        self
    }

    /// The padding's columns on the left.
    #[must_use]
    pub fn get_padding_left(&self) -> usize {
        usize::from(self.frame.padding[3])
    }

    /// The padding's columns on the right.
    #[must_use]
    pub fn get_padding_right(&self) -> usize {
        usize::from(self.frame.padding[1])
    }

    /// The columns padding, border and margins take, left and right
    /// together: Lip Gloss's `GetHorizontalFrameSize`.
    #[must_use]
    pub fn horizontal_frame(&self) -> usize {
        let [_, right, _, left] = self.frame.padding;
        let [_, m_right, _, m_left] = self.frame.margin;
        let [_, b_right, _, b_left] = self.border_widths();
        usize::from(right + left + m_right + m_left) + b_right + b_left
    }

    /// The columns left of the content and the rows above it that
    /// margins, border and padding take: where a block's text starts.
    #[must_use]
    pub fn frame_offset(&self) -> (usize, usize) {
        let [top, _, _, left] = self.frame.padding;
        let [m_top, _, _, m_left] = self.frame.margin;
        let [b_top, _, _, b_left] = self.border_widths();
        (
            usize::from(left + m_left) + b_left,
            usize::from(top + m_top) + b_top,
        )
    }

    /// The rows padding, border and margins take, above and below.
    #[must_use]
    pub fn vertical_frame(&self) -> usize {
        let [top, _, bottom, _] = self.frame.padding;
        let [m_top, _, m_bottom, _] = self.frame.margin;
        let [b_top, _, b_bottom, _] = self.border_widths();
        usize::from(top + bottom + m_top + m_bottom) + b_top + b_bottom
    }

    /// `text` in this style, reset after it, as one run: for text on one
    /// line. A plain style returns the text unchanged.
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
mod tests;
