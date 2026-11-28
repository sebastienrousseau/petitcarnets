//! A hundred items, ten a page, dots for the pages: Bubble Tea's
//! `paginator` example.
//!
//! `cargo run --example paginator`

use petitcarnets as tea;
use tea::bubbles::paginator::{self, Type};
use tea::style::{Color, Style};

pub(crate) struct Model {
    pub(crate) items: Vec<String>,
    pub(crate) paginator: paginator::Model,
}

pub(crate) fn new_model() -> Model {
    let items: Vec<String> = (1..101).map(|i| format!("Item {i}")).collect();
    let mut p = paginator::Model::new();
    p.kind = Type::Dots;
    p.per_page = 10;
    let _ = p.set_total_pages(items.len());
    let mut m = Model {
        items,
        paginator: p,
    };
    m.update_styles(true);
    m
}

impl Model {
    fn update_styles(&mut self, is_dark: bool) {
        let (active, inactive) = if is_dark { (252, 238) } else { (235, 250) };
        self.paginator.active_dot = Style::new().fg(Color::Palette(active)).paint("•");
        self.paginator.inactive_dot = Style::new().fg(Color::Palette(inactive)).paint("•");
    }
}

impl tea::Model for Model {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tea::request_background_color()
    }

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match &msg {
            tea::Msg::BackgroundColor(c) => {
                self.update_styles(c.is_dark());
                return tea::Cmd::none();
            }
            tea::Msg::KeyPress(k) if *k == "q" || *k == "esc" || *k == "ctrl+c" => {
                return tea::quit();
            }
            _ => {}
        }
        self.paginator.update(&msg)
    }

    fn view(&self) -> tea::View {
        let mut b = String::from("\n  Paginator Example\n\n");
        let (start, end) = self.paginator.slice_bounds(self.items.len());
        for item in &self.items[start..end] {
            b.push_str("  • ");
            b.push_str(item);
            b.push_str("\n\n");
        }
        b.push_str("  ");
        b.push_str(&self.paginator.view());
        b += "\n\n  h/l ←/→ page • q: quit\n";
        tea::View::new(b)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(new_model()).run()?;
    Ok(())
}
