//! Writing to the terminal while keeping track of its cursor and pen,
//! so each move and style change takes as few bytes as it can.
//!
//! It never moves with tabs or backspaces, and never counts on the
//! terminal turning `\n` into `\r\n`.

use super::cell::{Cell, Link, Pen};
use std::fmt::Write as _;

/// How the cursor may be moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Addressing {
    /// Anywhere on the screen, rows counted from its top (the alternate
    /// screen).
    Absolute,
    /// Only relative to where it is, rows counted from the frame's top
    /// (inline: where the frame sits on the screen is unknown).
    Relative,
}

/// The terminal's cursor and pen as the bytes written so far leave them.
#[derive(Debug, Clone)]
pub(crate) struct Writer {
    /// What to write.
    pub(crate) buf: String,
    /// The column, if known: writing into the last column leaves it
    /// unknown, as terminals differ there.
    pub(crate) x: Option<usize>,
    /// The row.
    pub(crate) y: usize,
    /// How many rows of the frame exist on the screen: below them,
    /// inline, only a line feed can make room.
    pub(crate) rows: usize,
    /// The window's width.
    pub(crate) width: usize,
    /// How the cursor may move.
    pub(crate) addressing: Addressing,
    pen: Pen,
    link: Link,
}

impl Writer {
    /// A writer with the cursor at the top left, rows already there.
    pub(crate) fn new(width: usize, rows: usize, addressing: Addressing) -> Self {
        Self {
            buf: String::new(),
            x: Some(0),
            y: 0,
            rows: rows.max(1),
            width,
            addressing,
            pen: Pen::DEFAULT,
            link: None,
        }
    }

    /// Writes bytes that do not move the cursor or change the pen.
    pub(crate) fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Moves the cursor to column `x` of row `y`.
    pub(crate) fn move_to(&mut self, x: usize, y: usize) {
        if self.x == Some(x) && self.y == y {
            return;
        }
        if self.addressing == Addressing::Relative && y >= self.rows {
            self.grow_to(y);
        }
        let relative = self.relative_move(x, y);
        let text = match self.addressing {
            Addressing::Absolute => shortest(relative, cup(x, y)),
            Addressing::Relative => relative,
        };
        self.buf.push_str(&text);
        self.x = Some(x);
        self.y = y;
    }

    /// Inline: makes rows down to `y` exist with line feeds from the
    /// last row there is, which scroll the screen when they reach its
    /// bottom. The cursor ends at the start of row `y`.
    fn grow_to(&mut self, y: usize) {
        let last = self.rows.max(1) - 1;
        self.buf.push_str(&vertical(self.y, last));
        for _ in last..y {
            self.buf.push_str("\r\n");
        }
        self.x = Some(0);
        self.y = y;
        self.rows = y + 1;
    }

    /// The shortest relative move to column `x` of row `y`.
    fn relative_move(&self, x: usize, y: usize) -> String {
        let mut best = vertical(self.y, y) + &horizontal(self.x, x);
        if y > self.y && y < self.rows {
            let feeds = "\r\n".repeat(y - self.y) + &horizontal(Some(0), x);
            best = shortest(best, feeds);
        }
        best
    }

    /// Sets the pen and link to the cell's, then writes it.
    pub(crate) fn put(&mut self, cell: &Cell) {
        if cell.is_continuation() {
            return;
        }
        self.style(&cell.pen, &cell.link);
        cell.push_text(&mut self.buf);
        self.advance(usize::from(cell.width));
    }

    /// Moves the tracked column on by `n`; reaching the right edge
    /// leaves it unknown, as terminals differ there.
    pub(crate) fn advance(&mut self, n: usize) {
        let width = self.width;
        self.x = self.x.map(|x| x + n).filter(|x| *x < width);
    }

    /// Changes the pen and link to `pen` and `link`.
    pub(crate) fn style(&mut self, pen: &Pen, link: &Link) {
        if self.link != *link {
            let target = link.as_deref().unwrap_or(";");
            let _ = write!(self.buf, "\x1b]8;{target}\x07");
            self.link.clone_from(link);
        }
        let sgr = self.pen.transition(pen);
        self.buf.push_str(&sgr);
        self.pen = *pen;
    }

    /// Back to no style and no link, as erasing needs and as the screen
    /// should be left after a frame.
    pub(crate) fn plain(&mut self) {
        self.style(&Pen::DEFAULT, &None);
    }

    /// Erases from the cursor to the end of the line.
    pub(crate) fn erase_line_right(&mut self) {
        self.plain();
        self.buf.push_str("\x1b[K");
    }

    /// Erases from the cursor to the end of the screen.
    pub(crate) fn erase_below(&mut self) {
        self.plain();
        self.buf.push_str("\x1b[J");
    }
}

/// `a`, unless `b` is shorter.
fn shortest(a: String, b: String) -> String {
    if b.len() < a.len() { b } else { a }
}

/// A control sequence with an optional count (1 is left out).
fn counted(n: usize, final_byte: char) -> String {
    if n == 1 {
        format!("\x1b[{final_byte}")
    } else {
        format!("\x1b[{n}{final_byte}")
    }
}

/// Up or down from row `from` to row `to`.
fn vertical(from: usize, to: usize) -> String {
    match to.cmp(&from) {
        std::cmp::Ordering::Less => counted(from - to, 'A'),
        std::cmp::Ordering::Greater => counted(to - from, 'B'),
        std::cmp::Ordering::Equal => String::new(),
    }
}

/// Left or right from column `from` (unknown: `None`) to column `to`.
fn horizontal(from: Option<usize>, to: usize) -> String {
    let home = if to == 0 {
        "\r".to_string()
    } else {
        format!("\r{}", counted(to, 'C'))
    };
    let column = format!("\x1b[{}G", to + 1);
    let step = match from {
        Some(f) if f == to => return String::new(),
        Some(f) if f < to => counted(to - f, 'C'),
        Some(f) => counted(f - to, 'D'),
        None => column.clone(),
    };
    shortest(shortest(step, column), home)
}

/// The absolute move to column `x` of row `y`.
fn cup(x: usize, y: usize) -> String {
    match (x, y) {
        (0, 0) => "\x1b[H".to_string(),
        (0, _) => format!("\x1b[{}H", y + 1),
        _ => format!("\x1b[{};{}H", y + 1, x + 1),
    }
}

#[cfg(test)]
mod tests;
