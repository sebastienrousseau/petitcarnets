//! The list's keys, Bubbles v2's defaults.

use crate::bubbles::key::Binding;

/// The keys of a list: Bubbles' `list.KeyMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// The item above.
    pub cursor_up: Binding,
    /// The item below.
    pub cursor_down: Binding,
    /// The page after.
    pub next_page: Binding,
    /// The page before.
    pub prev_page: Binding,
    /// The first item.
    pub go_to_start: Binding,
    /// The last item.
    pub go_to_end: Binding,
    /// Start filtering.
    pub filter: Binding,
    /// Clear the filter.
    pub clear_filter: Binding,
    /// Stop filtering, dropping the filter.
    pub cancel_while_filtering: Binding,
    /// Stop filtering, keeping the filter.
    pub accept_while_filtering: Binding,
    /// Show the full help.
    pub show_full_help: Binding,
    /// Close the full help.
    pub close_full_help: Binding,
    /// Quit.
    pub quit: Binding,
    /// Quit, whatever the state.
    pub force_quit: Binding,
}

impl Default for KeyMap {
    /// Bubbles v2's `list.DefaultKeyMap`.
    fn default() -> Self {
        let b = |keys: &[&str], key: &str, desc: &str| Binding::new(keys).with_help(key, desc);
        Self {
            cursor_up: b(&["up", "k"], "↑/k", "up"),
            cursor_down: b(&["down", "j"], "↓/j", "down"),
            prev_page: b(&["left", "h", "pgup", "b", "u"], "←/h/pgup", "prev page"),
            next_page: b(&["right", "l", "pgdown", "f", "d"], "→/l/pgdn", "next page"),
            go_to_start: b(&["home", "g"], "g/home", "go to start"),
            go_to_end: b(&["end", "G"], "G/end", "go to end"),
            filter: b(&["/"], "/", "filter"),
            clear_filter: b(&["esc"], "esc", "clear filter"),
            cancel_while_filtering: b(&["esc"], "esc", "cancel"),
            accept_while_filtering: b(
                &[
                    "enter",
                    "tab",
                    "shift+tab",
                    "ctrl+k",
                    "up",
                    "ctrl+j",
                    "down",
                ],
                "enter",
                "apply filter",
            ),
            show_full_help: b(&["?"], "?", "more"),
            close_full_help: b(&["?"], "?", "close help"),
            quit: b(&["q", "esc"], "q", "quit"),
            force_quit: Binding::new(&["ctrl+c"]),
        }
    }
}
