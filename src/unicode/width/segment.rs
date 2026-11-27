//! Grapheme cluster boundaries as `clipperhouse/uax29/v2` v2.7.0 finds
//! them (Unicode 17.0.0, Indic conjuncts included), for measuring text.
//! Input decoding splits with `rivo/uniseg` instead, in `grapheme.rs`.

/// Grapheme break property bits by code point. Each entry holds from its
/// code point up to the next entry's.
static PROPERTIES: [(u32, u32); 2168] = include!("../segment_table.in");

/// Property bits, numbered as uax29 numbers them.
const CR: u32 = 1;
const CONTROL: u32 = 1 << 1;
const EXTEND: u32 = 1 << 2;
const PICTOGRAPHIC: u32 = 1 << 3;
const CONSONANT: u32 = 1 << 4;
const CONJUNCT_EXTEND: u32 = 1 << 5;
const LINKER: u32 = 1 << 6;
const L: u32 = 1 << 7;
const LF: u32 = 1 << 8;
const LV: u32 = 1 << 9;
const LVT: u32 = 1 << 10;
const PREPEND: u32 = 1 << 11;
const REGIONAL: u32 = 1 << 12;
const SPACING_MARK: u32 = 1 << 13;
const T: u32 = 1 << 14;
const V: u32 = 1 << 15;
const ZWJ: u32 = 1 << 16;

/// Where an Indic conjunct (rule `GB9c`) stands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Conjunct {
    /// No consonant seen.
    #[default]
    None,
    /// A consonant, waiting for a linker.
    Consonant,
    /// A consonant and a linker: the next consonant joins.
    Linker,
}

/// What the rules remember of the code points already in the cluster.
#[derive(Debug, Default)]
struct Context {
    /// The last code point that was not Extend, and the one before it.
    last: u32,
    before_last: u32,
    /// Regional indicator pairs met so far.
    regional: usize,
    conjunct: Conjunct,
}

/// The property bits of `c`.
fn properties(c: char) -> u32 {
    let r = u32::from(c);
    let after = PROPERTIES.partition_point(|&(first, _)| first <= r);
    PROPERTIES[after - 1].1
}

/// Whether `bits` has any of `mask`.
const fn has(bits: u32, mask: u32) -> bool {
    bits & mask != 0
}

/// Rules `GB6`, `GB7` and `GB8`: Hangul syllable sequences.
const fn hangul(last: u32, current: u32) -> bool {
    has(current, L | V | LV | LVT) && has(last, L)
        || has(current, V | T) && has(last, LV | V)
        || has(current, T) && has(last, LVT | T)
}

/// Rules `GB3` to `GB9b`, which need nothing but the two code points: a
/// boundary (false), no boundary (true), or no answer.
const fn pair_rule(last: u32, current: u32) -> Option<bool> {
    if current | last == 0 {
        return Some(false);
    }
    if has(current, LF) && has(last, CR) {
        return Some(true);
    }
    if has(current | last, CONTROL | CR | LF) {
        return Some(false);
    }
    if hangul(last, current) || has(current, EXTEND | ZWJ | SPACING_MARK) || has(last, PREPEND) {
        return Some(true);
    }
    None
}

/// The conjunct state once `last` is part of the cluster.
fn conjunct_after(state: Conjunct, last: u32) -> Conjunct {
    if has(last, CONSONANT) {
        return state.max(Conjunct::Consonant);
    }
    if has(last, LINKER) && state >= Conjunct::Consonant {
        return Conjunct::Linker;
    }
    if has(last, LINKER | CONJUNCT_EXTEND) {
        return state;
    }
    Conjunct::None
}

impl Context {
    /// Takes in `last`, the code point now part of the cluster.
    fn push(&mut self, last: u32) {
        if !has(last, EXTEND) {
            self.before_last = self.last;
            self.last = last;
        }
        self.conjunct = conjunct_after(self.conjunct, last);
    }

    /// Whether `current` joins the cluster that ends with `last`.
    fn joins(&mut self, last: u32, current: u32) -> bool {
        if let Some(joins) = pair_rule(last, current) {
            return joins;
        }
        if self.conjunct == Conjunct::Linker && has(current, CONSONANT) {
            self.conjunct = Conjunct::Consonant;
            return true;
        }
        if has(current, PICTOGRAPHIC) && has(last, ZWJ) && has(self.before_last, PICTOGRAPHIC) {
            return true;
        }
        self.regional_pair(last, current)
    }

    /// Rules `GB12` and `GB13`: regional indicators join in pairs.
    fn regional_pair(&mut self, last: u32, current: u32) -> bool {
        if !has(current & last, REGIONAL) {
            return false;
        }
        self.regional += 1;
        self.regional % 2 == 1
    }
}

/// The length of the first grapheme cluster of `s`: uax29's
/// `Iterator.First`, ASCII shortcut included. Zero for an empty `s`.
pub(super) fn first_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let Some(&first) = bytes.first() else {
        return 0;
    };
    if first.is_ascii() && first != b'\r' && bytes.get(1).is_none_or(u8::is_ascii) {
        return 1;
    }
    split(s)
}

/// uax29's `splitFunc` for the first cluster of a non-empty `s`.
fn split(s: &str) -> usize {
    let mut chars = s.char_indices();
    let mut current = chars.next().map_or(0, |(_, c)| properties(c));
    let mut context = Context::default();
    for (at, c) in chars {
        let last = current;
        context.push(last);
        current = properties(c);
        if !context.joins(last, current) {
            return at;
        }
    }
    s.len()
}
