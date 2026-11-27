//! Suspending and resuming: Bubble Tea's `suspend` example. Ctrl-Z
//! suspends the program, as in any shell program (here by default; see
//! `Program::without_suspend`), and `fg` brings it back; ctrl+c
//! interrupts, q or esc quits.
//!
//! `cargo run --example suspend`

use petitcarnets as tea;
use std::process::ExitCode;

#[derive(Default)]
struct Model {
    quitting: bool,
    resumed: u32,
}

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::Resume => self.resumed += 1,
            tea::Msg::KeyPress(k) if k == "q" || k == "esc" => {
                self.quitting = true;
                return tea::quit();
            }
            tea::Msg::KeyPress(k) if k == "ctrl+c" => {
                self.quitting = true;
                return tea::interrupt();
            }
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        if self.quitting {
            return tea::View::default();
        }
        tea::View::new(format!(
            "\nPress ctrl-z to suspend, ctrl+c to interrupt, q, or esc to exit\n\
             Resumed {} times\n",
            self.resumed
        ))
    }
}

fn main() -> ExitCode {
    match tea::Program::new(Model::default()).run() {
        Ok(model) => {
            println!("resumed: {}", model.resumed);
            ExitCode::SUCCESS
        }
        Err(tea::Error::Interrupted) => ExitCode::from(130),
        Err(err) => {
            eprintln!("Error running program: {err}");
            ExitCode::FAILURE
        }
    }
}
