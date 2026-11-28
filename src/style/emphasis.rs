//! Colour and emphasis: the foreground and the background, bold, dim,
//! italic, underline and reverse, and inheriting or dropping them.

use super::{BOLD, Color, DIM, ITALIC, REVERSE, Style, UNDERLINE};

impl Style {
    /// With this text colour.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Color, Style};
    ///
    /// let red = Style::new().fg(Color::Rgb(255, 0, 0));
    /// assert_eq!(red.paint("hi"), "\x1b[38;2;255;0;0mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// With this background colour.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Color, Style};
    ///
    /// assert_eq!(
    ///     Style::new().bg(Color::BLUE).paint("hi"),
    ///     "\x1b[44mhi\x1b[0m"
    /// );
    /// assert_eq!(
    ///     Style::new().bg(Color::Ansi(9)).paint("hi"),
    ///     "\x1b[101mhi\x1b[0m"
    /// );
    /// ```
    #[must_use]
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Bold when `on`, as Lip Gloss's `Bold(on)`;
    /// `false` turns it off, and [`Style::inherit`] then leaves it off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().bold(true).paint("hi"), "\x1b[1mhi\x1b[0m");
    /// assert_eq!(Style::new().bold(true).bold(false).paint("hi"), "hi");
    /// ```
    #[must_use]
    pub const fn bold(self, on: bool) -> Self {
        self.with(BOLD, on)
    }

    /// Dimmed: Lip Gloss's `Faint` when `on`, as Lip Gloss's `Faint(on)`;
    /// `false` turns it off, and [`Style::inherit`] then leaves it off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().dim(true).paint("hi"), "\x1b[2mhi\x1b[0m");
    /// assert_eq!(Style::new().dim(true).dim(false).paint("hi"), "hi");
    /// ```
    #[must_use]
    pub const fn dim(self, on: bool) -> Self {
        self.with(DIM, on)
    }

    /// Italic when `on`, as Lip Gloss's `Italic(on)`;
    /// `false` turns it off, and [`Style::inherit`] then leaves it off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().italic(true).paint("hi"), "\x1b[3mhi\x1b[0m");
    /// assert_eq!(Style::new().italic(true).italic(false).paint("hi"), "hi");
    /// ```
    #[must_use]
    pub const fn italic(self, on: bool) -> Self {
        self.with(ITALIC, on)
    }

    /// Underlined when `on`, as Lip Gloss's `Underline(on)`;
    /// `false` turns it off, and [`Style::inherit`] then leaves it off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().underline(true).paint("hi"), "\x1b[4mhi\x1b[0m");
    /// assert_eq!(Style::new().underline(true).underline(false).paint("hi"), "hi");
    /// ```
    #[must_use]
    pub const fn underline(self, on: bool) -> Self {
        self.with(UNDERLINE, on)
    }

    /// Foreground and background swapped when `on`, as Lip Gloss's `Reverse(on)`;
    /// `false` turns it off, and [`Style::inherit`] then leaves it off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().reverse(true).paint("hi"), "\x1b[7mhi\x1b[0m");
    /// assert_eq!(Style::new().reverse(true).reverse(false).paint("hi"), "hi");
    /// ```
    #[must_use]
    pub const fn reverse(self, on: bool) -> Self {
        self.with(REVERSE, on)
    }

    /// With an attribute bit set or cleared, and marked as chosen.
    const fn with(mut self, bit: u8, on: bool) -> Self {
        if on {
            self.attributes |= bit;
        } else {
            self.attributes &= !bit;
        }
        self.chosen |= bit;
        self
    }

    /// This style with the colours and emphasis of `other` that this one
    /// does not set (an emphasis turned off with `false` counts as set):
    /// Lip Gloss's `Inherit`. The box is not inherited.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Color, Style};
    ///
    /// let base = Style::new().fg(Color::RED).italic(true).padding(0, 1);
    /// let style = Style::new().bold(true).inherit(base);
    /// assert_eq!(style.render("hi"), "\x1b[1;3;31mhi\x1b[0m");
    /// let green = Style::new().fg(Color::GREEN).inherit(base);
    /// assert_eq!(green.paint("hi"), "\x1b[3;32mhi\x1b[0m");
    /// let upright = Style::new().italic(false).inherit(base);
    /// assert_eq!(upright.paint("hi"), "\x1b[31mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn inherit(mut self, other: Self) -> Self {
        if self.fg.is_none() {
            self.fg = other.fg;
        }
        if self.bg.is_none() {
            self.bg = other.bg;
        }
        self.inherit_attributes(other)
    }

    /// The attributes of `other` this style did not choose, added.
    const fn inherit_attributes(mut self, other: Self) -> Self {
        self.attributes |= other.attributes & !self.chosen;
        self.chosen |= other.chosen;
        self
    }

    /// The same colours and emphasis, without padding, border, margins
    /// or size.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// let boxed = Style::new().bold(true).padding(0, 2).border(Border::NORMAL);
    /// assert_eq!(boxed.plain(), Style::new().bold(true));
    /// assert_eq!(boxed.plain().render("hi"), "\x1b[1mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn plain(mut self) -> Self {
        self.frame = Self::new().frame;
        self
    }
}
