//! Control sequences: reading their parameters, finding the handler for
//! each, and the handlers that move the cursor.

use super::{Vt, edit, modes};

/// A control sequence's parts.
#[derive(Debug, Clone, Copy)]
pub(super) struct Params<'a> {
    /// The private marker (`?`, `>`, `<` or `=`), or 0 for none.
    pub(super) marker: u8,
    /// The parameters: digits, `;` and `:`.
    pub(super) numbers: &'a str,
    /// The intermediate bytes, such as a space or `$`.
    pub(super) inter: &'a str,
}

/// Acts on a sequence; returns whether it was understood.
type Handler = fn(&mut Vt, &Params<'_>) -> bool;

/// Each sequence understood: private marker (0 for none),
/// intermediates, final byte, and the handler.
const TABLE: [(u8, &str, u8, Handler); 38] = [
    (0, "", b'H', cup),
    (0, "", b'f', cup),
    (0, "", b'A', cuu),
    (0, "", b'B', cud),
    (0, "", b'C', cuf),
    (0, "", b'D', cub),
    (0, "", b'E', cnl),
    (0, "", b'F', cpl),
    (0, "", b'G', cha),
    (0, "", b'`', cha),
    (0, "", b'd', vpa),
    (0, "", b'K', edit::el),
    (0, "", b'J', edit::ed),
    (0, "", b'X', edit::ech),
    (0, "", b'@', edit::ich),
    (0, "", b'P', edit::dch),
    (0, "", b'L', edit::il),
    (0, "", b'M', edit::dl),
    (0, "", b'S', edit::su),
    (0, "", b'T', edit::sd),
    (0, "", b'r', edit::decstbm),
    (0, "", b'm', modes::sgr),
    (b'?', "", b'h', modes::decset),
    (b'?', "", b'l', modes::decrst),
    (0, " ", b'q', modes::decscusr),
    (0, "", b't', modes::xtwinops),
    (b'>', "", b'u', modes::accept),
    (b'<', "", b'u', modes::accept),
    (b'=', "", b'u', modes::accept),
    (b'?', "", b'u', modes::accept),
    (b'>', "", b'm', modes::modify_other_keys),
    (b'?', "$", b'p', modes::accept),
    (0, "$", b'p', modes::accept),
    (0, "", b'c', modes::accept),
    (b'>', "", b'c', modes::accept),
    (b'>', "", b'q', modes::accept),
    (0, "", b'n', modes::accept),
    (b'?', "", b'n', modes::accept),
];

/// Acts on `CSI params final_byte`, logging it if not understood.
pub(super) fn dispatch(vt: &mut Vt, raw: &str, final_byte: u8) {
    let understood = parse(raw).is_some_and(|p| handler(&p, final_byte).is_some_and(|h| h(vt, &p)));
    if !understood {
        vt.log(format!(
            "CSI {}{}",
            raw.escape_debug(),
            char::from(final_byte)
        ));
    }
}

/// The handler for a sequence with these parts and final byte.
fn handler(p: &Params<'_>, final_byte: u8) -> Option<Handler> {
    TABLE
        .iter()
        .find(|e| e.0 == p.marker && e.1 == p.inter && e.2 == final_byte)
        .map(|e| e.3)
}

/// Splits what lies between `CSI` and the final byte into its parts;
/// `None` if something else is mixed in.
fn parse(raw: &str) -> Option<Params<'_>> {
    let (marker, rest) = match raw.as_bytes().first() {
        Some(&m @ (b'?' | b'>' | b'<' | b'=')) => (m, &raw[1..]),
        _ => (0, raw),
    };
    let split = rest
        .find(|c: char| !(c.is_ascii_digit() || c == ';' || c == ':'))
        .unwrap_or(rest.len());
    let (numbers, inter) = rest.split_at(split);
    let plain = inter.bytes().all(|b| (0x20..=0x2F).contains(&b));
    plain.then_some(Params {
        marker,
        numbers,
        inter,
    })
}

impl Params<'_> {
    /// Each parameter's main value (missing reads as 0).
    pub(super) fn values(&self) -> impl Iterator<Item = u16> + '_ {
        self.numbers.split(';').map(|p| {
            let main = p.split(':').next().unwrap_or("");
            main.parse::<u32>()
                .map_or(0, |n| u16::try_from(n).unwrap_or(u16::MAX))
        })
    }

    /// Parameter `i`, or `default` when missing or 0.
    pub(super) fn get(&self, i: usize, default: u16) -> u16 {
        match self.values().nth(i) {
            Some(0) | None => default,
            Some(n) => n,
        }
    }
}

/// CUP, HVP: moves to row and column, from 1.
fn cup(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.move_to(p.get(1, 1) - 1, p.get(0, 1) - 1);
    true
}

/// CUU: up, stopping at the top margin when below it.
fn cuu(vt: &mut Vt, p: &Params<'_>) -> bool {
    let y = vt.cursor.y;
    let floor = if y >= vt.top { vt.top } else { 0 };
    vt.move_to(vt.cursor.x, y.saturating_sub(p.get(0, 1)).max(floor));
    true
}

/// CUD: down, stopping at the bottom margin when above it.
fn cud(vt: &mut Vt, p: &Params<'_>) -> bool {
    let y = vt.cursor.y;
    let ceiling = if y <= vt.bottom {
        vt.bottom
    } else {
        vt.rows - 1
    };
    vt.move_to(vt.cursor.x, y.saturating_add(p.get(0, 1)).min(ceiling));
    true
}

/// CUF: right.
fn cuf(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.move_to(vt.cursor.x.saturating_add(p.get(0, 1)), vt.cursor.y);
    true
}

/// CUB: left.
fn cub(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.move_to(vt.cursor.x.saturating_sub(p.get(0, 1)), vt.cursor.y);
    true
}

/// CNL: down, to the first column.
fn cnl(vt: &mut Vt, p: &Params<'_>) -> bool {
    cud(vt, p) && cha(vt, &Params { numbers: "", ..*p })
}

/// CPL: up, to the first column.
fn cpl(vt: &mut Vt, p: &Params<'_>) -> bool {
    cuu(vt, p) && cha(vt, &Params { numbers: "", ..*p })
}

/// CHA, HPA: to a column, from 1.
fn cha(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.move_to(p.get(0, 1) - 1, vt.cursor.y);
    true
}

/// VPA: to a row, from 1.
fn vpa(vt: &mut Vt, p: &Params<'_>) -> bool {
    vt.move_to(vt.cursor.x, p.get(0, 1) - 1);
    true
}
