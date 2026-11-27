//! Text helpers the editing components share: grapheme clusters over a
//! line of characters, widths, and Bubbles' input sanitiser.

use crate::style::width_method;
use crate::unicode::first_width_cluster;

/// The grapheme clusters of `s`, with the cells each takes.
pub(super) fn clusters(s: &str) -> Vec<(&str, usize)> {
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let (cluster, w) = first_width_cluster(rest, width_method());
        let len = cluster
            .len()
            .max(rest.chars().next().map_or(1, char::len_utf8));
        out.push((&rest[..len], w));
        rest = &rest[len..];
    }
    out
}

/// The character range `(start, end)` of the grapheme cluster holding
/// the character at `col` in `line`; `(col, col)` past either end.
/// Bubbles v2's `graphemeAt`.
pub(super) fn grapheme_at(line: &[char], col: usize) -> (usize, usize) {
    if col >= line.len() {
        return (col, col);
    }
    let text: String = line.iter().collect();
    let mut start = 0;
    for (cluster, _) in clusters(&text) {
        let end = start + cluster.chars().count();
        if col < end {
            return (start, end);
        }
        start = end;
    }
    (col, col + 1)
}

/// Whether `c` is a space as Go's `unicode.IsSpace` says.
pub(super) fn is_space(c: char) -> bool {
    c.is_whitespace()
}

/// `chars` cleaned as Bubbles' `runeutil` sanitiser cleans input:
/// replacement characters and control characters dropped, line breaks
/// and tabs replaced.
pub(super) fn sanitize(chars: &[char], newline: &str, tab: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    for &c in chars {
        match c {
            '\u{FFFD}' => {}
            '\r' | '\n' => out.extend(newline.chars()),
            '\t' => out.extend(tab.chars()),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests;
