//! A progress bar: filled with a solid colour, a gradient or colours of
//! your own, with the percentage after it, and animated towards a new
//! value by a spring (harmonica's maths, ported). Bubbles' `progress`
//! package.
//!
//! ```
//! use petitcarnets::bubbles::progress;
//! use petitcarnets::style::strip_ansi;
//!
//! let bar = progress::Model::new().with_width(15);
//! assert_eq!(strip_ansi(&bar.view_as(0.5)), "▌▌▌▌▌░░░░░  50%");
//! ```

mod blend;
mod build;
mod spring;
mod view;

use super::next_id;
use crate::style::{Color, Style};
use crate::{Cmd, Msg as TeaMsg};
use spring::Spring;
use std::sync::Arc;
use std::time::Duration;

/// The default full character, a half block that takes two colours.
pub const DEFAULT_FULL_CHAR_HALF_BLOCK: char = '▌';
/// A full block.
pub const DEFAULT_FULL_CHAR_FULL_BLOCK: char = '█';
/// The default empty character.
pub const DEFAULT_EMPTY_CHAR_BLOCK: char = '░';

/// Frames a second of the animation.
const FPS: u32 = 60;
/// Bubbles' defaults.
const DEFAULT_WIDTH: usize = 40;
const DEFAULT_FREQUENCY: f64 = 18.0;
const DEFAULT_DAMPING: f64 = 1.0;

/// `#rrggbb`.
const fn hex(rgb: u32) -> Color {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color::Rgb(r, g, b)
}

/// Bubbles' colours: purple haze to neon pink, blueberry, slate grey.
const BLEND_START: Color = hex(0x5A_56E0);
const BLEND_END: Color = hex(0xEE_6FF8);
const FULL_COLOR: Color = hex(0x75_71F9);
const EMPTY_COLOR: Color = hex(0x60_6060);

/// The colour of a cell: given the percentage shown and the cell's
/// place, both from 0.0 to 1.0. Bubbles' `progress.ColorFunc`. Set
/// with [`Model::with_color_fn`], which takes any closure.
///
/// ```
/// use petitcarnets as tea;
/// use std::sync::Arc;
/// use tea::bubbles::progress::ColorFn;
/// use tea::style::Color;
///
/// let cut = 0.5;
/// let f: ColorFn = Arc::new(move |_total, cell| if cell < cut { Color::RED } else { Color::BLUE });
/// assert_eq!(f(1.0, 0.25), Color::RED);
/// assert_eq!(f(1.0, 0.75), Color::BLUE);
/// ```
pub type ColorFn = Arc<dyn Fn(f64, f64) -> Color + Send + Sync>;

/// Time for a bar's next animation frame: Bubbles' `progress.FrameMsg`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::progress;
/// use tea::style::strip_ansi;
/// use tea::testing::TestProgram;
/// # struct App(
/// #     progress::Model,
/// #     fn(&mut progress::Model) -> tea::Cmd<progress::Msg>,
/// # );
/// #
/// # impl tea::Model for App {
/// #     type Msg = progress::Msg;
/// #     fn init(&mut self) -> tea::Cmd<progress::Msg> {
/// #         (self.1)(&mut self.0)
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<progress::Msg>) -> tea::Cmd<progress::Msg> {
/// #         match msg {
/// #             tea::Msg::KeyPress(_) => (self.1)(&mut self.0),
/// #             _ => self.0.update(&msg),
/// #         }
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app sets the bar at the start, and again on a key.
/// let bar = progress::Model::new().with_width(10).without_percentage();
/// let mut test = TestProgram::new(App(bar, |p| p.set_percent(1.0)), 10, 1);
/// test.advance(Duration::from_millis(50));
/// let bar = &test.model().0;
/// assert!(bar.is_animating(), "a few frames in");
/// assert!(strip_ansi(&bar.view()).starts_with("▌"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameMsg {
    id: u64,
    tag: u64,
}

/// A progress bar's messages: only frames.
pub type Msg = FrameMsg;

