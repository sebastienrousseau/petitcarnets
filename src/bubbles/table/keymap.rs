//! The keys a table answers to, and the help they show.

use crate::bubbles::help::KeyMap as HelpKeyMap;
use crate::bubbles::key::Binding;

/// The keys of a table: Bubbles' `table.KeyMap`.
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
/// let mut keys = table::KeyMap::default();
/// assert!(keys.line_down.matches(&tea::Key::char('j')));
/// keys.line_down.set_enabled(false);
/// t = t.with_key_map(keys);
/// let _ = t.update::<()>(&down);
/// assert_eq!(t.cursor(), 0, "the key is off");
/// ```
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
