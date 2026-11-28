//! Setting the terminal's foreground, background and cursor colours:
//! Bubble Tea's `set-terminal-color` example. Choose a colour with j/k
//! and enter, type it in hex (`#ff00ff` or `#f0f`) and press enter; the
//! terminal gets its colours back when the program ends.
//!
//! `cargo run --example set-terminal-color`

use petitcarnets as tea;
use tea::Rgb;
use tea::bubbles::textinput;
use tea::style::Style;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ColorType {
    Foreground,
    Background,
    Cursor,
}

const CHOICES: [ColorType; 3] = [
    ColorType::Foreground,
    ColorType::Background,
    ColorType::Cursor,
];

impl ColorType {
    const fn name(self) -> &'static str {
        match self {
            Self::Foreground => "Foreground",
            Self::Background => "Background",
            Self::Cursor => "Cursor",
        }
    }
}

pub(crate) struct Model {
    pub(crate) ti: textinput::Model,
    /// The colour being typed, while one is.
    pub(crate) choice: Option<ColorType>,
    pub(crate) choice_index: usize,
    pub(crate) err: Option<String>,
    pub(crate) fg: Option<Rgb>,
    pub(crate) bg: Option<Rgb>,
    pub(crate) cc: Option<Rgb>,
}

pub(crate) fn initial_model() -> Model {
    let mut ti = textinput::Model::new();
    ti.placeholder = "#ff00ff".into();
    ti.char_limit = 156;
    ti.set_width(20);
    ti.set_virtual_cursor(false);
    Model {
        ti,
        choice: None,
        choice_index: 0,
        err: None,
        fg: None,
        bg: None,
        cc: None,
    }
}

/// go-colorful's `Hex`: `#rrggbb` or `#rgb`, with its error messages.
fn parse_hex(s: &str) -> Result<Rgb, String> {
    let not_hex = || format!("color: {s} is not a hex-color");
    let digits = s.strip_prefix('#').ok_or_else(not_hex)?;
    let size = match digits.len() {
        3 => 1,
        6 => 2,
        _ => return Err(not_hex()),
    };
    let mut rgb = [0_u8; 3];
    for (i, c) in rgb.iter_mut().enumerate() {
        let part = digits.get(i * size..(i + 1) * size).ok_or_else(not_hex)?;
        let v = u8::from_str_radix(part, 16).map_err(|_| {
            format!(
                "{}: strconv.ParseUint: parsing {part:?}: invalid syntax",
                not_hex()
            )
        })?;
        *c = if size == 1 { v * 17 } else { v };
    }
    Ok(Rgb {
        r: rgb[0],
        g: rgb[1],
        b: rgb[2],
    })
}

impl Model {
    fn back_to_choices(&mut self) {
        self.choice = None;
        self.choice_index = 0;
    }

    fn choose(&mut self, key: &str) {
        self.ti.blur();
        match key {
            "j" | "down" => self.choice_index = (self.choice_index + 1) % 3,
            "k" | "up" => self.choice_index = (self.choice_index + 2) % 3,
            "enter" => {
                let _ = self.ti.focus();
                self.choice = Some(CHOICES[self.choice_index]);
            }
            _ => {}
        }
    }

    fn submit(&mut self, choice: ColorType) {
        match parse_hex(&self.ti.value()) {
            Err(err) => self.err = Some(err),
            Ok(col) => {
                self.err = None;
                self.back_to_choices();
                self.ti.reset();
                match choice {
                    ColorType::Foreground => self.fg = Some(col),
                    ColorType::Background => self.bg = Some(col),
                    ColorType::Cursor => self.cc = Some(col),
                }
            }
        }
        self.ti.blur();
    }

    fn input(
        &mut self,
        choice: ColorType,
        msg: &tea::Msg<textinput::Msg>,
        key: &str,
    ) -> tea::Cmd<textinput::Msg> {
        let _ = self.ti.focus();
        match key {
            "esc" => {
                self.back_to_choices();
                self.err = None;
                self.ti.blur();
            }
            "enter" => self.submit(choice),
            _ => return self.ti.update(msg),
        }
        tea::Cmd::none()
    }
}

impl tea::Model for Model {
    type Msg = textinput::Msg;

    fn init(&mut self) -> tea::Cmd<textinput::Msg> {
        textinput::blink()
    }

    fn update(&mut self, msg: tea::Msg<textinput::Msg>) -> tea::Cmd<textinput::Msg> {
        let tea::Msg::KeyPress(k) = &msg else {
            return tea::Cmd::none();
        };
        let key = k.to_string();
        if key == "ctrl+c" || key == "q" {
            return tea::quit();
        }
        match self.choice {
            None => self.choose(&key),
            Some(choice) => return self.input(choice, &msg, &key),
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let instructions = Style::new()
            .width(40)
            .render("Choose a terminal-wide color to set. All settings will be cleared on exit.");
        let mut s = String::new();
        if self.choice.is_none() {
            s += &instructions;
            s += "\n\n";
            for (i, c) in CHOICES.iter().enumerate() {
                s += if i == self.choice_index { " > " } else { "   " };
                s += c.name();
                s += "\n";
            }
        } else {
            s += "Enter a color in hex format:\n\n";
            s += &self.ti.view();
            s += "\n";
        }
        if let Some(err) = &self.err {
            s += "\nError: ";
            s += err;
        }
        s += "\nPress q to quit";
        s += if self.choice.is_none() {
            ", j/k to move, and enter to select"
        } else {
            ", and enter to submit, esc to go back"
        };
        s += "\n";
        let mut v = tea::View::new(s);
        if let Some(mut c) = self.ti.cursor().filter(|_| self.ti.focused()) {
            c.y += 2; // account for the prompt
            c.color = self.cc;
            v = v.with_cursor(c);
        }
        v.background_color = self.bg;
        v.foreground_color = self.fg;
        v
    }
}

// `pub(crate)`: `tests/bubbles_examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
