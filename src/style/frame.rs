//! The box around the text: padding, margins, the border and its
//! sides, the size, and how much room the frame takes.

use super::{Border, Color, SIDES_SET, Style};

impl Style {
    /// Padding: `vertical` blank lines above and below, `horizontal`
    /// columns left and right, in the background colour. Lip Gloss's
    /// `Padding(vertical, horizontal)`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().padding(0, 2).render("hi"), "  hi  ");
    /// assert_eq!(Style::new().padding(1, 1).render("hi"), "    \n hi \n    ");
    /// ```
    #[must_use]
    pub const fn padding(mut self, vertical: u16, horizontal: u16) -> Self {
        self.frame.padding = [vertical, horizontal, vertical, horizontal];
        self
    }

    /// Padding on each side: top, right, bottom, left, as CSS orders them.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(
    ///     Style::new().padding_sides(0, 1, 0, 3).render("hi"),
    ///     "   hi "
    /// );
    /// ```
    #[must_use]
    pub const fn padding_sides(mut self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        self.frame.padding = [top, right, bottom, left];
        self
    }

    /// Padding on the left only changed.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(
    ///     Style::new().padding(0, 2).padding_left(0).render("hi"),
    ///     "hi  "
    /// );
    /// ```
    #[must_use]
    pub const fn padding_left(mut self, columns: u16) -> Self {
        self.frame.padding[3] = columns;
        self
    }

    /// Margins: blank space outside the border, `vertical` lines above
    /// and below and `horizontal` columns left and right.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().margin(0, 1).render("hi"), " hi ");
    /// assert_eq!(Style::new().margin(1, 0).render("hi"), "  \nhi\n  ");
    /// ```
    #[must_use]
    pub const fn margin(mut self, vertical: u16, horizontal: u16) -> Self {
        self.frame.margin = [vertical, horizontal, vertical, horizontal];
        self
    }

    /// Margins on each side: top, right, bottom, left.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().margin_sides(0, 0, 0, 2).render("hi"), "  hi");
    /// ```
    #[must_use]
    pub const fn margin_sides(mut self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        self.frame.margin = [top, right, bottom, left];
        self
    }

    /// The top margin only changed.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(
    ///     Style::new().margin(0, 1).margin_top(1).render("hi"),
    ///     "    \n hi "
    /// );
    /// ```
    #[must_use]
    pub const fn margin_top(mut self, rows: u16) -> Self {
        self.frame.margin[0] = rows;
        self
    }

    /// The top margin's rows.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().margin(2, 1).get_margin_top(), 2);
    /// assert_eq!(Style::new().get_margin_top(), 0);
    /// ```
    #[must_use]
    pub fn get_margin_top(&self) -> usize {
        usize::from(self.frame.margin[0])
    }

    /// A border on every side, unless [`Style::border_sides`] picks some.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// assert_eq!(
    ///     Style::new().border(Border::NORMAL).render("hi"),
    ///     "┌──┐\n│hi│\n└──┘"
    /// );
    /// ```
    #[must_use]
    pub const fn border(mut self, border: Border) -> Self {
        self.frame.border = Some(border);
        self
    }

    /// Which sides have the border: top, right, bottom, left, as Lip
    /// Gloss's `Border(border, top, right, bottom, left)` sets them.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// let rules = Style::new()
    ///     .border(Border::NORMAL)
    ///     .border_sides([true, false, true, false]);
    /// assert_eq!(rules.render("hi"), "──\nhi\n──");
    /// ```
    #[must_use]
    pub fn border_sides(mut self, [top, right, bottom, left]: [bool; 4]) -> Self {
        let bit = |on: bool, n: u8| u8::from(on) << n;
        self.frame.sides = SIDES_SET | bit(top, 0) | bit(right, 1) | bit(bottom, 2) | bit(left, 3);
        self
    }

