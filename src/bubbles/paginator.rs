//! Pages: which one is showing, out of how many, drawn as dots or as
//! numbers. Bubbles' `paginator` package.
//!
//! ```
//! use petitcarnets::bubbles::paginator::{self, Type};
//!
//! let mut p = paginator::Model::new();
//! p.per_page = 10;
//! p.set_total_pages(25);
//! assert_eq!(p.view(), "1/3");
//! p.next_page();
//! p.kind = Type::Dots;
//! assert_eq!(p.view(), "○•○");
//! assert_eq!(p.slice_bounds(25), (10, 20));
//! ```

use super::key::Binding;
use crate::{Cmd, Msg};

/// How pages are drawn: Bubbles' `paginator.Type`.
///
/// ```
/// use petitcarnets::bubbles::paginator::{self, Type};
///
/// let mut p = paginator::Model::new();
/// p.set_total_pages(3);
/// assert_eq!(p.kind, Type::Arabic);
/// assert_eq!(p.view(), "1/3");
/// p.kind = Type::Dots;
/// assert_eq!(p.view(), "•○○");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Type {
    /// Numbers, as `2/5`.
    #[default]
    Arabic,
    /// A dot a page.
    Dots,
}

/// The keys that turn pages: Bubbles' `paginator.KeyMap`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::paginator::KeyMap;
///
/// let keys = KeyMap::default();
/// assert!(keys.next_page.matches(&tea::Key::char('l')));
/// assert!(keys.prev_page.matches(&tea::Key::new(tea::KeyCode::PgUp)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// The page before.
    pub prev_page: Binding,
    /// The page after.
    pub next_page: Binding,
}

impl Default for KeyMap {
    /// Bubbles' `paginator.DefaultKeyMap`.
    fn default() -> Self {
        Self {
            prev_page: Binding::new(&["pgup", "left", "h"]),
            next_page: Binding::new(&["pgdown", "right", "l"]),
        }
    }
}

