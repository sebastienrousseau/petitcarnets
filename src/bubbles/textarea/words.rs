//! Moving by character and word, deleting words, case changes and
//! transposition.

use super::Model;
use crate::bubbles::text::{grapheme_at, is_space};

impl Model {
    /// Whether column `col` of the cursor's line is a space; not past
    /// the end.
    fn space_at(&self, col: usize) -> bool {
        self.row_chars().get(col).is_some_and(|c| is_space(*c))
    }

    /// Whether column `col` of the cursor's line is part of a word.
    fn word_at(&self, col: usize) -> bool {
        self.row_chars().get(col).is_some_and(|c| !is_space(*c))
    }

    /// A character back, to the start of its cluster.
    fn step_back(&mut self) {
        self.set_cursor_column(self.col - 1);
    }

    /// The word before the cursor, or the line break before the line.
    pub(super) fn delete_word_backward(&mut self) {
        if self.col == 0 {
            self.merge_line_above(self.row);
            return;
        }
        let old = self.col;
        self.back_to_word_start();
        let (col, end) = (self.col, old.min(self.line_len()));
        let _ = self.line_mut().drain(col..end);
    }

    /// Back past spaces, then to just after the space before the word,
    /// as Bubbles' word deletion goes.
    fn back_to_word_start(&mut self) {
        self.step_back();
        while self.space_at(self.col) && self.col > 0 {
            self.step_back();
        }
        while self.col > 0 {
            if self.space_at(self.col) {
                self.character_right();
                return;
            }
            self.step_back();
        }
    }

    /// The word after the cursor, or the line break at the end.
    pub(super) fn delete_word_forward(&mut self) {
        let old = self.col.min(self.line_len());
        self.col = old;
        if old >= self.line_len() {
            self.merge_line_below(self.row);
            return;
        }
        self.forward_past_word();
        let end = self.col.min(self.line_len());
        let _ = self.line_mut().drain(old..end);
        self.set_cursor_column(old);
    }

    /// On past spaces, then past the word.
    fn forward_past_word(&mut self) {
        while self.space_at(self.col) {
            self.character_right();
        }
        while self.word_at(self.col) {
            self.character_right();
        }
    }

    /// One grapheme cluster right, or to the next line's start.
    pub(super) fn character_right(&mut self) {
        if self.col < self.line_len() {
            let (_, end) = grapheme_at(self.row_chars(), self.col);
            self.set_cursor_column(end);
        } else if self.row + 1 < self.value.len() {
            self.row += 1;
            self.cursor_start();
        }
    }

    /// One grapheme cluster left, or to the end of the line before; with
    /// `inside_line`, also one cluster into that line.
    pub(super) fn character_left(&mut self, inside_line: bool) {
        if self.col == 0 && self.row != 0 {
            self.row -= 1;
            self.cursor_end();
            if !inside_line {
                return;
            }
        }
        if self.col > 0 {
            let (start, _) = grapheme_at(self.row_chars(), self.col - 1);
            self.set_cursor_column(start);
        }
    }

    /// To the start of the word before the cursor, across lines.
    pub(super) fn word_left(&mut self) {
        loop {
            let old = (self.row, self.col);
            self.character_left(true);
            if (self.row, self.col) == old {
                return;
            }
            if self.word_at(self.col) {
                break;
            }
        }
        while self.col > 0 && !self.space_at(self.col - 1) {
            self.step_back();
        }
    }

    /// To the end of the word after the cursor, across lines.
    pub(super) fn word_right(&mut self) {
        self.word_right_with(|_, c| c);
    }

    /// Whether the cursor is at the very end of the text.
    fn at_end(&self) -> bool {
        self.row + 1 == self.value.len() && self.col == self.line_len()
    }

    /// To the end of the word after the cursor, changing the first
    /// character of each cluster on the way with `change(index, c)`.
    pub(super) fn word_right_with(&mut self, change: impl Fn(usize, char) -> char) {
        while !self.word_at(self.col) && !self.at_end() {
            self.character_right();
        }
        let mut i = 0;
        while self.word_at(self.col) {
            let col = self.col;
            let c = &mut self.line_mut()[col];
            *c = change(i, *c);
            self.character_right();
            i += 1;
        }
    }

    /// Swaps the clusters before and under the cursor (at the end, the
    /// last two), and moves right.
    pub(super) fn transpose_left(&mut self) {
        if self.col == 0 || self.line_len() < 2 {
            return;
        }
        if self.col >= self.line_len() {
            self.step_back();
        }
        let col = self.col;
        if col == 0 {
            return;
        }
        let (prev, _) = grapheme_at(self.row_chars(), col - 1);
        let (_, end) = grapheme_at(self.row_chars(), col);
        self.line_mut()[prev..end].rotate_left(col - prev);
        self.set_cursor_column(end);
    }

    /// The word under or just before the cursor: Bubbles' `Word`.
    #[must_use]
    pub fn word(&self) -> String {
        let line = self.row_chars();
        let Some(col) = self.col.checked_sub(1).filter(|c| self.word_at(*c)) else {
            return String::new();
        };
        let start = line[..col]
            .iter()
            .rposition(|c| is_space(*c))
            .map_or(0, |p| p + 1);
        let end = line[col..]
            .iter()
            .position(|c| is_space(*c))
            .map_or(line.len(), |p| col + p);
        line[start..end].iter().collect()
    }
}
