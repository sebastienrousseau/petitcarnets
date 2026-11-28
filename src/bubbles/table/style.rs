//! The styles a table is drawn with.

use crate::style::{Color, Style};

/// A table's styles: Bubbles' `table.Styles`.
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
/// let plain = table::Styles {
///     header: tea::style::Style::new(),
///     cell: tea::style::Style::new(),
///     selected: tea::style::Style::new(),
/// };
/// let t = cities().with_styles(plain);
/// assert_eq!(t.view().lines().next(), Some("City Country"));
/// ```
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
