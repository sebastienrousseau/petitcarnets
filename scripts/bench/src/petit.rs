//! petitcarnets, driven through its public `Backend` trait: an 80x24
//! terminal whose input thread runs the measuring protocol.

use crate::capture::{Capture, Stats, drive};
use crate::scenario;
use petitcarnets as tea;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};

struct Bench {
    scenario: &'static str,
    n: usize,
    alt: bool,
}

impl tea::Model for Bench {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k == "q" => return tea::quit(),
            tea::Msg::KeyPress(k) if k == "j" => self.n += 1,
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        tea::View::new(scenario::view(self.scenario, self.n)).with_alt_screen(self.alt)
    }
}

/// An 80x24 terminal: output goes to a [`Capture`], input comes from
/// the protocol.
struct Backend {
    cap: Capture,
    steps: usize,
    stats: Arc<Mutex<Stats>>,
}

impl tea::Backend for Backend {
    fn enter(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn leave(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn size(&mut self) -> io::Result<tea::Size> {
        Ok(tea::Size::new(80, 24))
    }

    fn start_input(&mut self, sink: tea::InputSink) {
        let (cap, steps, stats) = (self.cap.clone(), self.steps, Arc::clone(&self.stats));
        drop(std::thread::spawn(move || {
            let key = |c| sink(tea::Input::KeyPress(tea::Key::char(c)));
            let s = drive(&cap, steps, &mut || {
                let _ = key('j');
            });
            *stats.lock().unwrap_or_else(PoisonError::into_inner) = s;
            let _ = key('q');
        }));
    }

    fn output(&mut self) -> &mut dyn Write {
        &mut self.cap
    }
}

/// Runs `scenario` for `steps` frames after the first; also returns the
/// bytes written after the last frame (the goodbye).
pub fn run(scenario: &'static str, alt: bool, steps: usize) -> (Stats, usize) {
    let stats = Arc::new(Mutex::new(Stats::default()));
    let cap = Capture::default();
    let backend = Backend {
        cap: cap.clone(),
        steps,
        stats: Arc::clone(&stats),
    };
    let model = Bench {
        scenario,
        n: 0,
        alt,
    };
    let end = tea::Program::new(model)
        .run_with(backend)
        .expect("petitcarnets run");
    assert_eq!(end.n, steps, "every step reached the model");
    let stats = stats.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let screen = if alt { "alt" } else { "inline" };
    crate::capture::dump(&format!("petitcarnets-{screen}-{scenario}"), &cap, &stats);
    let total = cap.snapshot().0;
    let measured = stats.first + stats.frames.iter().sum::<usize>();
    (stats, total - measured)
}
