//! Converting a 24-bit colour down to the 256 and 16 colour palettes, as
//! Go's x/ansi `Convert256` and `Convert16` do: the nearest colour in
//! xterm's 6 by 6 by 6 cube or its grey ramp, whichever is nearer in
//! `HSLuv` (go-colorful's `DistanceHSLuv`), then a fixed table down to 16.
//! The arithmetic follows Go's step by step, so the results match it.

use super::Rgb;

/// The RGB value of each 256-colour index, from x/ansi.
static PALETTE: [(u8, u8, u8); 256] = include!("palette.in");

/// The 16-colour index each 256-colour index becomes, from x/ansi.
static TO_16: [u8; 256] = include!("ansi16.in");

/// The cube's six levels.
const LEVELS: [i64; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

/// The RGB value of palette colour `n`.
pub(crate) fn palette(n: u8) -> Rgb {
    let (r, g, b) = PALETTE[usize::from(n)];
    Rgb::new(r, g, b)
}

/// The 16-colour index for palette colour `n`.
pub(crate) fn to_16(n: u8) -> u8 {
    TO_16[usize::from(n)]
}

/// Go's `to6Cube`: which of the cube's levels a channel is nearest.
fn to_cube(v: f64) -> usize {
    if v < 48.0 {
        0
    } else if v < 115.0 {
        1
    } else {
        usize::try_from(trunc((v - 35.0) / 40.0)).unwrap_or(5)
    }
}

/// A float from 0 to 765 truncated to an integer, as Go's `int(v)`.
fn trunc(v: f64) -> i64 {
    let (mut low, mut high) = (0, 766);
    while low + 1 < high {
        let mid = (low + high) / 2;
        if small(mid) <= v {
            low = mid;
        } else {
            high = mid;
        }
    }
    low
}

/// `n` (0 to 766) as a float.
fn small(n: i64) -> f64 {
    f64::from(i32::try_from(n).unwrap_or(0))
}

/// Go's `Convert256` for a 24-bit colour.
pub(crate) fn to_256(c: Rgb) -> u8 {
    let col = [c.r, c.g, c.b].map(|v| f64::from(u32::from(v) * 257) / 65535.0);
    let [r, g, b] = col.map(|v| v * 255.0);
    let (qr, qg, qb) = (to_cube(r), to_cube(g), to_cube(b));
    let cube = [LEVELS[qr], LEVELS[qg], LEVELS[qb]];
    let ci = 36 * qr + 6 * qg + qb;
    let index = |n: usize| u8::try_from(n).unwrap_or(u8::MAX);
    if cube == [trunc(r), trunc(g), trunc(b)] {
        return index(16 + ci);
    }
    let average = trunc(r + g + b) / 3;
    let grey_index = if average > 238 {
        23
    } else {
        (average - 3) / 10
    };
    let grey = small(8 + 10 * grey_index) / 255.0;
    let cube = cube.map(|v| small(v) / 255.0);
    let to_cube = distance(col, cube);
    let to_grey = distance(col, [grey; 3]);
    if to_cube <= to_grey {
        index(16 + ci)
    } else {
        index(232 + usize::try_from(grey_index).unwrap_or(0))
    }
}

/// go-colorful's `DistanceHSLuv`.
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (h1, s1, l1) = hsluv(a);
    let (h2, s2, l2) = hsluv(b);
    (sq((h1 - h2) / 100.0) + sq(s1 - s2) + sq(l1 - l2)).sqrt()
}

fn sq(v: f64) -> f64 {
    v * v
}

/// The white go-colorful's `HSLuv` uses.
const WHITE: [f64; 3] = [0.950_455_927_051_67, 1.0, 1.089_057_750_759_878];

/// An sRGB colour (channels 0 to 1) in `HSLuv`: hue, saturation and
/// lightness, by way of linear RGB, XYZ, CIE L*u*v* and `LCh`.
fn hsluv(srgb: [f64; 3]) -> (f64, f64, f64) {
    let (lightness, u, v) = luv(xyz(srgb.map(linearize)));
    let (lightness, chroma, hue) = lch(lightness, u, v);
    to_hsluv(lightness, chroma, hue)
}

