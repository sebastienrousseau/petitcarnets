//! Golden files: what a test expects, kept next to it and rewritten on
//! demand, as Go's `golden.RequireEqual` does.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The environment variable that rewrites golden files instead of
/// comparing with them, when set to `1`.
pub const UPDATE_GOLDEN: &str = "PETITCARNETS_UPDATE_GOLDEN";

/// Where the golden file called `name` lives:
/// `tests/golden/<name>.golden` under the package being tested (Cargo's
/// `CARGO_MANIFEST_DIR`, else the current directory, which is the same
/// under `cargo test`).
#[must_use]
pub fn golden_path(name: &str) -> PathBuf {
    let root = std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(PathBuf::new, PathBuf::from);
    root.join("tests")
        .join("golden")
        .join(format!("{name}.golden"))
}

/// Checks that `actual` is what the golden file called `name` holds
/// (see [`golden_path`]); with `PETITCARNETS_UPDATE_GOLDEN=1`, writes it
/// there instead. Line endings in the file are read as `\n`, so a
/// checkout that turns them into `\r\n` still matches.
///
/// ```no_run
/// use petitcarnets::testing::assert_golden;
///
/// assert_golden("greeting", "hello\n");
/// ```
///
/// # Panics
///
/// If the file differs from `actual` or cannot be read, or, when
/// updating, cannot be written.
pub fn assert_golden(name: &str, actual: &str) {
    let update = std::env::var_os(UPDATE_GOLDEN).is_some_and(|v| v == "1");
    check(&golden_path(name), actual, update);
}

/// Checks `actual` against the file at `path`, or writes it there if
/// `update`.
fn check(path: &Path, actual: &str, update: bool) {
    if update {
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(path, actual));
        if let Err(err) = written {
            panic!("cannot write {}: {err}", path.display());
        }
        return;
    }
    let expected = match std::fs::read_to_string(path) {
        Ok(text) => text.replace("\r\n", "\n"),
        Err(err) => panic!(
            "cannot read {}: {err}; run with {UPDATE_GOLDEN}=1 to write it",
            path.display()
        ),
    };
    if let Some(report) = difference(&expected, actual) {
        panic!(
            "{} differs from the actual output; run with {UPDATE_GOLDEN}=1 \
             to accept it\n{report}",
            path.display()
        );
    }
}

/// Where `actual` first differs from `expected`, line by line, and the
/// whole of `actual`; `None` if they are the same.
fn difference(expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    let mut want = expected.split('\n');
    let mut got = actual.split('\n');
    let mut line = 1;
    let (want, got) = loop {
        match (want.next(), got.next()) {
            (w, g) if w == g => line += 1,
            other => break other,
        }
    };
    let mut report = format!("line {line}:\n");
    let _ = writeln!(report, "  expected: {:?}", want.unwrap_or("<end>"));
    let _ = writeln!(report, "  actual:   {:?}", got.unwrap_or("<end>"));
    let _ = write!(report, "actual output:\n{actual}");
    Some(report)
}

/// `bytes` as readable text for a golden file: escape sequences and
/// other control characters spelled out (`\u{1b}` for ESC), with a new
/// line after each `\n` written.
///
/// ```
/// use petitcarnets::testing::escape;
///
/// assert_eq!(escape(b"\x1b[1mhi\r\n"), "\\u{1b}[1mhi\\r\\n\n");
/// ```
#[must_use]
pub fn escape(bytes: &[u8]) -> String {
    let mut out = String::new();
    for c in String::from_utf8_lossy(bytes).chars() {
        match c {
            '\n' => out.push_str("\\n\n"),
            '"' | '\'' => out.push(c),
            _ => out.extend(c.escape_debug()),
        }
    }
    out
}

#[cfg(test)]
mod tests;
