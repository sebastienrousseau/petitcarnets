//! Scrolling: the offsets, by page, half page, line and column, and
//! to the top or the bottom.

use super::Model;

impl Model {
    /// The first line showing.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// assert_eq!(v.y_offset(), 0);
    /// v.page_down();
    /// assert_eq!(v.y_offset(), 2);
    /// ```
    #[must_use]
    pub const fn y_offset(&self) -> usize {
        self.y_offset
    }

    /// Scrolls so line `n` is at the top, as far as the content goes.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.set_y_offset(1);
    /// assert_eq!(v.view(), "2\n3");
    /// v.set_y_offset(99);
    /// assert_eq!(v.y_offset(), 3, "as far as the content goes");
    /// ```
    pub fn set_y_offset(&mut self, n: usize) {
        self.y_offset = n.min(self.max_y_offset());
    }

    /// The first column showing.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(2, 1);
    /// v.set_content("abcdef");
    /// assert_eq!(v.x_offset(), 0);
    /// v.scroll_right(3);
    /// assert_eq!(v.x_offset(), 3);
    /// ```
    #[must_use]
    pub const fn x_offset(&self) -> usize {
        self.x_offset
    }

    /// Scrolls sideways to column `n`; nothing when soft wrapping.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(2, 1);
    /// v.set_content("abcdef");
    /// v.set_x_offset(2);
    /// assert_eq!(v.view(), "cd");
    /// v.set_x_offset(99);
    /// assert_eq!(v.x_offset(), 4, "the last columns show");
    /// ```
    pub fn set_x_offset(&mut self, n: usize) {
        if !self.soft_wrap {
            self.x_offset = n.min(self.longest.saturating_sub(self.width));
        }
    }

    /// How far a step sideways goes.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(2, 1);
    /// v.set_content("abcdef");
    /// v.set_horizontal_step(1);
    /// let _ = v.update::<()>(&tea::Msg::KeyPress(tea::Key::char('l')));
    /// assert_eq!(v.x_offset(), 1);
    /// ```
    pub const fn set_horizontal_step(&mut self, n: usize) {
        self.horizontal_step = n;
    }

    /// A page down.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.page_down();
    /// assert_eq!(v.view(), "3\n4");
    /// ```
    pub fn page_down(&mut self) {
        if !self.at_bottom() {
            self.scroll_down(self.inner_height());
        }
    }

    /// A page up.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.goto_bottom();
    /// v.page_up();
    /// assert_eq!(v.view(), "2\n3");
    /// ```
    pub fn page_up(&mut self) {
        if !self.at_top() {
            self.scroll_up(self.inner_height());
        }
    }

    /// Half a page down.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.half_page_down();
    /// assert_eq!(v.y_offset(), 1);
    /// ```
    pub fn half_page_down(&mut self) {
        if !self.at_bottom() {
            self.scroll_down(self.inner_height() / 2);
        }
    }

    /// Half a page up.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.goto_bottom();
    /// v.half_page_up();
    /// assert_eq!(v.y_offset(), 2);
    /// ```
    pub fn half_page_up(&mut self) {
        if !self.at_top() {
            self.scroll_up(self.inner_height() / 2);
        }
    }

    /// `n` lines down.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.scroll_down(2);
    /// assert_eq!(v.view(), "3\n4");
    /// v.scroll_down(9);
    /// assert!(v.at_bottom());
    /// ```
    pub fn scroll_down(&mut self, n: usize) {
        if self.at_bottom() || n == 0 || self.lines.is_empty() {
            return;
        }
        self.set_y_offset(self.y_offset + n);
    }

    /// `n` lines up.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.goto_bottom();
    /// v.scroll_up(1);
    /// assert_eq!(v.view(), "3\n4");
    /// ```
    pub fn scroll_up(&mut self, n: usize) {
        if self.at_top() || n == 0 || self.lines.is_empty() {
            return;
        }
        self.set_y_offset(self.y_offset.saturating_sub(n));
    }

    /// `n` columns left.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(2, 1);
    /// v.set_content("abcdef");
    /// v.set_x_offset(4);
    /// v.scroll_left(3);
    /// assert_eq!(v.view(), "bc");
    /// ```
    pub fn scroll_left(&mut self, n: usize) {
        self.set_x_offset(self.x_offset.saturating_sub(n));
    }

    /// `n` columns right.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(2, 1);
    /// v.set_content("abcdef");
    /// v.scroll_right(1);
    /// assert_eq!(v.view(), "bc");
    /// ```
    pub fn scroll_right(&mut self, n: usize) {
        self.set_x_offset(self.x_offset + n);
    }

    /// To the top.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.goto_bottom();
    /// v.goto_top();
    /// assert!(v.at_top());
    /// assert_eq!(v.view(), "1\n2");
    /// ```
    pub const fn goto_top(&mut self) {
        self.y_offset = 0;
    }

    /// To the bottom.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.goto_bottom();
    /// assert_eq!(v.view(), "4\n5");
    /// ```
    pub fn goto_bottom(&mut self) {
        self.y_offset = self.max_y_offset();
    }
}
