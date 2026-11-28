//! A simple program that queries and displays the window size: Bubble
//! Tea's `window-size` example. Any key but q, esc and ctrl+c asks for
//! the size, which is printed above the program.
//!
//! `cargo run --example window-size`

use petitcarnets as tea;

pub(crate) struct Model;

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k.matches_any(&["ctrl+c", "q", "esc"]) => tea::quit(),
            tea::Msg::KeyPress(_) => tea::request_window_size(),
            tea::Msg::WindowSize(size) => {
                tea::printf!("The window size is: {}x{}", size.width, size.height)
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        "\nWhen you're done press q to quit.\nPress any other key to query the window-size.\n"
            .into()
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Model).run()?;
    Ok(())
}
