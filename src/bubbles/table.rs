//! A table: columns with titles and widths, rows of cells, a selected
//! row that moves with the keys, scrolled in a viewport. Bubbles'
//! `table` package.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::table::{self, Column};
//!
//! let mut t = table::Model::new()
//!     .with_columns(vec![Column::new("City", 8), Column::new("Country", 8)])
//!     .with_rows(vec![
//!         vec!["Tokyo".into(), "Japan".into()],
//!         vec!["Delhi".into(), "India".into()],
//!     ])
//!     .with_height(3)
//!     .with_width(20)
//!     .with_focused(true);
//! let _ = t.update::<()>(&tea::Msg::KeyPress(tea::Key::char('j')));
//! assert_eq!(t.selected_row(), Some(&vec!["Delhi".to_string(), "India".to_string()]));
//! ```

mod build;
mod data;
mod keymap;
mod scroll;
mod style;

pub use keymap::KeyMap;
pub use style::Styles;

use super::help;
use super::viewport;
use crate::style::{Position, join_horizontal, join_vertical, truncate_with, width};
use crate::{Cmd, Msg};

/// A row: a cell for each column.
pub type Row = Vec<String>;

/// A column: Bubbles' `table.Column`.
///
/// ```
/// use petitcarnets::bubbles::table::{self, Column};
/// use petitcarnets::style::strip_ansi;
///
/// let t = table::Model::new()
///     .with_columns(vec![Column::new("Name", 6), Column::new("Hidden", 0)])
///     .with_width(10);
/// let headers = strip_ansi(&t.view()).lines().next().map(String::from);
/// assert_eq!(
///     headers.as_deref(),
///     Some(" Name   "),
///     "a 0 wide column is hidden"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The title.
    pub title: String,
    /// The width in cells; 0 hides the column.
    pub width: usize,
}

impl Column {
    /// A column titled `title`, `width` cells wide.
    ///
    /// ```
    /// use petitcarnets::bubbles::table::Column;
    ///
    /// let c = Column::new("City", 10);
    /// assert_eq!((c.title.as_str(), c.width), ("City", 10));
    /// ```
    #[must_use]
    pub fn new(title: &str, width: usize) -> Self {
        Self {
            title: title.to_string(),
            width,
        }
    }
}

