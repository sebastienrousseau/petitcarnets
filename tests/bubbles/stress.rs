//! Random keys, pastes and resizes, seeded, into the editing and
//! scrolling components: none may panic, whatever the text (wide,
//! combining, emoji) and however small the space.

use petitcarnets as tea;
use tea::bubbles::list::{self, DefaultDelegate, DefaultItem, Item};
use tea::bubbles::{textarea, textinput, viewport};
use tea::{Key, KeyCode, KeyMod, Size};

/// A small xorshift generator: the same keys on every run.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0 % n as u64).expect("small")
    }
}

const TEXT: [&str; 8] = [
    "a",
    "é",
    "e\u{301}",
    "字",
    "👍🏽",
    "\u{e01}\u{e33}",
    " ",
    "\t",
];
const CODES: [KeyCode; 12] = [
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::PgUp,
    KeyCode::PgDown,
    KeyCode::Delete,
    KeyCode::Backspace,
    KeyCode::Enter,
    KeyCode::Esc,
];

/// One random message.
fn message<M>(rng: &mut Rng) -> tea::Msg<M> {
    match rng.below(10) {
        0..=3 => {
            let text = TEXT[rng.below(TEXT.len())];
            let mut key = Key::new(KeyCode::Extended);
            key.text = text.into();
            tea::Msg::KeyPress(key)
        }
        4 => tea::Msg::KeyPress(Key::ctrl(char::from(
            b'a' + u8::try_from(rng.below(26)).expect("letter"),
        ))),
        5 => tea::Msg::KeyPress(Key::new(CODES[rng.below(CODES.len())]).with_mods(KeyMod::ALT)),
        6 => tea::Msg::KeyPress(Key::char('/')),
        7 => tea::Msg::Paste(
            (0..rng.below(6))
                .map(|_| TEXT[rng.below(TEXT.len())])
                .collect::<String>()
                + "\n",
        ),
        8 => tea::Msg::WindowSize(Size::new(
            u16::try_from(rng.below(12)).expect("small"),
            u16::try_from(rng.below(6)).expect("small"),
        )),
        _ => tea::Msg::KeyPress(Key::new(CODES[rng.below(CODES.len())])),
    }
}

#[test]
fn a_text_input_takes_any_keys() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for round in 0..60 {
        let mut input = textinput::Model::new();
        let _ = input.focus();
        input.char_limit = round % 7;
        input.set_width(round % 5);
        input.placeholder = "字👍🏽 placeholder".into();
        for _ in 0..100 {
            let _ = input.update(&message(&mut rng));
            let _ = input.view();
        }
    }
}

#[test]
fn a_text_area_takes_any_keys() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for round in 0..60 {
        let mut area = textarea::Model::new();
        let _ = area.focus();
        area.set_width(round % 6);
        area.set_height(round % 4);
        for _ in 0..100 {
            let _ = area.update(&message(&mut rng));
            let _ = area.view();
        }
    }
}

struct Thing(&'static str);

impl Item for Thing {
    fn filter_value(&self) -> String {
        self.0.into()
    }
}

impl DefaultItem for Thing {
    fn title(&self) -> String {
        self.0.into()
    }

    fn description(&self) -> String {
        self.0.repeat(3)
    }
}

#[test]
fn a_list_takes_any_keys() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    for round in 0..40 {
        let items = TEXT.iter().take(round % 9).map(|t| Thing(t)).collect();
        let mut l = list::Model::new(items, DefaultDelegate::new(), round % 7, round % 9);
        for _ in 0..100 {
            let _ = l.update(&message(&mut rng));
            let _ = l.view();
        }
    }
}

#[test]
fn a_viewport_takes_any_keys() {
    let mut rng = Rng(0x0f0f_1e1e_2d2d_3c3c);
    for round in 0..40 {
        let mut view = viewport::Model::new(round % 5, round % 4);
        view.set_content("字👍🏽 e\u{301}\n\n\u{e01}\u{e33} a very long line that goes on\nend");
        for _ in 0..100 {
            let _ = view.update(&message::<()>(&mut rng));
            let _ = view.view();
        }
    }
}
