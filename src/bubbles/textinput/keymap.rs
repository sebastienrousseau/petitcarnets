//! The keys a text input answers to.

use crate::bubbles::key::Binding;

/// The keys of a text input: Bubbles' `textinput.KeyMap`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// let keys = textinput::KeyMap::default();
/// assert!(keys.line_start.matches(&tea::Key::ctrl('a')));
/// assert!(
///     keys.accept_suggestion
///         .matches(&tea::Key::new(tea::KeyCode::Tab))
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// One character right.
    pub character_forward: Binding,
    /// One character left.
    pub character_backward: Binding,
    /// To the end of the word.
    pub word_forward: Binding,
    /// To the start of the word.
    pub word_backward: Binding,
    /// Delete the word before the cursor.
    pub delete_word_backward: Binding,
    /// Delete the word after the cursor.
    pub delete_word_forward: Binding,
    /// Delete everything after the cursor.
    pub delete_after_cursor: Binding,
    /// Delete everything before the cursor.
    pub delete_before_cursor: Binding,
    /// Delete the character before the cursor.
    pub delete_character_backward: Binding,
    /// Delete the character under the cursor.
    pub delete_character_forward: Binding,
    /// To the start.
    pub line_start: Binding,
    /// To the end.
    pub line_end: Binding,
    /// Paste the clipboard.
    pub paste: Binding,
    /// Take the suggestion.
    pub accept_suggestion: Binding,
    /// The next suggestion.
    pub next_suggestion: Binding,
    /// The suggestion before.
    pub prev_suggestion: Binding,
}

impl Default for KeyMap {
    /// Bubbles v2's `textinput.DefaultKeyMap`.
    fn default() -> Self {
        let b = Binding::new;
        Self {
            character_forward: b(&["right", "ctrl+f"]),
            character_backward: b(&["left", "ctrl+b"]),
            word_forward: b(&["alt+right", "ctrl+right", "alt+f"]),
            word_backward: b(&["alt+left", "ctrl+left", "alt+b"]),
            delete_word_backward: b(&["alt+backspace", "ctrl+w", "ctrl+backspace"]),
            delete_word_forward: b(&["alt+delete", "alt+d", "ctrl+delete"]),
            delete_after_cursor: b(&["ctrl+k"]),
            delete_before_cursor: b(&["ctrl+u"]),
            delete_character_backward: b(&["backspace", "shift+backspace", "ctrl+h"]),
            delete_character_forward: b(&["delete", "ctrl+d"]),
            line_start: b(&["home", "ctrl+a"]),
            line_end: b(&["end", "ctrl+e"]),
            paste: b(&["ctrl+v"]),
            accept_suggestion: b(&["tab"]),
            next_suggestion: b(&["down", "ctrl+n"]),
            prev_suggestion: b(&["up", "ctrl+p"]),
        }
    }
}
