//! The reader around the decoder: paste across reads, the escape
//! timeout on a simulated clock, and a deterministic fuzz run.

use super::*;
use crate::input::decode;
use crate::input::inputs;
use crate::key::{Key, KeyCode, KeyMod};

fn at(ms: u64) -> Instant {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *START.get_or_init(Instant::now) + Duration::from_millis(ms)
}

fn feed(reader: &mut Reader, bytes: &[u8], ms: u64) -> Vec<Input> {
    inputs(reader.feed(bytes, at(ms))).collect()
}

fn expire(reader: &mut Reader, ms: u64) -> Vec<Input> {
    inputs(reader.expire(at(ms))).collect()
}

fn press(code: KeyCode, mods: KeyMod) -> Input {
    Input::KeyPress(Key::new(code).with_mods(mods))
}

#[test]
fn a_paste_split_across_reads_arrives_whole() {
    let mut reader = Reader::default();
    assert_eq!(feed(&mut reader, b"\x1b[200~hel", 0), [Input::PasteStart]);
    assert_eq!(feed(&mut reader, b"lo\r\nwor", 1), []);
    assert_eq!(
        feed(&mut reader, b"ld\x1b[20", 2),
        [],
        "the end mark is cut off"
    );
    assert_eq!(
        feed(&mut reader, b"1~x", 3),
        [
            Input::Paste("hello\rworld".replace('\r', "\r\n")),
            Input::PasteEnd,
            Input::KeyPress(Key::char('x')),
        ]
    );
    assert_eq!(reader.deadline(), None);
}

#[test]
fn a_paste_keeps_escape_sequences_as_text_and_drops_bad_bytes() {
    let mut reader = Reader::default();
    let events = feed(&mut reader, b"\x1b[200~a\x1b[Ab\xffc\x1b[201~", 0);
    assert_eq!(
        events,
        [Input::PasteStart],
        "a bad byte might be cut off UTF-8"
    );
    let paste = Input::Paste("a\x1b[Abc".into());
    assert_eq!(expire(&mut reader, 50), [paste, Input::PasteEnd]);
}

#[test]
fn a_lone_escape_waits_for_the_timeout() {
    let mut reader = Reader::default();
    assert_eq!(feed(&mut reader, b"\x1b", 0), []);
    assert_eq!(reader.deadline(), Some(at(50)));
    assert_eq!(expire(&mut reader, 49), [], "too early");
    assert_eq!(expire(&mut reader, 50), [press(KeyCode::Esc, KeyMod::NONE)]);
    assert_eq!(reader.deadline(), None, "nothing left, no timer");
}

#[test]
fn an_escape_followed_in_time_starts_a_sequence() {
    let mut reader = Reader::default();
    assert_eq!(feed(&mut reader, b"\x1b", 0), []);
    assert_eq!(
        feed(&mut reader, b"[A", 10),
        [press(KeyCode::Up, KeyMod::NONE)]
    );
    assert_eq!(feed(&mut reader, b"\x1ba", 20), [], "alt+a might go on");
    assert_eq!(
        expire(&mut reader, 70),
        [press(KeyCode::Char('a'), KeyMod::ALT)]
    );
}

#[test]
fn a_cut_off_sequence_is_dropped_after_the_timeout() {
    let mut reader = Reader::default();
    assert_eq!(
        feed(&mut reader, b"x\x1b[1;", 0),
        [Input::KeyPress(Key::char('x'))]
    );
    assert_eq!(reader.deadline(), Some(at(50)));
    assert_eq!(
        feed(&mut reader, b"5", 30),
        [],
        "more bytes push the deadline"
    );
    assert_eq!(reader.deadline(), Some(at(80)));
    assert_eq!(expire(&mut reader, 80), [], "unknown, so not a key");
    assert_eq!(reader.deadline(), None);
    assert_eq!(
        feed(&mut reader, b"y", 90),
        [Input::KeyPress(Key::char('y'))]
    );
}

/// A xorshift generator, so the fuzz run is the same every time.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).unwrap()
    }

    fn byte(&mut self) -> u8 {
        self.next().to_le_bytes()[0]
    }
}

/// Bytes the decoder cares about, so random input hits its branches.
const INTERESTING: &[u8] =
    b"\x1b\x1b\x1b[[[]O_P^X;;;::<=>?$~^@umMRAtcy0123456789\x07\x9c\x9b\x8f\x18\xc3\xe2\xf0\x80";

fn random_input(rng: &mut XorShift, corpus: &[Vec<u8>]) -> Vec<u8> {
    let len = rng.below(48);
    match rng.below(3) {
        0 => (0..len).map(|_| rng.byte()).collect(),
        1 => (0..len)
            .map(|_| INTERESTING[rng.below(INTERESTING.len())])
            .collect(),
        _ => {
            let pick = corpus[rng.below(corpus.len())].clone();
            mutate(rng, pick)
        }
    }
}

/// A corpus input with a few bytes flipped, dropped, added or cut.
fn mutate(rng: &mut XorShift, mut bytes: Vec<u8>) -> Vec<u8> {
    for _ in 0..=rng.below(3) {
        let i = rng.below(bytes.len() + 1);
        match rng.below(4) {
            0 if i < bytes.len() => bytes[i] = rng.byte(),
            1 if i < bytes.len() => drop(bytes.remove(i)),
            2 => bytes.insert(i, INTERESTING[rng.below(INTERESTING.len())]),
            _ => bytes.truncate(i),
        }
    }
    bytes
}

/// Feeds `bytes` in random chunks, then flushes, checking every scan
/// uses no more than it has and every expired scan makes progress.
fn check(rng: &mut XorShift, bytes: &[u8]) {
    let mut scanner = Scanner::default();
    let (mut pending, mut events) = (Vec::new(), Vec::new());
    let mut rest = bytes;
    while !rest.is_empty() {
        let (chunk, tail) = rest.split_at(1 + rng.below(rest.len()));
        rest = tail;
        pending.extend_from_slice(chunk);
        let expired = rng.below(4) == 0;
        let used = scanner.scan(&pending, expired, &mut events);
        assert!(used <= pending.len(), "{bytes:02x?}");
        assert!(
            !expired || used > 0 || pending.is_empty(),
            "stuck on {bytes:02x?}"
        );
        let _ = pending.drain(..used);
    }
    for _ in 0..=bytes.len() {
        if pending.is_empty() {
            return;
        }
        let used = scanner.scan(&pending, true, &mut events);
        assert!(used > 0 && used <= pending.len(), "stuck on {bytes:02x?}");
        let _ = pending.drain(..used);
    }
    assert!(pending.is_empty(), "never drained {bytes:02x?}");
}

fn fuzz(runs: u64) {
    let corpus: Vec<Vec<u8>> = crate::input::tests::fixture()
        .into_iter()
        .map(|(chunks, _)| chunks.concat())
        .collect();
    let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
    for _ in 0..runs {
        let bytes = random_input(&mut rng, &corpus);
        if !bytes.is_empty() {
            let (n, _) = decode(&bytes);
            assert!(n >= 1 && n <= bytes.len(), "{bytes:02x?}");
        }
        check(&mut rng, &bytes);
    }
}

#[test]
fn fuzz_a_hundred_thousand_inputs() {
    fuzz(100_000);
}

/// Run with `cargo test --release --lib -- --ignored fuzz_ten_million`.
#[test]
#[ignore = "takes a while; CI runs it in release"]
fn fuzz_ten_million() {
    fuzz(10_000_000);
}
