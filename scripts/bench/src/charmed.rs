//! charmed-bubbletea 0.3.0: `Program::with_output` and `start` run it
//! headless, with messages injected through its `ProgramHandle`.

use crate::capture::{Capture, Stats, drive};
use crate::scenario;
use charmed::{Cmd, Message, Model, Program};

/// One step of the scenario.
struct Step;

struct Bench {
    scenario: &'static str,
    n: usize,
}

impl Model for Bench {
    fn init(&self) -> Option<Cmd> {
        None
    }

    fn update(&mut self, msg: Message) -> Option<Cmd> {
        if msg.is::<Step>() {
            self.n += 1;
        }
        None
    }

    fn view(&self) -> String {
        scenario::view(self.scenario, self.n)
    }
}

/// Runs `scenario` for `steps` frames after the first.
pub fn run(scenario: &'static str, alt: bool, steps: usize) -> Stats {
    let cap = Capture::default();
    let mut program = Program::new(Bench { scenario, n: 0 })
        .with_output(cap.clone())
        .without_signal_handler()
        .without_catch_panics();
    if alt {
        program = program.with_alt_screen();
    }
    let handle = program.start();
    let stats = drive(&cap, steps, &mut || {
        let _ = handle.send(Message::new(Step));
    });
    handle.quit();
    let end = handle.wait().expect("charmed-bubbletea run");
    assert_eq!(end.n, steps, "every step reached the model");
    let screen = if alt { "alt" } else { "inline" };
    crate::capture::dump(
        &format!("charmed-bubbletea-{screen}-{scenario}"),
        &cap,
        &stats,
    );
    stats
}
