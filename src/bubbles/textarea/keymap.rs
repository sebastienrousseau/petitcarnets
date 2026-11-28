//! The text area's keys, Bubbles v2's defaults.

use crate::bubbles::key::Binding;

/// The keys of a text area: Bubbles' `textarea.KeyMap`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let keys = textarea::KeyMap::default();
/// assert!(
///     keys.insert_newline
///         .matches(&tea::Key::new(tea::KeyCode::Enter))
/// );
/// assert_eq!(keys.select_all.help().desc, "select all");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// One character left.
    pub character_backward: Binding,
    /// One character right.
    pub character_forward: Binding,
    /// Delete to the end of the line.
    pub delete_after_cursor: Binding,
    /// Delete to the start of the line.
    pub delete_before_cursor: Binding,
    /// Delete the character before the cursor.
    pub delete_character_backward: Binding,
    /// Delete the character under the cursor.
    pub delete_character_forward: Binding,
    /// Delete the word before the cursor.
    pub delete_word_backward: Binding,
    /// Delete the word after the cursor.
    pub delete_word_forward: Binding,
    /// Break the line.
    pub insert_newline: Binding,
    /// To the end of the line.
    pub line_end: Binding,
    /// A line down.
    pub line_next: Binding,
    /// A line up.
    pub line_previous: Binding,
    /// To the start of the line.
    pub line_start: Binding,
    /// A page up.
    pub page_up: Binding,
    /// A page down.
    pub page_down: Binding,
    /// Paste the clipboard.
    pub paste: Binding,
    /// To the start of the word.
    pub word_backward: Binding,
    /// To the end of the word.
    pub word_forward: Binding,
    /// To the start of the text.
    pub input_begin: Binding,
    /// To the end of the text.
    pub input_end: Binding,
    /// Upper case the word after the cursor.
    pub uppercase_word_forward: Binding,
    /// Lower case the word after the cursor.
    pub lowercase_word_forward: Binding,
    /// Capitalise the word after the cursor.
    pub capitalize_word_forward: Binding,
    /// Swap the characters before and under the cursor.
    pub transpose_character_backward: Binding,
    /// Select a character right.
    pub select_character_forward: Binding,
    /// Select a character left.
    pub select_character_backward: Binding,
    /// Select to the end of the word.
    pub select_word_forward: Binding,
    /// Select to the start of the word.
    pub select_word_backward: Binding,
    /// Select a line up.
    pub select_line_up: Binding,
    /// Select a line down.
    pub select_line_down: Binding,
    /// Select everything.
    pub select_all: Binding,
    /// Copy the selection to the clipboard.
    pub copy_selection: Binding,
}

/// Keys, help key and help of each default binding, in field order.
const DEFAULTS: [(&[&str], &str, &str); 32] = [
    (&["left", "ctrl+b"], "left", "character backward"),
    (&["right", "ctrl+f"], "right", "character forward"),
    (&["ctrl+k"], "ctrl+k", "delete after cursor"),
    (&["ctrl+u"], "ctrl+u", "delete before cursor"),
    (
        &["backspace", "shift+backspace", "ctrl+h"],
        "backspace",
        "delete character backward",
    ),
    (&["delete", "ctrl+d"], "delete", "delete character forward"),
    (
        &["alt+backspace", "ctrl+w", "ctrl+backspace"],
        "alt+backspace",
        "delete word backward",
    ),
    (
        &["alt+delete", "alt+d", "ctrl+delete"],
        "alt+delete",
        "delete word forward",
    ),
    (&["enter", "ctrl+m"], "enter", "insert newline"),
    (&["end", "ctrl+e"], "end", "line end"),
    (&["down", "ctrl+n"], "down", "next line"),
    (&["up", "ctrl+p"], "up", "previous line"),
    (&["home", "ctrl+a"], "home", "line start"),
    (&["pgup"], "pgup", "page up"),
    (&["pgdown"], "pgdown", "page down"),
    (&["ctrl+v"], "ctrl+v", "paste"),
    (
        &["alt+left", "ctrl+left", "alt+b"],
        "alt+left",
        "word backward",
    ),
    (
        &["alt+right", "ctrl+right", "alt+f"],
        "alt+right",
        "word forward",
    ),
    (&["alt+<", "ctrl+home"], "alt+<", "input begin"),
    (&["alt+>", "ctrl+end"], "alt+>", "input end"),
    (&["alt+u"], "alt+u", "uppercase word forward"),
    (&["alt+l"], "alt+l", "lowercase word forward"),
    (&["alt+c"], "alt+c", "capitalize word forward"),
    (&["ctrl+t"], "ctrl+t", "transpose character backward"),
    (&["shift+right"], "shift+right", "select character forward"),
    (&["shift+left"], "shift+left", "select character backward"),
    (
        &["ctrl+shift+right", "alt+shift+right", "alt+shift+f"],
        "alt+shift+right",
        "select word forward",
    ),
    (
        &["ctrl+shift+left", "alt+shift+left", "alt+shift+b"],
        "alt+shift+left",
        "select word backward",
    ),
    (&["shift+up"], "shift+up", "select line up"),
    (&["shift+down"], "shift+down", "select line down"),
    (&["ctrl+g"], "ctrl+g", "select all"),
    (&["ctrl+shift+c"], "ctrl+shift+c", "copy selection"),
];

impl Default for KeyMap {
    /// Bubbles v2's `textarea.DefaultKeyMap`.
    fn default() -> Self {
        let mut all = DEFAULTS
            .iter()
            .map(|(keys, key, desc)| Binding::new(keys).with_help(key, desc));
        let mut b = || all.next().unwrap_or_default();
        Self {
            character_backward: b(),
            character_forward: b(),
            delete_after_cursor: b(),
            delete_before_cursor: b(),
            delete_character_backward: b(),
            delete_character_forward: b(),
            delete_word_backward: b(),
            delete_word_forward: b(),
            insert_newline: b(),
            line_end: b(),
            line_next: b(),
            line_previous: b(),
            line_start: b(),
            page_up: b(),
            page_down: b(),
            paste: b(),
            word_backward: b(),
            word_forward: b(),
            input_begin: b(),
            input_end: b(),
            uppercase_word_forward: b(),
            lowercase_word_forward: b(),
            capitalize_word_forward: b(),
            transpose_character_backward: b(),
            select_character_forward: b(),
            select_character_backward: b(),
            select_word_forward: b(),
            select_word_backward: b(),
            select_line_up: b(),
            select_line_down: b(),
            select_all: b(),
            copy_selection: b(),
        }
    }
}
