//! Bytes written per frame by petitcarnets, and by the other Rust ports
//! of Bubble Tea, for five scenarios on an 80x24 screen.
//!
//! `cargo run --release [-- --frames N] [--only NAME] [--dump DIR]`,
//! NAME one of `petitcarnets`, `charmed-bubbletea`, `bubbletea-rs`;
//! `--dump` keeps each run's bytes in DIR.
//!
//! `--check FILE` runs petitcarnets alone and fails if its bytes differ
//! from FILE (`expected.tsv`); `--bless FILE` writes them there.

#[cfg(feature = "others")]
mod bubbletea_rs;
mod capture;
#[cfg(feature = "others")]
mod charmed;
mod expected;
mod petit;
mod scenario;

use capture::Stats;

/// What to do with petitcarnets' bytes besides printing them.
enum Expect {
    /// Print the table only.
    Print,
    /// Fail if they differ from this file.
    Check(std::path::PathBuf),
    /// Write them to this file.
    Bless(std::path::PathBuf),
}

/// Command-line options.
struct Options {
    frames: usize,
    only: Option<String>,
    expect: Expect,
}

fn options() -> Options {
    let mut o = Options {
        frames: 100,
        only: None,
        expect: Expect::Print,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--frames" => {
                o.frames = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .expect("--frames N");
            }
            "--only" => o.only = args.next(),
            "--check" => o.expect = Expect::Check(args.next().expect("--check FILE").into()),
            "--bless" => o.expect = Expect::Bless(args.next().expect("--bless FILE").into()),
            "--dump" => {
                let dir = std::path::PathBuf::from(args.next().expect("--dump DIR"));
                std::fs::create_dir_all(&dir).expect("--dump DIR");
                let _ = capture::DUMP.set(dir);
            }
            other => panic!("unknown argument {other}"),
        }
    }
    o
}

/// One table row, or why there is none.
fn row(name: &str, mode: &str, scenario: &str, stats: Result<Stats, String>) -> String {
    let s = match stats {
        Ok(s) => s,
        Err(why) => {
            return format!("| {name} | {mode} | {scenario} | - | - | - | - | not run: {why} |");
        }
    };
    let (min, max) = (
        s.frames.iter().min().copied().unwrap_or(0),
        s.frames.iter().max().copied().unwrap_or(0),
    );
    let single = s.flushes.iter().filter(|&&f| f == 1).count();
    format!(
        "| {name} | {mode} | {scenario} | {} | {:.1} | {min} | {max} | {single}/{} one flush, {} timeouts |",
        s.first,
        s.mean(),
        s.frames.len(),
        s.timeouts
    )
}

fn wanted(o: &Options, name: &str) -> bool {
    o.only.as_deref().is_none_or(|only| only == name)
}

/// Runs petitcarnets on every scenario and screen, and checks or
/// writes the bytes; the process's exit code.
fn expect(o: &Options) -> std::process::ExitCode {
    let mut runs = Vec::new();
    for (screen, alt) in [("inline", false), ("alt", true)] {
        for scenario in scenario::NAMES {
            runs.push((screen, scenario, petit::run(scenario, alt, o.frames).0));
        }
    }
    let actual = match expected::render(o.frames, &runs) {
        Ok(text) => text,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match &o.expect {
        Expect::Bless(path) => {
            std::fs::write(path, &actual).expect("--bless FILE");
            println!("wrote {}", path.display());
        }
        Expect::Check(path) => {
            let want = std::fs::read_to_string(path).expect("--check FILE");
            let problems = expected::compare(&want, &actual);
            for p in &problems {
                eprintln!("FAIL {p}");
            }
            if !problems.is_empty() {
                eprintln!(
                    "petitcarnets writes different bytes per frame; if that is meant, \
                     run with --bless {} and update docs/benchmarks.md",
                    path.display()
                );
                return std::process::ExitCode::FAILURE;
            }
            println!("bytes per frame as expected ({} runs)", runs.len());
        }
        Expect::Print => {}
    }
    std::process::ExitCode::SUCCESS
}

fn main() -> std::process::ExitCode {
    let o = options();
    if !matches!(o.expect, Expect::Print) {
        return expect(&o);
    }
    let mut rows = Vec::new();
    let mut goodbyes = Vec::new();
    for (mode, alt) in [("inline", false), ("alt", true)] {
        for scenario in scenario::NAMES {
            if wanted(&o, "petitcarnets") {
                let (stats, goodbye) = petit::run(scenario, alt, o.frames);
                goodbyes.push(format!("petitcarnets {mode} {scenario}: {goodbye}"));
                rows.push(row("petitcarnets", mode, scenario, Ok(stats)));
            }
            #[cfg(feature = "others")]
            others(&o, mode, alt, scenario, &mut rows);
        }
    }
    println!("frames after the first: {}", o.frames);
    println!("\nframe text hashes (FNV-1a, frames 0..={}):", o.frames);
    for scenario in scenario::NAMES {
        println!("  {scenario}: {:016x}", scenario::hash(scenario, o.frames));
    }
    println!(
        "\n| implementation | screen | scenario | first frame bytes | mean bytes/frame | min | max | frames |"
    );
    println!("|---|---|---|---:|---:|---:|---:|---|");
    for r in rows {
        println!("{r}");
    }
    println!(
        "\nbytes after the last frame (teardown): {}",
        goodbyes.join(", ")
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(feature = "others")]
fn others(o: &Options, mode: &str, alt: bool, scenario: &'static str, rows: &mut Vec<String>) {
    if wanted(o, "charmed-bubbletea") {
        rows.push(row(
            "charmed-bubbletea",
            mode,
            scenario,
            Ok(charmed::run(scenario, alt, o.frames)),
        ));
    }
    if wanted(o, "bubbletea-rs") {
        rows.push(row(
            "bubbletea-rs",
            mode,
            scenario,
            bubbletea_rs::run(scenario, alt, o.frames),
        ));
    }
}