/// A table: Bubbles' `table.Model`.
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
/// let _ = t.update::<()>(&down);
/// assert_eq!(t.selected_row().map(|r| r[0].as_str()), Some("Delhi"));
/// ```
#[derive(Debug, Clone)]
pub struct Model {
    /// The keys.
    pub key_map: KeyMap,
    /// The help for the keys.
    pub help: help::Model,
    cols: Vec<Column>,
    rows: Vec<Row>,
    cursor: usize,
    focus: bool,
    styles: Styles,
    viewport: viewport::Model,
    start: usize,
    end: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// An empty, blurred table 20 rows high: `table.New`.
    ///
    /// ```
    /// use petitcarnets::bubbles::table;
    ///
    /// let t = table::Model::new();
    /// assert!(t.rows().is_empty() && t.columns().is_empty());
    /// assert!(!t.focused());
    /// assert_eq!(t.height(), 20);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let mut m = Self {
            key_map: KeyMap::default(),
            help: help::Model::new(),
            cols: Vec::new(),
            rows: Vec::new(),
            cursor: 0,
            focus: false,
            styles: Styles::default(),
            viewport: viewport::Model::new(0, 20),
            start: 0,
            end: 0,
        };
        m.update_viewport();
        m
    }

    /// Moves the selection on its keys, while focused.
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
    /// let mut t = cities();
    /// let _ = t.update::<()>(&down);
    /// assert_eq!(t.cursor(), 0, "blurred, keys do nothing");
    /// t.focus();
    /// let _ = t.update::<()>(&tea::Msg::KeyPress(tea::Key::char('G')));
    /// assert_eq!(t.cursor(), 2);
    /// ```
    pub fn update<M>(&mut self, msg: &Msg<M>) -> Cmd<M> {
        if let (true, Msg::KeyPress(key)) = (self.focus, msg) {
            let k = &self.key_map;
            let page = self.viewport.height();
            if k.line_up.matches(key) {
                self.move_up(1);
            } else if k.line_down.matches(key) {
                self.move_down(1);
            } else if k.page_up.matches(key) {
                self.move_up(page);
            } else if k.page_down.matches(key) {
                self.move_down(page);
            } else if k.half_page_up.matches(key) {
                self.move_up(page / 2);
            } else if k.half_page_down.matches(key) {
                self.move_down(page / 2);
            } else if k.goto_top.matches(key) {
                self.goto_top();
            } else if k.goto_bottom.matches(key) {
                self.goto_bottom();
            }
        }
        Cmd::none()
    }

    /// Whether it is focused: only a focused table takes keys.
    ///
    /// ```
    /// use petitcarnets::bubbles::table;
    ///
    /// assert!(!table::Model::new().focused());
    /// assert!(table::Model::new().with_focused(true).focused());
    /// ```
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focus
    }

    /// Focuses it.
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
    /// let mut t = cities();
    /// t.focus();
    /// let _ = t.update::<()>(&down);
    /// assert_eq!(t.cursor(), 1);
    /// ```
    pub fn focus(&mut self) {
        self.focus = true;
        self.update_viewport();
    }

    /// Blurs it.
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
    /// t.blur();
    /// assert!(!t.focused());
    /// let _ = t.update::<()>(&down);
    /// assert_eq!(t.cursor(), 0);
    /// ```
    pub fn blur(&mut self) {
        self.focus = false;
        self.update_viewport();
    }

    /// The headers, then the rows in view.
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
    /// let t = cities();
    /// assert_eq!(
    ///     strip_ansi(&t.view()),
    ///     " City   Country \n Tokyo  Japan       \n Delhi  India       \n Cairo  Egypt       "
    /// );
    /// ```
    #[must_use]
    pub fn view(&self) -> String {
        self.headers_view() + "\n" + &self.viewport.view()
    }

    /// The help for the keys.
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
    /// let t = cities();
    /// assert_eq!(strip_ansi(&t.help_view()), "↑/k up • ↓/j down");
    /// ```
    #[must_use]
    pub fn help_view(&self) -> String {
        self.help.view(&self.key_map)
    }

    /// Draws the rows around the cursor into the viewport.
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
    /// let before = t.view();
    /// t.update_viewport();
    /// assert_eq!(t.view(), before, "already drawn");
    /// ```
    pub fn update_viewport(&mut self) {
        let h = self.viewport.height();
        self.start = self.cursor.saturating_sub(h);
        self.end = (self.cursor + h).min(self.rows.len());
        let rendered: Vec<String> = (self.start..self.end).map(|r| self.render_row(r)).collect();
        let blocks: Vec<&str> = rendered.iter().map(String::as_str).collect();
        self.viewport
            .set_content(&join_vertical(Position::LEFT, &blocks));
    }

    /// The column titles, each in the header style.
    fn headers_view(&self) -> String {
        let cells: Vec<String> = self
            .cols
            .iter()
            .filter(|c| c.width > 0)
            .map(|c| self.styles.header.render(&fit(&c.title, c.width)))
            .collect();
        let blocks: Vec<&str> = cells.iter().map(String::as_str).collect();
        join_horizontal(Position::TOP, &blocks)
    }

    /// Row `r`, each cell in the cell style; the selected row's cells
    /// take the selected style over it. Go paints the selected style
    /// round the whole row, so a cell style with a colour ends it at the
    /// first cell (Bubbles' issue #729).
    fn render_row(&self, r: usize) -> String {
        let selected = r == self.cursor;
        let cells: Vec<String> = self.rows[r]
            .iter()
            .zip(&self.cols)
            .filter(|(_, c)| c.width > 0)
            .map(|(value, c)| self.render_cell(&fit(value, c.width), selected))
            .collect();
        cells.concat()
    }

    /// A cell's text in the cell style, or the selected style over it.
    fn render_cell(&self, text: &str, selected: bool) -> String {
        let cell = self.styles.cell;
        if !selected {
            return cell.render(text);
        }
        let (left, right) = (cell.get_padding_left(), cell.get_padding_right());
        let padded = format!("{}{text}{}", " ".repeat(left), " ".repeat(right));
        self.styles
            .selected
            .plain()
            .inherit(cell.plain())
            .paint(&padded)
    }
}

/// `text` cut with an ellipsis to `columns`, then padded to them.
fn fit(text: &str, columns: usize) -> String {
    let text = truncate_with(text, columns, "…");
    let gap = columns.saturating_sub(width(&text));
    text + &" ".repeat(gap)
}

#[cfg(test)]
mod tests;
