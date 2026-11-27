//! Cursor moves: the shortest that never uses tabs or backspaces.

use super::*;

fn moved(addressing: Addressing, from: (usize, usize), to: (usize, usize)) -> String {
    let mut w = Writer::new(80, 24, addressing);
    (w.x, w.y) = (Some(from.0), from.1);
    w.move_to(to.0, to.1);
    w.buf
}

#[test]
fn moves_take_the_fewest_bytes() {
    use Addressing::{Absolute, Relative};
    assert_eq!(moved(Absolute, (5, 5), (5, 5)), "");
    assert_eq!(moved(Absolute, (5, 5), (0, 5)), "\r");
    assert_eq!(moved(Absolute, (5, 5), (6, 5)), "\x1b[C");
    assert_eq!(moved(Absolute, (5, 5), (2, 5)), "\x1b[3D");
    assert_eq!(moved(Absolute, (5, 5), (0, 6)), "\r\n");
    assert_eq!(moved(Absolute, (5, 5), (40, 20)), "\x1b[21;41H");
    assert_eq!(moved(Relative, (5, 5), (40, 20)), "\x1b[15B\x1b[35C");
    assert_eq!(moved(Relative, (5, 5), (5, 2)), "\x1b[3A");
}

#[test]
fn inline_rows_below_the_frame_are_made_with_line_feeds() {
    let mut w = Writer::new(80, 1, Addressing::Relative);
    w.move_to(3, 2);
    assert_eq!(w.buf, "\r\n\r\n\x1b[3C");
    assert_eq!(w.rows, 3);
}

#[test]
fn the_last_column_leaves_the_column_unknown() {
    let mut w = Writer::new(3, 1, Addressing::Absolute);
    for c in ["a", "b", "c"] {
        w.put(&Cell::new(c, 1, Pen::DEFAULT, None));
    }
    assert_eq!(w.x, None);
    w.move_to(1, 0);
    assert!(w.buf.ends_with("\x1b[2G"), "{:?}", w.buf);
}

#[test]
fn links_open_and_close() {
    let mut w = Writer::new(10, 1, Addressing::Absolute);
    let link: Link = Some("id=1;http://x".into());
    w.put(&Cell::new("a", 1, Pen::DEFAULT, link));
    w.plain();
    assert_eq!(w.buf, "\x1b]8;id=1;http://x\x07a\x1b]8;;\x07");
}
