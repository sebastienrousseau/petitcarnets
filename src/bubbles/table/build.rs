//! Building a table: the `with_*` options, and setting its styles.

use super::{Column, KeyMap, Model, Row, Styles};

impl Model {
    /// With these columns: `table.WithColumns`.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
    ///
    /// let t = table::Model::new().with_columns(vec![Column::new("A", 3), Column::new("B", 3)]);
    /// assert_eq!(t.columns()[1], Column::new("B", 3));
    /// ```
    #[must_use]
    pub fn with_columns(mut self, cols: Vec<Column>) -> Self {
        self.set_columns(cols);
        self
    }

    /// With these rows: `table.WithRows`.
    ///
    /// ```
    /// use petitcarnets::bubbles::table;
    ///
    /// let t = table::Model::new().with_rows(vec![vec!["a".into()], vec!["b".into()]]);
    /// assert_eq!(t.rows().len(), 2);
    /// assert_eq!(t.selected_row(), Some(&vec!["a".to_string()]));
    /// ```
    #[must_use]
    pub fn with_rows(mut self, rows: Vec<Row>) -> Self {
        self.set_rows(rows);
        self
    }

    /// `rows` high, headers included: `table.WithHeight`.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
    ///
    /// let t = table::Model::new()
    ///     .with_columns(vec![Column::new("A", 3)])
    ///     .with_height(5);
    /// assert_eq!(t.height(), 4, "a line for the headers");
    /// ```
    #[must_use]
    pub fn with_height(mut self, rows: usize) -> Self {
        self.set_height(rows);
        self
    }

    /// `columns` wide: `table.WithWidth`.
    ///
    /// ```
    /// use petitcarnets::bubbles::table;
    ///
    /// assert_eq!(table::Model::new().with_width(30).width(), 30);
    /// ```
    #[must_use]
    pub fn with_width(mut self, columns: usize) -> Self {
        self.set_width(columns);
        self
    }

    /// Focused or not: `table.WithFocused`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::table::{self, Column};
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let down = tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Down));
    /// let mut t = cities().with_focused(true);
    /// assert!(t.focused());
    /// let _ = t.update::<()>(&down);
    /// assert_eq!(t.cursor(), 1);
    /// ```
    #[must_use]
    pub fn with_focused(mut self, focused: bool) -> Self {
        self.focus = focused;
        self.update_viewport();
        self
    }

    /// With these styles: `table.WithStyles`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::table::{self, Column};
    /// use tea::style::strip_ansi;
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let bold = tea::style::Style::new().bold();
    /// let styles = table::Styles {
    ///     header: bold,
    ///     ..table::Styles::default()
    /// };
    /// let t = cities().with_styles(styles);
    /// assert!(t.view().starts_with("\x1b[1mCity "));
    /// assert_eq!(strip_ansi(&t.view()).lines().next(), Some("City Country"));
    /// ```
    #[must_use]
    pub fn with_styles(mut self, styles: Styles) -> Self {
        self.set_styles(styles);
        self
    }

    /// With these keys: `table.WithKeyMap`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::table::{self, Column};
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let down = tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Down));
    /// let mut keys = table::KeyMap::default();
    /// keys.line_down = tea::bubbles::key::Binding::new(&["n"]);
    /// let mut t = cities().with_focused(true).with_key_map(keys);
    /// let _ = t.update::<()>(&down);
    /// assert_eq!(t.cursor(), 0);
    /// let _ = t.update::<()>(&tea::Msg::KeyPress(tea::Key::char('n')));
    /// assert_eq!(t.cursor(), 1);
    /// ```
    #[must_use]
    pub fn with_key_map(mut self, key_map: KeyMap) -> Self {
        self.key_map = key_map;
        self
    }

    /// Replaces the styles.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::table::{self, Column};
    /// use tea::style::strip_ansi;
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let mut t = cities();
    /// let selected = t.view().lines().nth(1).map(String::from);
    /// t.set_styles(table::Styles::default());
    /// assert_eq!(
    ///     t.view().lines().nth(1).map(String::from),
    ///     selected,
    ///     "the defaults"
    /// );
    /// ```
    pub fn set_styles(&mut self, styles: Styles) {
        self.styles = styles;
        self.update_viewport();
    }
}
