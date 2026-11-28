//! Help from key bindings: a short line, or full columns, cut to the
//! width with an ellipsis. Bubbles' `help` package.
//!
//! ```
//! use petitcarnets::bubbles::help::{self, KeyMap};
//! use petitcarnets::bubbles::key::Binding;
//! use petitcarnets::style::strip_ansi;
//!
//! struct Keys {
//!     up: Binding,
//!     quit: Binding,
//! }
//!
//! impl KeyMap for Keys {
//!     fn short_help(&self) -> Vec<Binding> {
//!         vec![self.up.clone(), self.quit.clone()]
//!     }
//!     fn full_help(&self) -> Vec<Vec<Binding>> {
//!         vec![vec![self.up.clone()], vec![self.quit.clone()]]
//!     }
//! }
//!
//! let keys = Keys {
//!     up: Binding::new(&["up"]).with_help("↑", "up"),
//!     quit: Binding::new(&["q"]).with_help("q", "quit"),
//! };
//! let mut help = help::Model::new();
//! assert_eq!(strip_ansi(&help.view(&keys)), "↑ up • q quit");
//! help.set_width(8);
//! assert_eq!(strip_ansi(&help.view(&keys)), "↑ up …");
//! ```

use super::key::Binding;
use crate::style::{Color, Position, Style, join_horizontal, width};
use crate::{Cmd, Msg};

/// Something with bindings to show in help: Bubbles' `help.KeyMap`.
pub trait KeyMap {
    /// The bindings for the short, one line help.
    fn short_help(&self) -> Vec<Binding>;
    /// The bindings for the full help, a column each group.
    fn full_help(&self) -> Vec<Vec<Binding>>;
}

/// The styles of help: Bubbles' `help.Styles`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Styles {
    /// The ellipsis shown when help is cut.
    pub ellipsis: Style,
    /// A key in the short help.
    pub short_key: Style,
    /// A description in the short help.
    pub short_desc: Style,
    /// The separator in the short help.
    pub short_separator: Style,
    /// A key in the full help.
    pub full_key: Style,
    /// A description in the full help.
    pub full_desc: Style,
    /// The separator between full help columns.
    pub full_separator: Style,
}

impl Styles {
    /// Bubbles' default styles for a dark (`is_dark`) or light
    /// background: `help.DefaultStyles`.
    #[must_use]
    pub fn new(is_dark: bool) -> Self {
        let pick = |dark: Color, light: Color| if is_dark { dark } else { light };
        let key = Style::new().fg(pick(
            Color::Rgb(0x62, 0x62, 0x62),
            Color::Rgb(0x90, 0x90, 0x90),
        ));
        let desc = Style::new().fg(pick(
            Color::Rgb(0x4A, 0x4A, 0x4A),
            Color::Rgb(0xB2, 0xB2, 0xB2),
        ));
        let sep = Style::new().fg(pick(
            Color::Rgb(0x3C, 0x3C, 0x3C),
            Color::Rgb(0xDA, 0xDA, 0xDA),
        ));
        Self {
            ellipsis: sep,
            short_key: key,
            short_desc: desc,
            short_separator: sep,
            full_key: key,
            full_desc: desc,
            full_separator: sep,
        }
    }
}

/// The help view: Bubbles' `help.Model`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// Whether to show the full help.
    pub show_all: bool,
    /// Between items of the short help.
    pub short_separator: String,
    /// Between columns of the full help.
    pub full_separator: String,
    /// Shown where help is cut to the width.
    pub ellipsis: String,
    /// The styles.
    pub styles: Styles,
    width: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Short help, with Bubbles' separators, ellipsis and dark styles:
    /// `help.New`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            show_all: false,
            short_separator: " • ".into(),
            full_separator: "    ".into(),
            ellipsis: "…".into(),
            styles: Styles::new(true),
            width: 0,
        }
    }

    /// Does nothing: help has no state of its own to change.
    pub fn update<M>(&mut self, _msg: &Msg<M>) -> Cmd<M> {
        Cmd::none()
    }

    /// The help for `keys`, full or short as [`Model::show_all`] says.
    #[must_use]
    pub fn view(&self, keys: &impl KeyMap) -> String {
        if self.show_all {
            self.full_help_view(&keys.full_help())
        } else {
            self.short_help_view(&keys.short_help())
        }
    }

    /// The most columns help takes; 0 is no limit.
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// The most columns help takes.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// One line of the bindings that are on, `key desc` with separators,
    /// cut with the ellipsis where the width ends.
    #[must_use]
    pub fn short_help_view(&self, bindings: &[Binding]) -> String {
        let separator = self.styles.short_separator.paint(&self.short_separator);
        let mut out = String::new();
        let mut total = 0;
        for b in bindings.iter().filter(|b| b.enabled()) {
            let sep = if total > 0 { separator.as_str() } else { "" };
            let item = format!(
                "{sep}{} {}",
                self.styles.short_key.paint(&b.help().key),
                self.styles.short_desc.paint(&b.help().desc)
            );
            let w = width(&item);
            if let Err(tail) = self.fits(total, w) {
                out.push_str(&tail);
                break;
            }
            total += w;
            out.push_str(&item);
        }
        out
    }

    /// A column for each group with a binding on: keys, then their
    /// descriptions, columns cut with the ellipsis where the width ends.
    #[must_use]
    pub fn full_help_view(&self, groups: &[Vec<Binding>]) -> String {
        let mut columns = Vec::new();
        let mut total = 0;
        for group in groups {
            let on: Vec<&Binding> = group.iter().filter(|b| b.enabled()).collect();
            if on.is_empty() {
                continue;
            }
            let column = self.column(&on, total > 0);
            let w = column.split('\n').map(width).max().unwrap_or(0);
            if let Err(tail) = self.fits(total, w) {
                columns.push(tail);
                break;
            }
            total += w;
            columns.push(column);
        }
        let blocks: Vec<&str> = columns.iter().map(String::as_str).collect();
        join_horizontal(Position::TOP, &blocks)
    }

    /// One column of the full help, after a separator if `separated`.
    fn column(&self, bindings: &[&Binding], separated: bool) -> String {
        let keys: Vec<&str> = bindings.iter().map(|b| b.help().key.as_str()).collect();
        let descs: Vec<&str> = bindings.iter().map(|b| b.help().desc.as_str()).collect();
        let sep = if separated {
            let blank = " ".repeat(width(&self.full_separator));
            let mut lines = vec![self.full_separator.clone()];
            lines.resize(bindings.len(), blank);
            self.styles.full_separator.render(&lines.join("\n"))
        } else {
            String::new()
        };
        let keys = self.styles.full_key.render(&keys.join("\n"));
        let descs = self.styles.full_desc.render(&descs.join("\n"));
        join_horizontal(Position::TOP, &[&sep, &keys, " ", &descs])
    }

    /// `Ok` if an item `w` wide fits after `total` columns; else the
    /// ellipsis to end with, empty if even that does not fit.
    fn fits(&self, total: usize, w: usize) -> Result<(), String> {
        if self.width == 0 || total + w <= self.width {
            return Ok(());
        }
        let tail = format!(" {}", self.styles.ellipsis.paint(&self.ellipsis));
        if total + width(&tail) < self.width {
            Err(tail)
        } else {
            Err(String::new())
        }
    }
}

#[cfg(test)]
mod tests;
