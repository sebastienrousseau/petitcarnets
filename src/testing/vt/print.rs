//! Printing text, the C0 controls, and moving down or up a line.

use super::Vt;
use crate::renderer::cell::Cell;

impl Vt {
    /// Prints a grapheme cluster `width` cells wide; a zero-width one
    /// joins the cell last printed.
    pub(super) fn print(&mut self, cluster: &str, width: usize) {
        if width == 0 {
            self.combine(cluster);
            return;
        }
        let limit = self.columns.min(u16::from(u8::MAX));
        let width = u16::try_from(width).unwrap_or(limit).min(limit);
        self.make_room(width);
        self.put(cluster, width);
        self.advance(width);
    }

    /// Adds a zero-width `mark` to the cell last printed, if any.
    fn combine(&mut self, mark: &str) {
        if let Some((x, y)) = self.last {
            self.screen_mut().cell_mut(x, y).append(mark);
        }
    }

    /// Wraps first if a wrap is pending or `width` cells do not fit;
    /// without autowrap, moves back so they end at the last column.
    fn make_room(&mut self, width: u16) {
        let autowrap = self.autowrap();
        if self.cursor.wrap && autowrap {
            self.wrap();
        }
        if self.cursor.x.saturating_add(width) > self.columns {
            if autowrap {
                self.wrap();
            } else {
                self.cursor.x = self.columns - width;
            }
        }
        self.cursor.wrap = false;
    }

    /// Writes the cluster and its continuations at the cursor.
    fn put(&mut self, cluster: &str, width: u16) {
        let (x, y) = (self.cursor.x, self.cursor.y);
        let (pen, link, blank) = (self.pen, self.link.clone(), self.blank());
        let wide = u8::try_from(width).unwrap_or(u8::MAX);
        let grid = self.screen_mut();
        *grid.cell_mut(x, y) = Cell::new(cluster, wide, pen, link.clone());
        for i in 1..width {
            *grid.cell_mut(x + i, y) = Cell::continuation(pen, link.clone());
        }
        grid.normalise(y, &blank);
        self.last = Some((x, y));
    }

    /// Moves the cursor past `width` cells; at the right edge it stays
    /// on the last column, a wrap pending if autowrap is on.
    fn advance(&mut self, width: u16) {
        let x = self.cursor.x.saturating_add(width);
        if x < self.columns {
            self.cursor.x = x;
        } else {
            self.cursor.x = self.columns - 1;
            self.cursor.wrap = self.autowrap();
        }
    }

    /// Goes to the start of the next line, scrolling if need be.
    fn wrap(&mut self) {
        self.cursor.x = 0;
        self.linefeed();
    }

    /// Moves down a line, scrolling the region at its bottom margin.
    pub(super) fn linefeed(&mut self) {
        self.cursor.wrap = false;
        if self.cursor.y == self.bottom {
            self.scroll_up(1);
        } else if self.cursor.y + 1 < self.rows {
            self.cursor.y += 1;
        }
    }

    /// Moves up a line, scrolling the region down at its top margin.
    pub(super) fn reverse_index(&mut self) {
        self.cursor.wrap = false;
        if self.cursor.y == self.top {
            self.scroll_down(1);
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
        }
    }

    /// Scrolls the region up `n` lines. Lines that leave the whole main
    /// screen go to the scrollback.
    pub(super) fn scroll_up(&mut self, n: u16) {
        let blank = self.blank();
        let (top, bottom) = (self.top, self.bottom);
        let lost = self.screen_mut().scroll_up(top, bottom, n, &blank);
        if !self.on_alt && top == 0 && bottom == self.rows - 1 {
            self.scrollback.extend(lost);
        }
    }

    /// Scrolls the region down `n` lines.
    pub(super) fn scroll_down(&mut self, n: u16) {
        let blank = self.blank();
        let (top, bottom) = (self.top, self.bottom);
        self.screen_mut().scroll_down(top, bottom, n, &blank);
    }

    /// Acts on a C0 control. Tab and backspace work but are logged, as
    /// a renderer should never send them.
    pub(super) fn control(&mut self, byte: u8) {
        match byte {
            b'\r' => self.move_to(0, self.cursor.y),
            0x0A..=0x0C => self.linefeed(),
            0x07 => {}
            b'\t' => self.tab(),
            0x08 => self.backspace(),
            _ => self.log(format!("C0 {byte:02X}")),
        }
    }

    /// Moves to the next multiple of 8, or the last column; logged.
    fn tab(&mut self) {
        self.log("C0 09".to_string());
        let next = (self.cursor.x / 8 + 1).saturating_mul(8);
        self.move_to(next, self.cursor.y);
    }

    /// Moves left one column; logged.
    fn backspace(&mut self) {
        self.log("C0 08".to_string());
        self.move_to(self.cursor.x.saturating_sub(1), self.cursor.y);
    }
}
