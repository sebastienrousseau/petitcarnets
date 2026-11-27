//! Finding a frame that is the last one moved up or down, as a log or
//! a pager does, so the terminal can scroll it (SU and SD, inside a
//! scroll region when only part of the screen moves) instead of having
//! every row written again.

use super::layout::{Frame, Row};
use super::writer::Writer;
use std::collections::HashMap;

/// Rows `top..=bottom` move up by `by` rows (down when negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Scroll {
    /// The region's first row.
    pub(crate) top: usize,
    /// The region's last row.
    pub(crate) bottom: usize,
    /// How far the rows move up; negative is down.
    pub(crate) by: isize,
}

/// The move that would carry most of `old`'s rows to where `new` has
/// them, if there is one.
pub(crate) fn detect(old: &Frame, new: &Frame) -> Option<Scroll> {
    if old.rows.len() != new.rows.len() || old.width != new.width {
        return None;
    }
    let by = best_shift(old, new)?;
    let matched: Vec<usize> = (0..new.rows.len())
        .filter(|&i| source(i, by).is_some_and(|j| same(old.rows.get(j), &new.rows[i])))
        .collect();
    let (first, last) = (*matched.first()?, *matched.last()?);
    let (from, to) = (source(first, by)?, source(last, by)?);
    Some(Scroll {
        top: first.min(from),
        bottom: last.max(to),
        by,
    })
}

/// The old row that new row `i` comes from after a move of `by`.
fn source(i: usize, by: isize) -> Option<usize> {
    i.checked_add_signed(by)
}

/// Whether `old` is there and shows the same as `new`.
fn same(old: Option<&Row>, new: &Row) -> bool {
    old.is_some_and(|o| o.hash == new.hash && o == new)
}

/// The most common distance between a changed row and an old row that
/// looks the same; blank rows do not count.
fn best_shift(old: &Frame, new: &Frame) -> Option<isize> {
    let at = by_hash(old);
    let mut votes: HashMap<isize, usize> = HashMap::new();
    for (i, row) in new.rows.iter().enumerate() {
        if same(old.rows.get(i), row) {
            continue;
        }
        for &j in at.get(&row.hash).into_iter().flatten() {
            let by = isize::try_from(j).ok()? - isize::try_from(i).ok()?;
            *votes.entry(by).or_default() += 1;
        }
    }
    votes
        .into_iter()
        .filter(|(_, n)| *n >= 2)
        .max_by_key(|&(by, n)| (n, std::cmp::Reverse(by.unsigned_abs()), by))
        .map(|(by, _)| by)
}

/// Where each row that is not blank sits, by hash.
fn by_hash(frame: &Frame) -> HashMap<u64, Vec<usize>> {
    let mut at: HashMap<u64, Vec<usize>> = HashMap::new();
    for (j, row) in frame.rows.iter().enumerate() {
        if row.content_end() > 0 {
            at.entry(row.hash).or_default().push(j);
        }
    }
    at
}

/// Writes the scroll and returns what the screen then shows.
pub(crate) fn apply(w: &mut Writer, old: &Frame, scroll: Scroll) -> Frame {
    let full = scroll.top == 0 && scroll.bottom + 1 == old.rows.len();
    w.plain();
    if !full {
        w.raw(&format!("\x1b[{};{}r", scroll.top + 1, scroll.bottom + 1));
    }
    let n = scroll.by.unsigned_abs();
    let code = if scroll.by > 0 { 'S' } else { 'T' };
    w.raw(&if n == 1 {
        format!("\x1b[{code}")
    } else {
        format!("\x1b[{n}{code}")
    });
    if !full {
        w.raw("\x1b[r");
        w.x = Some(0);
        w.y = 0;
    }
    shifted(old, scroll)
}

/// `old` with the region moved as the terminal moves it: rows leaving
/// it are gone, rows coming in are blank.
fn shifted(old: &Frame, scroll: Scroll) -> Frame {
    let mut rows = old.rows.clone();
    for (k, row) in rows
        .iter_mut()
        .enumerate()
        .take(scroll.bottom + 1)
        .skip(scroll.top)
    {
        *row = source(k, scroll.by)
            .filter(|j| (scroll.top..=scroll.bottom).contains(j))
            .map_or_else(|| Row::blank(old.width), |j| old.rows[j].clone());
    }
    Frame {
        width: old.width,
        rows,
    }
}

#[cfg(test)]
mod tests;
