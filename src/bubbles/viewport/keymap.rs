//! The keys a viewport answers to.

use crate::bubbles::key::Binding;

/// The keys of a viewport: Bubbles' `viewport.KeyMap`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::viewport;
///
/// let keys = viewport::KeyMap::default();
/// assert!(keys.page_down.matches(&tea::Key::char(' ')));
/// assert_eq!(keys.down.help().key, "↓/j");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// A page down.
    pub page_down: Binding,
    /// A page up.
    pub page_up: Binding,
    /// Half a page up.
    pub half_page_up: Binding,
    /// Half a page down.
    pub half_page_down: Binding,
    /// A line down.
    pub down: Binding,
    /// A line up.
    pub up: Binding,
    /// A step left.
    pub left: Binding,
    /// A step right.
    pub right: Binding,
}

impl Default for KeyMap {
    /// Bubbles v2's `viewport.DefaultKeyMap`.
    fn default() -> Self {
        let b = |keys: &[&str], key: &str, desc: &str| Binding::new(keys).with_help(key, desc);
        Self {
            page_down: b(&["pgdown", "space", "f"], "f/pgdn", "page down"),
            page_up: b(&["pgup", "b"], "b/pgup", "page up"),
            half_page_up: b(&["u", "ctrl+u"], "u", "½ page up"),
            half_page_down: b(&["d", "ctrl+d"], "d", "½ page down"),
            up: b(&["up", "k"], "↑/k", "up"),
            down: b(&["down", "j"], "↓/j", "down"),
            left: b(&["left", "h"], "←/h", "move left"),
            right: b(&["right", "l"], "→/l", "move right"),
        }
    }
}

impl KeyMap {
    /// No bindings at all, for a viewport driven by its owner (the text
    /// area's).
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(4, 1);
    /// v.set_content("a\nb");
    /// v.key_map = viewport::KeyMap::none();
    /// let _ = v.update::<()>(&tea::Msg::KeyPress(tea::Key::char('j')));
    /// assert_eq!(v.y_offset(), 0, "no key scrolls");
    /// ```
    #[must_use]
    pub fn none() -> Self {
        let off = Binding::default;
        Self {
            page_down: off(),
            page_up: off(),
            half_page_up: off(),
            half_page_down: off(),
            down: off(),
            up: off(),
            left: off(),
            right: off(),
        }
    }
}
