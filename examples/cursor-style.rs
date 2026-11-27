//! Changing the cursor's shape with the arrow keys; it blinks on every
//! other key. Bubble Tea's `cursor-style` example.
//!
//! `cargo run --example cursor-style`

use petitcarnets as tea;
use tea::{Cursor, CursorShape};

struct Shapes {
    shape: CursorShape,
    blink: bool,
}

/// The shape after `shape`, going round.
fn next(shape: CursorShape) -> CursorShape {
    match shape {
        CursorShape::Block => CursorShape::Underline,
        CursorShape::Underline => CursorShape::Bar,
        CursorShape::Bar => CursorShape::Block,
    }
}

impl tea::Model for Shapes {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        let tea::Msg::KeyPress(key) = msg else {
            return tea::Cmd::none();
        };
        match key.to_string().as_str() {
            "ctrl+c" | "q" => return tea::quit(),
            "h" | "left" => self.shape = next(next(self.shape)),
            "l" | "right" => self.shape = next(self.shape),
            _ => {}
        }
        self.blink = !self.blink;
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let adjective = if self.blink { "blinking" } else { "steady" };
        let noun = match self.shape {
            CursorShape::Block => "block",
            CursorShape::Underline => "underline",
            CursorShape::Bar => "bar",
        };
        let content = format!(
            "Press left/right to change the cursor style, q or ctrl+c to quit.\
             \n\n  <- This is the cursor (a {adjective} {noun})"
        );
        let cursor = Cursor::new(0, 2)
            .with_shape(self.shape)
            .with_blink(self.blink);
        tea::View::new(content).with_cursor(cursor)
    }
}

fn main() -> Result<(), tea::Error> {
    let shapes = Shapes {
        shape: CursorShape::Block,
        blink: true,
    };
    let _ = tea::Program::new(shapes).run()?;
    Ok(())
}
