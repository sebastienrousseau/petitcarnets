//! The environment a program runs in: its variables, which may be the
//! process's own or, for a program served over SSH, the client's.

/// Environment variables, as `NAME`, `value` pairs. Bubble Tea's
/// `uv.Environ`: give one to [`Program::with_environment`](crate::Program::with_environment)
/// to detect colours and terminal features from another environment
/// than the process's, as a server does for each SSH client.
///
/// ```
/// use petitcarnets as tea;
/// use tea::Environment;
///
/// let env: Environment = [("TERM", "xterm"), ("TERM", "kitty")].into_iter().collect();
/// assert_eq!(env.get("TERM"), Some("kitty"));
/// assert_eq!(env.get("COLORTERM"), None);
/// assert_eq!(env.var("COLORTERM"), "");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment(Vec<(String, String)>);

impl Environment {
    /// The process's environment. Variables whose name or value is not
    /// valid Unicode are left out.
    #[must_use]
    pub fn current() -> Self {
        Self(
            std::env::vars_os()
                .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
                .collect(),
        )
    }

    /// A variable's value, if it is set; when a name appears more than
    /// once, the last wins, as in Go.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// A variable's value, or empty when it is not set. Go's `Getenv`.
    #[must_use]
    pub fn var(&self, name: &str) -> &str {
        self.get(name).unwrap_or_default()
    }

    /// Go's `strconv.ParseBool` of a variable: true for `1`, `t`, `T`,
    /// `true`, `TRUE` and `True`; anything else, or unset, is false.
    pub(crate) fn flag(&self, name: &str) -> bool {
        matches!(self.var(name), "1" | "t" | "T" | "true" | "TRUE" | "True")
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for Environment {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(vars: I) -> Self {
        Self(
            vars.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests;
