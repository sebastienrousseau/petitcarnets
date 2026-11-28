//! The value, and moving the cursor within it.

use super::Model;
use crate::bubbles::text::sanitize;

impl Model {
    /// Replaces the text, cleaned (tabs and line breaks become spaces)
    /// and cut to the limit.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_value("a\tb\nc");
    /// assert_eq!(input.value(), "a b c");
    /// input.char_limit = 2;
    /// input.set_value("abc");
    /// assert_eq!(input.value(), "ab");
    /// ```
    pub fn set_value(&mut self, text: &str) {
        let chars = sanitize(&text.chars().collect::<Vec<_>>(), " ", " ");
        self.err = self.check(&chars);
        self.set_value_internal(chars);
        self.update_suggestions();
    }

    /// The text.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// let _ = input.focus();
    /// for c in "ok".chars() {
    ///     let _ = input.update(&tea::Msg::KeyPress(tea::Key::char(c)));
    /// }
    /// assert_eq!(input.value(), "ok");
    /// ```
    #[must_use]
    pub fn value(&self) -> String {
        self.value.iter().collect()
    }

    /// The cursor's position, in characters from the start.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// let _ = input.focus();
    /// input.set_value("abc");
    /// assert_eq!(input.position(), 3);
    /// let _ = input.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Left)));
    /// assert_eq!(input.position(), 2);
    /// ```
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Moves the cursor to `pos`, within the text.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_value("abc");
    /// input.set_cursor(1);
    /// assert_eq!(input.position(), 1);
    /// input.set_cursor(9);
    /// assert_eq!(input.position(), 3, "no further than the end");
    /// ```
    pub fn set_cursor(&mut self, pos: usize) {
        self.pos = pos.min(self.value.len());
        self.handle_overflow();
    }

    /// Moves the cursor to the start.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_value("abc");
    /// input.cursor_start();
    /// assert_eq!(input.position(), 0);
    /// ```
    pub fn cursor_start(&mut self) {
        self.set_cursor(0);
    }

    /// Moves the cursor to the end.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_value("abc");
    /// input.set_cursor(0);
    /// input.cursor_end();
    /// assert_eq!(input.position(), 3);
    /// ```
    pub fn cursor_end(&mut self) {
        self.set_cursor(self.value.len());
    }
}
