//! Cells per code point and per grapheme cluster.

/// `wcRuneWidth` by code point: Go's `unicode.Mn`, `Me` and `Cf` are
/// zero, then go-runewidth decides. Each entry holds from its code point
/// up to the next entry's.
static WC: [(u32, u8); 1002] = include!("../wcwidth_table.in");

/// displaywidth's width of a cluster by the code point it starts with,
/// laid out as [`WC`].
static LEAD: [(u32, u8); 961] = include!("../displaywidth_table.in");

/// The value `table` holds for `c`.
fn lookup(table: &[(u32, u8)], c: char) -> usize {
    let r = u32::from(c);
    let after = table.partition_point(|&(first, _)| first <= r);
    usize::from(table[after - 1].1)
}

/// The cells `c` takes on its own: Go's `wcRuneWidth`.
pub(super) fn wc_char(c: char) -> usize {
    if c.is_ascii_graphic() || c == ' ' {
        return 1;
    }
    lookup(&WC, c)
}

/// The cells `cluster` takes, one code point at a time: Go's
/// `wcClusterWidth`.
pub(super) fn wc_width(cluster: &str) -> usize {
    cluster.chars().map(wc_char).sum()
}

/// The cells the grapheme cluster `cluster` takes as a whole: Go's
/// `displaywidth` `graphemeWidth`, with ambiguous width taken as narrow.
/// A cluster led by a control is zero, and VS16 after the first code
/// point makes it wide.
pub(super) fn cluster_width(cluster: &str) -> usize {
    let mut chars = cluster.chars();
    let Some(first) = chars.next() else {
        return 0;
    };
    if first.is_ascii_control() {
        return 0;
    }
    if first.is_ascii() && cluster.len() == 1 {
        return 1;
    }
    if chars.next() == Some('\u{FE0F}') {
        return 2;
    }
    lookup(&LEAD, first)
}