/// A progress bar: Bubbles' `progress.Model`.
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::bubbles::progress;
/// use tea::style::strip_ansi;
/// use tea::testing::TestProgram;
/// # struct App(
/// #     progress::Model,
/// #     fn(&mut progress::Model) -> tea::Cmd<progress::Msg>,
/// # );
/// #
/// # impl tea::Model for App {
/// #     type Msg = progress::Msg;
/// #     fn init(&mut self) -> tea::Cmd<progress::Msg> {
/// #         (self.1)(&mut self.0)
/// #     }
/// #     fn update(&mut self, msg: tea::Msg<progress::Msg>) -> tea::Cmd<progress::Msg> {
/// #         match msg {
/// #             tea::Msg::KeyPress(_) => (self.1)(&mut self.0),
/// #             _ => self.0.update(&msg),
/// #         }
/// #     }
/// #     fn view(&self) -> tea::View {
/// #         tea::View::new(self.0.view())
/// #     }
/// # }
///
/// // The app sets the bar at the start, and again on a key.
/// let bar = progress::Model::new().with_width(10).without_percentage();
/// let mut test = TestProgram::new(App(bar, |p| p.set_percent(0.5)), 10, 1);
/// test.advance(Duration::from_secs(1));
/// assert_eq!(strip_ansi(&test.model().0.view()), "▌▌▌▌▌░░░░░");
/// ```
#[derive(Clone)]
pub struct Model {
    /// The character of the filled part.
    pub full: char,
    /// Its colour, when there is no gradient.
    pub full_color: Color,
    /// The character of the empty part.
    pub empty: char,
    /// Its colour.
    pub empty_color: Color,
    /// Whether to show the percentage.
    pub show_percentage: bool,
    /// Writes the percentage, given 0 to 100: Go's `" %3.0f%%"`. A
    /// closure, so it may capture state; a plain `fn` works too.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::sync::Arc;
    /// use tea::bubbles::progress;
    ///
    /// let unit = String::from("pct");
    /// let mut bar = progress::Model::new().with_width(10);
    /// bar.percent_format = Arc::new(move |p| format!(" {p:.0} {unit}"));
    /// assert!(tea::style::strip_ansi(&bar.view_as(0.5)).ends_with(" 50 pct"));
    /// ```
    pub percent_format: Arc<dyn Fn(f64) -> String + Send + Sync>,
    /// The percentage's style.
    pub percentage_style: Style,
    id: u64,
    tag: u64,
    width: usize,
    spring: Spring,
    shown: f64,
    target: f64,
    velocity: f64,
    blend: Vec<Color>,
    scale_blend: bool,
    color_fn: Option<ColorFn>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// A bar 40 cells wide in Bubbles' blueberry, with the percentage:
    /// `progress.New`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::strip_ansi;
    ///
    /// let bar = progress::Model::new();
    /// assert_eq!(bar.width(), 40);
    /// assert_eq!(bar.percent(), 0.0);
    /// assert!(strip_ansi(&bar.view()).ends_with("░   0%"));
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            full: DEFAULT_FULL_CHAR_HALF_BLOCK,
            full_color: FULL_COLOR,
            empty: DEFAULT_EMPTY_CHAR_BLOCK,
            empty_color: EMPTY_COLOR,
            show_percentage: true,
            percent_format: Arc::new(|p| format!(" {p:3.0}%")),
            percentage_style: Style::new(),
            id: next_id(),
            tag: 0,
            width: DEFAULT_WIDTH,
            spring: Spring::new(frame_seconds(), DEFAULT_FREQUENCY, DEFAULT_DAMPING),
            shown: 0.0,
            target: 0.0,
            velocity: 0.0,
            blend: Vec::new(),
            scale_blend: false,
            color_fn: None,
        }
    }

    /// The bar's id, which its frames carry.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    ///
    /// assert_ne!(progress::Model::new().id(), progress::Model::new().id());
    /// ```
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Moves the bar a frame on its own latest frame message.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::progress;
    /// use tea::style::strip_ansi;
    /// use tea::testing::TestProgram;
    /// # struct App(
    /// #     progress::Model,
    /// #     fn(&mut progress::Model) -> tea::Cmd<progress::Msg>,
    /// # );
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = progress::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<progress::Msg> {
    /// #         (self.1)(&mut self.0)
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<progress::Msg>) -> tea::Cmd<progress::Msg> {
    /// #         match msg {
    /// #             tea::Msg::KeyPress(_) => (self.1)(&mut self.0),
    /// #             _ => self.0.update(&msg),
    /// #         }
    /// #     }
    /// #     fn view(&self) -> tea::View {
    /// #         tea::View::new(self.0.view())
    /// #     }
    /// # }
    ///
    /// // The app sets the bar at the start, and again on a key.
    /// let bar = progress::Model::new().with_width(10).without_percentage();
    /// let mut test = TestProgram::new(App(bar, |p| p.set_percent(1.0)), 10, 1);
    /// test.advance(Duration::from_millis(100));
    /// let moving = strip_ansi(&test.model().0.view());
    /// assert!(
    ///     moving.contains("▌") && moving.contains("░"),
    ///     "part way: {moving}"
    /// );
    /// test.advance(Duration::from_secs(1));
    /// assert_eq!(strip_ansi(&test.model().0.view()), "▌".repeat(10));
    /// ```
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        match msg {
            TeaMsg::App(f) if f.id == self.id && f.tag == self.tag && self.is_animating() => {
                (self.shown, self.velocity) =
                    self.spring.update(self.shown, self.velocity, self.target);
                self.next_frame()
            }
            _ => Cmd::none(),
        }
    }

    /// The percentage it is going to, from 0.0 to 1.0.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    ///
    /// let mut bar = progress::Model::new();
    /// let _frames = bar.set_percent(0.25);
    /// assert_eq!(bar.percent(), 0.25, "the target, not what shows yet");
    /// ```
    #[must_use]
    pub const fn percent(&self) -> f64 {
        self.target
    }

    /// Animates to `p`, from 0.0 to 1.0.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    ///
    /// let mut bar = progress::Model::new();
    /// assert!(!bar.set_percent(1.5).is_none(), "frames to animate");
    /// assert_eq!(bar.percent(), 1.0, "clamped");
    /// assert!(bar.is_animating());
    /// ```
    pub fn set_percent(&mut self, p: f64) -> Cmd<Msg> {
        self.target = clamp_percent(p);
        self.tag += 1;
        self.next_frame()
    }

    /// Animates `v` further.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::progress;
    /// use tea::style::strip_ansi;
    /// use tea::testing::TestProgram;
    /// # struct App(
    /// #     progress::Model,
    /// #     fn(&mut progress::Model) -> tea::Cmd<progress::Msg>,
    /// # );
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = progress::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<progress::Msg> {
    /// #         (self.1)(&mut self.0)
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<progress::Msg>) -> tea::Cmd<progress::Msg> {
    /// #         match msg {
    /// #             tea::Msg::KeyPress(_) => (self.1)(&mut self.0),
    /// #             _ => self.0.update(&msg),
    /// #         }
    /// #     }
    /// #     fn view(&self) -> tea::View {
    /// #         tea::View::new(self.0.view())
    /// #     }
    /// # }
    ///
    /// // The app moves the bar at the start, and again on a key.
    /// let bar = progress::Model::new().with_width(10).without_percentage();
    /// let mut test = TestProgram::new(App(bar, |p| p.incr_percent(0.3)), 10, 1);
    /// test.advance(Duration::from_secs(1));
    /// test.press(tea::Key::char(' '));
    /// test.advance(Duration::from_secs(1));
    /// assert_eq!(test.model().0.percent(), 0.6);
    /// assert_eq!(strip_ansi(&test.model().0.view()), "▌▌▌▌▌▌░░░░");
    /// ```
    pub fn incr_percent(&mut self, v: f64) -> Cmd<Msg> {
        self.set_percent(self.target + v)
    }

    /// Animates `v` back.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    ///
    /// let mut bar = progress::Model::new();
    /// let _frames = bar.set_percent(0.5);
    /// let _frames = bar.decr_percent(0.2);
    /// assert!((bar.percent() - 0.3).abs() < 1e-9);
    /// let _frames = bar.decr_percent(1.0);
    /// assert_eq!(bar.percent(), 0.0, "clamped");
    /// ```
    pub fn decr_percent(&mut self, v: f64) -> Cmd<Msg> {
        self.set_percent(self.target - v)
    }

    /// Whether it is still moving.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use std::time::Duration;
    /// use tea::bubbles::progress;
    /// use tea::style::strip_ansi;
    /// use tea::testing::TestProgram;
    /// # struct App(
    /// #     progress::Model,
    /// #     fn(&mut progress::Model) -> tea::Cmd<progress::Msg>,
    /// # );
    /// #
    /// # impl tea::Model for App {
    /// #     type Msg = progress::Msg;
    /// #     fn init(&mut self) -> tea::Cmd<progress::Msg> {
    /// #         (self.1)(&mut self.0)
    /// #     }
    /// #     fn update(&mut self, msg: tea::Msg<progress::Msg>) -> tea::Cmd<progress::Msg> {
    /// #         match msg {
    /// #             tea::Msg::KeyPress(_) => (self.1)(&mut self.0),
    /// #             _ => self.0.update(&msg),
    /// #         }
    /// #     }
    /// #     fn view(&self) -> tea::View {
    /// #         tea::View::new(self.0.view())
    /// #     }
    /// # }
    ///
    /// // The app sets the bar at the start, and again on a key.
    /// let bar = progress::Model::new().with_width(10).without_percentage();
    /// let mut test = TestProgram::new(App(bar, |p| p.set_percent(1.0)), 10, 1);
    /// assert!(test.model().0.is_animating());
    /// test.advance(Duration::from_secs(1));
    /// assert!(!test.model().0.is_animating());
    /// ```
    #[must_use]
    pub fn is_animating(&self) -> bool {
        !((self.shown - self.target).abs() < 0.001 && self.velocity < 0.01)
    }

    /// The width.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    ///
    /// assert_eq!(progress::Model::new().width(), 40);
    /// assert_eq!(progress::Model::new().with_width(8).width(), 8);
    /// ```
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the width.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::strip_ansi;
    ///
    /// let mut bar = progress::Model::new().without_percentage();
    /// bar.set_width(6);
    /// assert_eq!(strip_ansi(&bar.view_as(0.5)), "▌▌▌░░░");
    /// ```
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// The next frame, a sixtieth of a second on.
    fn next_frame(&self) -> Cmd<Msg> {
        let msg = FrameMsg {
            id: self.id,
            tag: self.tag,
        };
        crate::tick(Duration::from_secs(1) / FPS, move |_| msg)
    }
}

/// A frame's length in seconds, as harmonica's `FPS` gives it: a
/// whole number of nanoseconds.
fn frame_seconds() -> f64 {
    (Duration::from_secs(1) / FPS).as_secs_f64()
}

/// `p` within 0.0 to 1.0; NaN is 0.
fn clamp_percent(p: f64) -> f64 {
    if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) }
}

#[cfg(test)]
mod tests;
