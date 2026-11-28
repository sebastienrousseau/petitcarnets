//! Borders: the characters of a box's edges and corners, Lip Gloss's
//! `Border`, and drawing them around a block of text.

use super::{SIDES_SET, Style, width};

/// The characters of a border: Lip Gloss's `Border`. Start from one of
/// the constants and change a field to make your own, as in
/// `Border { right: "├", ..Border::ROUNDED }`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::style::{Border, Style};
///
/// assert_eq!(
///     Style::new().border(Border::DOUBLE).render("hi"),
///     "╔══╗\n║hi║\n╚══╝"
/// );
/// let tee = Border {
///     right: "├",
///     ..Border::ROUNDED
/// };
/// assert_eq!(Style::new().border(tee).render("hi"), "╭──╮\n│hi├\n╰──╯");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Border {
    /// The top edge.
    pub top: &'static str,
    /// The bottom edge.
    pub bottom: &'static str,
    /// The left edge.
    pub left: &'static str,
    /// The right edge.
    pub right: &'static str,
    /// The top left corner.
    pub top_left: &'static str,
    /// The top right corner.
    pub top_right: &'static str,
    /// The bottom left corner.
    pub bottom_left: &'static str,
    /// The bottom right corner.
    pub bottom_right: &'static str,
}

impl Border {
    /// Thin lines with square corners: Lip Gloss's `NormalBorder`.
    pub const NORMAL: Self = Self {
        top: "─",
        bottom: "─",
        left: "│",
        right: "│",
        top_left: "┌",
        top_right: "┐",
        bottom_left: "└",
        bottom_right: "┘",
    };

    /// Thin lines with rounded corners: Lip Gloss's `RoundedBorder`.
    pub const ROUNDED: Self = Self {
        top_left: "╭",
        top_right: "╮",
        bottom_left: "╰",
        bottom_right: "╯",
        ..Self::NORMAL
    };

    /// Thick lines: Lip Gloss's `ThickBorder`.
    pub const THICK: Self = Self {
        top: "━",
        bottom: "━",
        left: "┃",
        right: "┃",
        top_left: "┏",
        top_right: "┓",
        bottom_left: "┗",
        bottom_right: "┛",
    };

    /// Double lines: Lip Gloss's `DoubleBorder`.
    pub const DOUBLE: Self = Self {
        top: "═",
        bottom: "═",
        left: "║",
        right: "║",
        top_left: "╔",
        top_right: "╗",
        bottom_left: "╚",
        bottom_right: "╝",
    };

    /// Spaces, which keep a border's room without drawing it: Lip
    /// Gloss's `HiddenBorder`.
    pub const HIDDEN: Self = Self {
        top: " ",
        bottom: " ",
        left: " ",
        right: " ",
        top_left: " ",
        top_right: " ",
        bottom_left: " ",
        bottom_right: " ",
    };
}

impl Style {
    /// Whether each side, top, right, bottom, left, has a border.
    fn sides(&self) -> [bool; 4] {
        if self.frame.border.is_none() {
            return [false; 4];
        }
        let sides = self.frame.sides;
        if sides & SIDES_SET == 0 {
            return [true; 4];
        }
        [0, 1, 2, 3].map(|bit| sides & (1 << bit) != 0)
    }

    /// The cells the border takes on each side: top, right, bottom, left.
    pub(super) fn border_widths(&self) -> [usize; 4] {
        let Some(border) = self.frame.border else {
            return [0; 4];
        };
        let [top, right, bottom, left] = self.sides();
        let edge = |on: bool, s: &str| if on { width(s).max(1) } else { 0 };
        [
            usize::from(top),
            edge(right, border.right),
            usize::from(bottom),
            edge(left, border.left),
        ]
    }

    /// `block` (lines all as wide) inside the border.
    pub(super) fn apply_border(&self, block: &str) -> String {
        let Some(border) = self.frame.border else {
            return block.to_string();
        };
        let [top, right, bottom, left] = self.sides();
        let edge = Style {
            fg: self.frame.border_fg,
            ..Style::new()
        };
        let inner = block.split('\n').map(width).max().unwrap_or(0);
        let mut out = Vec::new();
        let corner = |on: bool, s: &'static str| if on { s } else { "" };
        if top {
            let (l, r) = (
                corner(left, border.top_left),
                corner(right, border.top_right),
            );
            out.push(edge.paint(&horizontal(l, border.top, r, inner)));
        }
        for line in block.split('\n') {
            let l = if left {
                edge.paint(border.left)
            } else {
                String::new()
            };
            let r = if right {
                edge.paint(border.right)
            } else {
                String::new()
            };
            out.push(format!("{l}{line}{r}"));
        }
        if bottom {
            let (l, r) = (
                corner(left, border.bottom_left),
                corner(right, border.bottom_right),
            );
            out.push(edge.paint(&horizontal(l, border.bottom, r, inner)));
        }
        out.join("\n")
    }
}

/// An edge `inner` cells long between two corners, its characters
/// repeated in turn.
fn horizontal(left: &str, middle: &str, right: &str, inner: usize) -> String {
    let middle = if middle.is_empty() { " " } else { middle };
    let mut out = String::from(left);
    let mut cells = 0;
    for c in middle.chars().cycle() {
        if cells >= inner {
            break;
        }
        out.push(c);
        cells += width(c.encode_utf8(&mut [0; 4])).max(1);
    }
    out + right
}
