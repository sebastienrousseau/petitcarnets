//! Colour and emphasis, as ANSI escape codes, the visible width of
//! styled text, and the few layout tools the components need: padding,
//! borders, margins, joining blocks side by side or one above another,
//! wrapping, truncating and cutting by display width.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::style::{Color, Style};
//!
//! let title = Style::new().bold(true).fg(Color::RED).render("Title");
//! assert_eq!(tea::style::strip_ansi(&title), "Title");
//! assert_eq!(tea::style::width(&title), 5);
//! ```

mod border;
mod emphasis;
mod frame;
mod layout;
mod render;

pub use border::Border;
pub use layout::{
    Position, block_width, cut, height, join_horizontal, join_vertical, truncate_with, wrap,
};
#[cfg(feature = "bubbles")]
pub(crate) use layout::{round_clamped, to_f64};

/// A terminal colour.
///
/// ```
/// use petitcarnets as tea;
/// use tea::style::{Color, Style};
///
/// assert_eq!(Style::new().fg(Color::RED).paint("x"), "\x1b[31mx\x1b[0m");
/// assert_eq!(
///     Style::new().fg(Color::Ansi(9)).paint("x"),
///     "\x1b[91mx\x1b[0m"
/// );
/// assert_eq!(
///     Style::new().bg(Color::Palette(212)).paint("x"),
///     "\x1b[48;5;212mx\x1b[0m"
/// );
/// assert_eq!(
///     Style::new().fg(Color::Rgb(1, 2, 3)).paint("x"),
///     "\x1b[38;2;1;2;3mx\x1b[0m"
/// );
/// ```
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
    /// The attribute bits a setter chose, on or off, which
    /// [`Style::inherit`] does not take from the other style.
    chosen: u8,
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
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new(), Style::default());
    /// assert_eq!(Style::new().render("hi"), "hi");
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            attributes: 0,
            chosen: 0,
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

    /// `text` in this style, reset after it, as one run: for text on one
    /// line. A plain style returns the text unchanged.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Color, Style};
    ///
    /// assert_eq!(
    ///     Style::new().bold(true).fg(Color::GREEN).paint("ok"),
    ///     "\x1b[1;32mok\x1b[0m"
    /// );
    /// assert_eq!(Style::new().paint("ok"), "ok");
    /// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::style::WidthMethod;
///
/// // No program has asked the terminal, so code points are measured alone.
/// assert_eq!(tea::style::width_method(), WidthMethod::WcWidth);
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::style::{Style, width};
///
/// assert_eq!(width("hello"), 5);
/// assert_eq!(width("日本"), 4);
/// assert_eq!(width(&Style::new().bold(true).paint("hi")), 2);
/// ```
#[must_use]
pub fn width(text: &str) -> usize {
    width_method().string_width(text)
}

/// `text` without its ANSI escape sequences.
///
/// ```
/// use petitcarnets as tea;
///
/// assert_eq!(
///     tea::style::strip_ansi("\x1b[1mbold\x1b[0m text"),
///     "bold text"
/// );
/// ```
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
///
/// ```
/// use petitcarnets as tea;
/// use tea::style::truncate;
///
/// assert_eq!(truncate("hi", 5), "hi");
/// assert_eq!(truncate("hello", 3), "hel\x1b[0m");
/// assert_eq!(truncate("日本語", 3), "日\x1b[0m");
/// ```
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