/// Pages: Bubbles' `paginator.Model`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::paginator;
///
/// let mut p = paginator::Model::new();
/// p.per_page = 3;
/// p.set_total_pages(7);
/// let _ = p.update::<()>(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Right)));
/// assert_eq!(p.view(), "2/3");
/// assert_eq!(p.slice_bounds(7), (3, 6));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// How pages are drawn.
    pub kind: Type,
    /// The page showing, from 0.
    pub page: usize,
    /// Items a page.
    pub per_page: usize,
    /// How many pages.
    pub total_pages: usize,
    /// The dot for the page showing.
    pub active_dot: String,
    /// The dot for the other pages.
    pub inactive_dot: String,
    /// The numbers' format: the first `%d` is the page, the second the
    /// total, as Go's `ArabicFormat`.
    pub arabic_format: String,
    /// The keys.
    pub key_map: KeyMap,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// One page of one item, drawn as numbers: `paginator.New`.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let p = paginator::Model::new();
    /// assert_eq!((p.page, p.per_page, p.total_pages), (0, 1, 1));
    /// assert_eq!(p.view(), "1/1");
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            kind: Type::Arabic,
            page: 0,
            per_page: 1,
            total_pages: 1,
            active_dot: "•".into(),
            inactive_dot: "○".into(),
            arabic_format: "%d/%d".into(),
            key_map: KeyMap::default(),
        }
    }

    /// Sets the number of pages for `items` items, and returns it;
    /// nothing changes for no items or no items a page.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.per_page = 3;
    /// assert_eq!(p.set_total_pages(7), 3);
    /// assert_eq!(p.set_total_pages(0), 3, "no items change nothing");
    /// ```
    pub const fn set_total_pages(&mut self, items: usize) -> usize {
        if items < 1 || self.per_page < 1 {
            return self.total_pages;
        }
        self.total_pages = items.div_ceil(self.per_page);
        self.total_pages
    }

    /// The items on the page showing, out of `total`.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.per_page = 3;
    /// p.set_total_pages(7);
    /// assert_eq!(p.items_on_page(7), 3);
    /// p.page = 2;
    /// assert_eq!(p.items_on_page(7), 1);
    /// ```
    #[must_use]
    pub fn items_on_page(&self, total: usize) -> usize {
        let (start, end) = self.slice_bounds(total);
        end.saturating_sub(start)
    }

    /// The range of items, out of `length`, on the page showing: Go's
    /// `GetSliceBounds`.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.per_page = 3;
    /// p.page = 2;
    /// let items = ["a", "b", "c", "d", "e", "f", "g"];
    /// let (start, end) = p.slice_bounds(items.len());
    /// assert_eq!(&items[start..end], ["g"]);
    /// ```
    #[must_use]
    pub fn slice_bounds(&self, length: usize) -> (usize, usize) {
        let start = self.page * self.per_page;
        (start.min(length), (start + self.per_page).min(length))
    }

    /// The page before, if any.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.set_total_pages(2);
    /// p.page = 1;
    /// p.prev_page();
    /// assert_eq!(p.page, 0);
    /// p.prev_page();
    /// assert_eq!(p.page, 0, "already on the first");
    /// ```
    pub const fn prev_page(&mut self) {
        self.page = self.page.saturating_sub(1);
    }

    /// The page after, if any.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.set_total_pages(2);
    /// p.next_page();
    /// assert_eq!(p.page, 1);
    /// p.next_page();
    /// assert_eq!(p.page, 1, "already on the last");
    /// ```
    pub const fn next_page(&mut self) {
        if !self.on_last_page() {
            self.page += 1;
        }
    }

    /// Whether the last page is showing.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.set_total_pages(2);
    /// assert!(!p.on_last_page());
    /// p.next_page();
    /// assert!(p.on_last_page());
    /// ```
    #[must_use]
    pub const fn on_last_page(&self) -> bool {
        self.page + 1 >= self.total_pages
    }

    /// Whether the first page is showing.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.set_total_pages(2);
    /// assert!(p.on_first_page());
    /// p.next_page();
    /// assert!(!p.on_first_page());
    /// ```
    #[must_use]
    pub const fn on_first_page(&self) -> bool {
        self.page == 0
    }

    /// Turns the page on its keys.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::paginator;
    ///
    /// let mut p = paginator::Model::new();
    /// p.set_total_pages(3);
    /// let next = tea::Msg::KeyPress(tea::Key::char('l'));
    /// assert!(p.update::<()>(&next).is_none());
    /// assert_eq!(p.page, 1);
    /// let _ = p.update::<()>(&tea::Msg::KeyPress(tea::Key::char('h')));
    /// assert_eq!(p.page, 0);
    /// ```
    pub fn update<M>(&mut self, msg: &Msg<M>) -> Cmd<M> {
        if let Msg::KeyPress(key) = msg {
            if self.key_map.next_page.matches(key) {
                self.next_page();
            } else if self.key_map.prev_page.matches(key) {
                self.prev_page();
            }
        }
        Cmd::none()
    }

    /// The pages as dots or numbers.
    ///
    /// ```
    /// use petitcarnets::bubbles::paginator::{self, Type};
    ///
    /// let mut p = paginator::Model::new();
    /// p.set_total_pages(4);
    /// p.page = 1;
    /// assert_eq!(p.view(), "2/4");
    /// p.kind = Type::Dots;
    /// assert_eq!(p.view(), "○•○○");
    /// ```
    #[must_use]
    pub fn view(&self) -> String {
        match self.kind {
            Type::Dots => (0..self.total_pages)
                .map(|i| {
                    if i == self.page {
                        self.active_dot.as_str()
                    } else {
                        self.inactive_dot.as_str()
                    }
                })
                .collect(),
            Type::Arabic => {
                let page = (self.page + 1).to_string();
                let total = self.total_pages.to_string();
                self.arabic_format
                    .replacen("%d", &page, 1)
                    .replacen("%d", &total, 1)
            }
        }
    }
}

#[cfg(test)]
mod tests;
