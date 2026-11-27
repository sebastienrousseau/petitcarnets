//! A writer that keeps every byte and counts flushes, and the
//! protocol that turns it into bytes per frame.

use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long to wait after the first frame shows up, so set-up writes
/// that follow it land in the first frame.
const SETTLE_FIRST: Duration = Duration::from_millis(150);

/// How long to wait after each later frame's flush for stray writes.
const SETTLE: Duration = Duration::from_millis(5);

/// The longest wait for a frame.
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Default)]
struct Record {
    bytes: Vec<u8>,
    flushes: usize,
}

/// Counts bytes written and flushes; clones share the count.
#[derive(Debug, Clone, Default)]
pub struct Capture(Arc<(Mutex<Record>, Condvar)>);

impl Capture {
    /// Bytes written and flushes so far.
    pub fn snapshot(&self) -> (usize, usize) {
        let r = self.0.0.lock().unwrap_or_else(PoisonError::into_inner);
        (r.bytes.len(), r.flushes)
    }

    /// Everything written so far.
    pub fn bytes(&self) -> Vec<u8> {
        self.0
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .bytes
            .clone()
    }

    /// Records `buf`.
    pub fn add(&self, buf: &[u8]) {
        self.0
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .bytes
            .extend_from_slice(buf);
    }

    /// Records a flush.
    pub fn flushed(&self) {
        self.0
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .flushes += 1;
        self.0.1.notify_all();
    }

    /// Waits until there have been at least `n` flushes; false on time
    /// out.
    pub fn wait_flushes(&self, n: usize) -> bool {
        let until = Instant::now() + TIMEOUT;
        let mut r = self.0.0.lock().unwrap_or_else(PoisonError::into_inner);
        while r.flushes < n {
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            r = self
                .0
                .1
                .wait_timeout(r, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        true
    }
}

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.add(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushed();
        Ok(())
    }
}

/// What one run measured.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// Bytes up to and including the first frame (set-up included).
    pub first: usize,
    /// Bytes of each later frame.
    pub frames: Vec<usize>,
    /// Flushes seen for each later frame (1 means one frame per step).
    pub flushes: Vec<usize>,
    /// Steps whose frame never arrived.
    pub timeouts: usize,
}

impl Stats {
    /// Mean bytes per frame, first frame excluded.
    pub fn mean(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let (sum, len) = (
            self.frames.iter().sum::<usize>() as f64,
            self.frames.len() as f64,
        );
        sum / len
    }
}

/// Waits for the first frame, then calls `step` `steps` times, waiting
/// for each step's frame to be flushed before the next, so no two steps
/// share a frame.
pub fn drive(cap: &Capture, steps: usize, step: &mut dyn FnMut()) -> Stats {
    let mut stats = Stats::default();
    if !cap.wait_flushes(1) {
        stats.timeouts += 1;
    }
    std::thread::sleep(SETTLE_FIRST);
    let (mut mark, mut flushes) = cap.snapshot();
    stats.first = mark;
    for _ in 0..steps {
        step();
        if !cap.wait_flushes(flushes + 1) {
            stats.timeouts += 1;
        }
        std::thread::sleep(SETTLE);
        let (bytes, f) = cap.snapshot();
        stats.frames.push(bytes - mark);
        stats.flushes.push(f - flushes);
        (mark, flushes) = (bytes, f);
    }
    stats
}

/// Where `--dump` writes each run's bytes, when set.
pub static DUMP: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Writes everything `cap` saw to `<dump>/<name>.out`, and the frame
/// sizes (first frame, then each later one) to `<name>.frames`.
pub fn dump(name: &str, cap: &Capture, stats: &Stats) {
    let Some(dir) = DUMP.get() else { return };
    let sizes: Vec<String> = std::iter::once(stats.first)
        .chain(stats.frames.iter().copied())
        .map(|n| n.to_string())
        .collect();
    std::fs::write(dir.join(format!("{name}.out")), cap.bytes()).expect("dump");
    std::fs::write(dir.join(format!("{name}.frames")), sizes.join("\n") + "\n").expect("dump");
}
