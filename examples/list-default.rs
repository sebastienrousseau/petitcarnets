//! A list of favourite things with the default delegate: titles and
//! descriptions, pages, fuzzy filtering with `/`, and help. Bubble
//! Tea's `list-default` example.
//!
//! `cargo run --example list-default`

use petitcarnets as tea;
use tea::bubbles::list::{self, DefaultDelegate, DefaultItem, Item};
use tea::style::Style;

const DOC_STYLE: Style = Style::new().margin(1, 2);

/// A favourite thing.
pub(crate) struct Thing {
    title: &'static str,
    desc: &'static str,
}

impl Item for Thing {
    fn filter_value(&self) -> String {
        self.title.into()
    }
}

impl DefaultItem for Thing {
    fn title(&self) -> String {
        self.title.into()
    }

    fn description(&self) -> String {
        self.desc.into()
    }
}

pub(crate) struct Model {
    pub(crate) list: list::Model<Thing, DefaultDelegate>,
}

pub(crate) fn initial_model() -> Model {
    let things = [
        ("Raspberry Pi’s", "I have ’em all over my house"),
        ("Nutella", "It's good on toast"),
        ("Bitter melon", "It cools you down"),
        ("Nice socks", "And by that I mean socks without holes"),
        ("Eight hours of sleep", "I had this once"),
        ("Cats", "Usually"),
        ("Plantasia, the album", "My plants love it too"),
        ("Pour over coffee", "It takes forever to make though"),
        ("VR", "Virtual reality...what is there to say?"),
        ("Noguchi Lamps", "Such pleasing organic forms"),
        ("Linux", "Pretty much the best OS"),
        ("Business school", "Just kidding"),
        ("Pottery", "Wet clay is a great feeling"),
        ("Shampoo", "Nothing like clean hair"),
        ("Table tennis", "It’s surprisingly exhausting"),
        ("Milk crates", "Great for packing in your extra stuff"),
        ("Afternoon tea", "Especially the tea sandwich part"),
        ("Stickers", "The thicker the vinyl the better"),
        ("20° Weather", "Celsius, not Fahrenheit"),
        ("Warm light", "Like around 2700 Kelvin"),
        (
            "The vernal equinox",
            "The autumnal equinox is pretty good too",
        ),
        ("Gaffer’s tape", "Basically sticky fabric"),
        ("Terrycloth", "In other words, towel fabric"),
    ];
    let items = things
        .into_iter()
        .map(|(title, desc)| Thing { title, desc })
        .collect();
    let mut l = list::Model::new(items, DefaultDelegate::new(), 0, 0);
    l.title = "My Fave Things".into();
    Model { list: l }
}

impl tea::Model for Model {
    type Msg = list::Msg;

    fn update(&mut self, msg: tea::Msg<list::Msg>) -> tea::Cmd<list::Msg> {
        match &msg {
            tea::Msg::KeyPress(k) if *k == "ctrl+c" => return tea::quit(),
            tea::Msg::WindowSize(size) => {
                let w = usize::from(size.width).saturating_sub(DOC_STYLE.horizontal_frame());
                let h = usize::from(size.height).saturating_sub(DOC_STYLE.vertical_frame());
                self.list.set_size(w, h);
            }
            _ => {}
        }
        self.list.update(&msg)
    }

    fn view(&self) -> tea::View {
        tea::View::new(DOC_STYLE.render(&self.list.view())).with_alt_screen(true)
    }
}

// `pub(crate)`: `tests/bubbles.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
