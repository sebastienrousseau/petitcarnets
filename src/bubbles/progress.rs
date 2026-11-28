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
mod spring;

use super::next_id;
use crate::style::{Color, Style, round_clamped, to_f64, width};
use crate::{Cmd, Msg as TeaMsg};
use spring::Spring;
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
/// place, both from 0.0 to 1.0. Bubbles' `progress.ColorFunc`.
pub type ColorFn = fn(total: f64, current: f64) -> Color;

/// Time for a bar's next animation frame: Bubbles' `progress.FrameMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameMsg {
    id: u64,
    tag: u64,
}

/// A progress bar's messages: only frames.
pub type Msg = FrameMsg;

/// A progress bar: Bubbles' `progress.Model`.
#[derive(Debug, Clone)]
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
    /// Writes the percentage, given 0 to 100: Go's `" %3.0f%%"`.
    pub percent_format: fn(f64) -> String,
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            full: DEFAULT_FULL_CHAR_HALF_BLOCK,
            full_color: FULL_COLOR,
            empty: DEFAULT_EMPTY_CHAR_BLOCK,
            empty_color: EMPTY_COLOR,
            show_percentage: true,
            percent_format: |p| format!(" {p:3.0}%"),
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

    /// Bubbles' gradient, purple to pink: `progress.WithDefaultBlend`.
    #[must_use]
    pub fn with_default_blend(self) -> Self {
        self.with_colors(&[BLEND_START, BLEND_END])
    }

    /// No colours is the default solid colour, one a solid colour, more
    /// a gradient through them (24-bit colours only):
    /// `progress.WithColors`.
    #[must_use]
    pub fn with_colors(mut self, colors: &[Color]) -> Self {
        match colors {
            [] | [_] => {
                self.full_color = colors.first().copied().unwrap_or(FULL_COLOR);
                self.blend.clear();
                self.color_fn = None;
            }
            many => self.blend = many.to_vec(),
        }
        self
    }

    /// Each cell's colour from `f`: `progress.WithColorFunc`.
    #[must_use]
    pub fn with_color_fn(mut self, f: ColorFn) -> Self {
        self.color_fn = Some(f);
        self.blend.clear();
        self
    }

    /// The full and empty characters: `progress.WithFillCharacters`.
    #[must_use]
    pub const fn with_fill_characters(mut self, full: char, empty: char) -> Self {
        self.full = full;
        self.empty = empty;
        self
    }

    /// Without the percentage: `progress.WithoutPercentage`.
    #[must_use]
    pub const fn without_percentage(mut self) -> Self {
        self.show_percentage = false;
        self
    }

    /// `columns` wide, percentage included: `progress.WithWidth`.
    #[must_use]
    pub const fn with_width(mut self, columns: usize) -> Self {
        self.width = columns;
        self
    }

    /// The spring's frequency and damping: `progress.WithSpringOptions`.
    #[must_use]
    pub fn with_spring_options(mut self, frequency: f64, damping: f64) -> Self {
        self.set_spring_options(frequency, damping);
        self
    }

    /// Whether the gradient spans the filled part rather than the whole
    /// bar: `progress.WithScaled`.
    #[must_use]
    pub const fn with_scaled(mut self, on: bool) -> Self {
        self.scale_blend = on;
        self
    }

    /// Changes the spring.
    pub fn set_spring_options(&mut self, frequency: f64, damping: f64) {
        self.spring = Spring::new(frame_seconds(), frequency, damping);
    }

    /// The bar's id, which its frames carry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Moves the bar a frame on its own latest frame message.
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
    #[must_use]
    pub const fn percent(&self) -> f64 {
        self.target
    }

    /// Animates to `p`, from 0.0 to 1.0.
    pub fn set_percent(&mut self, p: f64) -> Cmd<Msg> {
        self.target = clamp_percent(p);
        self.tag += 1;
        self.next_frame()
    }

    /// Animates `v` further.
    pub fn incr_percent(&mut self, v: f64) -> Cmd<Msg> {
        self.set_percent(self.target + v)
    }

    /// Animates `v` back.
    pub fn decr_percent(&mut self, v: f64) -> Cmd<Msg> {
        self.set_percent(self.target - v)
    }

    /// Whether it is still moving.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        !((self.shown - self.target).abs() < 0.001 && self.velocity < 0.01)
    }

    /// The width.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the width.
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// The bar as it is in its animation.
    #[must_use]
    pub fn view(&self) -> String {
        self.view_as(self.shown)
    }

    /// The bar at `percent`, from 0.0 to 1.0, without animating.
    #[must_use]
    pub fn view_as(&self, percent: f64) -> String {
        let percent = clamp_percent(percent);
        let label = self.percentage_view(percent);
        self.bar_view(percent, width(&label)) + &label
    }

    /// The next frame, a sixtieth of a second on.
    fn next_frame(&self) -> Cmd<Msg> {
        let msg = FrameMsg {
            id: self.id,
            tag: self.tag,
        };
        crate::tick(Duration::from_secs(1) / FPS, move |_| msg)
    }

    /// The filled and empty cells, leaving `label` cells.
    fn bar_view(&self, percent: f64, label: usize) -> String {
        let total = self.width.saturating_sub(label);
        let filled = round_clamped(to_f64(total) * percent, total);
        let half = self.full == DEFAULT_FULL_CHAR_HALF_BLOCK;
        let mut out = if let Some(f) = self.color_fn {
            colored_cells(self.full, filled, total, half, |t| f(percent, t))
        } else if self.blend.is_empty() {
            Style::new()
                .fg(self.full_color)
                .paint(&self.full.to_string().repeat(filled))
        } else {
            self.blend_cells(filled, total, half)
        };
        let empty = self.empty.to_string().repeat(total - filled);
        out += &Style::new().fg(self.empty_color).paint(&empty);
        out
    }

    /// The filled cells in the gradient; a half block takes two colours.
    fn blend_cells(&self, filled: usize, total: usize, half: bool) -> String {
        let per = if half { 2 } else { 1 };
        let span = if self.scale_blend { filled } else { total };
        let colors = blend::blend_1d(span * per, &self.blend);
        let full = self.full.to_string();
        (0..filled)
            .map(|i| {
                let style = Style::new().fg(colors[i * per]);
                let style = if half {
                    style.bg(colors[i * per + 1])
                } else {
                    style
                };
                style.paint(&full)
            })
            .collect()
    }

    /// The percentage, written and styled.
    fn percentage_view(&self, percent: f64) -> String {
        if !self.show_percentage {
            return String::new();
        }
        self.percentage_style
            .paint(&(self.percent_format)(percent * 100.0))
    }
}

/// `filled` cells of `total` coloured by `color(place)`; a half block's
/// background takes the colour half a cell on.
fn colored_cells(
    full: char,
    filled: usize,
    total: usize,
    half: bool,
    color: impl Fn(f64) -> Color,
) -> String {
    let step = 0.5 / to_f64(total);
    (0..filled)
        .map(|i| {
            let current = to_f64(i) / to_f64(total);
            let style = Style::new().fg(color(current));
            let style = if half {
                style.bg(color((current + step).min(1.0)))
            } else {
                style
            };
            style.paint(&full.to_string())
        })
        .collect()
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
