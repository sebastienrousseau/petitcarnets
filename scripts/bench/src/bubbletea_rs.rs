//! bubbletea-rs 0.0.9: `ProgramBuilder::output` takes a tokio
//! `AsyncWrite`, so frames can be captured, but `Program::run` always
//! calls crossterm's `enable_raw_mode` (it needs a terminal: run under
//! `script` when there is none) and writes its set-up sequences (alt
//! screen, cursor) to the process's stdout, not to the writer.

use crate::capture::{Capture, Stats, drive};
use crate::scenario;
use bubbletea_rs::{Cmd, Model, Msg, Program};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

/// The scenario for the next model: `Model::init` takes no arguments.
static SCENARIO: Mutex<&str> = Mutex::new("counter");

/// One step of the scenario.
struct Step;

/// A nudge: the first frame is drawn after the first message.
struct Start;

struct Bench {
    scenario: &'static str,
    n: usize,
}

impl Model for Bench {
    fn init() -> (Self, Option<Cmd>) {
        let scenario = *SCENARIO.lock().expect("scenario");
        (Self { scenario, n: 0 }, None)
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if msg.is::<Step>() {
            self.n += 1;
        }
        None
    }

    fn view(&self) -> String {
        scenario::view(self.scenario, self.n)
    }
}

/// [`Capture`] as a tokio writer.
struct Async(Capture);

impl AsyncWrite for Async {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.0.add(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.0.flushed();
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Runs `scenario` for `steps` frames after the first, or says why it
/// could not.
pub fn run(scenario: &'static str, alt: bool, steps: usize) -> Result<Stats, String> {
    if !has_terminal() {
        return Err("no terminal for crossterm's enable_raw_mode; run under `script`".into());
    }
    *SCENARIO.lock().expect("scenario") = scenario;
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let cap = Capture::default();
    let program = Program::<Bench>::builder()
        .alt_screen(alt)
        .signal_handler(false)
        .catch_panics(false)
        .input(tokio::io::empty())
        .output(Async(cap.clone()))
        .build()
        .map_err(|e| e.to_string())?;
    let sender = program.sender();
    let task = rt.spawn(program.run());
    let _ = sender.send(Box::new(Start));
    let stats = drive(&cap, steps, &mut || {
        let _ = sender.send(Box::new(Step));
    });
    let _ = sender.send(Box::new(bubbletea_rs::QuitMsg));
    let end = rt
        .block_on(task)
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    assert_eq!(end.n, steps, "every step reached the model");
    let screen = if alt { "alt" } else { "inline" };
    crate::capture::dump(&format!("bubbletea-rs-{screen}-{scenario}"), &cap, &stats);
    Ok(stats)
}

/// Whether crossterm's `enable_raw_mode` can find a terminal: stdin, or
/// else `/dev/tty`.
fn has_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() || std::fs::File::open("/dev/tty").is_ok()
}
