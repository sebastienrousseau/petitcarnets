//! Drawing: each wrapped line with its prompt and number, the cursor
//! and the selection, the lines after the end, the placeholder, and the
//! real cursor.

use super::{FOCUS, Model, PromptInfo, USE_VIRTUAL_CURSOR};
use crate::bubbles::cursor;
use crate::bubbles::text::{clusters, grapheme_at};
use crate::style::{Color, Style, width, wrap};
use crate::{Cursor, Rgb};

impl Model {
    /// The text area: the lines in view, in the base style.
    #[must_use]
    pub fn view(&self) -> String {
        let mut vp = self.viewport.clone();
        vp.set_content(&self.render_lines());
        self.active_style().base.render(&vp.view())
    }

    /// Every line, wrapped, then a screen of lines after the end: what
    /// the viewport scrolls over. Bubbles' `view`.
    pub(super) fn render_lines(&self) -> String {
        if self.shows_placeholder() {
            return self.placeholder_view();
        }
        let (mut out, display) = self.render_text();
        let filler = self.end_of_buffer_line();
        for line in display..display + self.height {
            out += &self.prompt_view(line);
            out += &filler;
            out.push('\n');
        }
        out
    }

    /// Whether the placeholder shows: there is one and no text.
    fn shows_placeholder(&self) -> bool {
        let empty = self.value.len() == 1 && self.value[0].is_empty();
        empty && self.col == 0 && !self.placeholder.is_empty()
    }

    /// The text's wrapped lines, and how many there are.
    fn render_text(&self) -> (String, usize) {
        let mut cursor = self.virtual_cursor.clone();
        cursor.text_style = self.active_style().cursor_line();
        let mut out = String::new();
        let mut display = 0;
        for l in 0..self.value.len() {
            self.render_line(l, &mut display, &mut cursor, &mut out);
        }
        (out, display)
    }

    /// A line after the end, after its prompt.
    fn end_of_buffer_line(&self) -> String {
        let gutter = self.end_of_buffer_character.to_string();
        let gap = " ".repeat(self.width.saturating_sub(width(&gutter)));
        self.active_style().end_of_buffer().paint(&(gutter + &gap))
    }

    /// Line `l`'s wrapped lines, each with its prompt and number, from
    /// screen line `display`.
    fn render_line(
        &self,
        l: usize,
        display: &mut usize,
        cursor: &mut cursor::Model,
        out: &mut String,
    ) {
        let mine = l == self.row;
        let style = self.line_style(mine);
        let on = mine.then(|| self.line_info().row_offset);
        let mut base = 0;
        for (wl, wrapped) in self.wrap(&self.value[l]).into_iter().enumerate() {
            *out += &style.paint(&self.prompt_view(*display));
            *display += 1;
            *out += &self.line_number_view(if wl == 0 { l + 1 } else { 0 }, mine);
            let here = (on == Some(wl)).then_some(&mut *cursor);
            *out += &self.wrapped_line(&wrapped, (l, base), style, here);
            base += wrapped.len();
            out.push('\n');
        }
    }

    /// The style of the cursor's line, or of the others.
    fn line_style(&self, cursor_line: bool) -> Style {
        let styles = self.active_style();
        if cursor_line {
            styles.cursor_line()
        } else {
            styles.text()
        }
    }

    /// One wrapped line of line `l` starting at character `base`, with
    /// the selection, or the cursor if it is on it, padded to the width.
    fn wrapped_line(
        &self,
        wrapped: &[char],
        (l, base): (usize, usize),
        style: Style,
        cursor: Option<&mut cursor::Model>,
    ) -> String {
        let cells = text_width(wrapped);
        let (wrapped, padding) = match wrapped.split_last() {
            Some((' ', rest)) if cells > self.width => (rest, 0),
            _ => (wrapped, self.width.saturating_sub(cells)),
        };
        let body = if let Some(span) = self.selection_span(l, base, wrapped.len()) {
            self.selected_line(wrapped, span, style)
        } else if let Some(cursor) = cursor {
            self.cursor_line_text(wrapped, l, style, cursor)
        } else {
            paint_chars(style, wrapped)
        };
        body + &style.paint(&" ".repeat(padding))
    }

    /// A wrapped line with the selected part `(from, to)` highlighted.
    fn selected_line(&self, wrapped: &[char], (from, to): (usize, usize), style: Style) -> String {
        let selection = self.active_style().selection();
        paint_chars(style, &wrapped[..from])
            + &paint_chars(selection, &wrapped[from..to])
            + &paint_chars(style, &wrapped[to..])
    }

    /// The wrapped line the cursor is on, line `l`'s, with the cursor.
    fn cursor_line_text(
        &self,
        wrapped: &[char],
        l: usize,
        style: Style,
        cursor: &mut cursor::Model,
    ) -> String {
        let info = self.line_info();
        let at = info.column_offset.min(wrapped.len());
        let before = paint_chars(style, &wrapped[..at]);
        if self.col >= self.value[l].len() && info.char_offset >= self.width {
            cursor.set_char(" ");
            return before + &cursor.view();
        }
        before + &under_cursor(&wrapped[at..], style, cursor)
    }

