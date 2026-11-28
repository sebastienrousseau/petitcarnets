//! Moving the selection and scrolling the rows as Bubbles does.

use super::Model;

impl Model {
    /// Selects row `n`, scrolling it into view.
    pub fn set_cursor(&mut self, n: usize) {
        self.cursor = n.min(self.last_row());
        self.update_viewport();
        let h = self.viewport.height();
        if self.rows.is_empty() || h == 0 {
            return;
        }
        let at = self.cursor - self.start;
        let offset = self.viewport.y_offset();
        let offset = offset.min(at).max((at + 1).saturating_sub(h));
        self.viewport.set_y_offset(offset);
    }

    /// The last row's index.
    fn last_row(&self) -> usize {
        self.rows.len().saturating_sub(1)
    }

    /// Moves the selection `n` rows up, scrolling as Bubbles does.
    pub fn move_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n).min(self.last_row());
        let offset = self.offset_after_up(signed(n));
        self.update_viewport();
        self.viewport.set_y_offset(unsigned(offset));
    }

    /// Bubbles' scroll offset after moving up `n` rows.
    fn offset_after_up(&self, n: isize) -> isize {
        let h = signed(self.viewport.height());
        let (cursor, start) = (signed(self.cursor), signed(self.start));
        let offset = signed(self.viewport.y_offset());
        if start == 0 {
            clamp(offset, 0, cursor)
        } else if start < h {
            clamp(clamp(offset + n, 0, cursor), 0, h)
        } else if offset >= 1 {
            clamp(offset + n, 1, h)
        } else {
            offset
        }
    }

    /// Moves the selection `n` rows down, scrolling as Bubbles does.
    pub fn move_down(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.last_row());
        let offset = signed(self.viewport.y_offset());
        self.update_viewport();
        let offset = self.offset_after_down(offset, signed(n));
        self.viewport.set_y_offset(unsigned(offset));
    }

    /// Bubbles' scroll offset, `offset` before, after moving down `n`
    /// rows. (Go's last case, for an offset of 1, leaves it at 1.)
    fn offset_after_down(&self, offset: isize, n: isize) -> isize {
        let h = signed(self.viewport.height());
        let cursor = signed(self.cursor);
        if offset <= 0 {
            return if cursor >= offset + h {
                clamp(offset + 1, 0, 1)
            } else {
                offset
            };
        }
        let high = if self.end == self.rows.len() {
            h
        } else if cursor > self.half_shown() {
            cursor
        } else {
            return offset;
        };
        clamp(offset - n, 1, high)
    }

    /// Half the rows drawn.
    fn half_shown(&self) -> isize {
        signed(self.end - self.start) / 2
    }

    /// To the first row.
    pub fn goto_top(&mut self) {
        self.move_up(self.cursor);
    }

    /// To the last row.
    pub fn goto_bottom(&mut self) {
        self.move_down(self.rows.len());
    }
}

/// Bubbles' table `clamp`: `low` first, then `high`, which wins.
fn clamp(v: isize, low: isize, high: isize) -> isize {
    v.max(low).min(high)
}

/// `n` as a signed count.
fn signed(n: usize) -> isize {
    isize::try_from(n).unwrap_or(isize::MAX)
}

/// `n`, or 0 if negative.
fn unsigned(n: isize) -> usize {
    usize::try_from(n).unwrap_or(0)
}
