//! Drawing a view: only the lines that changed since the last frame are
//! rewritten.

use crate::msg::Size;
use crate::style::truncate;
use std::fmt::Write as _;
use std::io::{self, Write};

/// The last frame drawn, line by line.
#[derive(Debug)]
pub(crate) struct Renderer {
    lines: Vec<String>,
    size: Size,
}

impl Renderer {
    /// A renderer for a window of `size`.
    pub(crate) fn new(size: Size) -> Self {
        Self {
            lines: Vec::new(),
            size,
        }
    }

    /// A new window size: the next frame is drawn in full on a clear screen.
    pub(crate) fn resize(&mut self, size: Size, out: &mut dyn Write) -> io::Result<()> {
        self.size = size;
        self.lines.clear();
        out.write_all(b"\x1b[2J")
    }

    /// Draws `view`, cut to the window, rewriting only changed lines.
    pub(crate) fn render(&mut self, view: &str, out: &mut dyn Write) -> io::Result<()> {
        let (columns, rows) = (usize::from(self.size.width), usize::from(self.size.height));
        let frame: Vec<String> = view
            .lines()
            .take(rows)
            .map(|l| truncate(l, columns))
            .collect();
        let mut buf = String::new();
        for (row, line) in frame.iter().enumerate() {
            if self.lines.get(row) != Some(line) {
                let _ = write!(buf, "\x1b[{};1H{line}\x1b[K", row + 1);
            }
        }
        for row in frame.len()..self.lines.len() {
            let _ = write!(buf, "\x1b[{};1H\x1b[K", row + 1);
        }
        self.lines = frame;
        if buf.is_empty() {
            return Ok(());
        }
        out.write_all(buf.as_bytes())?;
        out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(r: &mut Renderer, view: &str) -> String {
        let mut out = Vec::new();
        r.render(view, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn the_first_frame_draws_every_line() {
        let mut r = Renderer::new(Size::new(80, 24));
        assert_eq!(draw(&mut r, "a\nb"), "\x1b[1;1Ha\x1b[K\x1b[2;1Hb\x1b[K");
    }

    #[test]
    fn an_unchanged_frame_writes_nothing_and_a_change_one_line() {
        let mut r = Renderer::new(Size::new(80, 24));
        let _ = draw(&mut r, "a\nb\nc");
        assert_eq!(draw(&mut r, "a\nb\nc"), "");
        assert_eq!(draw(&mut r, "a\nB\nc"), "\x1b[2;1HB\x1b[K");
    }

    #[test]
    fn a_shorter_frame_clears_the_lines_left_over() {
        let mut r = Renderer::new(Size::new(80, 24));
        let _ = draw(&mut r, "a\nb\nc");
        assert_eq!(draw(&mut r, "a"), "\x1b[2;1H\x1b[K\x1b[3;1H\x1b[K");
    }

    #[test]
    fn the_frame_is_cut_to_the_window() {
        let mut r = Renderer::new(Size::new(3, 2));
        assert_eq!(
            draw(&mut r, "abcdef\nxy\nzzz"),
            "\x1b[1;1Habc\x1b[0m\x1b[K\x1b[2;1Hxy\x1b[K"
        );
    }

    #[test]
    fn a_resize_clears_and_redraws_in_full() {
        let mut r = Renderer::new(Size::new(80, 24));
        let _ = draw(&mut r, "a");
        let mut out = Vec::new();
        r.resize(Size::new(40, 10), &mut out).unwrap();
        assert_eq!(out, b"\x1b[2J");
        assert_eq!(draw(&mut r, "a"), "\x1b[1;1Ha\x1b[K");
    }
}
