//! Drawing: the prompt, the visible part of the text with the cursor in
//! it, the rest of a suggestion, the placeholder, and the real cursor.

use super::{EchoMode, FOCUS, Model, StyleState, USE_VIRTUAL_CURSOR};
use crate::Cursor;
use crate::Rgb;
use crate::bubbles::text::{clusters, grapheme_at};
use crate::style::{Color, truncate, width};

impl Model {
    /// The prompt and the text (or the placeholder), with the cursor.
    #[must_use]
    pub fn view(&self) -> String {
        if self.value.is_empty() && !self.placeholder.is_empty() {
            return self.placeholder_view();
        }
        let (visible, pos) = self.visible();
        let visible_width = width(&visible.iter().collect::<String>());
        let (text, completion) = self.text_and_completion(visible, pos, visible_width);
        let padding = self.padding(visible_width, pos >= visible.len(), &completion);
        self.prompt_view() + &text + &completion + &padding
    }

    /// The part of the text that shows, and the cursor's place in it.
    fn visible(&self) -> (&[char], usize) {
        let end = self.offset_right.max(self.offset);
        let visible = &self.value[self.offset..end];
        (visible, self.cursor_in(visible.len()))
    }

    /// The cursor's place in the `len` characters that show.
    fn cursor_in(&self, len: usize) -> usize {
        self.pos.saturating_sub(self.offset).min(len)
    }

    /// Spaces to the width and a cell for the cursor, when the text fits.
    fn padding(&self, visible_width: usize, at_end: bool, completion: &str) -> String {
        if self.width == 0 || visible_width > self.width {
            return String::new();
        }
        let used = visible_width + usize::from(at_end) + width(completion);
        self.paint_text(&vec![' '; (self.width + 1).saturating_sub(used)])
    }

    /// The visible text with the cursor at `pos`, and the rest of the
    /// suggestion after it.
    fn text_and_completion(
        &self,
        visible: &[char],
        pos: usize,
        visible_width: usize,
    ) -> (String, String) {
        let styles = self.active_style();
        let mut cursor = self.virtual_cursor.clone();
        cursor.text_style = styles.text;
        let before = self.paint_text(&visible[..pos]);
        if pos < visible.len() {
            let (_, end) = grapheme_at(visible, pos);
            cursor.set_char(&self.echo(&visible[pos..end]));
            let text = before + &cursor.view() + &self.paint_text(&visible[end..]);
            return (
                text,
                self.completion_view(0, (self.width + 1).saturating_sub(visible_width)),
            );
        }
        let Some(next) = self.suggested_next() else {
            cursor.set_char(" ");
            return (before + &cursor.view(), String::new());
        };
        cursor.text_style = styles.suggestion;
        cursor.set_char(&self.echo(&[next]));
        let completion = self.completion_view(1, self.width.saturating_sub(visible_width));
        (before + &cursor.view(), completion)
    }

    /// `chars` as they show, in the text's style.
    fn paint_text(&self, chars: &[char]) -> String {
        self.active_style().text.paint(&self.echo(chars))
    }

    /// The character after the text in the suggestion offered, while
    /// focused. Go takes it at the cursor's place in the visible text,
    /// which is the wrong one once the text has scrolled; this takes
    /// the one after the whole text.
    fn suggested_next(&self) -> Option<char> {
        if !self.flags.get(FOCUS) {
            return None;
        }
        self.matched
            .get(self.current_suggestion)
            .and_then(|s| s.get(self.value.len()).copied())
    }

    /// The styles for the state, focused or blurred.
    pub(super) const fn active_style(&self) -> &StyleState {
        if self.flags.get(FOCUS) {
            &self.styles.focused
        } else {
            &self.styles.blurred
        }
    }

    /// `chars` as the echo mode shows them.
    fn echo(&self, chars: &[char]) -> String {
        let text: String = chars.iter().collect();
        match self.echo_mode {
            EchoMode::Normal => text,
            EchoMode::Password => self.echo_character.to_string().repeat(width(&text)),
            EchoMode::None => String::new(),
        }
    }

    /// The prompt in its style.
    fn prompt_view(&self) -> String {
        self.active_style().prompt.paint(&self.prompt)
    }

    /// The placeholder, its first cluster under the cursor, in the width
    /// and a cell for the cursor (or whole, with no width).
    fn placeholder_view(&self) -> String {
        let styles = self.active_style();
        let mut cursor = self.virtual_cursor.clone();
        cursor.text_style = styles.placeholder;
        let parts = clusters(&self.placeholder);
        let (first, first_width) = parts.first().copied().unwrap_or(("", 0));
        cursor.set_char(first);
        let rest = &self.placeholder[first.len()..];
        let mut v = cursor.view();
        if self.width == 0 {
            v += &styles.placeholder.paint(rest);
        } else {
            let room = (self.width + 1).saturating_sub(first_width);
            let shown = truncate(rest, room);
            let shown = shown.strip_suffix("\x1b[0m").unwrap_or(&shown).to_string();
            let gap = room.saturating_sub(width(&shown));
            v += &styles.placeholder.paint(&shown);
            v += &styles.placeholder.paint(&" ".repeat(gap));
        }
        self.prompt_view() + &v
    }

    /// The rest of the suggestion offered after the text, `skip` more
    /// characters in, in at most `room` cells.
    fn completion_view(&self, skip: usize, room: usize) -> String {
        let Some(suggestion) = self.matched.get(self.current_suggestion) else {
            return String::new();
        };
        if self.value.len() >= suggestion.len() || room == 0 {
            return String::new();
        }
        let tail: String = suggestion.iter().skip(self.value.len() + skip).collect();
        let tail = truncate(&tail, room);
        let tail = tail.strip_suffix("\x1b[0m").unwrap_or(&tail);
        self.active_style().suggestion.paint(tail)
    }

    /// Where the terminal's cursor goes, relative to the input's top
    /// left, when the input is focused and does not draw its own (see
    /// [`Model::set_virtual_cursor`]); else `None`.
    #[must_use]
    pub fn cursor(&self) -> Option<Cursor> {
        if self.flags.get(USE_VIRTUAL_CURSOR) || !self.flags.get(FOCUS) {
            return None;
        }
        let prompt = width(&self.prompt_view());
        let end = self.pos.max(self.offset);
        let before: String = self.value[self.offset..end].iter().collect();
        let mut x = prompt + width(&before);
        if self.width > 0 {
            x = x.min(self.width + prompt);
        }
        let style = self.styles.cursor;
        let mut c = Cursor::new(u16::try_from(x).unwrap_or(u16::MAX), 0)
            .with_shape(style.shape)
            .with_blink(style.blink);
        if let Color::Rgb(r, g, b) = style.color {
            c = c.with_color(Rgb::new(r, g, b));
        }
        Some(c)
    }
}
