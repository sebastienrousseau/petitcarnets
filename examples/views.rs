//! A program with two views, one after the other: Bubble Tea's `views`
//! example. Choose a task within ten seconds; then a progress bar fills
//! with a bounce and the program quits three seconds later.
//!
//! As in Go, the progress bar is drawn by hand, its colours a blend in
//! CIE L*u*v* (go-colorful's `BlendLuv`) and its motion an easing
//! (fogleman/ease's `OutBounce`), both ported below.
//!
//! `cargo run --example views`

use petitcarnets as tea;
use std::time::Duration;
use tea::style::{Color, Style};

const PROGRESS_BAR_WIDTH: usize = 71;
const PROGRESS_FULL_CHAR: &str = "█";
const PROGRESS_EMPTY_CHAR: &str = "░";
const DOT_CHAR: &str = " • ";

const KEYWORD_STYLE: Style = Style::new().fg(Color::Palette(211));
const SUBTLE_STYLE: Style = Style::new().fg(Color::Palette(241));
const TICKS_STYLE: Style = Style::new().fg(Color::Palette(79));
const CHECKBOX_STYLE: Style = Style::new().fg(Color::Palette(212));
const DOT_STYLE: Style = Style::new().fg(Color::Palette(236));
const MAIN_STYLE: Style = Style::new().margin_sides(0, 0, 0, 2);

pub(crate) enum Msg {
    Tick,
    Frame,
}

fn tick() -> tea::Cmd<Msg> {
    tea::tick(Duration::from_secs(1), |_| Msg::Tick)
}

fn frame() -> tea::Cmd<Msg> {
    tea::tick(Duration::from_secs(1) / 60, |_| Msg::Frame)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Model {
    pub(crate) choice: usize,
    pub(crate) chosen: bool,
    pub(crate) ticks: u32,
    pub(crate) frames: u32,
    pub(crate) progress: f64,
    pub(crate) loaded: bool,
    pub(crate) quitting: bool,
}

pub(crate) const fn initial_model() -> Model {
    Model {
        choice: 0,
        chosen: false,
        ticks: 10,
        frames: 0,
        progress: 0.0,
        loaded: false,
        quitting: false,
    }
}

impl Model {
    /// The first view's updates, where a task is chosen.
    fn update_choices(&mut self, msg: &tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(k) if k.matches_any(&["j", "down"]) => {
                self.choice = (self.choice + 1).min(3);
            }
            tea::Msg::KeyPress(k) if k.matches_any(&["k", "up"]) => {
                self.choice = self.choice.saturating_sub(1);
            }
            tea::Msg::KeyPress(k) if *k == "enter" => {
                self.chosen = true;
                return frame();
            }
            tea::Msg::App(Msg::Tick) => return self.count_down(),
            _ => {}
        }
        tea::Cmd::none()
    }

    /// The second view's updates, once a task is chosen.
    fn update_chosen(&mut self, msg: &tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::App(Msg::Frame) if !self.loaded => {
                self.frames += 1;
                self.progress = out_bounce(f64::from(self.frames) / 100.0);
                if self.progress >= 1.0 {
                    self.progress = 1.0;
                    self.loaded = true;
                    self.ticks = 3;
                    return tick();
                }
                frame()
            }
            tea::Msg::App(Msg::Tick) if self.loaded => self.count_down(),
            _ => tea::Cmd::none(),
        }
    }

    fn count_down(&mut self) -> tea::Cmd<Msg> {
        if self.ticks == 0 {
            self.quitting = true;
            return tea::quit();
        }
        self.ticks -= 1;
        tick()
    }

    fn choices_view(&self) -> String {
        let c = self.choice;
        let choices = [
            checkbox("Plant carrots", c == 0),
            checkbox("Go to the market", c == 1),
            checkbox("Read something", c == 2),
            checkbox("See friends", c == 3),
        ]
        .join("\n");
        let dot = DOT_STYLE.render(DOT_CHAR);
        format!(
            "What to do today?\n\n{choices}\n\nProgram quits in {} seconds\n\n{}{dot}{}{dot}{}",
            TICKS_STYLE.render(&self.ticks.to_string()),
            SUBTLE_STYLE.render("j/k, up/down: select"),
            SUBTLE_STYLE.render("enter: choose"),
            SUBTLE_STYLE.render("q, esc: quit"),
        )
    }

    fn chosen_view(&self) -> String {
        let kw = |s: &str| KEYWORD_STYLE.render(s);
        let msg = match self.choice {
            0 => format!(
                "Carrot planting?\n\nCool, we'll need {} and {}...",
                kw("libgarden"),
                kw("vegeutils")
            ),
            1 => format!(
                "A trip to the market?\n\nOkay, then we should install {} and {}...",
                kw("marketkit"),
                kw("libshopping")
            ),
            2 => format!(
                "Reading time?\n\nOkay, cool, then we’ll need a library. Yes, an {}.",
                kw("actual library")
            ),
            _ => format!(
                "It’s always good to see friends.\n\nFetching {} and {}...",
                kw("social-skills"),
                kw("conversationutils")
            ),
        };
        let label = if self.loaded {
            let ticks = TICKS_STYLE.render(&self.ticks.to_string());
            format!("Downloaded. Exiting in {ticks} seconds...")
        } else {
            "Downloading...".to_owned()
        };
        format!("{msg}\n\n{label}\n{}%", progressbar(self.progress))
    }
}

impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        tick()
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        // These keys always quit.
        if let tea::Msg::KeyPress(k) = &msg {
            if k.matches_any(&["q", "esc", "ctrl+c"]) {
                self.quitting = true;
                return tea::quit();
            }
        }
        if self.chosen {
            self.update_chosen(&msg)
        } else {
            self.update_choices(&msg)
        }
    }

    fn view(&self) -> tea::View {
        if self.quitting {
            return tea::View::new("\n  See you later!\n\n");
        }
        let s = if self.chosen {
            self.chosen_view()
        } else {
            self.choices_view()
        };
        tea::View::new(MAIN_STYLE.render(&format!("\n{s}\n")))
    }
}

fn checkbox(label: &str, checked: bool) -> String {
    if checked {
        CHECKBOX_STYLE.render(&format!("[x] {label}"))
    } else {
        format!("[ ] {label}")
    }
}

/// The bar `percent` (0 to 1) full, and the percentage, as `%3.0f`.
fn progressbar(percent: f64) -> String {
    let full_size = (0..=PROGRESS_BAR_WIDTH)
        .rev()
        .find(|&n| f64::from(u32::try_from(n).unwrap_or(0)) <= (71.0 * percent).round())
        .unwrap_or(0);
    let ramp = ramp(PROGRESS_BAR_WIDTH);
    let full: String = ramp[..full_size]
        .iter()
        .map(|style| style.render(PROGRESS_FULL_CHAR))
        .collect();
    let empty = SUBTLE_STYLE
        .render(PROGRESS_EMPTY_CHAR)
        .repeat(PROGRESS_BAR_WIDTH - full_size);
    format!("{full}{empty} {:>3.0}", (percent * 100.0).round())
}

/// fogleman/ease's `OutBounce`.
fn out_bounce(t: f64) -> f64 {
    if t < 4.0 / 11.0 {
        (121.0 * t * t) / 16.0
    } else if t < 8.0 / 11.0 {
        (363.0 / 40.0 * t * t) - (99.0 / 10.0 * t) + 17.0 / 5.0
    } else if t < 9.0 / 10.0 {
        (4356.0 / 361.0 * t * t) - (35442.0 / 1805.0 * t) + 16061.0 / 1805.0
    } else {
        (54.0 / 5.0 * t * t) - (513.0 / 25.0 * t) + 268.0 / 25.0
    }
}

/// `steps` styles blended from `#B14FFF` to `#00FFA3`: Go's
/// `makeRampStyles`.
fn ramp(steps: usize) -> Vec<Style> {
    let from = luv::from_hex([0xB1, 0x4F, 0xFF]);
    let to = luv::from_hex([0x00, 0xFF, 0xA3]);
    let count = f64::from(u32::try_from(steps).unwrap_or(1));
    (0..steps)
        .map(|i| {
            let t = f64::from(u32::try_from(i).unwrap_or(0)) / count;
            let blend = [0, 1, 2].map(|k| from[k] + t * (to[k] - from[k]));
            let [red, green, blue] = luv::to_rgb(blend).map(luv::byte);
            Style::new().fg(Color::Rgb(red, green, blue))
        })
        .collect()
}

