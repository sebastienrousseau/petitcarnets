//! Moving around the soft wrapped text: where the cursor is in its
//! wrapped line, up and down by screen line and by page, and scrolling
//! to keep it in view. Soft wrapping is Bubbles' `wrap`.

use super::{LineInfo, MIN_HEIGHT, Model};
use crate::bubbles::text::{clusters, grapheme_at, is_space};
use crate::style::width;

impl Model {
    /// `line` soft wrapped at the text's width, as Bubbles wraps it:
    /// at spaces, which stay at the end of their wrapped line, and inside
    /// words too long for a line. The last wrapped line always ends with
    /// a space, where the cursor sits at the end.
    pub(super) fn wrap(&self, line: &[char]) -> Vec<Vec<char>> {
        wrap(line, self.width)
    }

    /// Where the cursor is within its wrapped line.
    #[must_use]
    pub fn line_info(&self) -> LineInfo {
        let grid = self.wrap(&self.value[self.row]);
        let col = self.col;
        let mut start = 0;
        for (i, line) in grid.iter().enumerate() {
            let end = start + line.len();
            let next = (end == col && i + 1 < grid.len()).then_some(i + 1);
            if let Some(n) = next {
                return info_at(&grid, n, col, col, cells(line));
            }
            if end >= col {
                return info_at(&grid, i, start, col, cells(line));
            }
            start = end;
        }
        LineInfo::default()
    }

    /// The screen line the cursor is on, counting wrapped lines.
    pub(super) fn cursor_line_number(&self) -> usize {
        let above: usize = self.value[..self.row]
            .iter()
            .map(|l| self.wrap(l).len())
            .sum();
        above + self.line_info().row_offset
    }

    /// The screen lines the text takes.
    pub(super) fn total_visual_lines(&self) -> usize {
        self.value.iter().map(|l| self.wrap(l).len().max(1)).sum()
    }

    /// Scrolls so the cursor's line shows.
    pub(super) fn reposition_view(&mut self) {
        let height = self.viewport.height();
        if self.total_visual_lines() <= height {
            self.viewport.goto_top();
            return;
        }
        let (row, top) = (self.cursor_line_number(), self.viewport.y_offset());
        if row < top {
            self.viewport.scroll_up(top - row);
        } else {
            self.viewport
                .scroll_down((row + 1).saturating_sub(top + height));
        }
    }

    /// Moves the cursor `delta` screen lines down (or up if negative),
    /// keeping its column in cells as far as it goes.
    fn set_cursor_line_relative(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        let offset = self.last_char_offset.max(self.line_info().char_offset);
        self.last_char_offset = offset;
        self.step_lines(delta);
        let li = self.line_info();
        self.col = li.start_column;
        if li.width > 0 {
            self.keep_cells(offset, li.char_width);
        }
        self.reposition_view();
    }

    /// `delta` screen lines down, or up if negative.
    fn step_lines(&mut self, delta: isize) {
        for _ in 0..delta.unsigned_abs() {
            let li = self.line_info();
            if delta > 0 {
                self.step_down(&li);
            } else {
                self.step_up(&li);
            }
        }
    }

    /// Moves right from the wrapped line's start by up to `offset`
    /// cells, staying within its `room` cells, on cluster starts.
    fn keep_cells(&mut self, offset: usize, room: usize) {
        let line = &self.value[self.row];
        let mut col = self.col;
        let mut moved = 0;
        while moved < offset && col < line.len() && moved + 1 < room {
            let (_, end) = grapheme_at(line, col);
            moved += cells(&line[col..end]);
            col = end;
        }
        if col < line.len() {
            col = grapheme_at(line, col).0;
        }
        self.col = col;
    }

    /// One screen line down from `li`.
    fn step_down(&mut self, li: &LineInfo) {
        if li.row_offset + 1 >= li.height && self.row + 1 < self.value.len() {
            self.row += 1;
            self.col = 0;
        } else {
            self.col = (li.start_column + li.width + 2).min(self.value[self.row].len());
        }
    }

    /// One screen line up from `li`.
    fn step_up(&mut self, li: &LineInfo) {
        if li.row_offset == 0 && self.row > 0 {
            self.row -= 1;
            self.col = self.value[self.row].len();
        } else {
            self.col = li.start_column.saturating_sub(2);
        }
    }

    /// A screen line down.
    pub fn cursor_down(&mut self) {
        self.set_cursor_line_relative(1);
    }

    /// A screen line up.
    pub fn cursor_up(&mut self) {
        self.set_cursor_line_relative(-1);
    }

    /// Moves the cursor to column `col` of its line, at the start of the
    /// grapheme cluster there.
    pub fn set_cursor_column(&mut self, col: usize) {
        let line = &self.value[self.row];
        self.col = col.min(line.len());
        if self.col < line.len() {
            self.col = grapheme_at(line, self.col).0;
        }
        self.last_char_offset = 0;
    }

    /// To the start of the line.
    pub fn cursor_start(&mut self) {
        self.set_cursor_column(0);
    }

