//! Drawing views: Bubble Tea v2's cursed renderer, on a cell buffer.
//!
//! Each frame is laid out in cells (grapheme, width, style, hyperlink)
//! and compared with the frame on screen: unchanged rows are skipped,
//! changed rows rewrite only the cells that differ, and on the
//! alternate screen a frame that moved up or down is scrolled by the
//! terminal. A frame the same as the last one writes nothing.
//!
//! Inline (the default, as in Bubble Tea v2) the frame is drawn below
//! the shell prompt, with relative cursor moves only, and keeps its
//! bottom rows when it is taller than the window. On the alternate
//! screen it fills the window and is cut to it. Frames are wrapped in
//! synchronized output (mode 2026) where the terminal has it, else in
//! hiding and showing the cursor when it is shown.

pub(crate) mod cell;
mod diff;
mod insert;
pub(crate) mod layout;
mod scroll;
mod writer;

use crate::msg::Size;
use crate::unicode::WidthMethod;
use crate::view::View;
use layout::{Frame, layout};
use std::io::{self, Write};
use writer::{Addressing, Writer};

/// The frame on screen and the terminal's state as the renderer left
/// it.
#[derive(Debug)]
pub(crate) struct Renderer {
    size: Size,
    method: WidthMethod,
    sync: bool,
    alt: bool,
    screen: Frame,
    writer: Writer,
    cursor: Option<(usize, usize)>,
    shown: Option<bool>,
    pending: Pending,
    saved: Option<(Writer, Frame)>,
}

impl Renderer {
    /// A renderer for a window of `size`, inline, before any frame.
    pub(crate) fn new(size: Size) -> Self {
        let mut writer = Writer::new(usize::from(size.width), 1, Addressing::Relative);
        writer.x = None;
        Self {
            size,
            method: WidthMethod::WcWidth,
            sync: false,
            alt: false,
            screen: Frame::default(),
            writer,
            cursor: None,
            shown: None,
            pending: Pending::default(),
            saved: None,
        }
    }

    /// Whether frames are drawn on the alternate screen.
    pub(crate) const fn is_alt(&self) -> bool {
        self.alt
    }

    /// Wraps frames in synchronized output from now on.
    pub(crate) fn set_sync(&mut self, on: bool) {
        self.sync = on;
    }

    /// Measures text with `method` from the next frame on.
    pub(crate) fn set_width_method(&mut self, method: WidthMethod) {
        self.method = method;
    }

    /// A new window size: the next frame is drawn in full.
    pub(crate) fn resize(&mut self, size: Size) {
        self.size = size;
        self.writer.width = usize::from(size.width);
        self.pending.erase = true;
    }

    /// Clears the whole screen before the next frame, which is drawn
    /// from the top left. Bubble Tea's `ClearScreen`.
    pub(crate) fn clear_screen(&mut self) {
        self.pending.clear = true;
    }

    /// Draws on the alternate screen from now on (`alt`), or inline
    /// again. The caller switches the terminal's screen; the next frame
    /// is drawn in full. Inline, the cursor and frame are where the
    /// switch left them, as the terminal saves and restores the cursor.
    pub(crate) fn switch_screen(&mut self, alt: bool) {
        if alt == self.alt {
            return;
        }
        let (width, height) = (usize::from(self.size.width), usize::from(self.size.height));
        if alt {
            let inline = (self.writer.clone(), std::mem::take(&mut self.screen));
            self.saved = Some(inline);
            self.writer = Writer::new(width, height, Addressing::Absolute);
        } else {
            let fresh = || {
                (
                    Writer::new(width, 1, Addressing::Relative),
                    Frame::default(),
                )
            };
            let (writer, screen) = self.saved.take().unwrap_or_else(fresh);
            self.writer = writer;
            self.writer.width = width;
            self.screen = screen;
        }
        self.alt = alt;
        self.pending.erase = true;
        self.pending.switched = true;
    }

    /// Draws `view`'s content and cursor, writing only what changed.
    pub(crate) fn render(&mut self, view: &View, out: &mut dyn Write) -> io::Result<()> {
        let frame = self.layout(&view.content);
        let cursor = self.cursor_of(view, &frame);
        if self.unchanged(&frame, cursor) {
            return Ok(());
        }
        let mut w = self.writer.clone();
        let old = self.prepare(&mut w);
        self.paint(&mut w, &old, &frame);
        w.plain();
        self.place_cursor(&mut w, cursor);
        let bytes = self.wrap(&std::mem::take(&mut w.buf), cursor.is_some());
        self.writer = w;
        self.screen = frame;
        self.cursor = cursor;
        self.shown = Some(cursor.is_some());
        self.pending = Pending::default();
        write_out(out, &bytes)
    }

