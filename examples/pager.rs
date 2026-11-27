//! A pager: a viewport over a long text, line numbers in the gutter,
//! a title above and the scroll position below, the mouse wheel on.
//! Bubble Tea's `pager` example (it reads `artichoke.md`; this one
//! carries its own text, and Go's search highlights are not ported).
//!
//! `cargo run --example pager`

use petitcarnets as tea;
use tea::bubbles::viewport::{self, GutterContext};
use tea::style::{Border, Position, Style, block_width, height, join_horizontal};

/// The text to page through.
pub(crate) const CONTENT: &str = "# Little notebooks

A notebook is a small thing to carry and a large thing to keep. It holds
the shopping list and the plan for the garden, the address of a friend
and the first line of a poem.

## Why paper

Paper does not run out of battery. It does not ask to be updated. It
opens at the page you last wrote on, and it forgets nothing unless you
tear the page out.

## Why a terminal

A terminal is the notebook of the machine. Programs write to it line by
line, and a good program leaves the page tidy: what it drew is gone when
it is done, and what it printed stays in the scrollback.

## How to keep one

- Write the date at the top of each page.
- Number the pages, so an index can find them.
- Leave the last pages blank for the index.
- Keep a pencil with it; a pen runs dry at the worst moment.

## What to put in it

Anything you would be sorry to forget: a recipe, a measurement, a
thought on the train, the name of the song in the café. A notebook is
not a diary; it does not need to be read again, only to be there when
it is needed.

## A last word

Fill one, and start the next. The shelf of full notebooks is the best
record of a life there is, and the cheapest.";

/// The title's box, open on the right.
const TITLE: Style = Style::new()
    .border(Border {
        right: "├",
        ..Border::ROUNDED
    })
    .padding(0, 1);
/// The scroll position's box, open on the left.
const INFO: Style = Style::new()
    .border(Border {
        left: "┤",
        ..Border::ROUNDED
    })
    .padding(0, 1);

pub(crate) struct Model {
    pub(crate) content: &'static str,
    pub(crate) ready: bool,
    pub(crate) viewport: viewport::Model,
}

pub(crate) fn initial_model() -> Model {
    Model {
        content: CONTENT,
        ready: false,
        viewport: viewport::Model::new(0, 0),
    }
}

/// Line numbers, a bar, and `~` after the end.
fn gutter(info: GutterContext) -> String {
    if info.soft {
        "     │ ".into()
    } else if info.index >= info.total_lines {
        "   ~ │ ".into()
    } else {
        format!("{:4} │ ", info.index + 1)
    }
}

impl Model {
    fn header_view(&self) -> String {
        let title = TITLE.render("Mr. Pager");
        let line = "─".repeat(self.viewport.width().saturating_sub(block_width(&title)));
        join_horizontal(Position::CENTER, &[&title, &line])
    }

    fn footer_view(&self) -> String {
        let info = INFO.render(&format!(
            "{:3.0}%:{:3.0}%",
            self.viewport.scroll_percent() * 100.0,
            self.viewport.horizontal_scroll_percent() * 100.0
        ));
        let line = "─".repeat(self.viewport.width().saturating_sub(block_width(&info)));
        join_horizontal(Position::CENTER, &[&line, &info])
    }
}

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match &msg {
            tea::Msg::KeyPress(k) if *k == "ctrl+c" || *k == "q" || *k == "esc" => {
                return tea::quit();
            }
            tea::Msg::WindowSize(size) => {
                let margin = height(&self.header_view()) + height(&self.footer_view());
                let (w, h) = (
                    usize::from(size.width),
                    usize::from(size.height).saturating_sub(margin),
                );
                if self.ready {
                    self.viewport.set_width(w);
                    self.viewport.set_height(h);
                } else {
                    self.viewport = viewport::Model::new(w, h);
                    self.viewport.y_position = height(&self.header_view());
                    self.viewport.left_gutter = Some(gutter);
                    self.viewport.set_content(self.content);
                    self.ready = true;
                }
            }
            _ => {}
        }
        self.viewport.update(&msg)
    }

    fn view(&self) -> tea::View {
        let content = if self.ready {
            format!(
                "{}\n{}\n{}",
                self.header_view(),
                self.viewport.view(),
                self.footer_view()
            )
        } else {
            "\n  Initializing...".into()
        };
        tea::View::new(content)
            .with_alt_screen(true)
            .with_mouse_mode(tea::MouseMode::CellMotion)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
