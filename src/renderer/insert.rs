//! Printing lines above an inline frame: Bubble Tea's `insertAbove`,
//! with wrapped rows counted as the terminal wraps them.

use super::cell::Cell;
use super::layout::{Pens, line_cells};
use super::writer::{Addressing, Writer};
use crate::unicode::WidthMethod;
use std::cmp::Ordering;
use std::fmt::Write as _;

/// The bytes that print `text`'s lines, each erased first and ended
/// with `\r\n`, with autowrap on while they are written so long lines
/// wrap; and how many rows they take on a screen `width` columns wide.
/// Styles go through the same cells as frames, and end with each line.
pub(crate) fn encode(text: &str, width: usize, method: WidthMethod) -> (String, usize) {
    let mut pens = Pens::default();
    let mut w = Writer::new(usize::MAX, 1, Addressing::Relative);
    let mut rows = 0;
    w.raw("\x1b[?7h");
    for line in text.split('\n') {
        let cells = line_cells(line, usize::MAX, method, &mut pens);
        rows += wrapped_rows(&cells, width);
        w.raw("\x1b[K");
        for cell in &cells {
            w.put(cell);
        }
        w.plain();
        w.raw("\r\n");
    }
    w.raw("\x1b[?7l");
    (w.buf, rows)
}

/// How many rows `cells` take when the terminal wraps them at `width`:
/// a wide character that does not fit at the end of a row moves whole
/// to the next.
fn wrapped_rows(cells: &[Cell], width: usize) -> usize {
    let mut rows = 1;
    let mut x = 0;
    for cell in cells.iter().filter(|c| !c.is_continuation()) {
        let w = usize::from(cell.width);
        if x > 0 && x + w > width {
            rows += 1;
            x = 0;
        }
        x += w;
    }
    rows
}

/// The moves that open `rows` blank rows above a frame `height` rows
/// tall whose row `y` holds the cursor (below the frame after it
/// shrank), as Bubble Tea does: to the
/// frame's last row, line feeds that scroll the screen when needed, up
/// to where the new rows go, and an insert that pushes the frame down.
/// The cursor ends at the first new row.
pub(crate) fn make_room(y: usize, height: usize, rows: usize) -> String {
    let mut s = String::from("\r");
    let last = height - 1;
    match last.cmp(&y) {
        Ordering::Greater => {
            let _ = write!(s, "\x1b[{}B", last - y);
        }
        Ordering::Less => {
            let _ = write!(s, "\x1b[{}A", y - last);
        }
        Ordering::Equal => {}
    }
    s.push_str(&"\n".repeat(rows));
    let up = rows + last;
    if up > 0 {
        let _ = write!(s, "\x1b[{up}A");
    }
    let _ = write!(s, "\x1b[{rows}L");
    s
}

#[cfg(test)]
mod tests;
