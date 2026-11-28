//! Using the model a program returns when it ends: Bubble Tea's
//! `result` example. Choose a tea with the arrows and enter; `main`
//! prints the choice after the program has given the terminal back.
//!
//! `cargo run --example result`

use petitcarnets as tea;

const CHOICES: [&str; 3] = ["Taro", "Coffee", "Lychee"];

#[derive(Default)]
pub(crate) struct Model {
    pub(crate) cursor: usize,
    pub(crate) choice: Option<&'static str>,
}

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        let tea::Msg::KeyPress(k) = msg else {
            return tea::Cmd::none();
        };
        match k.to_string().as_str() {
            "ctrl+c" | "q" | "esc" => return tea::quit(),
            "enter" => {
                self.choice = Some(CHOICES[self.cursor]);
                return tea::quit();
            }
            "down" | "j" => self.cursor = (self.cursor + 1) % CHOICES.len(),
            "up" | "k" => self.cursor = (self.cursor + CHOICES.len() - 1) % CHOICES.len(),
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let mut s = String::from("What kind of Bubble Tea would you like to order?\n\n");
        for (i, choice) in CHOICES.iter().enumerate() {
            s += if self.cursor == i { "(•) " } else { "( ) " };
            s += choice;
            s += "\n";
        }
        s += "\n(press q to quit)\n";
        tea::View::new(s)
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let model = tea::Program::new(Model::default()).run()?;
    if let Some(choice) = model.choice {
        println!("\n---\nYou chose {choice}!");
    }
    Ok(())
}
