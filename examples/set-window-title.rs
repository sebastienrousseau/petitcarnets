//! Setting the window title from the view. The title the terminal had
//! comes back when the program ends. Bubble Tea's `set-window-title`
//! example.
//!
//! `cargo run --example set-window-title`

use petitcarnets as tea;

const WINDOW_TITLE: &str = "Hello, petitcarnets";

struct Titled;

impl tea::Model for Titled {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(_) => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        tea::View::new(format!(
            "The window title has been set to '{WINDOW_TITLE}'. It will be put back on exit.\
             \n\nPress any key to quit."
        ))
        .with_window_title(WINDOW_TITLE)
    }
}

fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Titled).run()?;
    Ok(())
}
