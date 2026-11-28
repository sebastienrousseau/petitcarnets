//! Editing: keys to actions, insertion with its limits, deletion, and
//! joining and splitting lines.

use super::{KeyMap, MAX_LINES, Model, Msg, PASTE_PENDING};
use crate::bubbles::key::Binding;
use crate::bubbles::text::{grapheme_at, sanitize};
use crate::bubbles::textinput::edit::deletes_whole;
use crate::unicode::{to_lower, to_upper};
use crate::{Cmd, Key};

/// What an action asks of the update.
pub(super) enum Step {
    /// Go on.
    Done,
    /// Go on, and run this.
    Run(Cmd<Msg>),
    /// End the update now, as Go does when a new line is over the limit.
    Stop,
}

/// A binding of the key map.
type Field = fn(&KeyMap) -> &Binding;

/// What a binding does.
type Handler = fn(&mut Model) -> Step;

/// Does `f`, then goes on.
fn done(m: &mut Model, f: impl FnOnce(&mut Model)) -> Step {
    f(m);
    Step::Done
}

/// Ends any selection, does `f`, then goes on.
fn moving(m: &mut Model, f: impl FnOnce(&mut Model)) -> Step {
    m.clear_selection();
    done(m, f)
}

/// Deletes the selection if there is one, else does `f`.
fn deleting(m: &mut Model, f: impl FnOnce(&mut Model)) -> Step {
    if m.has_selection() {
        m.delete_selection();
    } else {
        f(m);
    }
    Step::Done
}

/// Moves with the selection following the cursor.
fn selecting(m: &mut Model, f: impl FnOnce(&mut Model)) -> Step {
    m.start_keyboard_selection();
    f(m);
    m.update_keyboard_selection();
    Step::Done
}

/// The bindings and what they do, in the order Bubbles tries them.
const ACTIONS: [(Field, Handler); 32] = [
    (
        |k| &k.delete_after_cursor,
        |m| deleting(m, Model::delete_after_cursor_or_join),
    ),
    (
        |k| &k.delete_before_cursor,
        |m| deleting(m, Model::delete_before_cursor_or_join),
    ),
    (
        |k| &k.delete_character_backward,
        |m| deleting(m, Model::delete_character_backward),
    ),
    (
        |k| &k.delete_character_forward,
        |m| deleting(m, Model::delete_character_forward),
    ),
    (
        |k| &k.delete_word_backward,
        |m| deleting(m, Model::delete_word_backward),
    ),
    (
        |k| &k.delete_word_forward,
        |m| deleting(m, Model::delete_word_forward),
    ),
    (|k| &k.insert_newline, Model::insert_newline),
    (|k| &k.line_end, |m| moving(m, Model::cursor_end)),
    (|k| &k.line_start, |m| moving(m, Model::cursor_start)),
    (
        |k| &k.character_forward,
        |m| moving(m, Model::character_right),
    ),
    (|k| &k.line_next, |m| moving(m, Model::cursor_down)),
    (|k| &k.word_forward, |m| moving(m, Model::word_right)),
    (|k| &k.paste, Model::paste_clipboard),
    (
        |k| &k.character_backward,
        |m| moving(m, |m| m.character_left(false)),
    ),
    (|k| &k.line_previous, |m| moving(m, Model::cursor_up)),
    (|k| &k.word_backward, |m| moving(m, Model::word_left)),
    (|k| &k.input_begin, |m| moving(m, Model::move_to_begin)),
    (|k| &k.input_end, |m| moving(m, Model::move_to_end)),
    (|k| &k.page_up, |m| moving(m, Model::page_up)),
    (|k| &k.page_down, |m| moving(m, Model::page_down)),
    (
        |k| &k.lowercase_word_forward,
        |m| done(m, |m| m.word_right_with(|_, c| to_lower(c))),
    ),
    (
        |k| &k.uppercase_word_forward,
        |m| done(m, |m| m.word_right_with(|_, c| to_upper(c))),
    ),
    (
        |k| &k.capitalize_word_forward,
        |m| done(m, |m| m.word_right_with(capitalize)),
    ),
    (
        |k| &k.transpose_character_backward,
        |m| done(m, Model::transpose_left),
    ),
    (
        |k| &k.select_character_forward,
        |m| selecting(m, Model::character_right),
    ),
    (
        |k| &k.select_character_backward,
        |m| selecting(m, |m| m.character_left(false)),
    ),
    (
        |k| &k.select_word_forward,
        |m| selecting(m, Model::word_right),
    ),
    (
        |k| &k.select_word_backward,
        |m| selecting(m, Model::word_left),
    ),
    (|k| &k.select_line_up, |m| selecting(m, Model::cursor_up)),
    (
        |k| &k.select_line_down,
        |m| selecting(m, Model::cursor_down),
    ),
    (|k| &k.select_all, |m| done(m, Model::select_all)),
    (|k| &k.copy_selection, |m| Step::Run(m.copy_selection())),
];

