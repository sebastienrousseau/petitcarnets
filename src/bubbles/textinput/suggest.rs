//! Suggestions: setting them, and those that match the value.

use super::Model;

impl Model {
    /// Sets the suggestions, matched by prefix as the text is typed.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.show_suggestions = true;
    /// input.set_suggestions(&["apple", "apricot", "banana"]);
    /// input.set_value("ap");
    /// assert_eq!(input.matched_suggestions(), ["apple", "apricot"]);
    /// ```
    pub fn set_suggestions(&mut self, suggestions: &[&str]) {
        self.suggestions = suggestions.iter().map(|s| s.chars().collect()).collect();
        self.update_suggestions();
    }

    /// Every suggestion.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.set_suggestions(&["apple", "banana"]);
    /// input.set_value("ap");
    /// assert_eq!(input.available_suggestions(), ["apple", "banana"]);
    /// ```
    #[must_use]
    pub fn available_suggestions(&self) -> Vec<String> {
        self.suggestions
            .iter()
            .map(|s| s.iter().collect())
            .collect()
    }

    /// The suggestions that match the text.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.show_suggestions = true;
    /// input.set_suggestions(&["apple", "banana"]);
    /// input.set_value("b");
    /// assert_eq!(input.matched_suggestions(), ["banana"]);
    /// ```
    #[must_use]
    pub fn matched_suggestions(&self) -> Vec<String> {
        self.matched.iter().map(|s| s.iter().collect()).collect()
    }

    /// Which matched suggestion is offered.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.show_suggestions = true;
    /// input.set_suggestions(&["apple", "apricot"]);
    /// let _ = input.focus();
    /// input.set_value("ap");
    /// assert_eq!(input.current_suggestion_index(), 0);
    /// let _ = input.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Down)));
    /// assert_eq!(input.current_suggestion_index(), 1);
    /// ```
    #[must_use]
    pub const fn current_suggestion_index(&self) -> usize {
        self.current_suggestion
    }

    /// The matched suggestion offered, or nothing.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// let mut input = textinput::Model::new();
    /// input.show_suggestions = true;
    /// input.set_suggestions(&["apple", "apricot"]);
    /// let _ = input.focus();
    /// input.set_value("apr");
    /// assert_eq!(input.current_suggestion(), "apricot");
    /// let _ = input.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Tab)));
    /// assert_eq!(input.value(), "apricot", "Tab takes it");
    /// ```
    #[must_use]
    pub fn current_suggestion(&self) -> String {
        self.matched
            .get(self.current_suggestion)
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    }
}
