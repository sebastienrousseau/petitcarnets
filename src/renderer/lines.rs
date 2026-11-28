//! Accessible output: frames as plain lines, for screen readers and
//! braille displays (Bubble Tea's issue #780).
//!
//! Nothing is redrawn and the cursor is never moved: each frame's lines
//! that differ from the line in the same place in the frame before are
//! printed once, in order, each ended with `\r\n`, and blank lines are
//! left out. A new line is printed where the terminal's cursor is, so
//! the terminal (and the screen reader following it) sees an ordinary
//! program writing lines. No alternate screen, no synchronized output,
//! no hidden cursor; the colour profile decides the styles, and the
//! program picks no colour unless it was asked for one.

use super::layout::{Pens, line_cells, lines};
use super::writer::{Addressing, Writer};
use crate::color::ColorProfile;
use crate::unicode::WidthMethod;

/// Shows the cursor, which the terminal hid when it was entered.
const SHOW_CURSOR: &str = "\x1b[?25h";

/// The lines of the last frame, as printed.
#[derive(Debug, Clone, Default)]
pub(crate) struct Lines {
    shown: Vec<String>,
    /// Whether to show the terminal's cursor before the first output.
    show_cursor: bool,
}

impl Lines {
    /// A line printer that shows the cursor first if `show_cursor`.
    pub(crate) const fn new(show_cursor: bool) -> Self {
        Self {
            shown: Vec::new(),
            show_cursor,
        }
    }

    /// The bytes for a frame of `content`: every non-blank line that
    /// differs from the line in its place in the frame before.
    pub(crate) fn frame(
        &mut self,
        content: &str,
        method: WidthMethod,
        profile: ColorProfile,
    ) -> String {
        let new = encode(content, method, profile);
        let mut out = String::new();
        for (i, line) in new.iter().enumerate() {
            if !line.is_empty() && self.shown.get(i) != Some(line) {
                out.push_str(line);
                out.push_str("\r\n");
            }
        }
        self.shown = new;
        self.first(out)
    }

    /// The bytes that print `text`, every line of it, blank ones too:
    /// Bubble Tea's `Println`.
    pub(crate) fn print(
        &mut self,
        text: &str,
        method: WidthMethod,
        profile: ColorProfile,
    ) -> String {
        let mut out = String::new();
        for line in encode(text, method, profile) {
            out.push_str(&line);
            out.push_str("\r\n");
        }
        self.first(out)
    }

    /// Forgets the last frame, so the next is printed in full.
    pub(crate) fn forget(&mut self) {
        self.shown.clear();
    }

    /// `out`, with the cursor shown first if this is the first output.
    fn first(&mut self, out: String) -> String {
        if out.is_empty() || !self.show_cursor {
            return out;
        }
        self.show_cursor = false;
        format!("{SHOW_CURSOR}{out}")
    }
}

/// Each line of `content` as it is written with `profile`: its styles
/// ended at its end, and trailing spaces left out.
fn encode(content: &str, method: WidthMethod, profile: ColorProfile) -> Vec<String> {
    let mut pens = Pens::default();
    lines(content)
        .into_iter()
        .map(|line| {
            let mut w = Writer::new(usize::MAX, 1, Addressing::Relative);
            w.profile = profile;
            for cell in &line_cells(line, usize::MAX, method, &mut pens) {
                w.put(cell);
            }
            w.plain();
            w.buf.trim_end_matches(' ').to_owned()
        })
        .collect()
}

#[cfg(test)]
mod tests;
