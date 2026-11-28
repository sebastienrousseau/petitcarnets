//! The rows and the columns, the size, and the selected row.

use super::{Column, Model, Row};
use crate::style::height;

impl Model {
    /// The selected row.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
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
    /// assert_eq!(
    ///     t.selected_row(),
    ///     Some(&vec!["Tokyo".to_string(), "Japan".to_string()])
    /// );
    /// t.set_rows(Vec::new());
    /// assert_eq!(t.selected_row(), None);
    /// ```
    #[must_use]
    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.cursor)
    }

    /// The rows.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let t = cities();
    /// assert_eq!(t.rows().len(), 3);
    /// assert_eq!(t.rows()[2][0], "Cairo");
    /// ```
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The columns.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let t = cities();
    /// let titles: Vec<&str> = t.columns().iter().map(|c| c.title.as_str()).collect();
    /// assert_eq!(titles, ["City", "Country"]);
    /// ```
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.cols
    }

    /// Replaces the rows, keeping the cursor on one.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
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
    /// t.set_cursor(2);
    /// t.set_rows(vec![vec!["Lagos".into(), "Nigeria".into()]]);
    /// assert_eq!(t.cursor(), 0, "kept on a row");
    /// assert_eq!(t.selected_row().map(|r| r[0].as_str()), Some("Lagos"));
    /// ```
    pub fn set_rows(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        self.update_viewport();
    }

    /// Replaces the columns.
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
    /// t.set_columns(vec![Column::new("City", 5)]);
    /// assert_eq!(strip_ansi(&t.view()).lines().next(), Some(" City  "));
    /// ```
    pub fn set_columns(&mut self, cols: Vec<Column>) {
        self.cols = cols;
        self.update_viewport();
    }

    /// Sets the width.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
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
    /// t.set_width(12);
    /// assert_eq!(t.width(), 12);
    /// ```
    pub fn set_width(&mut self, columns: usize) {
        self.viewport.set_width(columns);
        self.update_viewport();
    }

    /// Sets the height, headers included.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
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
    /// t.set_height(3);
    /// assert_eq!(t.height(), 2, "a line for the headers");
    /// ```
    pub fn set_height(&mut self, rows: usize) {
        self.viewport
            .set_height(rows.saturating_sub(height(&self.headers_view())));
        self.update_viewport();
    }

    /// The rows' height.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let t = cities();
    /// assert_eq!(t.height(), 3);
    /// ```
    #[must_use]
    pub const fn height(&self) -> usize {
        self.viewport.height()
    }

    /// The width.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::{self, Column};
    /// # fn cities() -> table::Model {
    /// #     let mut t = table::Model::new()
    /// #         .with_columns(vec![Column::new("City", 5), Column::new("Country", 7)])
    /// #         .with_height(4)
    /// #         .with_width(20);
    /// #     t.from_values("Tokyo,Japan\nDelhi,India\nCairo,Egypt", ",");
    /// #     t
    /// # }
    ///
    /// let t = cities();
    /// assert_eq!(t.width(), 20);
    /// ```
    #[must_use]
    pub const fn width(&self) -> usize {
        self.viewport.width()
    }

    /// The selected row's index.
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
    /// assert_eq!(t.cursor(), 0);
    /// let _ = t.update::<()>(&down);
    /// assert_eq!(t.cursor(), 1);
    /// ```
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Rows from `value`: a line each, cells split at `separator`.
    ///
    /// ```
    /// use petitcarnets::bubbles::table;
    ///
    /// let mut t = table::Model::new();
    /// t.from_values("1;Tokyo\n2;Delhi", ";");
    /// assert_eq!(t.rows()[1], ["2", "Delhi"]);
    /// ```
    pub fn from_values(&mut self, value: &str, separator: &str) {
        let rows = value
            .split('\n')
            .map(|line| line.split(separator).map(String::from).collect())
            .collect();
        self.set_rows(rows);
    }
}