    /// Lays out `content` for the screen in use.
    fn layout(&self, content: &str) -> Frame {
        let (width, height) = (usize::from(self.size.width), usize::from(self.size.height));
        if self.alt {
            return layout(content, width, Some(height), self.method);
        }
        let mut frame = layout(content, width, None, self.method);
        let extra = frame.rows.len().saturating_sub(height);
        drop(frame.rows.drain(..extra));
        frame
    }

    /// Where the view's cursor goes in `frame`, kept inside it; rows cut
    /// from the top of a tall inline frame move it up with them.
    fn cursor_of(&self, view: &View, frame: &Frame) -> Option<(usize, usize)> {
        let c = view.cursor?;
        let lines = layout::lines(&view.content).len();
        let cut = if self.alt {
            0
        } else {
            lines.saturating_sub(frame.rows.len())
        };
        let rows = if self.alt {
            usize::from(self.size.height)
        } else {
            frame.rows.len()
        };
        let x = usize::from(c.x).min(frame.width.saturating_sub(1));
        let y = usize::from(c.y).saturating_sub(cut);
        Some((x, y.min(rows.saturating_sub(1))))
    }

    /// Whether drawing `frame` with `cursor` would change nothing.
    fn unchanged(&self, frame: &Frame, cursor: Option<(usize, usize)>) -> bool {
        let pending = self.pending.erase
            || self.pending.clear
            || self.pending.switched
            || self.shown.is_none();
        !pending && *frame == self.screen && cursor == self.cursor
    }

    /// Clears what a clear or erase asks for, and returns what the
    /// screen then shows.
    fn prepare(&mut self, w: &mut Writer) -> Frame {
        if self.pending.clear || (self.pending.erase && self.alt) {
            w.plain();
            w.raw("\x1b[H\x1b[2J");
            (w.x, w.y) = (Some(0), 0);
            if !self.alt {
                w.rows = usize::from(self.size.height).max(1);
            }
            return Frame::default();
        }
        if self.pending.erase {
            w.move_to(0, 0);
            w.erase_below();
            return Frame::default();
        }
        std::mem::take(&mut self.screen)
    }

    /// Writes what turns `old` into `new`.
    fn paint(&self, w: &mut Writer, old: &Frame, new: &Frame) {
        if self.alt {
            if let Some(scroll) = scroll::detect(old, new) {
                let mut plain = w.clone();
                diff::draw(&mut plain, old, new);
                let shifted = scroll::apply(w, old, scroll);
                diff::draw(w, &shifted, new);
                if plain.buf.len() <= w.buf.len() {
                    *w = plain;
                }
                return;
            }
        }
        diff::draw(w, old, new);
        if !self.alt {
            inline_tail(w, old, new);
        }
    }

    /// Puts the cursor where the view wants it. Inline with the cursor
    /// hidden, it is not left at the right edge, as Bubble Tea does.
    fn place_cursor(&self, w: &mut Writer, cursor: Option<(usize, usize)>) {
        match cursor {
            Some((x, y)) => w.move_to(x, y),
            None if !self.alt => {
                let edge = usize::from(self.size.width).saturating_sub(1);
                if w.x.is_none_or(|x| x >= edge) {
                    let y = w.y;
                    w.move_to(0, y);
                }
            }
            None => {}
        }
    }

    /// The frame's bytes, with the cursor hidden and shown or the whole
    /// wrapped in synchronized output, as Bubble Tea does.
    fn wrap(&self, body: &str, show: bool) -> String {
        let has = !body.is_empty();
        let (hide_before, show_after) = self.visibility(has, show);
        let bracket = self.sync && has;
        let mut out = String::with_capacity(body.len() + 32);
        push_if(&mut out, bracket, "\x1b[?2026h");
        push_if(&mut out, hide_before, "\x1b[?25l");
        out.push_str(body);
        push_if(&mut out, show_after, "\x1b[?25h");
        push_if(&mut out, bracket, "\x1b[?2026l");
        out
    }

