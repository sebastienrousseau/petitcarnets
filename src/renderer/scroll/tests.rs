//! Scroll detection.

use super::*;
use crate::renderer::layout::layout;
use crate::renderer::writer::Addressing;
use crate::unicode::WidthMethod;

fn frame(lines: &[&str]) -> Frame {
    layout(
        &lines.join("\n"),
        10,
        Some(lines.len()),
        WidthMethod::WcWidth,
    )
}

#[test]
fn a_whole_frame_moved_up_is_found() {
    let old = frame(&["a", "b", "c", "d", "e"]);
    let new = frame(&["c", "d", "e", "f", "g"]);
    let s = detect(&old, &new).unwrap();
    assert_eq!((s.top, s.bottom, s.by), (0, 4, 2));
    let mut w = Writer::new(10, 5, Addressing::Absolute);
    let shifted = apply(&mut w, &old, s);
    assert_eq!(w.buf, "\x1b[2S");
    assert_eq!(shifted.rows[0], new.rows[0]);
    assert_eq!(shifted.rows[4], Row::blank(10));
}

#[test]
fn a_region_moved_down_is_found() {
    let old = frame(&["head", "a", "b", "c", "foot"]);
    let new = frame(&["head", "z", "a", "b", "foot"]);
    let s = detect(&old, &new).unwrap();
    assert_eq!((s.top, s.bottom, s.by), (1, 3, -1));
    let mut w = Writer::new(10, 5, Addressing::Absolute);
    let shifted = apply(&mut w, &old, s);
    assert_eq!(w.buf, "\x1b[2;4r\x1b[T\x1b[r");
    assert_eq!((w.x, w.y), (Some(0), 0), "a region homes the cursor");
    assert_eq!(shifted.rows[2], new.rows[2]);
    assert_eq!(shifted.rows[1], Row::blank(10));
}

#[test]
fn unrelated_or_blank_frames_do_not_scroll() {
    assert!(detect(&frame(&["a", "b", "c"]), &frame(&["x", "y", "z"])).is_none());
    assert!(detect(&frame(&["", "", ""]), &frame(&["", "", "a"])).is_none());
    assert!(detect(&frame(&["a", "b"]), &frame(&["b"])).is_none());
}