    /// The border's colour.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Color, Style};
    ///
    /// let boxed = Style::new()
    ///     .border(Border::NORMAL)
    ///     .border_fg(Color::RED)
    ///     .render("hi");
    /// assert!(boxed.starts_with("\x1b[31m┌"));
    /// assert_eq!(tea::style::strip_ansi(&boxed), "┌──┐\n│hi│\n└──┘");
    /// ```
    #[must_use]
    pub const fn border_fg(mut self, color: Color) -> Self {
        self.frame.border_fg = Some(color);
        self
    }

    /// At least `columns` wide, border included; longer lines are
    /// wrapped at the width less the padding, as Lip Gloss's `Width`.
    /// 0 is no width.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().width(6).render("hi"), "hi    ");
    /// assert_eq!(Style::new().width(4).render("one two"), "one \ntwo ");
    /// ```
    #[must_use]
    pub const fn width(mut self, columns: u16) -> Self {
        self.frame.width = columns;
        self
    }

    /// At least `rows` high, border included; blank lines are added
    /// below. 0 is no height.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().height(3).render("hi"), "hi\n  \n  ");
    /// ```
    #[must_use]
    pub const fn height(mut self, rows: u16) -> Self {
        self.frame.height = rows;
        self
    }

    /// The padding's columns on the left.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(Style::new().padding_sides(0, 1, 0, 3).get_padding_left(), 3);
    /// ```
    #[must_use]
    pub fn get_padding_left(&self) -> usize {
        usize::from(self.frame.padding[3])
    }

    /// The padding's columns on the right.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::Style;
    ///
    /// assert_eq!(
    ///     Style::new().padding_sides(0, 1, 0, 3).get_padding_right(),
    ///     1
    /// );
    /// ```
    #[must_use]
    pub fn get_padding_right(&self) -> usize {
        usize::from(self.frame.padding[1])
    }

    /// The columns padding, border and margins take, left and right
    /// together: Lip Gloss's `GetHorizontalFrameSize`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// let style = Style::new()
    ///     .padding(0, 1)
    ///     .border(Border::NORMAL)
    ///     .margin(0, 2);
    /// assert_eq!(style.horizontal_frame(), 8);
    /// assert_eq!(tea::style::block_width(&style.render("hi")), 2 + 8);
    /// ```
    #[must_use]
    pub fn horizontal_frame(&self) -> usize {
        let [_, right, _, left] = self.frame.padding;
        let [_, m_right, _, m_left] = self.frame.margin;
        let [_, b_right, _, b_left] = self.border_widths();
        usize::from(right + left + m_right + m_left) + b_right + b_left
    }

    /// The columns left of the content and the rows above it that
    /// margins, border and padding take: where a block's text starts.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// let style = Style::new()
    ///     .margin(1, 2)
    ///     .border(Border::NORMAL)
    ///     .padding(0, 1);
    /// assert_eq!(style.frame_offset(), (4, 2));
    /// let lines: Vec<_> = style.render("hi").lines().map(String::from).collect();
    /// assert_eq!(&lines[2][4 + 2..], "hi │  ");
    /// ```
    #[must_use]
    pub fn frame_offset(&self) -> (usize, usize) {
        let [top, _, _, left] = self.frame.padding;
        let [m_top, _, _, m_left] = self.frame.margin;
        let [b_top, _, _, b_left] = self.border_widths();
        (
            usize::from(left + m_left) + b_left,
            usize::from(top + m_top) + b_top,
        )
    }

    /// The rows padding, border and margins take, above and below.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::style::{Border, Style};
    ///
    /// let style = Style::new().padding(1, 0).border(Border::NORMAL);
    /// assert_eq!(style.vertical_frame(), 4);
    /// assert_eq!(style.render("hi").lines().count(), 1 + 4);
    /// ```
    #[must_use]
    pub fn vertical_frame(&self) -> usize {
        let [top, _, bottom, _] = self.frame.padding;
        let [m_top, _, m_bottom, _] = self.frame.margin;
        let [b_top, _, b_bottom, _] = self.border_widths();
        usize::from(top + bottom + m_top + m_bottom) + b_top + b_bottom
    }
}
