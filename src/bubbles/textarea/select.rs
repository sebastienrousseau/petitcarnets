//! Selecting text: with Shift and the moves, everything, or the mouse
//! (through [`Model::begin_selection`] and the like), and what happens
//! to it: deleted by typing, copied with OSC 52.

use super::{HAS_SELECTION, Model, SELECTING};
use crate::bubbles::text::clusters;
use crate::{Cmd, set_clipboard};

/// A place in the text: a line and a column in characters. Bubbles'
/// `textarea.Position`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// use textarea::Position;
///
/// let mut area = textarea::Model::new();
/// area.set_value("ab\ncd");
/// area.select_all();
/// let end = Position { row: 1, col: 2 };
/// assert_eq!(area.selection(), Some((Position::default(), end)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Position {
    /// The line, from 0.
    pub row: usize,
    /// The column, in characters.
    pub col: usize,
}

impl Model {
    /// The columns before the text: prompt and line numbers.
    fn gutter_width(&self) -> usize {
        let numbers = if self.show_line_numbers {
            super::digits(self.max_height) + 2
        } else {
            0
        };
        self.prompt_width + numbers
    }

    /// The place in the text at cell `x` of screen row `y` of the view.
    #[must_use]
    pub fn position_at(&self, x: usize, y: usize) -> Position {
        let target = y + self.viewport.y_offset();
        let content_x = x.saturating_sub(self.gutter_width());
        let mut display = 0;
        for (row, line) in self.value.iter().enumerate() {
            let mut base = 0;
            for wrapped in self.wrap(line) {
                if display == target {
                    let col = base + column_index(&wrapped, content_x);
                    return Position {
                        row,
                        col: col.min(line.len()),
                    };
                }
                base += wrapped.len();
                display += 1;
            }
        }
        let row = self.value.len() - 1;
        Position {
            row,
            col: self.value[row].len(),
        }
    }

    /// Starts a mouse selection at cell `x` of row `y`.
    pub fn begin_selection(&mut self, x: usize, y: usize) {
        let pos = self.position_at(x, y);
        self.select_from(pos, pos);
        self.flags.set(SELECTING, true);
        self.move_cursor_to(pos);
    }

    /// Extends a mouse selection to cell `x` of row `y`.
    pub fn extend_selection(&mut self, x: usize, y: usize) {
        if !self.flags.get(SELECTING) {
            return;
        }
        let pos = self.position_at(x, y);
        self.head = pos;
        self.flags.set(HAS_SELECTION, true);
        self.move_cursor_to(pos);
    }

    /// Ends a mouse selection; an empty one is dropped.
    pub fn end_selection(&mut self) {
        self.flags.set(SELECTING, false);
        if self.anchor == self.head {
            self.clear_selection();
        }
    }

    /// Selects everything.
    pub fn select_all(&mut self) {
        let row = self.value.len() - 1;
        let end = Position {
            row,
            col: self.value[row].len(),
        };
        self.select_from(Position::default(), end);
        self.flags.set(SELECTING, false);
    }

    /// Drops the selection.
    pub fn clear_selection(&mut self) {
        self.flags.set(HAS_SELECTION, false);
        self.flags.set(SELECTING, false);
        self.anchor = Position::default();
        self.head = Position::default();
    }

    /// Whether some text is selected.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        self.flags.get(HAS_SELECTION) && self.anchor != self.head
    }

    /// The selection's start and end, in order.
    #[must_use]
    pub fn selection(&self) -> Option<(Position, Position)> {
        if !self.has_selection() {
            return None;
        }
        let (a, b) = (self.anchor, self.head);
        Some(if b < a { (b, a) } else { (a, b) })
    }

    /// The selected text, lines joined with `\n`.
    #[must_use]
    pub fn selected_text(&self) -> String {
        let Some((start, end)) = self.selection() else {
            return String::new();
        };
        let lines = self
            .value
            .iter()
            .enumerate()
            .take(end.row + 1)
            .skip(start.row);
        let parts: Vec<String> = lines
            .map(|(row, line)| {
                let from = if row == start.row {
                    start.col.min(line.len())
                } else {
                    0
                };
                let to = if row == end.row {
                    end.col.min(line.len())
                } else {
                    line.len()
                };
                line[from..to.max(from)].iter().collect()
            })
            .collect();
        parts.join("\n")
    }

    /// Deletes the selected text, the cursor where it started.
    pub fn delete_selection(&mut self) {
        let Some((start, end)) = self.selection() else {
            return;
        };
        let head_col = start.col.min(self.value[start.row].len());
        let tail_col = end.col.min(self.value[end.row].len());
        let mut merged = self.value[start.row][..head_col].to_vec();
        merged.extend_from_slice(&self.value[end.row][tail_col..]);
        let _ = self.value.drain(start.row + 1..=end.row);
        self.value[start.row] = merged;
        self.row = start.row;
        self.set_cursor_column(head_col);
        self.clear_selection();
    }

    /// Copies the selection to the clipboard (OSC 52).
    #[must_use]
    pub fn copy_selection<M>(&self) -> Cmd<M> {
        if self.has_selection() {
            set_clipboard(&self.selected_text())
        } else {
            Cmd::none()
        }
    }

    /// Starts a keyboard selection at the cursor, if none is going.
    pub(super) fn start_keyboard_selection(&mut self) {
        let here = Position {
            row: self.row,
            col: self.col,
        };
        if !self.flags.get(HAS_SELECTION) {
            self.anchor = here;
        }
        self.flags.set(HAS_SELECTION, true);
        self.head = here;
    }

    /// The selection follows the cursor.
    pub(super) fn update_keyboard_selection(&mut self) {
        self.head = Position {
            row: self.row,
            col: self.col,
        };
    }

    /// Sets the selection.
    fn select_from(&mut self, anchor: Position, head: Position) {
        self.anchor = anchor;
        self.head = head;
        self.flags.set(HAS_SELECTION, true);
    }

    /// Puts the cursor at `pos`, within the text.
    fn move_cursor_to(&mut self, pos: Position) {
        self.row = pos.row.min(self.value.len() - 1);
        self.col = pos.col.min(self.value[self.row].len());
    }

    /// The selected part, `(from, to)`, of the wrapped line of `row`
    /// that starts at character `base` and is `length` long.
    pub(super) fn selection_span(
        &self,
        row: usize,
        base: usize,
        length: usize,
    ) -> Option<(usize, usize)> {
        let (from, to) = self.selected_columns(row)?;
        let (from, to) = (from.max(base), to.min(base + length));
        (from < to).then(|| (from - base, to - base))
    }

    /// The selected columns of line `row`, if any.
    fn selected_columns(&self, row: usize) -> Option<(usize, usize)> {
        let (start, end) = self.selection()?;
        if !(start.row..=end.row).contains(&row) {
            return None;
        }
        let len = self.value[row].len();
        let from = if row == start.row {
            start.col.min(len)
        } else {
            0
        };
        let to = if row == end.row {
            end.col.min(len)
        } else {
            len
        };
        Some((from, to))
    }
}

/// The character at cell `x` of `line`, by whole grapheme clusters.
fn column_index(line: &[char], x: usize) -> usize {
    let text: String = line.iter().collect();
    let (mut cells, mut i) = (0, 0);
    for (cluster, w) in clusters(&text) {
        if cells + w > x {
            return i;
        }
        cells += w;
        i += cluster.chars().count();
    }
    line.len()
}
