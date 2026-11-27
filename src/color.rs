//! Colours a terminal reports, such as its background.

/// A 24-bit colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

impl Rgb {
    /// The colour `r`, `g`, `b`.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Whether the colour is dark: its HSL lightness, rounded to three
    /// places, is under one half. Bubble Tea's `IsDark`, used to pick
    /// colours that show on the terminal's background.
    #[must_use]
    pub fn is_dark(self) -> bool {
        let [r, g, b] = [self.r, self.g, self.b].map(|c| f64::from(c) / 255.0);
        let lightness = f64::midpoint(r.max(g).max(b), r.min(g).min(b));
        (lightness * 1000.0).round() / 1000.0 < 0.5
    }

    /// Parses a colour as X11 writes one in a terminal's reply, as
    /// Go's `ansi.XParseColor` does: `#rgb`, `#rrggbb`,
    /// `rgb:r/g/b` or `rgba:r/g/b/a`, each `rgb:` part being 1 to 4 hex
    /// digits.
    pub(crate) fn parse(s: &[u8]) -> Option<Self> {
        if let Some(hex) = s.strip_prefix(b"#") {
            return hash_colour(hex);
        }
        let (rest, parts) = match s.strip_prefix(b"rgb:") {
            Some(rest) => (rest, 3),
            None => (s.strip_prefix(b"rgba:")?, 4),
        };
        let values: Vec<&[u8]> = rest.split(|&b| b == b'/').collect();
        if values.len() != parts {
            return None;
        }
        let [r, g, b] = [0, 1, 2].map(|i| channel(values[i]));
        Some(Self { r, g, b })
    }
}

/// `#rgb` (each digit times 17) or `#rrggbb`. Go goes through floats,
/// `v / 15 * 65535 + 0.5`, which is exactly `v * 4369` (and `v * 257` for
/// two digits) for every digit value, so integers give the same bytes.
fn hash_colour(hex: &[u8]) -> Option<Rgb> {
    let (width, scale) = match hex.len() {
        3 => (1, 4369),
        6 => (2, 257),
        _ => return None,
    };
    if !hex.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut rgb = [0u8; 3];
    for (i, out) in rgb.iter_mut().enumerate() {
        let digits = &hex[i * width..(i + 1) * width];
        let value = digits
            .iter()
            .fold(0u64, |v, d| v * 16 + u64::from(hex_digit(*d)));
        *out = narrow(value * scale);
    }
    Some(Rgb::new(rgb[0], rgb[1], rgb[2]))
}

/// The value of an ASCII hex digit.
fn hex_digit(d: u8) -> u8 {
    char::from(d)
        .to_digit(16)
        .and_then(|v| u8::try_from(v).ok())
        .unwrap_or(0)
}

/// One `rgb:` channel: hex digits parsed as Go's `strconv.ParseUint`
/// with a 32-bit limit (an error is 0, an overflow the largest value),
/// then narrowed to a byte.
fn channel(digits: &[u8]) -> u8 {
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_hexdigit) {
        return 0;
    }
    let value = digits.iter().try_fold(0u64, |v, d| {
        let v = v * 16 + u64::from(hex_digit(*d));
        u32::try_from(v).is_ok().then_some(v)
    });
    narrow(value.unwrap_or(u64::from(u32::MAX)))
}

/// Go's `shift`: a value over a byte loses its low byte, then the result
/// is cut to a byte.
fn narrow(v: u64) -> u8 {
    let v = if v > 0xFF { v >> 8 } else { v };
    v.to_le_bytes()[0]
}
