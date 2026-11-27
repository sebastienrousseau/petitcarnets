//! Shows key presses, repeats and releases, which terminals that speak
//! the kitty keyboard protocol report (kitty, foot, `WezTerm`, Ghostty
//! and others). Bubble Tea's `keyboard-enhancements` example.
//!
//! `cargo run --example keyboard-enhancements`

use petitcarnets as tea;
use tea::{Key, KeyboardEnhancements, Msg};

#[derive(Default)]
pub(crate) struct Keys {
    supported: Option<String>,
    pub(crate) events: Vec<String>,
}

/// A key as `"A" (shift+a)`: what it prints as, then the keystroke.
fn describe(key: &Key) -> String {
    let repeat = if key.is_repeat { ", repeat" } else { "" };
    format!("{:?} ({}{repeat})", key.to_string(), key.keystroke())
}

impl tea::Model for Keys {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        let line = match msg {
            Msg::KeyPress(key) if key == "ctrl+c" || key == "esc" => return tea::quit(),
            Msg::KeyPress(key) => format!("press   {}", describe(&key)),
            Msg::KeyRelease(key) => format!("release {}", describe(&key)),
            Msg::KeyboardEnhancements(report) => {
                self.supported = Some(if report.supports_event_types() {
                    "this terminal reports releases".to_string()
                } else {
                    "this terminal does not report releases".to_string()
                });
                return tea::Cmd::none();
            }
            _ => return tea::Cmd::none(),
        };
        self.events.push(line);
        let excess = self.events.len().saturating_sub(12);
        drop(self.events.drain(..excess));
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let support = self.supported.as_deref().unwrap_or("press a key");
        let content = format!(
            "\n  Press and release keys; esc or ctrl+c quits ({support}).\n\n  {}",
            self.events.join("\n  ")
        );
        tea::View::new(content)
            .with_keyboard_enhancements(KeyboardEnhancements::new().report_event_types())
    }
}

// `pub(crate)`: `tests/golden.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let keys = tea::Program::new(Keys::default()).run()?;
    println!("{} key events", keys.events.len());
    Ok(())
}
