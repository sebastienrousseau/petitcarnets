//! Control sequence parameters, packed as Go's `ansi` package packs
//! them: a 64-bit integer per parameter, wrapping on overflow, whose low
//! 31 bits are the value (all ones when the parameter is missing) and
//! whose upper bits mark a parameter followed by a `:` sub-parameter.

/// The upper bits: Go's `HasMoreFlag` (`math.MinInt32`, sign extended).
const HAS_MORE: i64 = -0x8000_0000;
/// The value bits: Go's `ParamMask`.
const MASK: i64 = !HAS_MORE;
/// A missing parameter: Go's `MissingParam`.
const MISSING: i64 = MASK;
/// Room for parameters in a control sequence (Go's `MaxParamsSize`).
pub(super) const CSI_MAX: usize = 32;
/// Room for parameters in a device control string.
pub(super) const DCS_MAX: usize = 16;

/// One packed parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Param(i64);

impl Param {
    /// The value, or `default` when it is missing. Go's `Param.Param`.
    pub(super) const fn get(self, default: i64) -> i64 {
        let value = self.0 & MASK;
        if value == MISSING { default } else { value }
    }

    /// Whether a `:` sub-parameter follows. Go's `Param.HasMore`.
    pub(super) const fn has_more(self) -> bool {
        self.0 & HAS_MORE != 0
    }
}

/// The parameters of one sequence.
#[derive(Debug, Clone)]
pub(super) struct Params {
    values: [i64; CSI_MAX],
    len: usize,
}

impl Params {
    /// How many there are.
    pub(super) const fn len(&self) -> usize {
        self.len
    }

    /// Parameter `i`, which must exist.
    pub(super) const fn at(&self, i: usize) -> Param {
        Param(self.values[i])
    }

    /// Parameter `i` or `default`, and whether it exists. Go's
    /// `Params.Param`, without the has-more flag.
    pub(super) const fn param(&self, i: usize, default: i64) -> (i64, bool) {
        if i < self.len {
            (Param(self.values[i]).get(default), true)
        } else {
            (default, false)
        }
    }

    /// Every parameter in order.
    pub(super) fn iter(&self) -> impl Iterator<Item = Param> + '_ {
        self.values[..self.len].iter().map(|&v| Param(v))
    }
}

/// Reads parameter bytes (`0x30..=0x3F`) from `b[i..]` as Go's
/// `parseCsi` and `parseDcs` do, keeping at most `max`. The first
/// parameter starts at 0, not missing; digits concatenate across any
/// `<=>?` bytes. Returns the parameters and where reading stopped.
pub(super) fn scan(b: &[u8], mut i: usize, max: usize) -> (Params, usize) {
    let mut p = Params {
        values: [0; CSI_MAX],
        len: 0,
    };
    let start = i;
    while i < b.len() && p.len < max && (0x30..=0x3F).contains(&b[i]) {
        p.push(b[i], max);
        i += 1;
    }
    if i > start && p.len < max {
        p.len += 1;
    }
    (p, i)
}

impl Params {
    /// Reads one parameter byte: a digit, or `;` or `:` starting the next
    /// parameter (`:` marking this one as followed by a sub-parameter).
    fn push(&mut self, byte: u8, max: usize) {
        if byte.is_ascii_digit() {
            push_digit(&mut self.values[self.len], byte);
        }
        if byte == b':' {
            self.values[self.len] |= HAS_MORE;
        }
        if byte == b';' || byte == b':' {
            self.len += 1;
            if self.len < max {
                self.values[self.len] = MISSING;
            }
        }
    }
}

/// Appends a decimal digit to a packed value, as Go's wrapping `int`
/// arithmetic does.
fn push_digit(value: &mut i64, digit: u8) {
    if *value == MISSING {
        *value = 0;
    }
    *value = value.wrapping_mul(10).wrapping_add(i64::from(digit - b'0'));
}

/// Skips intermediate bytes (`0x20..=0x2F`) from `b[i..]`: the last one
/// read, or 0, and where reading stopped.
pub(super) fn intermediate(b: &[u8], mut i: usize) -> (u8, usize) {
    let mut last = 0;
    while i < b.len() && (0x20..=0x2F).contains(&b[i]) {
        last = b[i];
        i += 1;
    }
    (last, i)
}

/// The prefix byte (`<`, `=`, `>` or `?`) at `b[i]`, or 0. It is not
/// skipped: the parameter scan reads it as a parameter byte, as in Go.
pub(super) fn prefix(b: &[u8], i: usize) -> u8 {
    b.get(i)
        .copied()
        .filter(|c| (b'<'..=b'?').contains(c))
        .unwrap_or(0)
}
