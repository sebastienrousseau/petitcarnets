//! Go's `ansi.StringWidth`: the DEC parser state machine steps over
//! escape sequences byte by byte, and text is measured where it prints.

use super::{WidthMethod, cells, first_cluster};

/// ansi's `parser.Table`: `action << 4 | next state`, by
/// `state << 8 | byte`.
static TABLE: [u8; 3840] = include!("../parser_table.in");

/// The ground state.
const GROUND: u8 = 0;
/// The state the parser enters on a UTF-8 lead byte.
const UTF8: u8 = 14;
/// The print action.
const PRINT: u8 = 9;

/// The cells `text` takes, as Go's `stringWidth` counts them.
pub(super) fn string_width(text: &str, method: WidthMethod) -> usize {
    let bytes = text.as_bytes();
    let (mut at, mut state, mut width) = (0, GROUND, 0);
    while let Some(&byte) = bytes.get(at) {
        let entry = TABLE[usize::from(state) << 8 | usize::from(byte)];
        state = entry & 0x0F;
        if entry >> 4 == PRINT || state == UTF8 {
            // A printable ASCII byte or a UTF-8 lead byte, so a character
            // starts here.
            let (len, cells) = measure(&text[at..], method);
            width += cells;
            at += len;
            state = GROUND;
        } else {
            at += 1;
        }
    }
    width
}

/// The length and width of what prints at the start of `s`: one code
/// point with `WcWidth`, one grapheme cluster with `GraphemeWidth`.
fn measure(s: &str, method: WidthMethod) -> (usize, usize) {
    if let (WidthMethod::WcWidth, Some(c)) = (method, s.chars().next()) {
        return (c.len_utf8(), cells::wc_char(c));
    }
    let (cluster, width) = first_cluster(s, method);
    (cluster.len().max(1), width)
}
