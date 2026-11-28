//! Filtering: turning it on and off, clearing it, and its state and
//! text.

use super::{FILTERING_ENABLED, FilterState, Item, ItemDelegate, Model};

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// Turns filtering on or off; off drops any filter.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let key = |c| tea::Msg::KeyPress(tea::Key::char(c));
    /// let mut l = fruits();
    /// l.set_filtering_enabled(false);
    /// let _ = l.update(&key('/'));
    /// assert!(!l.setting_filter(), "`/` does nothing");
    /// ```
    pub fn set_filtering_enabled(&mut self, on: bool) {
        self.flags.set(FILTERING_ENABLED, on);
        if !on {
            self.reset_filtering();
        }
        self.update_keybindings();
    }

    /// Whether filtering is on.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// assert!(l.filtering_enabled());
    /// l.set_filtering_enabled(false);
    /// assert!(!l.filtering_enabled());
    /// ```
    #[must_use]
    pub const fn filtering_enabled(&self) -> bool {
        self.flags.get(FILTERING_ENABLED)
    }

    /// Drops the filter.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// l.set_filter_text("an");
    /// assert_eq!(l.visible_items().len(), 1);
    /// l.reset_filter();
    /// assert_eq!(l.filter_state(), list::FilterState::Unfiltered);
    /// assert_eq!(l.visible_items().len(), 3);
    /// ```
    pub fn reset_filter(&mut self) {
        self.reset_filtering();
    }

    /// Whether and how it is filtered.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let key = |c| tea::Msg::KeyPress(tea::Key::char(c));
    /// use tea::bubbles::list::FilterState;
    ///
    /// let mut l = fruits();
    /// assert_eq!(l.filter_state(), FilterState::Unfiltered);
    /// let _ = l.update(&key('/'));
    /// assert_eq!(l.filter_state(), FilterState::Filtering);
    /// let _ = l.update(&key('a'));
    /// let _ = l.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Enter)));
    /// assert_eq!(l.filter_state().to_string(), "filter applied");
    /// ```
    #[must_use]
    pub const fn filter_state(&self) -> FilterState {
        self.filter_state
    }

    /// The filter.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// assert_eq!(l.filter_value(), "");
    /// l.set_filter_text("an");
    /// assert_eq!(l.filter_value(), "an");
    /// ```
    #[must_use]
    pub fn filter_value(&self) -> String {
        self.filter_input.value()
    }

    /// Whether the filter is being typed.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let key = |c| tea::Msg::KeyPress(tea::Key::char(c));
    /// let mut l = fruits();
    /// let _ = l.update(&key('/'));
    /// assert!(l.setting_filter());
    /// assert!(!l.is_filtered(), "not yet accepted");
    /// ```
    #[must_use]
    pub fn setting_filter(&self) -> bool {
        self.filter_state == FilterState::Filtering
    }

    /// Whether a filter is set.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// assert!(!l.is_filtered());
    /// l.set_filter_text("an");
    /// assert!(l.is_filtered());
    /// ```
    #[must_use]
    pub fn is_filtered(&self) -> bool {
        self.filter_state == FilterState::FilterApplied
    }
}
