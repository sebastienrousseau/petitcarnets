//! The cells of one screen, and the edits that move them about.

use crate::renderer::cell::Cell;

/// The cells of one screen, row by row.
#[derive(Debug, Clone)]
pub(super) struct Grid {
    /// The rows, top first; each holds one cell per column.
    rows: Vec<Vec<Cell>>,
    /// How many cells each row holds.
    columns: usize,
}

impl Grid {
    /// A grid of `columns` by `rows` cells, each a copy of `blank`.
    pub(super) fn new(columns: u16, rows: u16, blank: &Cell) -> Self {
        let row = vec![blank.clone(); usize::from(columns)];
        Self {
            rows: vec![row; usize::from(rows)],
            columns: usize::from(columns),
        }
    }

    /// The cell at column `x` of row `y`.
    pub(super) fn cell(&self, x: u16, y: u16) -> &Cell {
        &self.rows[usize::from(y)][usize::from(x)]
    }

    /// The cell at column `x` of row `y`, to change.
    pub(super) fn cell_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        &mut self.rows[usize::from(y)][usize::from(x)]
    }

    /// What row `y` shows, trailing spaces left out.
    pub(super) fn row_text(&self, y: u16) -> String {
        text_of(&self.rows[usize::from(y)])
    }

    /// Cuts or pads the grid to `columns` by `rows` with plain blanks.
    pub(super) fn resize(&mut self, columns: u16, rows: u16) {
        let width = usize::from(columns);
        for row in &mut self.rows {
            row.resize(width, Cell::BLANK);
        }
        self.rows
            .resize(usize::from(rows), vec![Cell::BLANK; width]);
        self.columns = width;
        for y in 0..rows {
            self.normalise(y, &Cell::BLANK);
        }
    }

    /// Fills columns `from..to` of row `y` with `blank`.
    pub(super) fn fill(&mut self, y: u16, from: u16, to: u16, blank: &Cell) {
        let to = usize::from(to).min(self.columns);
        let from = usize::from(from).min(to);
        self.rows[usize::from(y)][from..to].fill(blank.clone());
        self.normalise(y, blank);
    }

    /// Fills rows `from..to` with `blank`.
    pub(super) fn fill_rows(&mut self, from: u16, to: u16, blank: &Cell) {
        for y in from..to {
            self.fill(y, 0, u16::MAX, blank);
        }
    }

    /// Inserts `n` blanks at column `x` of row `y`; cells pushed past
    /// the right edge are lost.
    pub(super) fn insert_cells(&mut self, x: u16, y: u16, n: u16, blank: &Cell) {
        let x = usize::from(x);
        let n = usize::from(n).min(self.columns - x);
        let row = &mut self.rows[usize::from(y)];
        for _ in 0..n {
            row.insert(x, blank.clone());
        }
        row.truncate(self.columns);
        self.normalise(y, blank);
    }

    /// Deletes `n` cells at column `x` of row `y`; blanks come in at the
    /// right edge.
    pub(super) fn delete_cells(&mut self, x: u16, y: u16, n: u16, blank: &Cell) {
        let x = usize::from(x);
        let n = usize::from(n).min(self.columns - x);
        let row = &mut self.rows[usize::from(y)];
        drop(row.drain(x..x + n));
        row.resize(self.columns, blank.clone());
        self.normalise(y, blank);
    }

    /// Moves rows `top..=bottom` up `n` lines, blanks coming in at the
    /// bottom; the text of the rows pushed out, top first.
    pub(super) fn scroll_up(&mut self, top: u16, bottom: u16, n: u16, blank: &Cell) -> Vec<String> {
        let (top, end) = (usize::from(top), usize::from(bottom) + 1);
        let n = usize::from(n).min(end - top);
        let lost: Vec<String> = self.rows.drain(top..top + n).map(|r| text_of(&r)).collect();
        for _ in 0..n {
            self.rows.insert(end - n, vec![blank.clone(); self.columns]);
        }
        lost
    }

    /// Moves rows `top..=bottom` down `n` lines, blanks coming in at
    /// the top.
    pub(super) fn scroll_down(&mut self, top: u16, bottom: u16, n: u16, blank: &Cell) {
        let (top, end) = (usize::from(top), usize::from(bottom) + 1);
        let n = usize::from(n).min(end - top);
        drop(self.rows.drain(end - n..end));
        for _ in 0..n {
            self.rows.insert(top, vec![blank.clone(); self.columns]);
        }
    }

    /// Blanks what is left of any wide cell in row `y` that an edit cut
    /// in two: a leader without all its continuations, or a continuation
    /// without its leader.
    pub(super) fn normalise(&mut self, y: u16, blank: &Cell) {
        let row = &mut self.rows[usize::from(y)];
        let mut x = 0;
        while x < row.len() {
            let width = usize::from(row[x].width).max(1);
            if row[x].is_continuation() || !covered(row, x, width) {
                row[x] = blank.clone();
                x += 1;
            } else {
                x += width;
            }
        }
    }
}

/// Whether the `width - 1` cells after `row[x]` are all its
/// continuations.
fn covered(row: &[Cell], x: usize, width: usize) -> bool {
    x + width <= row.len() && row[x + 1..x + width].iter().all(Cell::is_continuation)
}

/// What a row shows, trailing spaces left out.
fn text_of(row: &[Cell]) -> String {
    let mut out = String::new();
    for cell in row {
        cell.push_text(&mut out);
    }
    out.truncate(out.trim_end_matches(' ').len());
    out
}