/// The first character of a word upper cased, the rest kept.
fn capitalize(i: usize, c: char) -> char {
    if i == 0 { to_upper(c) } else { c }
}

impl Model {
    /// A key press: an edit, or the text it types.
    pub(super) fn press(&mut self, key: &Key) -> Step {
        let handler = ACTIONS
            .iter()
            .find(|(binding, _)| binding(&self.key_map).matches(key))
            .map(|(_, h)| *h);
        if let Some(handler) = handler {
            return handler(self);
        }
        if !key.text.is_empty() {
            self.delete_selection();
            self.insert_from_user(&key.text.chars().collect::<Vec<_>>());
        }
        Step::Done
    }

    /// Enter: the selection goes, and the line breaks, unless that would
    /// be a line over the limit.
    fn insert_newline(&mut self) -> Step {
        self.delete_selection();
        if self.at_content_limit() {
            return Step::Stop;
        }
        self.col = self.col.min(self.line_len());
        self.split_line(self.row, self.col);
        Step::Done
    }

    /// Ctrl-V: the selection goes, and the clipboard is asked for.
    fn paste_clipboard(&mut self) -> Step {
        self.delete_selection();
        self.flags.set(PASTE_PENDING, true);
        Step::Run(crate::read_clipboard())
    }

    /// The cursor's line.
    pub(super) fn row_chars(&self) -> &[char] {
        &self.value[self.row]
    }

    /// The cursor line's length.
    pub(super) fn line_len(&self) -> usize {
        self.value[self.row].len()
    }

    /// The cursor's line, to change.
    pub(super) fn line_mut(&mut self) -> &mut Vec<char> {
        &mut self.value[self.row]
    }

    /// Inserts typed or pasted text at the cursor, cleaned (tabs become
    /// four spaces) and cut to the limits: characters, lines and wrapped
    /// lines.
    pub(super) fn insert_from_user(&mut self, chars: &[char]) {
        let mut chars = sanitize(chars, "\n", "    ");
        if self.char_limit > 0 {
            let room = self.char_limit.saturating_sub(self.length());
            if room == 0 {
                return;
            }
            chars.truncate(room);
        }
        let split = chars.split(|c| *c == '\n').map(<[char]>::to_vec).collect();
        let lines = self.within_limits(split);
        if !lines.is_empty() {
            self.splice_lines(lines);
            self.last_char_offset = 0;
        }
    }

    /// `lines` cut to the limits on lines and wrapped lines; none if
    /// even the first does not fit.
    fn within_limits(&self, mut lines: Vec<Vec<char>>) -> Vec<Vec<char>> {
        let room = (MAX_LINES + 1).saturating_sub(self.value.len());
        lines.truncate(room);
        if self.max_content_height > 0 {
            self.within_content_height(&mut lines);
        }
        lines
    }

    /// `lines` cut so the wrapped lines stay within the content height.
    fn within_content_height(&self, lines: &mut Vec<Vec<char>>) {
        let budget = self
            .max_content_height
            .saturating_sub(self.total_visual_lines());
        while lines.len() > 1 && self.visual_lines_for_insert(lines) > budget {
            let _ = lines.pop();
        }
        if self.visual_lines_for_insert(lines) > budget {
            lines.clear();
        }
    }

