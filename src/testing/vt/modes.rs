//! Control sequences that set the pen, modes, the cursor's style and the
//! title stack, and the screen switches modes 47, 1047 and 1049 make.

use super::csi::Params;
use super::grid::Grid;
use super::{Saved, Vt};

/// SGR: sets the pen; no parameters resets it.
pub(super) fn sgr(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.pen.apply_sgr(p.numbers);
    true
}

/// DECSET: sets each mode listed.
pub(super) fn decset(vt: &mut Vt, p: &Params<'_>) -> bool {
    set_modes(vt, p, true)
}

/// DECRST: resets each mode listed.
pub(super) fn decrst(vt: &mut Vt, p: &Params<'_>) -> bool {
    set_modes(vt, p, false)
}

/// Sets or resets each mode listed; an empty list is not understood.
fn set_modes(vt: &mut Vt, p: &Params<'_>, on: bool) -> bool {
    if p.numbers.is_empty() {
        return false;
    }
    for n in p.values() {
        vt.set_mode(n, on);
    }
    true
}

/// DECSCUSR: the cursor's style.
pub(super) fn decscusr(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.cursor_style = u8::try_from(p.get(0, 0)).unwrap_or(u8::MAX);
    true
}

/// XTWINOPS: pushes (22) or pops (23) the title; nothing else is
/// understood.
pub(super) fn xtwinops(vt: &mut Vt, p: &Params<'_>) -> bool {
    match p.get(0, 0) {
        22 => vt.titles.push(vt.title.clone()),
        23 => {
            if let Some(title) = vt.titles.pop() {
                vt.title = title;
            }
        }
        _ => return false,
    }
    true
}

/// modifyOtherKeys (`CSI > 4 ; n m`, `CSI > 4 m`): sets the level, 0
/// with none.
pub(super) fn modify_other_keys(vt: &mut Vt, p: &Params<'_>) -> bool {
    if p.get(0, 0) != 4 {
        return false;
    }
    vt.other_keys = u8::try_from(p.values().nth(1).unwrap_or(0)).unwrap_or(u8::MAX);
    true
}

/// The kitty keyboard flags: push (`CSI > flags u`), pop (`CSI < n u`)
/// and set (`CSI = flags ; mode u`: 1 replaces, 2 adds, 3 removes).
pub(super) fn kitty(vt: &mut Vt, p: &Params<'_>) -> bool {
    let flags = u8::try_from(p.values().next().unwrap_or(0)).unwrap_or(u8::MAX);
    match p.marker {
        b'>' => vt.keyboard.push(flags),
        b'<' => {
            let n = usize::from(p.get(0, 1)).min(vt.keyboard.len());
            vt.keyboard.truncate(vt.keyboard.len() - n);
        }
        _ => {
            let now = vt.keyboard_flags();
            let set = match p.get(1, 1) {
                2 => now | flags,
                3 => now & !flags,
                _ => flags,
            };
            let _ = vt.keyboard.pop();
            vt.keyboard.push(set);
        }
    }
    true
}

/// Sequences understood that change nothing: queries.
pub(super) fn accept(_: &mut Vt, _: &Params<'_>) -> bool {
    true
}

impl Vt {
    /// Sets or resets DEC private mode `n`, doing what it does.
    fn set_mode(&mut self, n: u16, on: bool) {
        match n {
            1049 => self.alt_with_cursor(on),
            47 | 1047 => self.switch_screen(on, n == 1047),
            2026 if !on && self.mode(2026) => self.sync_frames += 1,
            _ => {}
        }
        if on {
            let _ = self.modes.insert(n);
        } else {
            let _ = self.modes.remove(&n);
        }
    }

    /// Mode 1049: saves the cursor and shows a cleared alternate screen,
    /// or shows the main screen again and restores the cursor.
    fn alt_with_cursor(&mut self, on: bool) {
        if on == self.on_alt {
            return;
        }
        if on {
            self.saved_alt = self.save();
            self.switch_screen(true, true);
        } else {
            self.switch_screen(false, false);
            self.restore(self.saved_alt);
        }
    }

    /// Shows the alternate screen (cleared first if `clear`) or the main
    /// one.
    fn switch_screen(&mut self, alt: bool, clear: bool) {
        if alt && !self.on_alt && clear {
            self.alt = Grid::new(self.columns, self.rows, &self.blank());
        }
        self.on_alt = alt;
        self.cursor.wrap = false;
    }

    /// The cursor's place and pen.
    pub(super) fn save(&self) -> Saved {
        Saved {
            x: self.cursor.x,
            y: self.cursor.y,
            pen: self.pen,
        }
    }

    /// Puts back a saved place and pen.
    pub(super) fn restore(&mut self, saved: Saved) {
        self.move_to(saved.x, saved.y);
        self.pen = saved.pen;
    }
}