    /// Whether to hide the cursor before a frame and show it after:
    /// when its visibility changes, and without synchronized output
    /// around any frame that moves a shown cursor, against flicker.
    fn visibility(&self, has: bool, show: bool) -> (bool, bool) {
        let did_show = self.shown == Some(true);
        let changed = self.shown.is_none() || did_show != show || self.pending.switched;
        let flicker = !self.sync && has && show && did_show;
        ((changed && !show) || flicker, (changed && show) || flicker)
    }

    /// Prints `text` above an inline frame, which stays below it: Bubble
    /// Tea's `Println`. Lines longer than the window wrap, counted with
    /// the width method in use, wide characters included. Does nothing
    /// on the alternate screen, as in Bubble Tea.
    pub(crate) fn insert_above(&mut self, text: &str, out: &mut dyn Write) -> io::Result<()> {
        if self.alt || text.is_empty() {
            return Ok(());
        }
        let width = usize::from(self.size.width);
        let (lines, rows) = insert::encode(text, width, self.method);
        let height = self.screen.rows.len().max(1);
        let mut s = insert::make_room(self.writer.y, height, rows);
        s.push_str(&lines);
        (self.writer.x, self.writer.y, self.writer.rows) = (Some(0), 0, height);
        if rows + height > usize::from(self.size.height) {
            self.pending.erase = true;
        } else if let Some((x, y)) = self.cursor {
            self.writer.move_to(x, y);
            s.push_str(&std::mem::take(&mut self.writer.buf));
        }
        write_out(out, &s)
    }

    /// Leaves the screen after the last frame of a program that quit or
    /// was interrupted. Inline, every row of the frame stays and the
    /// cursor goes to the start of a fresh line just below it, with
    /// anything further down erased; an empty last frame leaves nothing,
    /// the cursor where the frame started. This differs on purpose from
    /// Bubble Tea v2.2.0, which erases the frame's last row (its issue
    /// #1869, pull request #1798). On the alternate screen the cursor
    /// goes to the start of the last row, as in Bubble Tea. A hidden
    /// cursor is shown.
    pub(crate) fn finish(&mut self, out: &mut dyn Write) -> io::Result<()> {
        let rows = self.screen.rows.len();
        self.close(out, if self.alt { None } else { Some(rows) })
    }

    /// Leaves the screen as Bubble Tea's renderer does when it closes,
    /// for a program that was killed or panicked: the cursor at the start
    /// of the frame's last row and, inline, everything from there down
    /// erased.
    pub(crate) fn abandon(&mut self, out: &mut dyn Write) -> io::Result<()> {
        let rows = self.screen.rows.len();
        self.close(
            out,
            if self.alt {
                None
            } else {
                Some(rows.max(1) - 1)
            },
        )
    }

    /// Moves inline to the start of row `row` and erases from there down,
    /// or on the alternate screen (`None`) to its last row; then shows a
    /// hidden cursor. Only the first call writes anything.
    fn close(&mut self, out: &mut dyn Write, row: Option<usize>) -> io::Result<()> {
        let Some(shown) = self.shown.take() else {
            return Ok(());
        };
        let mut w = self.writer.clone();
        w.plain();
        match row {
            Some(row) => {
                w.move_to(0, row);
                w.raw("\x1b[J");
            }
            None => w.move_to(0, usize::from(self.size.height).max(1) - 1),
        }
        if !shown {
            w.raw("\x1b[?25h");
        }
        write_out(out, &w.buf)
    }
}

/// What the next frame must do even if it is the same as the last.
#[derive(Debug, Clone, Copy, Default)]
struct Pending {
    /// Draw in full on a cleared frame area.
    erase: bool,
    /// Clear the whole screen and draw from its top left.
    clear: bool,
    /// The screen was switched: the cursor is shown or hidden again.
    switched: bool,
}

/// Inline: erases rows a shorter frame no longer uses, and makes sure
/// every row of the frame exists on screen.
fn inline_tail(w: &mut Writer, old: &Frame, new: &Frame) {
    let rows = new.rows.len();
    if rows < old.rows.len() {
        w.move_to(0, rows);
        w.erase_below();
    }
    if rows > w.rows {
        w.move_to(0, rows - 1);
    }
}

/// Appends `s` when `on`.
fn push_if(out: &mut String, on: bool, s: &str) {
    if on {
        out.push_str(s);
    }
}

/// Writes and flushes `s`, if there is anything to write.
fn write_out(out: &mut dyn Write, s: &str) -> io::Result<()> {
    if s.is_empty() {
        return Ok(());
    }
    out.write_all(s.as_bytes())?;
    out.flush()
}

#[cfg(test)]
mod tests;
