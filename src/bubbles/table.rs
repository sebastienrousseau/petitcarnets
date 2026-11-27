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

mod scroll;

use super::help::{self, KeyMap as HelpKeyMap};
use super::key::Binding;
use super::viewport;
use crate::style::{
    Color, Position, Style, height, join_horizontal, join_vertical, truncate_with, width,
};
use crate::{Cmd, Msg};

/// A row: a cell for each column.
pub type Row = Vec<String>;

/// A column: Bubbles' `table.Column`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The title.
    pub title: String,
    /// The width in cells; 0 hides the column.
    pub width: usize,
}

impl Column {
    /// A column titled `title`, `width` cells wide.
    #[must_use]
    pub fn new(title: &str, width: usize) -> Self {
        Self {
            title: title.to_string(),
            width,
        }
    }
}

/// The keys of a table: Bubbles' `table.KeyMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// A row up.
    pub line_up: Binding,
    /// A row down.
    pub line_down: Binding,
    /// A page up.
    pub page_up: Binding,
    /// A page down.
    pub page_down: Binding,
    /// Half a page up.
    pub half_page_up: Binding,
    /// Half a page down.
    pub half_page_down: Binding,
    /// To the first row.
    pub goto_top: Binding,
    /// To the last row.
    pub goto_bottom: Binding,
}

impl Default for KeyMap {
    /// Bubbles v2's `table.DefaultKeyMap`.
    fn default() -> Self {
        let b = |keys: &[&str], key: &str, desc: &str| Binding::new(keys).with_help(key, desc);
        Self {
            line_up: b(&["up", "k"], "↑/k", "up"),
            line_down: b(&["down", "j"], "↓/j", "down"),
            page_up: b(&["b", "pgup"], "b/pgup", "page up"),
            page_down: b(&["f", "pgdown", "space"], "f/pgdn", "page down"),
            half_page_up: b(&["u", "ctrl+u"], "u", "½ page up"),
            half_page_down: b(&["d", "ctrl+d"], "d", "½ page down"),
            goto_top: b(&["home", "g"], "g/home", "go to start"),
            goto_bottom: b(&["end", "G"], "G/end", "go to end"),
        }
    }
}

impl HelpKeyMap for KeyMap {
    fn short_help(&self) -> Vec<Binding> {
        vec![self.line_up.clone(), self.line_down.clone()]
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        vec![
            vec![
                self.line_up.clone(),
                self.line_down.clone(),
                self.goto_top.clone(),
                self.goto_bottom.clone(),
            ],
            vec![
                self.page_up.clone(),
                self.page_down.clone(),
                self.half_page_up.clone(),
                self.half_page_down.clone(),
            ],
        ]
    }
}

/// A table's styles: Bubbles' `table.Styles`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Styles {
    /// Each column's title.
    pub header: Style,
    /// Each cell.
    pub cell: Style,
    /// The selected row.
    pub selected: Style,
}

impl Default for Styles {
    /// Bubbles' `table.DefaultStyles`.
    fn default() -> Self {
        Self {
            selected: Style::new().bold().fg(Color::Palette(212)),
            header: Style::new().bold().padding(0, 1),
            cell: Style::new().padding(0, 1),
        }
    }
}

/// A table: Bubbles' `table.Model`.
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

    /// With these columns: `table.WithColumns`.
    #[must_use]
    pub fn with_columns(mut self, cols: Vec<Column>) -> Self {
        self.set_columns(cols);
        self
    }

    /// With these rows: `table.WithRows`.
    #[must_use]
    pub fn with_rows(mut self, rows: Vec<Row>) -> Self {
        self.set_rows(rows);
        self
    }

    /// `rows` high, headers included: `table.WithHeight`.
    #[must_use]
    pub fn with_height(mut self, rows: usize) -> Self {
        self.set_height(rows);
        self
    }

    /// `columns` wide: `table.WithWidth`.
    #[must_use]
    pub fn with_width(mut self, columns: usize) -> Self {
        self.set_width(columns);
        self
    }

    /// Focused or not: `table.WithFocused`.
    #[must_use]
    pub fn with_focused(mut self, focused: bool) -> Self {
        self.focus = focused;
        self.update_viewport();
        self
    }

    /// With these styles: `table.WithStyles`.
    #[must_use]
    pub fn with_styles(mut self, styles: Styles) -> Self {
        self.set_styles(styles);
        self
    }

    /// With these keys: `table.WithKeyMap`.
    #[must_use]
    pub fn with_key_map(mut self, key_map: KeyMap) -> Self {
        self.key_map = key_map;
        self
    }

    /// Replaces the styles.
    pub fn set_styles(&mut self, styles: Styles) {
        self.styles = styles;
        self.update_viewport();
    }

    /// Moves the selection on its keys, while focused.
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
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focus
    }

    /// Focuses it.
    pub fn focus(&mut self) {
        self.focus = true;
        self.update_viewport();
    }

    /// Blurs it.
    pub fn blur(&mut self) {
        self.focus = false;
        self.update_viewport();
    }

    /// The headers, then the rows in view.
    #[must_use]
    pub fn view(&self) -> String {
        self.headers_view() + "\n" + &self.viewport.view()
    }

    /// The help for the keys.
    #[must_use]
    pub fn help_view(&self) -> String {
        self.help.view(&self.key_map)
    }

    /// Draws the rows around the cursor into the viewport.
    pub fn update_viewport(&mut self) {
        let h = self.viewport.height();
        self.start = self.cursor.saturating_sub(h);
        self.end = (self.cursor + h).min(self.rows.len());
        let rendered: Vec<String> = (self.start..self.end).map(|r| self.render_row(r)).collect();
        let blocks: Vec<&str> = rendered.iter().map(String::as_str).collect();
        self.viewport
            .set_content(&join_vertical(Position::LEFT, &blocks));
    }

    /// The selected row.
    #[must_use]
    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.cursor)
    }

    /// The rows.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The columns.
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.cols
    }

    /// Replaces the rows, keeping the cursor on one.
    pub fn set_rows(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        self.update_viewport();
    }

    /// Replaces the columns.
    pub fn set_columns(&mut self, cols: Vec<Column>) {
        self.cols = cols;
        self.update_viewport();
    }

    /// Sets the width.
    pub fn set_width(&mut self, columns: usize) {
        self.viewport.set_width(columns);
        self.update_viewport();
    }

    /// Sets the height, headers included.
    pub fn set_height(&mut self, rows: usize) {
        self.viewport
            .set_height(rows.saturating_sub(height(&self.headers_view())));
        self.update_viewport();
    }

    /// The rows' height.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.viewport.height()
    }

    /// The width.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.viewport.width()
    }

    /// The selected row's index.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Rows from `value`: a line each, cells split at `separator`.
    pub fn from_values(&mut self, value: &str, separator: &str) {
        let rows = value
            .split('\n')
            .map(|line| line.split(separator).map(String::from).collect())
            .collect();
        self.set_rows(rows);
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
