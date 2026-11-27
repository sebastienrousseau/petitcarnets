//! A small terminal emulator for tests. It reads the bytes a renderer
//! writes and keeps the screen a terminal would show (xterm's rules), so
//! a test can check that the screen is the frame that was meant.
//!
//! Anything it does not understand, and the controls a good renderer
//! never sends (tab and backspace), is logged in [`Vt::unknown`].

mod csi;
mod edit;
mod escape;
mod grid;
mod modes;
mod print;
mod stream;

#[cfg(test)]
mod tests;

use crate::ansi::{self, Token};
use crate::renderer::cell::{Cell, Link, Pen};
use crate::unicode::WidthMethod;
use grid::Grid;
use std::collections::BTreeSet;

/// Where the cursor is.
#[derive(Debug, Clone, Copy, Default)]
struct Cursor {
    /// Column, from 0.
    x: u16,
    /// Row, from 0.
    y: u16,
    /// Whether the last column was just written, so the next character
    /// wraps first (with autowrap on).
    wrap: bool,
}

/// A cursor saved by `ESC 7` or mode 1049: its place and its pen.
#[derive(Debug, Clone, Copy, Default)]
struct Saved {
    /// Column.
    x: u16,
    /// Row.
    y: u16,
    /// The pen.
    pen: Pen,
}

/// A terminal emulator for tests: feed it bytes, then look at the
/// screen.
#[derive(Debug)]
pub(crate) struct Vt {
    /// Width in cells.
    columns: u16,
    /// Height in cells.
    rows: u16,
    /// The main screen.
    main: Grid,
    /// The alternate screen.
    alt: Grid,
    /// Whether the alternate screen is shown.
    on_alt: bool,
    /// The cursor.
    cursor: Cursor,
    /// The cell last printed, which a zero-width mark joins.
    last: Option<(u16, u16)>,
    /// The pen new text is written with.
    pen: Pen,
    /// The hyperlink new text is written with.
    link: Link,
    /// The cursor `ESC 7` saved.
    saved: Saved,
    /// The cursor mode 1049 saved.
    saved_alt: Saved,
    /// First row of the scroll region.
    top: u16,
    /// Last row of the scroll region.
    bottom: u16,
    /// The DEC private modes that are set.
    modes: BTreeSet<u16>,
    /// The last cursor style set (`CSI n SP q`).
    cursor_style: u8,
    /// The window title.
    title: String,
    /// Titles pushed by `CSI 22 t`.
    titles: Vec<String>,
    /// Main screen lines scrolled off the top, oldest first.
    scrollback: Vec<String>,
    /// How many synchronised updates were opened and closed.
    sync_frames: usize,
    /// Every OSC payload, in order.
    osc_log: Vec<String>,
    /// Everything not understood, in order.
    unknown: Vec<String>,
    /// Bytes of a sequence or code point cut off by the end of a feed.
    partial: Vec<u8>,
    /// How text width is measured.
    method: WidthMethod,
}

impl Vt {
    /// A `columns` by `rows` terminal (at least 1 by 1) on its main
    /// screen: autowrap on, cursor shown at the top left, default pen.
    pub(crate) fn new(columns: u16, rows: u16) -> Self {
        let (columns, rows) = (columns.max(1), rows.max(1));
        Self {
            columns,
            rows,
            main: Grid::new(columns, rows, &Cell::BLANK),
            alt: Grid::new(columns, rows, &Cell::BLANK),
            on_alt: false,
            cursor: Cursor::default(),
            last: None,
            pen: Pen::DEFAULT,
            link: None,
            saved: Saved::default(),
            saved_alt: Saved::default(),
            top: 0,
            bottom: rows - 1,
            modes: BTreeSet::from([7, 25]),
            cursor_style: 0,
            title: String::new(),
            titles: Vec::new(),
            scrollback: Vec::new(),
            sync_frames: 0,
            osc_log: Vec::new(),
            unknown: Vec::new(),
            partial: Vec::new(),
            method: WidthMethod::default(),
        }
    }

    /// The same terminal, measuring text with `method`.
    pub(crate) fn with_width_method(mut self, method: WidthMethod) -> Self {
        self.method = method;
        self
    }

