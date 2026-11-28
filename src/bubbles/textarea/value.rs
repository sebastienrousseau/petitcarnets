//! The value: setting it, inserting into it and reading it, and where
//! the cursor and the scroll are.

use super::Model;

impl Model {
    /// Replaces the text.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_value("one\ntwo");
    /// area.set_value("three");
    /// assert_eq!(area.value(), "three");
    /// assert_eq!(
    ///     (area.line(), area.column()),
    ///     (0, 5),
    ///     "the cursor at the end"
    /// );
    /// ```
    pub fn set_value(&mut self, text: &str) {
        self.reset();
        self.insert_string(text);
        self.reposition_view();
    }

    /// Inserts `text` at the cursor, as if typed or pasted.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_value("ad");
    /// area.set_cursor_column(1);
    /// area.insert_string("b\nc");
    /// assert_eq!(area.value(), "ab\ncd");
    /// ```
    pub fn insert_string(&mut self, text: &str) {
        self.insert_from_user(&text.chars().collect::<Vec<_>>());
        self.recalculate_height();
    }

    /// Inserts one character at the cursor.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.insert_rune('h');
    /// area.insert_rune('i');
    /// assert_eq!(area.value(), "hi");
    /// ```
    pub fn insert_rune(&mut self, c: char) {
        self.insert_from_user(&[c]);
        self.recalculate_height();
    }

    /// The text, lines joined with `\n`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let _ = area.focus();
    /// area.insert_string("a");
    /// let _ = area.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Enter)));
    /// area.insert_string("b");
    /// assert_eq!(area.value(), "a\nb");
    /// ```
    #[must_use]
    pub fn value(&self) -> String {
        let lines: Vec<String> = self.value.iter().map(|l| l.iter().collect()).collect();
        lines.join("\n")
    }

    /// Characters in the text, a line break counting one.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_value("ab\nc");
    /// assert_eq!(area.length(), 4, "the line break counts one");
    /// ```
    #[must_use]
    pub fn length(&self) -> usize {
        self.value.iter().map(Vec::len).sum::<usize>() + self.value.len() - 1
    }

    /// How many lines.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// assert_eq!(area.line_count(), 1);
    /// area.set_value("a\nb\nc");
    /// assert_eq!(area.line_count(), 3);
    /// ```
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.value.len()
    }

    /// The cursor's line, from 0.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_value("a\nb\nc");
    /// assert_eq!(area.line(), 2);
    /// area.cursor_up();
    /// assert_eq!(area.line(), 1);
    /// ```
    #[must_use]
    pub const fn line(&self) -> usize {
        self.row
    }

    /// The cursor's column, in characters.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_value("hello");
    /// assert_eq!(area.column(), 5);
    /// area.cursor_start();
    /// assert_eq!(area.column(), 0);
    /// ```
    #[must_use]
    pub const fn column(&self) -> usize {
        self.col
    }

    /// The first screen line showing.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let _ = area.focus();
    /// area.set_height(2);
    /// area.set_value("1\n2\n3\n4");
    /// let _ = area.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Up)));
    /// assert_eq!(area.scroll_y_offset(), 1, "lines 2 and 3 show");
    /// ```
    #[must_use]
    pub const fn scroll_y_offset(&self) -> usize {
        self.viewport.y_offset()
    }

    /// How far down it has scrolled, from 0.0 to 1.0.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let _ = area.focus();
    /// area.set_height(2);
    /// area.set_value("1\n2\n3\n4");
    /// let key = |code| tea::Msg::KeyPress(tea::Key::new(code));
    /// for _ in 0..3 {
    ///     let _ = area.update(&key(tea::KeyCode::Up));
    /// }
    /// assert_eq!(area.scroll_percent(), 0.0, "at the top");
    /// for _ in 0..3 {
    ///     let _ = area.update(&key(tea::KeyCode::Down));
    /// }
    /// assert!(area.scroll_percent() > 0.0, "scrolled down");
    /// ```
    #[must_use]
    pub fn scroll_percent(&self) -> f64 {
        self.viewport.scroll_percent()
    }
}
