//! Sending activity to a program in real time through a channel:
//! Bubble Tea's `realtime` example. A thread sends an event at random
//! intervals of 100 to 1000 milliseconds; a task waits for each on the
//! channel, and the model asks for the next wait when one arrives. (Go
//! starts the sender as a command that never returns; here it is a
//! thread of its own, started by `main`.)
//!
//! `cargo run --example realtime`

use petitcarnets as tea;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tea::bubbles::spinner;

pub(crate) enum Msg {
    /// Activity happened. In a real program (a chat, say), it would carry
    /// data.
    Response,
    /// The channel is closed: no more activity will come.
    Closed,
    Spinner(spinner::TickMsg),
}

pub(crate) struct Model {
    /// Where activity arrives.
    pub(crate) sub: Arc<Mutex<Receiver<()>>>,
    /// How many responses arrived.
    pub(crate) responses: u64,
    pub(crate) spinner: spinner::Model,
    pub(crate) quitting: bool,
}

impl Model {
    pub(crate) fn new(sub: Receiver<()>) -> Self {
        Self {
            sub: Arc::new(Mutex::new(sub)),
            responses: 0,
            spinner: spinner::Model::new(),
            quitting: false,
        }
    }
}

/// A command that waits for activity on the channel.
fn wait_for_activity(sub: &Arc<Mutex<Receiver<()>>>) -> tea::Cmd<Msg> {
    let sub = Arc::clone(sub);
    tea::Cmd::task(move || {
        let received = sub.lock().map(|rx| rx.recv());
        match received {
            Ok(Ok(())) => Msg::Response,
            _ => Msg::Closed,
        }
    })
}

impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        tea::batch([
            self.spinner.tick().map(Msg::Spinner),
            wait_for_activity(&self.sub),
        ])
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(_) => {
                self.quitting = true;
                tea::quit()
            }
            tea::Msg::App(Msg::Response) => {
                self.responses += 1; // record external activity
                wait_for_activity(&self.sub) // wait for the next event
            }
            tea::Msg::App(Msg::Spinner(tick)) => {
                self.spinner.update(&tea::Msg::App(tick)).map(Msg::Spinner)
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        let mut s = format!(
            "\n {} Events received: {}\n\n Press any key to exit\n",
            self.spinner.view(),
            self.responses
        );
        if self.quitting {
            s.push('\n');
        }
        tea::View::new(s)
    }
}

// `pub(crate)`: `tests/bubbles_examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let (tx, rx) = mpsc::channel();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut seed = u64::from(nanos.subsec_nanos()) | 1;
    // Simulate a process that sends events at an irregular interval.
    let _ = std::thread::spawn(move || {
        loop {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            std::thread::sleep(Duration::from_millis(seed % 900 + 100));
            if tx.send(()).is_err() {
                return;
            }
        }
    });
    let _ = tea::Program::new(Model::new(rx)).run()?;
    Ok(())
}
