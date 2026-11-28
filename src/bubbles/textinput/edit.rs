//! Editing: keys to actions, insertion, deletion, moving by character
//! and word, suggestions, and scrolling the text to keep the cursor in
//! the width.

use super::{EchoMode, KeyMap, Model, Msg, PASTE_PENDING};
use crate::bubbles::text::{clusters, grapheme_at, is_space, sanitize};
use crate::style::width;
use crate::{Cmd, Key};

/// What a key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    DeleteWordBackward,
    DeleteCharacterBackward,
    WordBackward,
    CharacterBackward,
    WordForward,
    CharacterForward,
    LineStart,
    DeleteCharacterForward,
    LineEnd,
    DeleteAfterCursor,
    DeleteBeforeCursor,
    Paste,
    DeleteWordForward,
    NextSuggestion,
    PrevSuggestion,
}

/// A binding of the key map.
type Field = fn(&KeyMap) -> &crate::bubbles::key::Binding;

/// The bindings in the order Bubbles tries them.
const ACTIONS: [(Field, Action); 15] = [
    (|k| &k.delete_word_backward, Action::DeleteWordBackward),
    (
        |k| &k.delete_character_backward,
        Action::DeleteCharacterBackward,
    ),
    (|k| &k.word_backward, Action::WordBackward),
    (|k| &k.character_backward, Action::CharacterBackward),
    (|k| &k.word_forward, Action::WordForward),
    (|k| &k.character_forward, Action::CharacterForward),
    (|k| &k.line_start, Action::LineStart),
    (
        |k| &k.delete_character_forward,
        Action::DeleteCharacterForward,
    ),
    (|k| &k.line_end, Action::LineEnd),
    (|k| &k.delete_after_cursor, Action::DeleteAfterCursor),
    (|k| &k.delete_before_cursor, Action::DeleteBeforeCursor),
    (|k| &k.paste, Action::Paste),
    (|k| &k.delete_word_forward, Action::DeleteWordForward),
    (|k| &k.next_suggestion, Action::NextSuggestion),
    (|k| &k.prev_suggestion, Action::PrevSuggestion),
];

impl Model {
    /// A key press: an edit, or the text it types.
    pub(super) fn press(&mut self, key: &Key) -> Cmd<Msg> {
        if self.key_map.accept_suggestion.matches(key) && !self.matched.is_empty() {
            let suggestion = &self.matched[self.current_suggestion];
            let rest: Vec<char> = suggestion.iter().skip(self.value.len()).copied().collect();
            self.value.extend(rest);
            self.cursor_end();
        }
        let action = ACTIONS
            .iter()
            .find(|(binding, _)| binding(&self.key_map).matches(key))
            .map(|(_, a)| *a);
        let cmd = match action {
            Some(Action::Paste) => {
                self.flags.set(PASTE_PENDING, true);
                crate::read_clipboard()
            }
            Some(a) => {
                self.apply(a);
                Cmd::none()
            }
            None => {
                self.insert_from_user(&key.text);
                Cmd::none()
            }
        };
        self.update_suggestions();
        cmd
    }

    /// Does `action`.
    fn apply(&mut self, action: Action) {
        match action {
            Action::DeleteWordBackward => self.delete_word_backward(),
            Action::DeleteCharacterBackward => self.delete_character_backward(),
            Action::WordBackward => self.word_backward(),
            Action::CharacterBackward => self.set_cursor(self.cluster_before()),
            Action::WordForward => self.word_forward(),
            Action::CharacterForward => self.set_cursor(self.cluster_after()),
            other => self.apply_more(other),
        }
    }

    /// The rest of [`Model::apply`].
    fn apply_more(&mut self, action: Action) {
        match action {
            Action::LineStart => self.cursor_start(),
            Action::DeleteCharacterForward => self.delete_character_forward(),
            Action::LineEnd => self.cursor_end(),
            Action::DeleteAfterCursor => self.delete_after_cursor(),
            Action::DeleteBeforeCursor => self.delete_before_cursor(),
            Action::DeleteWordForward => self.delete_word_forward(),
            Action::NextSuggestion => self.next_suggestion(),
            Action::PrevSuggestion => self.prev_suggestion(),
            _ => {}
        }
    }

    /// Where the grapheme cluster before the cursor starts.
    fn cluster_before(&self) -> usize {
        if self.pos == 0 {
            return 0;
        }
        grapheme_at(&self.value, self.pos - 1).0
    }

    /// Where the grapheme cluster under the cursor ends.
    fn cluster_after(&self) -> usize {
        grapheme_at(&self.value, self.pos).1.min(self.value.len())
    }

