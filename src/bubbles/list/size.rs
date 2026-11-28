//! The list's width and height.

use super::{Item, ItemDelegate, Model};

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// The width.
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
    /// assert_eq!(fruits().width(), 30);
    /// ```
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// The height.
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
    /// assert_eq!(fruits().height(), 14);
    /// ```
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Sets the width.
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
    /// l.set_width(12);
    /// assert_eq!(l.width(), 12);
    /// assert!(
    ///     strip_ansi(&l.view())
    ///         .lines()
    ///         .all(|line| tea::style::width(line) <= 12)
    /// );
    /// ```
    pub fn set_width(&mut self, columns: usize) {
        self.set_size(columns, self.height);
    }

    /// Sets the height.
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
    /// l.set_height(9);
    /// assert_eq!(l.height(), 9);
    /// assert_eq!(l.paginator.total_pages, 3, "room for one item a page");
    /// ```
    pub fn set_height(&mut self, rows: usize) {
        self.set_size(self.width, rows);
    }

    /// Sets the size.
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
    /// l.set_size(40, 20);
    /// assert_eq!((l.width(), l.height()), (40, 20));
    /// assert_eq!(l.paginator.total_pages, 1);
    /// ```
    pub fn set_size(&mut self, width: usize, height: usize) {
        let prompt = crate::style::width(&self.styles.title.render(&self.filter_input.prompt));
        self.width = width;
        self.height = height;
        self.help.set_width(width);
        let spinner = crate::style::width(&self.spinner.view());
        self.filter_input
            .set_width(width.saturating_sub(prompt + spinner));
        self.update_pagination();
        self.update_keybindings();
    }
}
