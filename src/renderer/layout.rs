//! Laying a view's text out in cells: styles and hyperlinks read from
//! its escape sequences, tabs expanded, wide characters kept whole, and
//! each line cut at the window's edge.

use super::cell::{Cell, Link, Pen};
use crate::ansi::{Token, tokens};
use crate::unicode::WidthMethod;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Tab stops are every this many columns.
const TAB: usize = 8;

/// One row of a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    /// Exactly the frame's width of cells.
    pub(crate) cells: Vec<Cell>,
    /// A hash of the cells, to find unchanged and moved rows quickly.
    pub(crate) hash: u64,
}

impl Row {
    /// A row of `width` blank cells.
    pub(crate) fn blank(width: usize) -> Self {
        Self::new(vec![Cell::BLANK; width])
    }

    /// A row of these cells.
    pub(crate) fn new(cells: Vec<Cell>) -> Self {
        let mut hasher = Fnv::default();
        cells.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
            cells,
        }
    }

    /// Where the row's content ends: after its last cell that is not
    /// blank.
    pub(crate) fn content_end(&self) -> usize {
        self.cells
            .iter()
            .rposition(|c| !c.is_blank())
            .map_or(0, |i| i + 1)
    }
}

/// A frame: rows of cells, all as wide as the window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Frame {
    /// Columns.
    pub(crate) width: usize,
    /// Rows, top first.
    pub(crate) rows: Vec<Row>,
}

/// The style and hyperlink in force while text is read; they carry
/// over from one line to the next, as on a terminal.
#[derive(Debug, Default)]
pub(crate) struct Pens {
    pen: Pen,
    link: Link,
}

impl Pens {
    /// Applies a sequence that changes the style or link; others are
    /// dropped.
    fn apply(&mut self, token: Token<'_>) {
        match token {
            Token::Csi {
                params,
                final_byte: b'm',
            } if params
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b';' || b == b':') =>
            {
                self.pen.apply_sgr(params);
            }
            Token::Osc(data) => {
                if let Some(link) = data.strip_prefix("8;") {
                    self.set_link(link);
                }
            }
            _ => {}
        }
    }

    /// OSC 8's `params;url`; an empty URL ends the link.
    fn set_link(&mut self, link: &str) {
        let url = link.split_once(';').map_or("", |(_, url)| url);
        self.link = if url.is_empty() {
            None
        } else {
            Some(Rc::from(link))
        };
    }
}

/// One line being laid out.
struct Line {
    cells: Vec<Cell>,
    limit: usize,
    x: usize,
    last: Option<usize>,
    full: bool,
}

impl Line {
    /// Room for `limit` columns.
    fn new(limit: usize) -> Self {
        Self {
            cells: Vec::new(),
            limit,
            x: 0,
            last: None,
            full: false,
        }
    }

    /// Adds a cluster `width` columns wide at the current column; one
    /// that does not fit ends the line, so a wide character is never
    /// split.
    fn push(&mut self, text: &str, width: usize, pens: &Pens) {
        if self.full || self.x + width > self.limit {
            self.full = true;
            return;
        }
        let (x, end) = (self.x, self.x + width);
        if self.cells.len() < end {
            self.cells.resize(end, Cell::BLANK);
        }
        self.unsplit(x, end);
        let w = u8::try_from(width).unwrap_or(u8::MAX);
        self.cells[x] = Cell::new(text, w, pens.pen, pens.link.clone());
        self.cover(x + 1, end, pens);
        self.last = Some(x);
        self.x = end;
    }

    /// Marks cells `from..to` as covered by the wide cell to their left.
    fn cover(&mut self, from: usize, to: usize, pens: &Pens) {
        for cell in &mut self.cells[from..to] {
            *cell = Cell::continuation(pens.pen, pens.link.clone());
        }
    }

    /// Blanks what is left of wide cells that cells `from..to` are about
    /// to cover in part (only after a carriage return).
    fn unsplit(&mut self, from: usize, to: usize) {
        let mut i = from;
        while i > 0 && self.cells[i].is_continuation() {
            i -= 1;
            self.cells[i] = Cell::BLANK;
        }
        let mut j = to;
        while self.cells.get(j).is_some_and(Cell::is_continuation) {
            self.cells[j] = Cell::BLANK;
            j += 1;
        }
    }

    /// Adds a zero-width cluster to the last cell written.
    fn mark(&mut self, text: &str) {
        if let Some(cell) = self.last.and_then(|i| self.cells.get_mut(i)) {
            cell.append(text);
        }
    }

    /// Spaces up to the next tab stop, in the current style.
    fn tab(&mut self, pens: &Pens) {
        let to = (self.x / TAB + 1) * TAB;
        while self.x < to && !self.full {
            self.push(" ", 1, pens);
        }
    }

    /// Reads one token.
    fn read(&mut self, token: Token<'_>, pens: &mut Pens) {
        match token {
            Token::Text(text, 0) => self.mark(text),
            Token::Text(text, width) => self.push(text, width, pens),
            Token::Control(b'\t') => self.tab(pens),
            Token::Control(b'\r') => {
                self.x = 0;
                self.full = false;
            }
            Token::Control(_) => {}
            other => pens.apply(other),
        }
    }
}

/// Lays out one line of text in cells, at most `limit` columns.
pub(crate) fn line_cells(
    text: &str,
    limit: usize,
    method: WidthMethod,
    pens: &mut Pens,
) -> Vec<Cell> {
    let mut line = Line::new(limit);
    for token in tokens(text, method) {
        line.read(token, pens);
    }
    line.cells
}

/// The lines of `content`, as Go splits a view: by `\n`, with `\r\n`
/// read as `\n`, and no lines at all for empty content. A `\r` inside
/// a line goes back to its left edge, and what follows writes over it.
pub(crate) fn lines(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }
    content
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect()
}

/// Lays `content` out `width` columns wide. With `rows`, the frame has
/// exactly that many rows (the alternate screen: extra lines are cut);
/// without, one row per line.
pub(crate) fn layout(
    content: &str,
    width: usize,
    rows: Option<usize>,
    method: WidthMethod,
) -> Frame {
    let mut pens = Pens::default();
    let mut out: Vec<Row> = lines(content)
        .into_iter()
        .take(rows.unwrap_or(usize::MAX))
        .map(|line| {
            let mut cells = line_cells(line, width, method, &mut pens);
            cells.resize(width, Cell::BLANK);
            Row::new(cells)
        })
        .collect();
    if let Some(rows) = rows {
        out.resize_with(rows, || Row::blank(width));
    }
    Frame { width, rows: out }
}

/// FNV-1a: a fast hash for rows, which only need to tell rows apart
/// within one renderer.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests;
