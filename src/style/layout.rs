//! Putting blocks of text together and cutting them to size, by display
//! width: Lip Gloss's `JoinHorizontal`, `JoinVertical` and `Height`, and
//! `x/ansi`'s `Truncate`, `Cut` and `Wrap`.

use super::{truncate, width, width_method};
use crate::ansi::{Token, spans};

/// Where a block goes when it is shorter or narrower than the others:
/// Lip Gloss's `Position`, a fraction from the top (or left), 0.0, to the
/// bottom (or right), 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position(pub f64);

impl Position {
    /// At the top.
    pub const TOP: Self = Self(0.0);
    /// At the bottom.
    pub const BOTTOM: Self = Self(1.0);
    /// At the left.
    pub const LEFT: Self = Self(0.0);
    /// At the right.
    pub const RIGHT: Self = Self(1.0);
    /// In the middle.
    pub const CENTER: Self = Self(0.5);

    /// `room` split into the part before the block and the part after.
    fn split(self, room: usize) -> (usize, usize) {
        let before = round_clamped(to_f64(room) * self.0.clamp(0.0, 1.0), room);
        (before, room - before)
    }
}

/// `n` as a float (exact up to `u32::MAX`, which larger values become).
pub(crate) fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// `v` rounded half away from zero, as Go's `math.Round`, and clamped
/// to `0..=max`; NaN is 0.
pub(crate) fn round_clamped(v: f64, max: usize) -> usize {
    let v = v.round();
    let (mut low, mut high) = (0, max);
    if v.is_nan() || v <= 0.0 {
        return 0;
    }
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if to_f64(mid) <= v {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// The number of lines in `text`: Lip Gloss's `Height`.
///
/// ```
/// assert_eq!(petitcarnets::style::height("a\nb"), 2);
/// ```
#[must_use]
pub fn height(text: &str) -> usize {
    text.split('\n').count()
}

/// The cells the widest line of `text` takes: Lip Gloss's `Width`.
/// [`width`] counts every line together.
///
/// ```
/// assert_eq!(petitcarnets::style::block_width("ab\nabcd"), 4);
/// ```
#[must_use]
pub fn block_width(text: &str) -> usize {
    text.split('\n').map(width).max().unwrap_or(0)
}

/// The blocks side by side, the shorter ones placed at `pos` and every
/// block padded to its widest line: Lip Gloss's `JoinHorizontal`.
///
/// ```
/// use petitcarnets::style::{Position, join_horizontal};
///
/// assert_eq!(join_horizontal(Position::TOP, &["a\nb", "cc"]), "acc\nb  ");
/// ```
#[must_use]
pub fn join_horizontal(pos: Position, blocks: &[&str]) -> String {
    if blocks.len() < 2 {
        return blocks.concat();
    }
    let split: Vec<Vec<&str>> = blocks.iter().map(|b| b.split('\n').collect()).collect();
    let tallest = split.iter().map(Vec::len).max().unwrap_or(0);
    let columns: Vec<(Vec<&str>, usize)> = split
        .into_iter()
        .map(|lines| {
            let wide = lines.iter().map(|l| width(l)).max().unwrap_or(0);
            let (above, below) = pos.split(tallest - lines.len());
            let mut placed = vec![""; above];
            placed.extend(lines);
            placed.extend(std::iter::repeat_n("", below));
            (placed, wide)
        })
        .collect();
    let rows: Vec<String> = (0..tallest)
        .map(|i| {
            let cells = columns
                .iter()
                .map(|(lines, wide)| pad_right(lines[i], *wide));
            cells.collect::<String>()
        })
        .collect();
    rows.join("\n")
}

/// The blocks one above another, each line placed at `pos` in the width
/// of the widest: Lip Gloss's `JoinVertical`.
///
/// ```
/// use petitcarnets::style::{Position, join_vertical};
///
/// assert_eq!(join_vertical(Position::RIGHT, &["a", "bcd"]), "  a\nbcd");
/// ```
#[must_use]
pub fn join_vertical(pos: Position, blocks: &[&str]) -> String {
    if blocks.len() < 2 {
        return blocks.concat();
    }
    let lines: Vec<&str> = blocks.iter().flat_map(|b| b.split('\n')).collect();
    let widest = lines.iter().map(|l| width(l)).max().unwrap_or(0);
    let placed: Vec<String> = lines
        .iter()
        .map(|l| {
            let (left, right) = pos.split(widest - width(l));
            format!("{}{l}{}", " ".repeat(left), " ".repeat(right))
        })
        .collect();
    placed.join("\n")
}

/// `line` padded with spaces to `columns`.
fn pad_right(line: &str, columns: usize) -> String {
    format!("{line}{}", " ".repeat(columns.saturating_sub(width(line))))
}

/// `text` cut to `columns` cells with `tail` (such as `"…"`) in place of
/// what was cut: `x/ansi`'s `Truncate`. Text that fits is returned as it
/// is; a wide character is never split.
///
/// ```
/// assert_eq!(petitcarnets::style::truncate_with("hello", 4, "…"), "hel…\x1b[0m");
/// assert_eq!(petitcarnets::style::truncate_with("hi", 4, "…"), "hi");
/// ```
#[must_use]
pub fn truncate_with(text: &str, columns: usize, tail: &str) -> String {
    if width(text) <= columns {
        return text.to_string();
    }
    let room = columns.saturating_sub(width(tail));
    let cut = truncate(text, room);
    let body = cut.strip_suffix("\x1b[0m").unwrap_or(&cut);
    format!("{body}{tail}\x1b[0m")
}

/// The cells from `left` up to `right` of `text`, keeping its escape
/// sequences: `x/ansi`'s `Cut`. A wide character that crosses the left
/// edge is kept whole, and one that would cross the right edge is left
/// out, so nothing is split and the cut is at most `right - left` cells.
///
/// ```
/// assert_eq!(petitcarnets::style::cut("ab字cd", 1, 4), "b字");
/// assert_eq!(petitcarnets::style::cut("ab字cd", 3, 6), "字c");
/// ```
#[must_use]
pub fn cut(text: &str, left: usize, right: usize) -> String {
    let mut out = String::new();
    let mut at = 0;
    for (token, raw) in spans(text, width_method()) {
        match token {
            Token::Text(_, w) => {
                if at + w > left {
                    out.push_str(raw);
                }
                at += w;
            }
            Token::Control(_) => {}
            _ => out.push_str(raw),
        }
    }
    let cut = truncate(&out, right.saturating_sub(left));
    cut.strip_suffix("\x1b[0m")
        .map_or_else(|| cut.clone(), str::to_string)
}

/// `text` wrapped at `columns` cells: at spaces where it can, and inside
/// a word too long for a line, never inside a grapheme or a wide
/// character. Trailing spaces at a break are dropped. Escape sequences
/// are kept. Lip Gloss's `Wrap`, for its `Width`.
///
/// ```
/// assert_eq!(petitcarnets::style::wrap("the quick brown", 9), "the quick\nbrown");
/// assert_eq!(petitcarnets::style::wrap("abcdef", 4), "abcd\nef");
/// ```
#[must_use]
pub fn wrap(text: &str, columns: usize) -> String {
    if columns == 0 {
        return text.to_string();
    }
    let lines: Vec<String> = text.split('\n').map(|l| wrap_line(l, columns)).collect();
    lines.join("\n")
}

/// One line wrapped.
fn wrap_line(line: &str, columns: usize) -> String {
    let mut wrap = Wrapper {
        columns,
        ..Wrapper::default()
    };
    for (token, raw) in spans(line, width_method()) {
        match token {
            Token::Text(" ", _) => wrap.space(),
            Token::Text(_, w) => wrap.cluster(raw, w),
            _ => wrap.word.push_str(raw),
        }
    }
    wrap.finish()
}

/// The state of wrapping one line: the lines done, the line being
/// filled, the word being read, and the spaces before it.
#[derive(Default)]
struct Wrapper {
    columns: usize,
    out: Vec<String>,
    line: String,
    line_width: usize,
    word: String,
    word_width: usize,
    spaces: usize,
}

impl Wrapper {
    /// A space: the word before it is done.
    fn space(&mut self) {
        self.flush_word();
        self.spaces += 1;
    }

    /// A cluster of the word; a word wider than a line is broken.
    fn cluster(&mut self, raw: &str, w: usize) {
        if self.word_width + w > self.columns {
            self.flush_word();
            self.break_line();
        }
        self.word.push_str(raw);
        self.word_width += w;
    }

    /// Moves the word onto the line, or onto a new line if it does not
    /// fit after the spaces.
    fn flush_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let gap = self.gap();
        if self.line_width > 0 && self.line_width + gap + self.word_width > self.columns {
            self.break_line();
        } else {
            self.line.push_str(&" ".repeat(gap));
            self.line_width += gap;
        }
        self.line.push_str(&std::mem::take(&mut self.word));
        self.line_width += std::mem::take(&mut self.word_width);
        self.spaces = 0;
    }

    /// The spaces kept before the word: none at the start of a line
    /// after a break, as at a break they are dropped.
    fn gap(&self) -> usize {
        let after_break = self.line_width == 0 && !self.out.is_empty();
        if after_break { 0 } else { self.spaces }
    }

    /// Ends the line.
    fn break_line(&mut self) {
        if self.line_width > 0 || !self.line.is_empty() {
            self.out.push(std::mem::take(&mut self.line));
        }
        self.line_width = 0;
        self.spaces = 0;
    }

    /// The wrapped lines.
    fn finish(mut self) -> String {
        self.flush_word();
        if self.line_width + self.spaces <= self.columns {
            self.line.push_str(&" ".repeat(self.spaces));
        }
        if !self.line.is_empty() || self.out.is_empty() {
            self.out.push(self.line);
        }
        self.out.join("\n")
    }
}