    /// Inserts typed or pasted text at the cursor, cleaned and cut to
    /// the limit.
    pub(super) fn insert_from_user(&mut self, text: &str) {
        let mut paste = sanitize(&text.chars().collect::<Vec<_>>(), " ", " ");
        if self.char_limit > 0 {
            let room = self.char_limit.saturating_sub(self.value.len());
            if room == 0 {
                return;
            }
            paste.truncate(room);
        }
        let (head, tail) = self.value.split_at(self.pos);
        let value = [head, &paste, tail].concat();
        self.pos += paste.len();
        self.err = self.check(&value);
        self.set_value_internal(value);
    }

    /// Replaces the text, cut to the limit, keeping the cursor inside.
    pub(super) fn set_value_internal(&mut self, mut chars: Vec<char>) {
        let was_empty = self.value.is_empty();
        chars.truncate(if self.char_limit > 0 {
            self.char_limit
        } else {
            usize::MAX
        });
        let len = chars.len();
        self.value = chars;
        if (self.pos == 0 && was_empty) || self.pos > len {
            self.pos = len;
        }
        self.handle_overflow();
    }

    /// Deletes the character before the cursor: a whole emoji or flag,
    /// one code point of anything else.
    fn delete_character_backward(&mut self) {
        self.err = None;
        if self.value.is_empty() || self.pos == 0 {
            return;
        }
        let (start, _) = grapheme_at(&self.value, self.pos - 1);
        let from = if deletes_whole(&self.value[start..self.pos]) {
            start
        } else {
            self.pos - 1
        };
        let _ = self.value.drain(from..self.pos);
        self.err = self.check(&self.value);
        self.set_cursor(from);
    }

    /// Deletes the grapheme cluster under the cursor.
    fn delete_character_forward(&mut self) {
        if self.pos < self.value.len() {
            let end = self.cluster_after();
            let _ = self.value.drain(self.pos..end);
            self.err = self.check(&self.value);
        }
    }

    /// Deletes everything before the cursor.
    fn delete_before_cursor(&mut self) {
        let _ = self.value.drain(..self.pos);
        self.err = self.check(&self.value);
        self.offset = 0;
        self.set_cursor(0);
    }

    /// Deletes everything after the cursor.
    fn delete_after_cursor(&mut self) {
        self.value.truncate(self.pos);
        self.err = self.check(&self.value);
        self.cursor_end();
    }

    /// Deletes back to the start of the word before the cursor; with the
    /// text hidden, everything before it.
    fn delete_word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() {
            return;
        }
        if self.echo_mode != EchoMode::Normal {
            self.delete_before_cursor();
            return;
        }
        let old = self.pos;
        let start = word_start(&self.value, self.pos - 1);
        let _ = self.value.drain(start..old);
        self.err = self.check(&self.value);
        self.set_cursor(start);
    }

    /// Deletes to the end of the word after the cursor; with the text
    /// hidden, everything after it.
    fn delete_word_forward(&mut self) {
        if self.pos >= self.value.len() {
            return;
        }
        if self.echo_mode != EchoMode::Normal {
            self.delete_after_cursor();
            return;
        }
        let end = self.word_end(self.pos);
        let _ = self.value.drain(self.pos..end);
        self.err = self.check(&self.value);
        self.set_cursor(self.pos);
    }

    /// Where the word at or after `from` ends: spaces, then non-spaces.
    fn word_end(&self, from: usize) -> usize {
        let mut end = from;
        while end < self.value.len() && is_space(self.value[end]) {
            end += 1;
        }
        while end < self.value.len() && !is_space(self.value[end]) {
            end += 1;
        }
        end
    }

    /// To the start of the word before the cursor.
    fn word_backward(&mut self) {
        if self.pos == 0 {
            return;
        }
        if self.echo_mode != EchoMode::Normal {
            self.cursor_start();
            return;
        }
        self.set_cursor(word_back(&self.value, self.pos));
    }

    /// To the end of the word after the cursor.
    fn word_forward(&mut self) {
        if self.pos >= self.value.len() {
            return;
        }
        if self.echo_mode != EchoMode::Normal {
            self.cursor_end();
            return;
        }
        self.set_cursor(self.word_end(self.pos));
    }

    /// Scrolls the text so the cursor stays within the width, a whole
    /// grapheme cluster at a time: Bubbles' `handleOverflow`.
    pub(super) fn handle_overflow(&mut self) {
        let text: String = self.value.iter().collect();
        if self.width == 0 || width(&text) <= self.width {
            self.offset = 0;
            self.offset_right = self.value.len();
        } else {
            self.scroll_to_cursor(&cluster_cells(&text));
        }
    }

    /// Moves the visible part to show the cursor, given the text's
    /// clusters.
    fn scroll_to_cursor(&mut self, cells: &[(usize, usize)]) {
        let (pos, columns) = (self.pos, self.width);
        self.offset_right = self.offset_right.min(self.value.len());
        if pos < self.offset {
            self.offset = pos;
            self.offset_right = fill_right(cells, pos, columns);
        } else if pos >= self.offset_right {
            self.offset_right = pos;
            self.offset = fill_left(cells, pos, columns);
        }
    }

    /// The matched suggestions for the text, by prefix, ignoring case.
    pub(super) fn update_suggestions(&mut self) {
        if !self.show_suggestions {
            return;
        }
        if self.value.is_empty() || self.suggestions.is_empty() {
            self.matched.clear();
            return;
        }
        let typed = self.value.iter().collect::<String>().to_lowercase();
        let matches: Vec<Vec<char>> = self
            .suggestions
            .iter()
            .filter(|s| {
                s.iter()
                    .collect::<String>()
                    .to_lowercase()
                    .starts_with(&typed)
            })
            .cloned()
            .collect();
        if matches != self.matched {
            self.current_suggestion = 0;
        }
        self.matched = matches;
    }

    /// Offers the next matched suggestion.
    fn next_suggestion(&mut self) {
        self.current_suggestion += 1;
        if self.current_suggestion >= self.matched.len() {
            self.current_suggestion = 0;
        }
    }

    /// Offers the matched suggestion before.
    fn prev_suggestion(&mut self) {
        self.current_suggestion = match self.current_suggestion {
            0 => self.matched.len().saturating_sub(1),
            n => n - 1,
        };
    }
}

