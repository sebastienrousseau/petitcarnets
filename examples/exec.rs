//! Runs another program in the terminal, then carries on: Bubble Tea's
//! `exec` example. Press e to open `$EDITOR` (vim if it is not set), a to
//! toggle the alternate screen, q to quit. Mouse reports are on, to show
//! that every mode comes back after the editor; keys typed after it are
//! listed when the program ends.
//!
//! `cargo run --example exec`, or give the command to run instead:
//! `cargo run --example exec -- sh -c 'read line; echo "got $line"'`

use petitcarnets as tea;
use std::io;
use std::process::{Command, ExitCode, ExitStatus};

enum Msg {
    EditorFinished(io::Result<ExitStatus>),
}

#[derive(Default)]
struct Model {
    alt_screen: bool,
    error: Option<String>,
    runs: u32,
    keys: Vec<String>,
}

/// The command from the arguments, else `$EDITOR`, else vim.
fn command() -> Command {
    let mut args = std::env::args().skip(1);
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let mut command = Command::new(args.next().unwrap_or(editor));
    let _ = command.args(args);
    command
}

impl tea::Model for Model {
    type Msg = Msg;

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(k) if k == "a" => self.alt_screen = !self.alt_screen,
            tea::Msg::KeyPress(k) if k == "e" => return tea::exec(command(), Msg::EditorFinished),
            tea::Msg::KeyPress(k) if k == "q" || k == "ctrl+c" => return tea::quit(),
            tea::Msg::KeyPress(k) if self.runs > 0 => self.keys.push(k.to_string()),
            tea::Msg::App(Msg::EditorFinished(Err(err))) => {
                self.error = Some(err.to_string());
                return tea::quit();
            }
            tea::Msg::App(Msg::EditorFinished(Ok(_))) => self.runs += 1,
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let text = match &self.error {
            Some(err) => format!("Error: {err}\n"),
            None => format!(
                "Press 'e' to open your EDITOR.\nPress 'a' to toggle the altscreen\n\
                 Press 'q' to quit.\nRuns so far: {}\n",
                self.runs
            ),
        };
        tea::View::new(text)
            .with_alt_screen(self.alt_screen)
            .with_mouse_mode(tea::MouseMode::CellMotion)
    }
}

fn main() -> ExitCode {
    match tea::Program::new(Model::default()).run() {
        Ok(model) if model.error.is_none() => {
            println!("runs: {}", model.runs);
            println!("keys after: {}", model.keys.join(" "));
            ExitCode::SUCCESS
        }
        Ok(model) => {
            eprintln!("Error running program: {}", model.error.unwrap_or_default());
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("Error running program: {err}");
            ExitCode::FAILURE
        }
    }
}
