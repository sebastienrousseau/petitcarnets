//! Control sequences that erase, insert, delete and scroll. Erased cells
//! keep the pen's background colour, as xterm's do.

use super::Vt;
use super::csi::Params;

/// EL: erases to the right (0), to the left (1) or the whole line (2).
pub(super) fn el(vt: &mut Vt, p: &Params<'_>) -> bool {
    let (x, y) = (vt.cursor.x, vt.cursor.y);
    let (from, to) = match p.get(0, 0) {
        0 => (x, u16::MAX),
        1 => (0, x + 1),
        2 => (0, u16::MAX),
        _ => return false,
    };
    let blank = vt.blank();
    vt.screen_mut().fill(y, from, to, &blank);
    vt.cursor.wrap = false;
    true
}

/// ED: erases below (0), above (1), all (2), or all and the scrollback
/// (3).
pub(super) fn ed(vt: &mut Vt, p: &Params<'_>) -> bool {
    let mode = p.get(0, 0);
    let Some((rows, line)) = ed_parts(mode, vt.cursor.x, vt.cursor.y, vt.rows) else {
        return false;
    };
    if mode == 3 {
        vt.scrollback.clear();
    }
    let (y, blank) = (vt.cursor.y, vt.blank());
    let grid = vt.screen_mut();
    grid.fill_rows(rows.0, rows.1, &blank);
    grid.fill(y, line.0, line.1, &blank);
    vt.cursor.wrap = false;
    true
}

/// What ED `mode` erases with the cursor at `x`, `y` on a screen of
/// `rows`: whole rows `from..to`, and columns `from..to` of row `y`.
fn ed_parts(mode: u16, x: u16, y: u16, rows: u16) -> Option<((u16, u16), (u16, u16))> {
    match mode {
        0 => Some(((y + 1, rows), (x, u16::MAX))),
        1 => Some(((0, y), (0, x + 1))),
        2 | 3 => Some(((0, rows), (0, 0))),
        _ => None,
    }
}

/// ECH: erases cells from the cursor on.
pub(super) fn ech(vt: &mut Vt, p: &Params<'_>) -> bool {
    let (x, y) = (vt.cursor.x, vt.cursor.y);
    let blank = vt.blank();
    vt.screen_mut()
        .fill(y, x, x.saturating_add(p.get(0, 1)), &blank);
    vt.cursor.wrap = false;
    true
}

/// ICH: inserts blank cells at the cursor.
pub(super) fn ich(vt: &mut Vt, p: &Params<'_>) -> bool {
    let (x, y) = (vt.cursor.x, vt.cursor.y);
    let blank = vt.blank();
    vt.screen_mut().insert_cells(x, y, p.get(0, 1), &blank);
    vt.cursor.wrap = false;
    true
}

/// DCH: deletes cells at the cursor.
pub(super) fn dch(vt: &mut Vt, p: &Params<'_>) -> bool {
    let (x, y) = (vt.cursor.x, vt.cursor.y);
    let blank = vt.blank();
    vt.screen_mut().delete_cells(x, y, p.get(0, 1), &blank);
    vt.cursor.wrap = false;
    true
}

/// IL: inserts blank lines at the cursor's row, pushing the region's
/// lower lines out; only inside the region. Goes to the first column.
pub(super) fn il(vt: &mut Vt, p: &Params<'_>) -> bool {
    if in_region(vt) {
        let (y, bottom, blank) = (vt.cursor.y, vt.bottom, vt.blank());
        vt.screen_mut().scroll_down(y, bottom, p.get(0, 1), &blank);
        vt.move_to(0, y);
    }
    true
}

/// DL: deletes lines at the cursor's row, blanks coming in at the
/// region's bottom; only inside the region. Goes to the first column.
pub(super) fn dl(vt: &mut Vt, p: &Params<'_>) -> bool {
    if in_region(vt) {
        let (y, bottom, blank) = (vt.cursor.y, vt.bottom, vt.blank());
        drop(vt.screen_mut().scroll_up(y, bottom, p.get(0, 1), &blank));
        vt.move_to(0, y);
    }
    true
}

/// Whether the cursor is inside the scroll region.
fn in_region(vt: &Vt) -> bool {
    (vt.top..=vt.bottom).contains(&vt.cursor.y)
}

/// SU: scrolls the region up.
pub(super) fn su(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.scroll_up(p.get(0, 1));
    true
}

/// SD: scrolls the region down.
pub(super) fn sd(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.scroll_down(p.get(0, 1));
    true
}

/// DECSTBM: sets the scroll region (rows from 1; missing means the
/// whole screen) and homes the cursor. A region of under two rows is
/// ignored, as xterm ignores it.
pub(super) fn decstbm(vt: &mut Vt, p: &Params<'_>) -> bool {
    let top = p.get(0, 1) - 1;
    let bottom = p.get(1, vt.rows).min(vt.rows) - 1;
    if top < bottom {
        (vt.top, vt.bottom) = (top, bottom);
        vt.move_to(0, 0);
    }
    true
}
