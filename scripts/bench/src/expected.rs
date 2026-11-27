//! The bytes petitcarnets writes, checked against committed values:
//! `--check FILE` fails when any differ, `--bless FILE` writes them.
//!
//! Each line of FILE is `screen scenario first total`, tab separated:
//! the bytes up to and including the first frame, and the bytes of the
//! frames after it, together. Lines starting with `#` are comments.

use crate::capture::Stats;
use std::fmt::Write as _;

/// What one run must show to count: one flush per step and no timeouts.
fn sound(stats: &Stats) -> Result<(), String> {
    let single = stats.flushes.iter().all(|&f| f == 1);
    if stats.timeouts == 0 && single {
        Ok(())
    } else {
        Err(format!(
            "unsound run: {} timeouts, flushes per step {:?}",
            stats.timeouts, stats.flushes
        ))
    }
}

/// One line of the file for a run.
fn line(screen: &str, scenario: &str, stats: &Stats) -> String {
    let total: usize = stats.frames.iter().sum();
    format!("{screen}\t{scenario}\t{}\t{total}", stats.first)
}

/// The file's text for `runs`, each `(screen, scenario, stats)`.
pub fn render(frames: usize, runs: &[(&str, &str, Stats)]) -> Result<String, String> {
    let mut out = format!(
        "# Bytes petitcarnets writes, from `cargo run --release --no-default-features -- --bless expected.tsv`.\n\
         # screen\tscenario\tfirst frame\tthe {frames} frames after it\n"
    );
    for (screen, scenario, stats) in runs {
        sound(stats).map_err(|e| format!("{screen} {scenario}: {e}"))?;
        let _ = writeln!(out, "{}", line(screen, scenario, stats));
    }
    Ok(out)
}

/// Every difference between `expected` (the file's text) and `actual`
/// (what [`render`] made), line by line, comments left out.
pub fn compare(expected: &str, actual: &str) -> Vec<String> {
    let data = |text: &str| -> Vec<String> {
        text.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(str::to_string)
            .collect()
    };
    let (want, got) = (data(expected), data(actual));
    let mut problems: Vec<String> = want
        .iter()
        .filter(|w| !got.contains(w))
        .map(|w| {
            let key: Vec<&str> = w.split('\t').take(2).collect();
            let now = got
                .iter()
                .find(|g| g.split('\t').take(2).eq(key.iter().copied()))
                .map_or("missing", String::as_str);
            format!("expected {w:?}, got {now:?}")
        })
        .collect();
    problems.extend(
        got.iter()
            .filter(|g| {
                !want
                    .iter()
                    .any(|w| w.split('\t').take(2).eq(g.split('\t').take(2)))
            })
            .map(|g| format!("not expected: {g:?}")),
    );
    problems
}
