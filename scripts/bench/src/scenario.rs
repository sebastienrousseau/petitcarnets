//! The five scenarios: the exact text each frame shows. The Go program
//! in `go/scenario.go` builds the same strings; both print a hash of
//! every frame's text so the two can be checked against each other.

/// Scenario names, in table order.
pub const NAMES: [&str; 5] = ["counter", "list", "log", "spinner", "full"];

/// Braille spinner frames, as in Bubbles' `spinner.MiniDot`.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Letters for the full-redraw scenario: even frames use the capitals,
/// odd frames the small letters, so every cell changes every frame.
const LETTERS: [&[u8; 26]; 2] = [b"ABCDEFGHIJKLMNOPQRSTUVWXYZ", b"abcdefghijklmnopqrstuvwxyz"];

/// The text of frame `n` (0 is the first frame) of `scenario`.
pub fn view(scenario: &str, n: usize) -> String {
    match scenario {
        "counter" => format!("Count: {n}\n\n+/- or arrows to change, q to quit"),
        "list" => list(n),
        "log" => (n..n + 24).map(log_line).collect::<Vec<_>>().join("\n"),
        "spinner" => format!("{} Loading forty-two items...", SPINNER[n % SPINNER.len()]),
        "full" => (0..24)
            .map(|r| full_row(r, n))
            .collect::<Vec<_>>()
            .join("\n"),
        other => panic!("unknown scenario {other}"),
    }
}

/// Twenty items; the selected one has a marker and bold cyan SGR.
fn list(n: usize) -> String {
    let selected = n % 20;
    let rows: Vec<String> = (0..20)
        .map(|i| {
            if i == selected {
                format!("\x1b[1;36m> item {i:02}\x1b[0m")
            } else {
                format!("  item {i:02}")
            }
        })
        .collect();
    rows.join("\n")
}

/// Log line number `k`.
fn log_line(k: usize) -> String {
    format!(
        "{k:06} GET /api/v1/items/{:<4} 200 {:>3} ms user={:04}",
        k * 7 % 1000,
        k * 37 % 997,
        k * 13 % 10000
    )
}

/// Row `r` of frame `n`: 79 letters chosen by a hash of column, row
/// and frame, so no row is a shifted copy of a row of the frame before
/// and nothing can be scrolled or shifted into place.
fn full_row(r: usize, n: usize) -> String {
    let letters = LETTERS[n % 2];
    (0..79)
        .map(|c| letters[(mix(c, r, n) % 26) as usize] as char)
        .collect()
}

/// A 32-bit hash of a cell's column, row and frame.
fn mix(c: usize, r: usize, n: usize) -> u32 {
    #[allow(clippy::cast_possible_truncation)]
    let [c, r, n] = [c as u32, r as u32, n as u32];
    let mut x = c
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(r.wrapping_mul(0x85eb_ca77))
        .wrapping_add(n.wrapping_mul(0xc2b2_ae3d));
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    x
}

/// FNV-1a over frames 0..=frames of `scenario`, each followed by a 0
/// byte.
pub fn hash(scenario: &str, frames: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for n in 0..=frames {
        for b in view(scenario, n).bytes().chain([0]) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}
