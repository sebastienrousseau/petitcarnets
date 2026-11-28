//! How many cells text takes on a terminal, measured as Go's
//! `charmbracelet/x/ansi` v0.12.0 measures it: `WcWidth` (each code point
//! on its own, as `wcwidth` does) or `GraphemeWidth` (each grapheme
//! cluster as a whole).
//!
//! The tables are generated from the Go packages by
//! scripts/gen-width-tables.py, and tests/fixtures/width.tsv holds what Go
//! answers for a few thousand strings.

mod cells;
mod parser;
mod segment;

/// How text width is measured. Go's `ansi.Method`.
///
/// A program measures with [`WidthMethod::WcWidth`] until the terminal
/// reports that it supports grapheme clusters (mode 2027), as Bubble Tea
/// does; [`crate::style::width`] follows the running program's choice.
///
/// ```
/// use petitcarnets as tea;
/// use tea::style::WidthMethod;
///
/// // A thumbs up with a skin tone: two code points, one grapheme.
/// let thumb = "\u{1f44d}\u{1f3fd}";
/// assert_eq!(WidthMethod::WcWidth.string_width(thumb), 4);
/// assert_eq!(WidthMethod::GraphemeWidth.string_width(thumb), 2);
/// assert_eq!(
///     WidthMethod::GraphemeWidth.string_width("\x1b[1mok\x1b[0m"),
///     2
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WidthMethod {
    /// Each code point on its own, as `wcwidth` does: Go's `ansi.WcWidth`.
    #[default]
    WcWidth,
    /// Each grapheme cluster as a whole: Go's `ansi.GraphemeWidth`.
    GraphemeWidth,
}

impl WidthMethod {
    /// The cells `text` takes, escape sequences left out: Go's
    /// `Method.StringWidth`.
    #[must_use]
    pub fn string_width(self, text: &str) -> usize {
        parser::string_width(text, self)
    }

    /// The cells the grapheme cluster `cluster` takes.
    fn cluster_width(self, cluster: &str) -> usize {
        match self {
            Self::WcWidth => cells::wc_width(cluster),
            Self::GraphemeWidth => cells::cluster_width(cluster),
        }
    }
}

/// The first grapheme cluster of `s` and its width: Go's
/// `ansi.FirstGraphemeCluster`. An empty `s` gives an empty cluster.
pub(crate) fn first_cluster(s: &str, method: WidthMethod) -> (&str, usize) {
    let cluster = &s[..segment::first_len(s)];
    (cluster, method.cluster_width(cluster))
}

#[cfg(test)]
mod tests;
