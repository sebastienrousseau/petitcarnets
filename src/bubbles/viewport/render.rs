//! What a viewport shows: the lines in view, cut sideways or soft
//! wrapped, with the gutter, padded to the size, in the style.

use super::{GutterContext, Model};
use crate::style::{Style, cut, width};

impl Model {
    /// The content, padded to the size, in [`Model::style`].
    #[must_use]
    pub fn view(&self) -> String {
        if self.width == 0 || self.height == 0 {
            return String::new();
        }
        let inner_width = self.width.saturating_sub(self.style.horizontal_frame());
        let inner_height = self.height.saturating_sub(self.style.vertical_frame());
        let size = |n: usize| u16::try_from(n).unwrap_or(u16::MAX);
        let contents = Style::new()
            .width(size(inner_width))
            .height(size(inner_height))
            .render(&self.visible_lines().join("\n"));
        self.style.render(&contents)
    }

    /// The columns content gets: the width less the style's frame and
    /// the gutter.
    pub(super) fn inner_width(&self) -> usize {
        let gutter = self
            .left_gutter
            .as_ref()
            .map_or(0, |g| width(&g(GutterContext::default())));
        self.width
            .saturating_sub(self.style.horizontal_frame() + gutter)
    }

    /// The rows content gets.
    pub(super) fn inner_height(&self) -> usize {
        self.height.saturating_sub(self.style.vertical_frame())
    }

    /// The furthest it scrolls down.
    pub(super) fn max_y_offset(&self) -> usize {
        let total = self.locate(0).0;
        (total + self.style.vertical_frame()).saturating_sub(self.height)
    }

    /// For the screen line `y`: the total of screen lines, the content
    /// line it is on, and how far into that line's wraps: Go's
    /// `calculateLine`.
    pub(super) fn locate(&self, y: usize) -> (usize, usize, usize) {
        if !self.soft_wrap {
            return (self.lines.len(), y.min(self.lines.len()), 0);
        }
        let room = self.inner_width().max(1);
        let (mut total, mut index, mut into) = (0, self.lines.len(), 0);
        for (i, line) in self.lines.iter().enumerate() {
            let tall = width(line).div_ceil(room).max(1);
            if (total..total + tall).contains(&y) {
                (index, into) = (i, y - total);
            }
            total += tall;
        }
        (total, index, into)
    }

    /// The lines in view, cut, wrapped and with the gutter.
    pub(super) fn visible_lines(&self) -> Vec<String> {
        if self.inner_height() == 0 || self.inner_width() == 0 {
            return Vec::new();
        }
        let (total, first, into) = self.locate(self.y_offset);
        let lines = self.lines_from(first, total);
        if self.soft_wrap && !self.fits() {
            return self.wrapped(&lines, total, first, into);
        }
        self.with_gutter(self.cut_sideways(lines), total, first)
    }

    /// Whether every line shows whole, unscrolled.
    fn fits(&self) -> bool {
        self.x_offset == 0 && self.longest <= self.inner_width()
    }

    /// `lines` cut to the columns showing, unless they all fit.
    fn cut_sideways(&self, lines: Vec<String>) -> Vec<String> {
        if self.fits() {
            return lines;
        }
        let (left, right) = (self.x_offset, self.x_offset + self.inner_width());
        lines.iter().map(|l| cut(l, left, right)).collect()
    }

    /// A screen of content lines from line `first`, padded with blank
    /// lines when asked to fill the height.
    fn lines_from(&self, first: usize, total: usize) -> Vec<String> {
        let rows = self.inner_height();
        let end = (first + rows).min(self.lines.len()).max(first);
        let mut lines: Vec<String> = if total > 0 {
            (first..end).map(|i| self.styled(i)).collect()
        } else {
            Vec::new()
        };
        if self.fill_height && lines.len() < rows {
            lines.resize(rows, String::new());
        }
        lines
    }

    /// Content line `i`, in its line style if any.
    fn styled(&self, i: usize) -> String {
        let line = &self.lines[i];
        self.style_line
            .as_ref()
            .map_or_else(|| line.clone(), |f| f(i).render(line))
    }

    /// The gutter before each line.
    fn with_gutter(&self, lines: Vec<String>, total: usize, first: usize) -> Vec<String> {
        let Some(gutter) = &self.left_gutter else {
            return lines;
        };
        lines
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                let at = GutterContext {
                    index: i + first,
                    total_lines: total,
                    soft: false,
                };
                gutter(at) + &l
            })
            .collect()
    }

    /// The lines soft wrapped at the width, starting `into` wraps in.
    fn wrapped(&self, lines: &[String], total: usize, first: usize, into: usize) -> Vec<String> {
        let columns = self.inner_width();
        let mut out = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let w = width(line);
            let mut start = 0;
            loop {
                let part = if w <= columns {
                    line.clone()
                } else {
                    cut(line, start, start + columns)
                };
                let gutter = self.left_gutter.as_ref().map_or_else(String::new, |g| {
                    g(GutterContext {
                        index: i + first,
                        total_lines: total,
                        soft: start > 0,
                    })
                });
                out.push(gutter + &part);
                start += columns;
                if start >= w {
                    break;
                }
            }
        }
        let rows = self.inner_height();
        out.into_iter().skip(into).take(rows).collect()
    }
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("key_map", &self.key_map)
            .field("soft_wrap", &self.soft_wrap)
            .field("fill_height", &self.fill_height)
            .field("mouse_wheel_enabled", &self.mouse_wheel_enabled)
            .field("mouse_wheel_delta", &self.mouse_wheel_delta)
            .field("y_position", &self.y_position)
            .field("style", &self.style)
            .field(
                "left_gutter",
                &super::super::opaque(self.left_gutter.as_ref()),
            )
            .field(
                "style_line",
                &super::super::opaque(self.style_line.as_ref()),
            )
            .field("y_offset", &self.y_offset)
            .field("x_offset", &self.x_offset)
            .field("horizontal_step", &self.horizontal_step)
            .field("lines", &self.lines)
            .field("longest", &self.longest)
            .finish()
    }
}
