//! Showing or hiding the title, the filter, the status bar, the
//! pagination and the help, and the status bar's item names.

use super::{
    Item, ItemDelegate, Model, SHOW_FILTER, SHOW_HELP, SHOW_PAGINATION, SHOW_STATUS_BAR, SHOW_TITLE,
};

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// Shows or hides the title.
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
    /// assert!(strip_ansi(&l.view()).contains("List"));
    /// l.set_show_title(false);
    /// assert!(!strip_ansi(&l.view()).contains("List"));
    /// ```
    pub fn set_show_title(&mut self, on: bool) {
        self.flags.set(SHOW_TITLE, on);
        self.update_pagination();
    }

    /// Whether the title shows.
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
    /// assert!(l.show_title());
    /// l.set_show_title(false);
    /// assert!(!l.show_title());
    /// ```
    #[must_use]
    pub const fn show_title(&self) -> bool {
        self.flags.get(SHOW_TITLE)
    }

    /// Shows or hides the filter input.
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
    /// assert!(strip_ansi(&l.view()).contains("Filter:"));
    /// l.set_show_filter(false);
    /// assert!(!strip_ansi(&l.view()).contains("Filter:"));
    /// ```
    pub fn set_show_filter(&mut self, on: bool) {
        self.flags.set(SHOW_FILTER, on);
        self.update_pagination();
    }

    /// Whether the filter input shows.
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
    /// assert!(l.show_filter());
    /// l.set_show_filter(false);
    /// assert!(!l.show_filter());
    /// ```
    #[must_use]
    pub const fn show_filter(&self) -> bool {
        self.flags.get(SHOW_FILTER)
    }

    /// Shows or hides the status bar.
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
    /// assert!(strip_ansi(&l.view()).contains("3 items"));
    /// l.set_show_status_bar(false);
    /// assert!(!strip_ansi(&l.view()).contains("3 items"));
    /// ```
    pub fn set_show_status_bar(&mut self, on: bool) {
        self.flags.set(SHOW_STATUS_BAR, on);
        self.update_pagination();
    }

    /// Whether the status bar shows.
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
    /// assert!(l.show_status_bar());
    /// l.set_show_status_bar(false);
    /// assert!(!l.show_status_bar());
    /// ```
    #[must_use]
    pub const fn show_status_bar(&self) -> bool {
        self.flags.get(SHOW_STATUS_BAR)
    }

    /// The status bar's name for one item and for several.
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
    /// l.set_status_bar_item_name("fruit", "fruits");
    /// assert!(strip_ansi(&l.view()).contains("3 fruits"));
    /// ```
    pub fn set_status_bar_item_name(&mut self, singular: &str, plural: &str) {
        singular.clone_into(&mut self.item_name_singular);
        plural.clone_into(&mut self.item_name_plural);
    }

    /// The status bar's names for items.
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
    /// let l = fruits();
    /// assert_eq!(l.status_bar_item_name(), ("item", "items"));
    /// ```
    #[must_use]
    pub fn status_bar_item_name(&self) -> (&str, &str) {
        (&self.item_name_singular, &self.item_name_plural)
    }

    /// Shows or hides the pagination.
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
    /// // Two items on the first page, one on the second: a dot a page.
    /// let mut l = fruits();
    /// assert!(strip_ansi(&l.view()).contains("••"));
    /// l.set_show_pagination(false);
    /// assert!(!strip_ansi(&l.view()).contains("••"));
    /// ```
    pub fn set_show_pagination(&mut self, on: bool) {
        self.flags.set(SHOW_PAGINATION, on);
        self.update_pagination();
    }

    /// Whether the pagination shows.
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
    /// assert!(l.show_pagination());
    /// l.set_show_pagination(false);
    /// assert!(!l.show_pagination());
    /// ```
    #[must_use]
    pub const fn show_pagination(&self) -> bool {
        self.flags.get(SHOW_PAGINATION)
    }

    /// Shows or hides the help.
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
    /// assert!(strip_ansi(&l.view()).contains("↑/k up"));
    /// l.set_show_help(false);
    /// assert!(!strip_ansi(&l.view()).contains("↑/k up"));
    /// ```
    pub fn set_show_help(&mut self, on: bool) {
        self.flags.set(SHOW_HELP, on);
        self.update_pagination();
    }

    /// Whether the help shows.
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
    /// assert!(l.show_help());
    /// l.set_show_help(false);
    /// assert!(!l.show_help());
    /// ```
    #[must_use]
    pub const fn show_help(&self) -> bool {
        self.flags.get(SHOW_HELP)
    }
}