    /// Puts `lines` in at the cursor, the first joining the cursor's
    /// line and the text after the cursor ending the last.
    fn splice_lines(&mut self, lines: Vec<Vec<char>>) {
        let col = self.col;
        let tail = self.line_mut().split_off(col);
        let mut lines = lines.into_iter();
        let first = lines.next().unwrap_or_default();
        self.col += first.len();
        self.line_mut().extend(first);
        for line in lines {
            self.row += 1;
            self.col = line.len();
            self.value.insert(self.row, line);
        }
        self.line_mut().extend(tail);
    }

    /// How many more wrapped lines inserting `lines` at the cursor takes.
    fn visual_lines_for_insert(&self, lines: &[Vec<char>]) -> usize {
        let Some((first, rest)) = lines.split_first() else {
            return 0;
        };
        let (head, tail) = self.row_chars().split_at(self.col);
        let mut merged = [head, first.as_slice()].concat();
        if rest.is_empty() {
            merged.extend_from_slice(tail);
        }
        let mut total = self.wrap(&merged).len();
        for (i, l) in rest.iter().enumerate() {
            let content = if i + 1 == rest.len() {
                [l.as_slice(), tail].concat()
            } else {
                l.clone()
            };
            total += self.wrap(&content).len();
        }
        total.saturating_sub(self.wrap(self.row_chars()).len())
    }

    /// Whether another line would go over the limit.
    pub(super) fn at_content_limit(&self) -> bool {
        if self.max_content_height > 0 {
            return self.total_visual_lines() >= self.max_content_height;
        }
        self.max_height > 0 && self.value.len() >= self.max_height
    }

    /// Ctrl-K: the rest of the line, or the line break at its end.
    fn delete_after_cursor_or_join(&mut self) {
        let (col, len) = (self.col, self.line_len());
        if col >= len {
            self.col = len;
            self.merge_line_below(self.row);
            return;
        }
        self.line_mut().truncate(col);
        self.set_cursor_column(col);
    }

    /// Ctrl-U: the line before the cursor, or the line break before it.
    fn delete_before_cursor_or_join(&mut self) {
        let col = self.col.min(self.line_len());
        self.col = col;
        if col == 0 {
            self.merge_line_above(self.row);
            return;
        }
        let _ = self.line_mut().drain(..col);
        self.set_cursor_column(0);
    }

    /// Backspace: a whole emoji or flag, one code point of anything
    /// else, or the line break before the line.
    fn delete_character_backward(&mut self) {
        let col = self.col.min(self.line_len());
        self.col = col;
        if col == 0 {
            self.merge_line_above(self.row);
            return;
        }
        let (s, _) = grapheme_at(self.row_chars(), col - 1);
        let start = if deletes_whole(&self.row_chars()[s..col]) {
            s
        } else {
            col - 1
        };
        let _ = self.line_mut().drain(start..col);
        self.set_cursor_column(start);
    }

    /// Delete: the grapheme cluster under the cursor, or the line break
    /// at the end.
    fn delete_character_forward(&mut self) {
        let col = self.col;
        if col < self.line_len() {
            let (_, end) = grapheme_at(self.row_chars(), col);
            let _ = self.line_mut().drain(col..end);
        }
        if col >= self.line_len() {
            self.merge_line_below(self.row);
        }
    }

    /// Joins line `row` and the one below.
    pub(super) fn merge_line_below(&mut self, row: usize) {
        if row + 1 >= self.value.len() {
            return;
        }
        let below = self.value.remove(row + 1);
        self.value[row].extend(below);
    }

    /// Joins line `row` to the one above, the cursor at the join.
    pub(super) fn merge_line_above(&mut self, row: usize) {
        if row == 0 {
            return;
        }
        self.col = self.value[row - 1].len();
        self.row -= 1;
        let line = self.value.remove(row);
        self.value[row - 1].extend(line);
    }

    /// Breaks line `row` at `col`, the cursor at the new line's start.
    fn split_line(&mut self, row: usize, col: usize) {
        let tail = self.value[row].split_off(col);
        self.value.insert(row + 1, tail);
        self.col = 0;
        self.row += 1;
    }
}
