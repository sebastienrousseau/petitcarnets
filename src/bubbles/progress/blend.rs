//! Blending colours in CIE L*a*b*, as Lip Gloss's `Blend1D` does with
//! `go-colorful` v1.4.1's `BlendLab`, for the progress bar's gradient.

use crate::style::{Color, round_clamped, to_f64};

/// The D65 white point.
const D65: [f64; 3] = [0.95047, 1.00000, 1.08883];

/// Linear sRGB to XYZ, as `go-colorful` has it.
const TO_XYZ: [[f64; 3]; 3] = [
    [
        0.412_390_799_265_959_5,
        0.357_584_339_383_877_96,
        0.180_480_788_401_834_3,
    ],
    [
        0.212_639_005_871_510_36,
        0.715_168_678_767_755_9,
        0.072_192_315_360_733_71,
    ],
    [
        0.019_330_818_715_591_85,
        0.119_194_779_794_625_99,
        0.950_532_152_249_660_6,
    ],
];

/// XYZ to linear sRGB.
const TO_RGB: [[f64; 3]; 3] = [
    [
        3.240_969_941_904_521_3,
        -1.537_383_177_570_093_5,
        -0.498_610_760_293_003_3,
    ],
    [
        -0.969_243_636_280_879_8,
        1.875_967_501_507_720_6,
        0.041_555_057_407_175_61,
    ],
    [
        0.055_630_079_696_993_61,
        -0.203_976_958_888_976_57,
        1.056_971_514_242_878_6,
    ],
];

/// `steps` colours from `colors`, each segment between two stops the
/// same size (the first ones a step longer when it does not divide):
/// Lip Gloss's `Blend1D`. Fewer steps than stops give the first stops.
pub(super) fn blend_1d(steps: usize, colors: &[Color]) -> Vec<Color> {
    if steps <= colors.len() || colors.len() < 2 {
        let fill = colors.first().copied().unwrap_or(Color::BLACK);
        return (0..steps)
            .map(|i| colors.get(i).copied().unwrap_or(fill))
            .collect();
    }
    let labs: Vec<[f64; 3]> = colors.iter().map(|c| lab(channels(*c))).collect();
    let segments = labs.len() - 1;
    let mut out = Vec::with_capacity(steps);
    for (i, pair) in labs.windows(2).enumerate() {
        let size = steps / segments + usize::from(i < steps % segments);
        out.extend(segment(pair[0], pair[1], size));
    }
    out
}

/// `size` colours from `from` to `to`, both ends included.
fn segment(from: [f64; 3], to: [f64; 3], size: usize) -> impl Iterator<Item = Color> {
    let divisor = to_f64(size.saturating_sub(1));
    (0..size).map(move |j| {
        let t = if size > 1 { to_f64(j) / divisor } else { 0.0 };
        to_color(mix(from, to, t))
    })
}

/// A colour's channels from 0.0 to 1.0; only 24-bit colours blend, the
/// others are black.
fn channels(c: Color) -> [f64; 3] {
    match c {
        Color::Rgb(r, g, b) => [r, g, b].map(|v| f64::from(v) / 255.0),
        _ => [0.0; 3],
    }
}

/// `from` to `to` by `t`.
fn mix(from: [f64; 3], to: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| from[i] + t * (to[i] - from[i]))
}

/// `matrix` times `v`, summed left to right as Go writes it.
fn apply(matrix: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    matrix.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
}

/// sRGB to L*a*b*.
fn lab(rgb: [f64; 3]) -> [f64; 3] {
    let [x, y, z] = apply(&TO_XYZ, rgb.map(linearize));
    let fy = lab_f(y / D65[1]);
    [
        1.16 * fy - 0.16,
        5.0 * (lab_f(x / D65[0]) - fy),
        2.0 * (fy - lab_f(z / D65[2])),
    ]
}

/// L*a*b* to a 24-bit colour, clamped, rounded as Go's colour
/// conversion rounds it.
fn to_color([light, a, b]: [f64; 3]) -> Color {
    let l2 = (light + 0.16) / 1.16;
    let xyz = [
        D65[0] * lab_finv(l2 + a / 5.0),
        D65[1] * lab_finv(l2),
        D65[2] * lab_finv(l2 - b / 2.0),
    ];
    let [red, green, blue] = apply(&TO_RGB, xyz).map(|v| {
        let v = delinearize(v).clamp(0.0, 1.0);
        let wide = round_clamped((v * 65535.0 + 0.5).floor(), 65535);
        u8::try_from(wide >> 8).unwrap_or(u8::MAX)
    });
    Color::Rgb(red, green, blue)
}

/// sRGB to linear.
fn linearize(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear to sRGB.
fn delinearize(v: f64) -> f64 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// The L*a*b* function.
fn lab_f(t: f64) -> f64 {
    if t > 6.0 / 29.0 * 6.0 / 29.0 * 6.0 / 29.0 {
        t.cbrt()
    } else {
        t / 3.0 * 29.0 / 6.0 * 29.0 / 6.0 + 4.0 / 29.0
    }
}

/// Its inverse.
fn lab_finv(t: f64) -> f64 {
    if t > 6.0 / 29.0 {
        t * t * t
    } else {
        3.0 * 6.0 / 29.0 * 6.0 / 29.0 * (t - 4.0 / 29.0)
    }
}
