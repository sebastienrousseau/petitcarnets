//! A short list with a delegate of its own: pick a dinner with the
//! arrows and Enter. Bubble Tea's `list-simple` example.
//!
//! `cargo run --example list-simple`

use petitcarnets as tea;
use tea::bubbles::list::{self, Item, ItemDelegate};
use tea::style::{Color, Style};

const LIST_HEIGHT: usize = 14;
const DEFAULT_WIDTH: usize = 20;

/// The example's styles.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Styles {
    title: Style,
    item: Style,
    selected_item: Style,
    pagination: Style,
    help: Style,
    quit_text: Style,
}

impl Styles {
    fn new(is_dark: bool) -> Self {
        let defaults = list::Styles::new(is_dark);
        Self {
            title: Style::new().margin_sides(0, 0, 0, 2),
            item: Style::new().padding_left(4),
            selected_item: Style::new().padding_left(2).fg(Color::Palette(170)),
            pagination: defaults.pagination_style.padding_left(4),
            help: defaults.help_style.padding_sides(1, 0, 1, 4),
            quit_text: Style::new().margin_sides(1, 0, 2, 4),
        }
    }
}

/// A dish.
pub(crate) struct Dish(&'static str);

impl Item for Dish {
    fn filter_value(&self) -> String {
        String::new()
    }
}

/// Draws a dish as `n. name`, the selected one after `>`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Delegate(Styles);

impl ItemDelegate<Dish> for Delegate {
    fn render(&self, list: &list::Model<Dish, Self>, index: usize, item: &Dish) -> String {
        let text = format!("{}. {}", index + 1, item.0);
        if index == list.index() {
            self.0.selected_item.render(&format!("> {text}"))
        } else {
            self.0.item.render(&text)
        }
    }

    fn height(&self) -> usize {
        1
    }

    fn spacing(&self) -> usize {
        0
    }
}

pub(crate) struct Model {
    pub(crate) list: list::Model<Dish, Delegate>,
    pub(crate) choice: Option<&'static str>,
    pub(crate) styles: Styles,
    pub(crate) quitting: bool,
}

pub(crate) fn initial_model() -> Model {
    let dishes = [
        "Ramen",
        "Tomato Soup",
        "Hamburgers",
        "Cheeseburgers",
        "Currywurst",
        "Okonomiyaki",
        "Pasta",
        "Fillet Mignon",
        "Caviar",
        "Just Wine",
    ];
    let styles = Styles::new(true);
    let items = dishes.into_iter().map(Dish).collect();
    let mut l = list::Model::new(items, Delegate(styles), DEFAULT_WIDTH, LIST_HEIGHT);
    l.title = "What do you want for dinner?".into();
    l.set_show_status_bar(false);
    l.set_filtering_enabled(false);
    let mut m = Model {
        list: l,
        choice: None,
        styles,
        quitting: false,
    };
    m.update_styles(true);
    m
}

impl Model {
    fn update_styles(&mut self, is_dark: bool) {
        self.styles = Styles::new(is_dark);
        self.list.styles.title = self.styles.title;
        self.list.styles.pagination_style = self.styles.pagination;
        self.list.styles.help_style = self.styles.help;
        self.list.set_delegate(Delegate(self.styles));
    }
}

impl tea::Model for Model {
    type Msg = list::Msg;

    fn update(&mut self, msg: tea::Msg<list::Msg>) -> tea::Cmd<list::Msg> {
        match &msg {
            tea::Msg::WindowSize(size) => {
                self.list.set_width(usize::from(size.width));
                return tea::Cmd::none();
            }
            tea::Msg::KeyPress(k) if *k == "q" || *k == "ctrl+c" => {
                self.quitting = true;
                return tea::quit();
            }
            tea::Msg::KeyPress(k) if *k == "enter" => {
                self.choice = self.list.selected_item().map(|d| d.0);
                return tea::quit();
            }
            _ => {}
        }
        self.list.update(&msg)
    }

    fn view(&self) -> tea::View {
        if let Some(choice) = self.choice {
            return tea::View::new(
                self.styles
                    .quit_text
                    .render(&format!("{choice}? Sounds good to me.")),
            );
        }
        if self.quitting {
            return tea::View::new(self.styles.quit_text.render("Not hungry? That’s cool."));
        }
        tea::View::new(format!("\n{}", self.list.view()))
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