    /// The prompt for screen line `display`, in its style.
    fn prompt_view(&self, display: usize) -> String {
        let prompt = self.prompt_fn.map_or_else(
            || self.prompt.clone(),
            |f| {
                let p = f(PromptInfo {
                    line_number: display,
                    focused: self.flags.get(FOCUS),
                });
                format!(
                    "{}{p}",
                    " ".repeat(self.prompt_width.saturating_sub(width(&p)))
                )
            },
        );
        self.active_style().prompt().paint(&prompt)
    }

    /// Line number `n` (blank for 0), right aligned to the digits of
    /// the most lines, a space each side.
    fn line_number_view(&self, n: usize, cursor_line: bool) -> String {
        if !self.show_line_numbers {
            return String::new();
        }
        let label = if n == 0 {
            " ".to_string()
        } else {
            n.to_string()
        };
        let digits = super::digits(self.max_height);
        let label = format!(" {label:>digits$} ");
        let styles = self.active_style();
        if cursor_line {
            styles
                .cursor_line()
                .paint(&styles.cursor_line_number().paint(&label))
        } else {
            styles.text().paint(&styles.line_number().paint(&label))
        }
    }

    /// The placeholder, word wrapped, with the cursor on its first
    /// character.
    fn placeholder_view(&self) -> String {
        let wrapped = wrap(&self.placeholder, self.width);
        let mut lines: Vec<&str> = wrapped.trim().lines().collect();
        if lines.is_empty() {
            lines.push("");
        }
        let mut out = String::new();
        for i in 0..self.height {
            out += &self.placeholder_line(i, lines.get(i).copied());
            out.push('\n');
        }
        out
    }

    /// Screen line `i` of the placeholder: its `line`, or after the end.
    fn placeholder_line(&self, i: usize, line: Option<&str>) -> String {
        let styles = self.active_style();
        let style = self
            .line_style(line.is_some())
            .inherit(styles.placeholder());
        let style = if line.is_some() {
            style
        } else {
            styles.placeholder()
        };
        let mut out = style.paint(&self.prompt_view(i));
        let Some(line) = line else {
            return out
                + &styles
                    .end_of_buffer()
                    .paint(&self.end_of_buffer_character.to_string());
        };
        out += &self.line_number_view(usize::from(i == 0), true);
        let gap = style.paint(&" ".repeat(self.width.saturating_sub(width(line))));
        let text = if i == 0 {
            self.placeholder_first(line, style)
        } else {
            style.paint(line)
        };
        out + &text + &gap
    }

    /// The placeholder's first line, the cursor on its first cluster.
    fn placeholder_first(&self, line: &str, style: Style) -> String {
        let placeholder = self.active_style().placeholder();
        let mut cursor = self.virtual_cursor.clone();
        cursor.text_style = placeholder;
        let first = clusters(line).first().map_or("", |(c, _)| *c);
        cursor.set_char(first);
        style.paint(&cursor.view()) + &style.paint(&placeholder.paint(&line[first.len()..]))
    }

    /// Where the terminal's cursor goes, relative to the text area's top
    /// left, when it is focused and does not draw its own (see
    /// [`Model::set_virtual_cursor`]); else `None`.
    #[must_use]
    pub fn cursor(&self) -> Option<Cursor> {
        if self.flags.get(USE_VIRTUAL_CURSOR) || !self.flags.get(FOCUS) {
            return None;
        }
        let info = self.line_info();
        let (left, top) = self.active_style().base.frame_offset();
        let x = info.char_offset
            + width(&self.prompt_view(0))
            + width(&self.line_number_view(0, false))
            + left;
        let y = (self.cursor_line_number() + top).saturating_sub(self.viewport.y_offset());
        let to16 = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
        let style = self.styles.cursor;
        let mut c = Cursor::new(to16(x), to16(y))
            .with_shape(style.shape)
            .with_blink(style.blink);
        if let Color::Rgb(r, g, b) = style.color {
            c = c.with_color(Rgb::new(r, g, b));
        }
        Some(c)
    }
}

/// The cells `chars` take.
fn text_width(chars: &[char]) -> usize {
    width(&chars.iter().collect::<String>())
}

/// `chars` in `style`.
fn paint_chars(style: Style, chars: &[char]) -> String {
    style.paint(&chars.iter().collect::<String>())
}

/// The cluster at the start of `rest` under the cursor, then the rest.
fn under_cursor(rest: &[char], style: Style, cursor: &mut cursor::Model) -> String {
    let end = grapheme_at(rest, 0).1.min(rest.len());
    cursor.set_char(&rest[..end].iter().collect::<String>());
    style.paint(&cursor.view()) + &paint_chars(style, &rest[end..])
}
