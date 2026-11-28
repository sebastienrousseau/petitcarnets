//! A program that leaves nothing behind when it quits: Bubble Tea's
//! `vanish` example. Its last view is empty, so the inline frame is
//! erased.
//!
//! `cargo run --example vanish`

use petitcarnets as tea;

#[derive(Default)]
pub(crate) struct Model(pub(crate) bool);

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        if let tea::Msg::KeyPress(_) = msg {
            self.0 = true;
            return tea::quit();
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        if self.0 {
            return tea::View::new("");
        }
        "Press any key to quit.\n(When this program quits, it will vanish without a trace.)".into()
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Model::default()).run()?;
    Ok(())
}
