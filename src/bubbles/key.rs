//! Key bindings: the keys that do something, the help that says so, and
//! whether they are on. Bubbles' `key` package.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::key::{self, Binding};
//!
//! let up = Binding::new(&["up", "k"]).with_help("↑/k", "move up");
//! assert!(up.matches(&tea::Key::char('k')));
//! assert!(key::matches(&tea::Key::new(tea::KeyCode::Up), &[&up]));
//! assert_eq!(up.help().key, "↑/k");
//! ```

use crate::Key;

/// A binding: keys, their help, and whether it is on. Bubbles'
/// `key.Binding`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::key::Binding;
///
/// let quit = Binding::new(&["q", "ctrl+c"]).with_help("q", "quit");
/// assert!(quit.matches(&tea::Key::ctrl('c')));
/// assert!(!quit.matches(&tea::Key::char('x')));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Binding {
    keys: Vec<String>,
    help: Help,
    disabled: bool,
}

/// What a binding shows in help: Bubbles' `key.Help`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::key::{Binding, Help};
///
/// let b = Binding::new(&["q"]).with_help("q", "quit");
/// let want = Help {
///     key: "q".into(),
///     desc: "quit".into(),
/// };
/// assert_eq!(b.help(), &want);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Help {
    /// The key as shown, such as `"↑/k"`.
    pub key: String,
    /// What it does.
    pub desc: String,
}

impl Binding {
    /// A binding for `keys`, written as keys print (`"ctrl+c"`, `"up"`,
    /// `"?"`): `key.NewBinding(key.WithKeys(...))`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let b = Binding::new(&["up", "k"]);
    /// assert!(b.matches(&tea::Key::new(tea::KeyCode::Up)));
    /// assert!(b.matches(&tea::Key::char('k')));
    /// ```
    #[must_use]
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
            ..Self::default()
        }
    }

    /// With help text: `key.WithHelp`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let b = Binding::new(&["up", "k"]).with_help("↑/k", "move up");
    /// assert_eq!(b.help().key, "↑/k");
    /// assert_eq!(b.help().desc, "move up");
    /// ```
    #[must_use]
    pub fn with_help(mut self, key: &str, desc: &str) -> Self {
        self.set_help(key, desc);
        self
    }

    /// Turned off: `key.WithDisabled`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let b = Binding::new(&["a"]).with_disabled();
    /// assert!(!b.enabled());
    /// assert!(!b.matches(&tea::Key::char('a')));
    /// ```
    #[must_use]
    pub fn with_disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Replaces the keys.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let mut b = Binding::new(&["q"]);
    /// b.set_keys(&["esc"]);
    /// assert!(b.matches(&tea::Key::new(tea::KeyCode::Esc)));
    /// assert!(!b.matches(&tea::Key::char('q')));
    /// ```
    pub fn set_keys(&mut self, keys: &[&str]) {
        self.keys = keys.iter().map(|k| (*k).to_string()).collect();
    }

    /// The keys.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let b = Binding::new(&["q", "ctrl+c"]);
    /// assert_eq!(b.keys(), ["q", "ctrl+c"]);
    /// ```
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Replaces the help.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let mut b = Binding::new(&["q"]);
    /// b.set_help("q", "quit");
    /// assert_eq!(b.help().desc, "quit");
    /// ```
    pub fn set_help(&mut self, key: &str, desc: &str) {
        self.help = Help {
            key: key.to_string(),
            desc: desc.to_string(),
        };
    }

    /// The help.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::{Binding, Help};
    ///
    /// assert_eq!(Binding::new(&["q"]).help(), &Help::default());
    /// let b = Binding::new(&["q"]).with_help("q", "quit");
    /// assert_eq!(b.help().key, "q");
    /// ```
    #[must_use]
    pub const fn help(&self) -> &Help {
        &self.help
    }

    /// Whether the binding is on: not disabled, and with keys. A
    /// binding that is off matches nothing and is left out of help.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// assert!(Binding::new(&["q"]).enabled());
    /// assert!(!Binding::new(&[]).enabled(), "no keys, so off");
    /// assert!(!Binding::new(&["q"]).with_disabled().enabled());
    /// ```
    #[must_use]
    pub fn enabled(&self) -> bool {
        !self.disabled && !self.keys.is_empty()
    }

    /// Turns the binding on or off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let mut b = Binding::new(&["q"]);
    /// b.set_enabled(false);
    /// assert!(!b.matches(&tea::Key::char('q')));
    /// b.set_enabled(true);
    /// assert!(b.matches(&tea::Key::char('q')));
    /// ```
    pub const fn set_enabled(&mut self, on: bool) {
        self.disabled = !on;
    }

    /// Removes the keys and the help, so the binding never matches.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let mut b = Binding::new(&["q"]).with_help("q", "quit");
    /// b.unbind();
    /// assert!(b.keys().is_empty() && b.help().key.is_empty());
    /// assert!(!b.matches(&tea::Key::char('q')));
    /// ```
    pub fn unbind(&mut self) {
        self.keys.clear();
        self.help = Help::default();
    }

    /// Whether `key` is one of the keys and the binding is on, comparing
    /// as Go compares `msg.String()`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    ///
    /// let b = Binding::new(&["shift+tab", "?"]);
    /// let back_tab = tea::Key::new(tea::KeyCode::Tab).with_mods(tea::KeyMod::SHIFT);
    /// assert!(b.matches(&back_tab));
    /// assert!(b.matches(&tea::Key::char('?')));
    /// assert!(!b.matches(&tea::Key::new(tea::KeyCode::Tab)));
    /// ```
    #[must_use]
    pub fn matches(&self, key: &Key) -> bool {
        self.enabled() && self.keys.iter().any(|k| *key == k.as_str())
    }
}

/// Whether `key` matches any of `bindings`: Bubbles' `key.Matches`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::key::{self, Binding};
///
/// let up = Binding::new(&["up", "k"]);
/// let down = Binding::new(&["down", "j"]);
/// assert!(key::matches(&tea::Key::char('j'), &[&up, &down]));
/// assert!(!key::matches(&tea::Key::char('x'), &[&up, &down]));
/// ```
#[must_use]
pub fn matches(key: &Key, bindings: &[&Binding]) -> bool {
    bindings.iter().any(|b| b.matches(key))
}

#[cfg(test)]
mod tests;
