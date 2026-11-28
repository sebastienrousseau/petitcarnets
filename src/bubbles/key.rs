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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Binding {
    keys: Vec<String>,
    help: Help,
    disabled: bool,
}

/// What a binding shows in help: Bubbles' `key.Help`.
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
    #[must_use]
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|k| (*k).to_string()).collect(),
            ..Self::default()
        }
    }

    /// With help text: `key.WithHelp`.
    #[must_use]
    pub fn with_help(mut self, key: &str, desc: &str) -> Self {
        self.set_help(key, desc);
        self
    }

    /// Turned off: `key.WithDisabled`.
    #[must_use]
    pub fn with_disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Replaces the keys.
    pub fn set_keys(&mut self, keys: &[&str]) {
        self.keys = keys.iter().map(|k| (*k).to_string()).collect();
    }

    /// The keys.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Replaces the help.
    pub fn set_help(&mut self, key: &str, desc: &str) {
        self.help = Help {
            key: key.to_string(),
            desc: desc.to_string(),
        };
    }

    /// The help.
    #[must_use]
    pub const fn help(&self) -> &Help {
        &self.help
    }

    /// Whether the binding is on: not disabled, and with keys. A
    /// binding that is off matches nothing and is left out of help.
    #[must_use]
    pub fn enabled(&self) -> bool {
        !self.disabled && !self.keys.is_empty()
    }

    /// Turns the binding on or off.
    pub const fn set_enabled(&mut self, on: bool) {
        self.disabled = !on;
    }

    /// Removes the keys and the help, so the binding never matches.
    pub fn unbind(&mut self) {
        self.keys.clear();
        self.help = Help::default();
    }

    /// Whether `key` is one of the keys and the binding is on, comparing
    /// as Go compares `msg.String()`.
    #[must_use]
    pub fn matches(&self, key: &Key) -> bool {
        self.enabled() && self.keys.iter().any(|k| *key == k.as_str())
    }
}

/// Whether `key` matches any of `bindings`: Bubbles' `key.Matches`.
#[must_use]
pub fn matches(key: &Key, bindings: &[&Binding]) -> bool {
    bindings.iter().any(|b| b.matches(key))
}

#[cfg(test)]
mod tests;
