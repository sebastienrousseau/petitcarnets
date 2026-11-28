//! Building a bar: the `with_*` options, and the spring's settings.

use super::spring::Spring;
use super::{BLEND_END, BLEND_START, ColorFn, FULL_COLOR, Model, frame_seconds};
use crate::style::Color;

impl Model {
    /// Bubbles' gradient, purple to pink: `progress.WithDefaultBlend`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    ///
    /// let bar = progress::Model::new().with_width(4).without_percentage();
    /// let blend = bar.clone().with_default_blend();
    /// assert_ne!(
    ///     blend.view_as(1.0),
    ///     bar.view_as(1.0),
    ///     "a gradient, not one colour"
    /// );
    /// assert!(blend.view_as(1.0).starts_with("\x1b[38;2;90;86;224;"));
    /// ```
    #[must_use]
    pub fn with_default_blend(self) -> Self {
        self.with_colors(&[BLEND_START, BLEND_END])
    }

    /// No colours is the default solid colour, one a solid colour, more
    /// a gradient through them (24-bit colours only):
    /// `progress.WithColors`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::Color;
    ///
    /// let bar = progress::Model::new().with_width(5).without_percentage();
    /// let green = bar.clone().with_colors(&[Color::GREEN]);
    /// assert!(green.view_as(1.0).starts_with("\x1b[32m▌▌▌▌▌"));
    /// assert_eq!(green.full_color, Color::GREEN);
    /// ```
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
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::Color;
    ///
    /// let red_then_blue = |_total, cell| if cell < 0.5 { Color::RED } else { Color::BLUE };
    /// let bar = progress::Model::new()
    ///     .with_fill_characters('█', ' ')
    ///     .with_color_fn(red_then_blue)
    ///     .with_width(4)
    ///     .without_percentage();
    /// assert!(bar.view_as(1.0).starts_with("\x1b[31m█"));
    /// assert!(bar.view_as(1.0).contains("\x1b[34m█"));
    /// ```
    #[must_use]
    pub fn with_color_fn(mut self, f: ColorFn) -> Self {
        self.color_fn = Some(f);
        self.blend.clear();
        self
    }

    /// The full and empty characters: `progress.WithFillCharacters`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::strip_ansi;
    ///
    /// let bar = progress::Model::new()
    ///     .with_fill_characters('#', '.')
    ///     .with_width(10)
    ///     .without_percentage();
    /// assert_eq!(strip_ansi(&bar.view_as(0.3)), "###.......");
    /// ```
    #[must_use]
    pub const fn with_fill_characters(mut self, full: char, empty: char) -> Self {
        self.full = full;
        self.empty = empty;
        self
    }

    /// Without the percentage: `progress.WithoutPercentage`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::strip_ansi;
    ///
    /// let bar = progress::Model::new().with_width(4);
    /// assert_eq!(
    ///     strip_ansi(&bar.clone().without_percentage().view_as(0.5)),
    ///     "▌▌░░"
    /// );
    /// assert!(!bar.without_percentage().show_percentage);
    /// ```
    #[must_use]
    pub const fn without_percentage(mut self) -> Self {
        self.show_percentage = false;
        self
    }

    /// `columns` wide, percentage included: `progress.WithWidth`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::{strip_ansi, width};
    ///
    /// let bar = progress::Model::new().with_width(12);
    /// assert_eq!(bar.width(), 12);
    /// assert_eq!(strip_ansi(&bar.view_as(1.0)), "▌▌▌▌▌▌▌ 100%");
    /// assert_eq!(width(&bar.view_as(0.0)), 12, "the percentage included");
    /// ```
    #[must_use]
    pub const fn with_width(mut self, columns: usize) -> Self {
        self.width = columns;
        self
    }

    /// The spring's frequency and damping: `progress.WithSpringOptions`.
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
    /// // A slow spring takes longer to get there.
    /// let slow = progress::Model::new().with_spring_options(2.0, 1.0);
    /// let mut test = TestProgram::new(App(slow, |p| p.set_percent(1.0)), 10, 1);
    /// test.advance(Duration::from_secs(1));
    /// assert!(test.model().0.is_animating());
    /// let quick = progress::Model::new();
    /// let mut test = TestProgram::new(App(quick, |p| p.set_percent(1.0)), 10, 1);
    /// test.advance(Duration::from_secs(1));
    /// assert!(!test.model().0.is_animating());
    /// ```
    #[must_use]
    pub fn with_spring_options(mut self, frequency: f64, damping: f64) -> Self {
        self.set_spring_options(frequency, damping);
        self
    }

    /// Whether the gradient spans the filled part rather than the whole
    /// bar: `progress.WithScaled`.
    ///
    /// ```
    /// use petitcarnets::bubbles::progress;
    /// use petitcarnets::style::Color;
    ///
    /// let colors = [Color::Rgb(255, 0, 0), Color::Rgb(0, 0, 255)];
    /// let bar = progress::Model::new()
    ///     .with_colors(&colors)
    ///     .with_fill_characters('█', ' ')
    ///     .with_width(10)
    ///     .without_percentage();
    /// let scaled = bar.clone().with_scaled(true);
    /// assert!(
    ///     scaled.view_as(0.5).contains("\x1b[38;2;0;0;255m█"),
    ///     "blue by the middle"
    /// );
    /// assert!(!bar.view_as(0.5).contains("\x1b[38;2;0;0;255m█"));
    /// ```
    #[must_use]
    pub const fn with_scaled(mut self, on: bool) -> Self {
        self.scale_blend = on;
        self
    }

    /// Changes the spring.
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
    /// let mut bar = progress::Model::new();
    /// bar.set_spring_options(2.0, 1.0);
    /// let mut test = TestProgram::new(App(bar, |p| p.set_percent(1.0)), 10, 1);
    /// test.advance(Duration::from_secs(1));
    /// assert!(
    ///     test.model().0.is_animating(),
    ///     "a slow spring is still moving"
    /// );
    /// ```
    pub fn set_spring_options(&mut self, frequency: f64, damping: f64) {
        self.spring = Spring::new(frame_seconds(), frequency, damping);
    }
}