/// Where Bubbles' word deletion stops, walking left from `p`: past
/// spaces, then to just after the next space, or to the start. As in
/// Go, a word at the very start takes the spaces before it too.
pub(in crate::bubbles) fn word_start(value: &[char], mut p: usize) -> usize {
    while is_space(value[p]) && p > 0 {
        p -= 1;
    }
    while p > 0 {
        if is_space(value[p]) {
            return p + 1;
        }
        p -= 1;
    }
    p
}

/// Each grapheme cluster of `text` as its first character's index and
/// its width, with the end of the text last.
fn cluster_cells(text: &str) -> Vec<(usize, usize)> {
    let mut at = 0;
    let mut cells: Vec<(usize, usize)> = clusters(text)
        .into_iter()
        .map(|(c, w)| {
            let start = at;
            at += c.chars().count();
            (start, w)
        })
        .collect();
    cells.push((at, 0));
    cells
}

/// The index of the cluster starting at character `at`.
fn cluster_index(cells: &[(usize, usize)], at: usize) -> usize {
    cells
        .iter()
        .position(|(start, _)| *start >= at)
        .unwrap_or(cells.len() - 1)
}

/// How far right the text shows from `offset`, a cell left for the
/// cursor: Bubbles' forward walk.
fn fill_right(cells: &[(usize, usize)], offset: usize, columns: usize) -> usize {
    let mut i = cluster_index(cells, offset);
    let mut w = 0;
    while i + 1 < cells.len() && w <= columns {
        w += cells[i].1;
        if w <= columns + 1 {
            i += 1;
        }
    }
    cells[i].0
}

/// Where the text starts to show up to `right`: Bubbles' backward walk,
/// which stops a cluster short of the start.
fn fill_left(cells: &[(usize, usize)], right: usize, columns: usize) -> usize {
    let end = cluster_index(cells, right);
    end.checked_sub(1)
        .map_or(0, |last| cells[walk_back(cells, last, columns) + 1].0)
}

/// From cluster `i`, back while the clusters fill less than `columns`:
/// the cluster before the first that shows.
fn walk_back(cells: &[(usize, usize)], mut i: usize, columns: usize) -> usize {
    let mut filled = 0;
    while i > 0 && filled < columns {
        filled += cells[i].1;
        if filled <= columns {
            i -= 1;
        }
    }
    i
}

/// Where Bubbles' word moves stop, walking left from `from`: past
/// spaces, then to the start of the word.
fn word_back(value: &[char], from: usize) -> usize {
    let start_of_spaces = value[..from]
        .iter()
        .rposition(|c| !is_space(*c))
        .map_or(0, |p| p + 1);
    value[..start_of_spaces]
        .iter()
        .rposition(|c| is_space(*c))
        .map_or(0, |p| p + 1)
}

/// Whether Backspace removes the cluster `chars` whole: emoji, flags
/// and variation sequences, as Bubbles v2's textarea decides. Anything
/// else loses one code point, so an accent can be retyped without
/// erasing its letter.
pub(in crate::bubbles) fn deletes_whole(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(i, &c)| {
        let r = u32::from(c);
        let zwj_emoji = c == '\u{200d}' && chars.get(i + 1).is_some_and(|n| !n.is_alphabetic());
        zwj_emoji
            || (0xFE00..=0xFE0F).contains(&r)
            || (0xE0100..=0xE01EF).contains(&r)
            || (0x1F1E6..=0x1F1FF).contains(&r)
            || (0x1F3FB..=0x1F3FF).contains(&r)
            || (0xE0020..=0xE007F).contains(&r)
    })
}
