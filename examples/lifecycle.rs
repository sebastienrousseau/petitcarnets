//! How the terminal is given back, whatever ends the program: quit with
//! q, panic in `update` with p, panic in a task with t, or send the
//! process `SIGINT` or `SIGTERM`. Pasted text is printed after it quits.
//! The real-terminal tests drive it.
//!
//! `cargo run --example lifecycle`

use petitcarnets as tea;
use std::process::ExitCode;

#[derive(Default)]
struct Lifecycle {
    pastes: Vec<String>,
}

impl tea::Model for Lifecycle {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k == "p" => panic!("p pressed: a panic in update"),
            tea::Msg::KeyPress(k) if k == "t" => {
                tea::Cmd::task(|| panic!("t pressed: a panic in a task"))
            }
            tea::Msg::KeyPress(k) if k == "q" || k == "ctrl+c" => tea::quit(),
            tea::Msg::Paste(text) => {
                self.pastes.push(text);
                tea::Cmd::none()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        format!(
            "\n  pid {}\n\n  q quits, p panics in update, t panics in a task;\n  \
             or send SIGINT or SIGTERM",
            std::process::id()
        )
        .into()
    }
}

fn main() -> ExitCode {
    match tea::Program::new(Lifecycle::default()).run() {
        Ok(model) => {
            println!("quit");
            for text in model.pastes {
                println!("pasted: {text}");
            }
            ExitCode::SUCCESS
        }
        Err(tea::Error::Interrupted) => {
            eprintln!("interrupted");
            ExitCode::from(130)
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
