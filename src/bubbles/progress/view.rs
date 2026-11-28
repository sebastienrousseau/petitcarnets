//! Drawing the bar: its filled and empty cells, in a colour, a
//! gradient or colours of your own, and the percentage after it.

use super::{DEFAULT_FULL_CHAR_HALF_BLOCK, Model, blend, clamp_percent};
use crate::style::{Color, Style, round_clamped, to_f64, width};

impl Model {
    /// The bar as it is in its animation.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::strip_ansi;
    ///
    /// let mut bar = progress::Model::new().with_width(10);
    /// let _frames = bar.set_percent(1.0);
    /// assert_eq!(
    ///     strip_ansi(&bar.view()),
    ///     "░░░░░   0%",
    ///     "it has not moved yet"
    /// );
    /// ```
    #[must_use]
    pub fn view(&self) -> String {
        self.view_as(self.shown)
    }

    /// The bar at `percent`, from 0.0 to 1.0, without animating.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::strip_ansi;
    ///
    /// let bar = progress::Model::new().with_width(10);
    /// assert_eq!(strip_ansi(&bar.view_as(0.6)), "▌▌▌░░  60%");
    /// assert_eq!(strip_ansi(&bar.view_as(2.0)), "▌▌▌▌▌ 100%", "clamped");
    /// ```
    #[must_use]
    pub fn view_as(&self, percent: f64) -> String {
        let percent = clamp_percent(percent);
        let label = self.percentage_view(percent);
        self.bar_view(percent, width(&label)) + &label
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