/// go-colorful's `LinearRgbToXyz`.
fn xyz([r, g, b]: [f64; 3]) -> [f64; 3] {
    [
        0.412_390_799_265_959_5 * r + 0.357_584_339_383_877_96 * g + 0.180_480_788_401_834_3 * b,
        0.212_639_005_871_510_36 * r + 0.715_168_678_767_755_9 * g + 0.072_192_315_360_733_71 * b,
        0.019_330_818_715_591_85 * r + 0.119_194_779_794_625_99 * g + 0.950_532_152_249_660_6 * b,
    ]
}

/// go-colorful's `linearize`.
fn linearize(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// go-colorful's `XyzToLuvWhiteRef`, with [`WHITE`].
fn luv([x, y, z]: [f64; 3]) -> (f64, f64, f64) {
    let threshold = 6.0 / 29.0 * 6.0 / 29.0 * 6.0 / 29.0;
    let l = if y / WHITE[1] <= threshold {
        y / WHITE[1] * (29.0 / 3.0 * 29.0 / 3.0 * 29.0 / 3.0) / 100.0
    } else {
        1.16 * (y / WHITE[1]).cbrt() - 0.16
    };
    let (u_prime, v_prime) = uv(x, y, z);
    let (white_u, white_v) = uv(WHITE[0], WHITE[1], WHITE[2]);
    (
        l,
        13.0 * l * (u_prime - white_u),
        13.0 * l * (v_prime - white_v),
    )
}

/// go-colorful's `xyz_to_uv`.
fn uv(x: f64, y: f64, z: f64) -> (f64, f64) {
    let denom = x + 15.0 * y + 3.0 * z;
    if denom == 0.0 {
        (0.0, 0.0)
    } else {
        (4.0 * x / denom, 9.0 * y / denom)
    }
}

/// go-colorful's `LuvToLuvLCh`.
fn lch(l: f64, u: f64, v: f64) -> (f64, f64, f64) {
    let h = if (v - u).abs() > 1e-4 && u.abs() > 1e-4 {
        (57.295_779_513_082_32 * v.atan2(u) + 360.0) % 360.0
    } else {
        0.0
    };
    (l, (sq(u) + sq(v)).sqrt(), h)
}

/// go-colorful's `LuvLChToHSLuv`.
fn to_hsluv(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let (c, l) = (c * 100.0, l * 100.0);
    let s = if l > 99.999_999_9 || l < 0.000_000_01 {
        0.0
    } else {
        c / max_chroma(l, h) * 100.0
    };
    (h, (s / 100.0).clamp(0.0, 1.0), (l / 100.0).clamp(0.0, 1.0))
}

/// sRGB's matrix from XYZ, as `HSLuv`'s bounds use it.
const M: [[f64; 3]; 3] = [
    [
        3.240_969_941_904_521_4,
        -1.537_383_177_570_093_5,
        -0.498_610_760_293_003_3,
    ],
    [
        -0.969_243_636_280_879_8,
        1.875_967_501_507_720_7,
        0.041_555_057_407_175_61,
    ],
    [
        0.055_630_079_696_993_61,
        -0.203_976_958_888_976_57,
        1.056_971_514_242_878_6,
    ],
];

/// go-colorful's `maxChromaForLH`.
fn max_chroma(l: f64, h: f64) -> f64 {
    let theta = h / 360.0 * std::f64::consts::PI * 2.0;
    bounds(l)
        .iter()
        .map(|&(x, y)| y / (theta.sin() - x * theta.cos()))
        .filter(|&length| length > 0.0)
        .fold(f64::MAX, f64::min)
}

/// go-colorful's `getBounds`.
fn bounds(l: f64) -> [(f64, f64); 6] {
    const KAPPA: f64 = 903.296_296_296_296_3;
    const EPSILON: f64 = 0.008_856_451_679_035_631;
    let sub1 = (l + 16.0).powi(3) / 1_560_896.0;
    let sub2 = if sub1 > EPSILON { sub1 } else { l / KAPPA };
    let mut out = [(0.0, 0.0); 6];
    for (i, m) in M.iter().enumerate() {
        for (k, k_f) in [(0, 0.0), (1, 1.0)] {
            let top1 = (284_517.0 * m[0] - 94_839.0 * m[2]) * sub2;
            let top2 = (838_422.0 * m[2] + 769_860.0 * m[1] + 731_718.0 * m[0]) * l * sub2
                - 769_860.0 * k_f * l;
            let bottom = (632_260.0 * m[2] - 126_452.0 * m[1]) * sub2 + 126_452.0 * k_f;
            out[i * 2 + k] = (top1 / bottom, top2 / bottom);
        }
    }
    out
}
