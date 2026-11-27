//! Turning the frame on screen into the next one: unchanged rows are
//! skipped by their hashes, and changed rows rewrite only the cells
//! that differ, erasing a row's old tail with one sequence when that is
//! shorter.

use super::cell::Cell;
use super::layout::{Frame, Row};
use super::writer::Writer;

/// Changed cells this close together are rewritten as one run, which
/// costs less than moving the cursor over the gap.
const GAP: usize = 4;

/// Erasing to the end of the line takes this many bytes.
const ERASE_COST: usize = 3;

/// Draws `new` over `old`, which the screen shows now; both are as wide
/// as the window.
pub(crate) fn draw(w: &mut Writer, old: &Frame, new: &Frame) {
    for (y, row) in new.rows.iter().enumerate() {
        match old.rows.get(y) {
            Some(was) if was.hash == row.hash && was == row => {}
            Some(was) => update_row(w, y, was, row),
            None => update_row(w, y, &Row::blank(new.width), row),
        }
    }
}

/// Rewrites what changed in row `y`, from `old` to `new`.
fn update_row(w: &mut Writer, y: usize, old: &Row, new: &Row) {
    let (new_end, old_end) = (new.content_end(), old.content_end());
    let erase_from = tail_start(old, new, new_end, old_end);
    let limit = erase_from.unwrap_or_else(|| new_end.max(old_end));
    for (from, to) in spans(&old.cells, &new.cells, limit) {
        let from = rewind(w, y, from, &old.cells, &new.cells);
        w.move_to(from, y);
        for cell in &new.cells[from..to] {
            w.put(cell);
        }
    }
    if let Some(x) = erase_from {
        w.move_to(x, y);
        w.erase_line_right();
    }
}

/// Where to start writing a run that starts at `from` in row `y`: from
/// the cursor instead, when it is a few cells to the left on the same
/// row, as writing those cells again costs less than moving.
fn rewind(w: &Writer, y: usize, from: usize, old: &[Cell], new: &[Cell]) -> usize {
    match w.x {
        Some(x) if w.y == y && x < from && from - x < ERASE_COST => {
            let start = widen_left(old, new, x);
            if start == x { x } else { from }
        }
        _ => from,
    }
}

/// Where to erase the rest of the row from, when the old row ran on
/// past the new one's end far enough that erasing beats writing spaces.
fn tail_start(old: &Row, new: &Row, new_end: usize, old_end: usize) -> Option<usize> {
    if old_end <= new_end {
        return None;
    }
    // Cells from the new end to the first change are blank in both, so
    // erasing from the new end is the same, and needs no move past them.
    let first = (new_end..old_end).find(|&i| old.cells[i] != new.cells[i])?;
    (old_end - first > ERASE_COST).then_some(new_end)
}

/// The runs of cells that differ before `limit`, as `(from, to)`, close
/// runs joined and widened so no wide cell is cut.
fn spans(old: &[Cell], new: &[Cell], limit: usize) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < limit {
        if old[i] == new[i] {
            i += 1;
            continue;
        }
        let to = run_end(old, new, i, limit);
        join(&mut out, widen_left(old, new, i), to);
        i = to;
    }
    out
}

/// Where the run of changed cells starting at `i` ends, widened so no
/// wide cell is cut.
fn run_end(old: &[Cell], new: &[Cell], i: usize, limit: usize) -> usize {
    let mut to = i + 1;
    while to < limit && old[to] != new[to] {
        to += 1;
    }
    widen_right(old, new, to)
}

/// Adds the run `from..to`, joining it to the last one when the gap
/// between them is small.
fn join(out: &mut Vec<(usize, usize)>, from: usize, to: usize) {
    match out.last_mut() {
        Some(last) if from <= last.1 + GAP => last.1 = last.1.max(to),
        _ => out.push((from, to)),
    }
}

/// Moves `i` left off any cell covered by a wide cell, old or new.
fn widen_left(old: &[Cell], new: &[Cell], mut i: usize) -> usize {
    while i > 0 && (old[i].is_continuation() || new[i].is_continuation()) {
        i -= 1;
    }
    i
}

/// Moves the end `to` right past any cell covered by a wide cell.
fn widen_right(old: &[Cell], new: &[Cell], mut to: usize) -> usize {
    while to < new.len() && (old[to].is_continuation() || new[to].is_continuation()) {
        to += 1;
    }
    to
}

#[cfg(test)]
mod tests;