    /// To the end of the line.
    pub fn cursor_end(&mut self) {
        self.set_cursor_column(self.value[self.row].len());
    }

    /// To the start of the text.
    pub fn move_to_begin(&mut self) {
        self.row = 0;
        self.set_cursor_column(0);
        self.reposition_view();
    }

    /// To the end of the text.
    pub fn move_to_end(&mut self) {
        self.row = self.value.len() - 1;
        self.set_cursor_column(self.value[self.row].len());
        self.reposition_view();
    }

    /// A page up: to the top of the view, or a page above it.
    pub fn page_up(&mut self) {
        let (top, line) = (self.viewport.y_offset(), self.cursor_line_number());
        if line > top {
            self.set_cursor_line_relative(signed(top) - signed(line));
            return;
        }
        self.set_cursor_line_relative(-signed(self.height));
    }

    /// A page down: to the bottom of the view, or a page below it.
    pub fn page_down(&mut self) {
        let into = signed(self.cursor_line_number()) - signed(self.viewport.y_offset());
        if into < signed(self.height) - 1 {
            self.set_cursor_line_relative(signed(self.height) - 1 - into);
            return;
        }
        self.set_cursor_line_relative(signed(self.height));
    }

    /// With a dynamic height, the height follows the content.
    pub(super) fn recalculate_height(&mut self) {
        if !self.dynamic_height {
            return;
        }
        let total = self.total_visual_lines();
        let mut h = total.max(self.min_height.max(MIN_HEIGHT));
        if self.max_height > 0 {
            h = h.min(self.max_height);
        }
        let most = total.saturating_sub(h);
        if self.viewport.y_offset() > most {
            self.viewport.set_y_offset(most);
        }
        self.set_height(h);
    }
}

/// The cells `chars` take.
fn cells(chars: &[char]) -> usize {
    width(&chars.iter().collect::<String>())
}

/// Where the cursor is when it is at character `col` of wrapped line
/// `i` of `grid`, which starts at character `start`; `char_width` is the
/// width Go gives, that of the line the cursor was found at the end of.
fn info_at(grid: &[Vec<char>], i: usize, start: usize, col: usize, char_width: usize) -> LineInfo {
    let line = &grid[i];
    let into = col - start;
    LineInfo {
        char_offset: cells(&line[..into.min(line.len())]),
        column_offset: into,
        height: grid.len(),
        row_offset: i,
        start_column: start,
        width: line.len(),
        char_width,
    }
}

/// `n` as a signed count.
fn signed(n: usize) -> isize {
    isize::try_from(n).unwrap_or(isize::MAX)
}

/// Bubbles' `wrap`, cluster by cluster, measured with the program's
/// width method.
pub(super) fn wrap(line: &[char], columns: usize) -> Vec<Vec<char>> {
    let text: String = line.iter().collect();
    let mut wrap = SoftWrap {
        columns,
        lines: vec![Vec::new()],
        word: Vec::new(),
    };
    for (cluster, w) in clusters(&text) {
        let mut chars = cluster.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if is_space(c) => wrap.space(),
            _ => wrap.cluster(cluster, w),
        }
    }
    wrap.finish()
}

/// Soft wrapping one line, as Bubbles does.
struct SoftWrap {
    columns: usize,
    lines: Vec<Vec<char>>,
    word: Vec<char>,
}

impl SoftWrap {
    /// The wrapped line being filled.
    fn last(&mut self) -> &mut Vec<char> {
        let last = self.lines.len() - 1;
        &mut self.lines[last]
    }

    /// The cells of the line being filled and the word together.
    fn filled(&self) -> usize {
        self.lines.last().map_or(0, |l| cells(l)) + cells(&self.word)
    }

    /// A space ends the word: it goes on this line with the space, or
    /// on the next if they do not fit.
    fn space(&mut self) {
        if self.filled() + 1 > self.columns {
            self.lines.push(Vec::new());
        }
        let word = std::mem::take(&mut self.word);
        let last = self.last();
        last.extend(word);
        last.push(' ');
    }

    /// A cluster of the word; as in Go, it is counted twice when testing
    /// whether the word still fits, and a word that does not is broken.
    fn cluster(&mut self, cluster: &str, w: usize) {
        self.word.extend(cluster.chars());
        if cells(&self.word) + w > self.columns {
            if !self.last().is_empty() {
                self.lines.push(Vec::new());
            }
            let word = std::mem::take(&mut self.word);
            self.last().extend(word);
        }
    }

    /// The last word, and a space after it for the cursor, on this line
    /// or the next.
    fn finish(mut self) -> Vec<Vec<char>> {
        let filled = self.filled();
        let mut word = std::mem::take(&mut self.word);
        word.push(' ');
        if filled > self.columns {
            self.lines.push(word);
            return self.lines;
        }
        if filled == self.columns {
            let _ = word.pop();
            self.last().extend(word);
            self.lines.push(vec![' ']);
            return self.lines;
        }
        self.last().extend(word);
        self.lines
    }
}
