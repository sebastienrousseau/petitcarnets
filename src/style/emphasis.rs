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

    /// Bold.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().bold().paint("hi"), "\x1b[1mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn bold(self) -> Self {
        self.with(BOLD)
    }

    /// Dimmed.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().dim().paint("hi"), "\x1b[2mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn dim(self) -> Self {
        self.with(DIM)
    }

    /// Italic.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().italic().paint("hi"), "\x1b[3mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn italic(self) -> Self {
        self.with(ITALIC)
    }

    /// Underlined.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().underline().paint("hi"), "\x1b[4mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn underline(self) -> Self {
        self.with(UNDERLINE)
    }

    /// Foreground and background swapped.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().reverse().paint("hi"), "\x1b[7mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn reverse(self) -> Self {
        self.with(REVERSE)
    }

    /// With an attribute bit set.
    const fn with(mut self, bit: u8) -> Self {
        self.attributes |= bit;
        self
    }

    /// This style with the colours and emphasis of `other` that this one
    /// does not set: Lip Gloss's `Inherit`. The box is not inherited.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Color, Style};
    ///
    /// let base = Style::new().fg(Color::RED).italic().padding(0, 1);
    /// let style = Style::new().bold().inherit(base);
    /// assert_eq!(style.render("hi"), "\x1b[1;3;31mhi\x1b[0m");
    /// let green = Style::new().fg(Color::GREEN).inherit(base);
    /// assert_eq!(green.paint("hi"), "\x1b[3;32mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn inherit(mut self, other: Self) -> Self {
        if self.fg.is_none() {
            self.fg = other.fg;
        }
        if self.bg.is_none() {
            self.bg = other.bg;
        }
        self.attributes |= other.attributes;
        self
    }

    /// The same colours and emphasis, without padding, border, margins
    /// or size.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// let boxed = Style::new().bold().padding(0, 2).border(Border::NORMAL);
    /// assert_eq!(boxed.plain(), Style::new().bold());
    /// assert_eq!(boxed.plain().render("hi"), "\x1b[1mhi\x1b[0m");
    /// ```
    #[must_use]
    pub const fn plain(mut self) -> Self {
        self.frame = Self::new().frame;
        self
    }
}