    /// Reads `bytes`. A sequence or code point cut off at the end waits
    /// for the next feed; invalid UTF-8 reads as U+FFFD.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        self.partial.extend_from_slice(bytes);
        let (text, held) = stream::split_utf8(&self.partial);
        let cut = stream::complete_len(&text, self.method);
        self.partial = text.as_bytes()[cut..].to_vec();
        self.partial.extend(held);
        for token in ansi::tokens(&text[..cut], self.method) {
            self.token(token);
        }
    }

    /// Acts on one token.
    fn token(&mut self, token: Token<'_>) {
        if !keeps_last(&token) {
            self.last = None;
        }
        match token {
            Token::Text(cluster, width) => self.print(cluster, width),
            Token::Control(byte) => self.control(byte),
            Token::Csi { params, final_byte } => csi::dispatch(self, params, final_byte),
            Token::Osc(payload) => self.osc(payload),
            Token::Other(s) => self.escape(s),
        }
    }

    /// A new size, `columns` by `rows` (at least 1 by 1): rows and
    /// columns are cut or padded with blanks, not reflowed; the cursor
    /// stays inside and the scroll region is the whole screen again.
    pub(crate) fn resize(&mut self, columns: u16, rows: u16) {
        let (columns, rows) = (columns.max(1), rows.max(1));
        self.main.resize(columns, rows);
        self.alt.resize(columns, rows);
        (self.columns, self.rows) = (columns, rows);
        (self.top, self.bottom) = (0, rows - 1);
        self.move_to(self.cursor.x, self.cursor.y);
    }

    /// The cell at column `x` of row `y` on the screen shown.
    pub(crate) fn cell(&self, x: u16, y: u16) -> &Cell {
        self.screen().cell(x, y)
    }

    /// What row `y` shows: continuation cells skipped, trailing spaces
    /// left out.
    pub(crate) fn row_text(&self, y: u16) -> String {
        self.screen().row_text(y)
    }

    /// What the screen shows: each row's text, joined by newlines,
    /// trailing empty rows left out.
    pub(crate) fn text(&self) -> String {
        let mut lines: Vec<String> = (0..self.rows).map(|y| self.row_text(y)).collect();
        while lines.last().is_some_and(String::is_empty) {
            drop(lines.pop());
        }
        lines.join("\n")
    }

    /// The cursor's column and row, from 0.
    pub(crate) fn cursor(&self) -> (u16, u16) {
        (self.cursor.x, self.cursor.y)
    }

    /// Whether the cursor is shown (mode 25).
    pub(crate) fn cursor_visible(&self) -> bool {
        self.mode(25)
    }

    /// The last cursor style set by `CSI n SP q`, 0 at first.
    pub(crate) fn cursor_style(&self) -> u8 {
        self.cursor_style
    }

    /// Whether the alternate screen is shown.
    pub(crate) fn alt_screen(&self) -> bool {
        self.on_alt
    }

    /// Whether autowrap is on (mode 7).
    pub(crate) fn autowrap(&self) -> bool {
        self.mode(7)
    }

    /// Whether DEC private mode `n` is set.
    pub(crate) fn mode(&self, n: u16) -> bool {
        self.modes.contains(&n)
    }

    /// The pen new text is written with.
    pub(crate) fn pen(&self) -> Pen {
        self.pen
    }

    /// The window title.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// The text of main screen lines scrolled off the top, oldest first.
    pub(crate) fn scrollback(&self) -> &[String] {
        &self.scrollback
    }

    /// Whether a synchronised update (`CSI ? 2026 h`) is open.
    pub(crate) fn in_sync(&self) -> bool {
        self.mode(2026)
    }

    /// How many synchronised updates were opened and closed.
    pub(crate) fn sync_frames(&self) -> usize {
        self.sync_frames
    }

    /// Every OSC payload, in order.
    pub(crate) fn osc_log(&self) -> &[String] {
        &self.osc_log
    }

    /// Everything not understood, and every tab and backspace, in order.
    pub(crate) fn unknown(&self) -> &[String] {
        &self.unknown
    }

    /// The screen shown.
    fn screen(&self) -> &Grid {
        if self.on_alt { &self.alt } else { &self.main }
    }

    /// The screen shown, to change.
    fn screen_mut(&mut self) -> &mut Grid {
        if self.on_alt {
            &mut self.alt
        } else {
            &mut self.main
        }
    }

    /// What erasing leaves: a blank that keeps the pen's background.
    fn blank(&self) -> Cell {
        let pen = Pen {
            bg: self.pen.bg,
            ..Pen::DEFAULT
        };
        Cell { pen, ..Cell::BLANK }
    }

    /// Moves the cursor to column `x` of row `y`, kept on the screen.
    fn move_to(&mut self, x: u16, y: u16) {
        self.cursor = Cursor {
            x: x.min(self.columns - 1),
            y: y.min(self.rows - 1),
            wrap: false,
        };
    }

    /// Logs something not understood.
    fn log(&mut self, what: String) {
        self.unknown.push(what);
    }
}

/// Whether a zero-width mark after `token` still joins the cell last
/// printed: only text, OSC and SGR leave the cursor where it was.
fn keeps_last(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::Text(..)
            | Token::Osc(_)
            | Token::Csi {
                final_byte: b'm',
                ..
            }
    )
}