/// go-colorful's CIE L*u*v* conversions, D65 white.
mod luv {
    const D65: [f64; 3] = [0.950_47, 1.0, 1.088_83];

    /// Linear sRGB to CIE XYZ, go-colorful's matrix.
    const TO_XYZ: [[f64; 3]; 3] = [
        [
            0.412_390_799_265_959_5,
            0.357_584_339_383_877_96,
            0.180_480_788_401_834_3,
        ],
        [
            0.212_639_005_871_510_36,
            0.715_168_678_767_755_9,
            0.072_192_315_360_733_71,
        ],
        [
            0.019_330_818_715_591_85,
            0.119_194_779_794_625_99,
            0.950_532_152_249_660_6,
        ],
    ];

    /// CIE XYZ to linear sRGB.
    const TO_RGB: [[f64; 3]; 3] = [
        [
            3.240_969_941_904_521_3,
            -1.537_383_177_570_093_5,
            -0.498_610_760_293_003_3,
        ],
        [
            -0.969_243_636_280_879_8,
            1.875_967_501_507_720_6,
            0.041_555_057_407_175_61,
        ],
        [
            0.055_630_079_696_993_61,
            -0.203_976_958_888_976_57,
            1.056_971_514_242_878_6,
        ],
    ];

    fn times(matrix: &[[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
        matrix.map(|row| row[0] * vector[0] + row[1] * vector[1] + row[2] * vector[2])
    }

    fn linearize(channel: f64) -> f64 {
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    fn delinearize(channel: f64) -> f64 {
        if channel <= 0.003_130_8 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        }
    }

    /// The u' and v' of a colour in XYZ.
    fn chromaticity(xyz: [f64; 3]) -> [f64; 2] {
        let denom = xyz[0] + 15.0 * xyz[1] + 3.0 * xyz[2];
        if denom == 0.0 {
            [0.0, 0.0]
        } else {
            [4.0 * xyz[0] / denom, 9.0 * xyz[1] / denom]
        }
    }

    /// An sRGB colour's L*u*v*.
    pub(super) fn from_hex(rgb: [u8; 3]) -> [f64; 3] {
        let xyz = times(&TO_XYZ, rgb.map(|c| linearize(f64::from(c) / 255.0)));
        let ratio = xyz[1] / D65[1];
        let lightness = if ratio <= (6.0 / 29.0_f64).powi(3) {
            ratio * (29.0 / 3.0_f64).powi(3) / 100.0
        } else {
            1.16 * ratio.cbrt() - 0.16
        };
        let (colour, white) = (chromaticity(xyz), chromaticity(D65));
        [
            lightness,
            13.0 * lightness * (colour[0] - white[0]),
            13.0 * lightness * (colour[1] - white[1]),
        ]
    }

    /// An L*u*v* colour's sRGB, 0 to 1 (and outside, out of gamut).
    pub(super) fn to_rgb(luv: [f64; 3]) -> [f64; 3] {
        let lightness = luv[0];
        let luminance = if lightness <= 0.08 {
            D65[1] * lightness * 100.0 * (3.0 / 29.0_f64).powi(3)
        } else {
            D65[1] * ((lightness + 0.16) / 1.16).powi(3)
        };
        let white = chromaticity(D65);
        let xyz = if lightness == 0.0 {
            [0.0; 3]
        } else {
            let up = luv[1] / (13.0 * lightness) + white[0];
            let vp = luv[2] / (13.0 * lightness) + white[1];
            [
                luminance * 9.0 * up / (4.0 * vp),
                luminance,
                luminance * (12.0 - 3.0 * up - 20.0 * vp) / (4.0 * vp),
            ]
        };
        times(&TO_RGB, xyz).map(delinearize)
    }

    /// Go's `int64(f*255)` for a channel, kept to a byte.
    pub(super) fn byte(channel: f64) -> u8 {
        (0..=255_u8)
            .rev()
            .find(|&n| f64::from(n) <= channel * 255.0)
            .unwrap_or(0)
    }
}

// `pub(crate)`: `tests/examples.rs` includes this file for its model.
pub(crate) fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(initial_model()).run()?;
    Ok(())
}
