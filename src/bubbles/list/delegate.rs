//! The default delegate: a title and a description for each item, the
//! selected one marked with a bar, the filter's matches underlined.

use super::{DefaultItemStyles, FilterState, Item, ItemDelegate, Model};
use crate::style::{Style, truncate_with};

/// An item with a title and a description: Bubbles' `list.DefaultItem`.
///
/// ```
/// use petitcarnets::bubbles::list::{self, DefaultDelegate, DefaultItem, Item};
/// use petitcarnets::style::strip_ansi;
///
/// struct Task(&'static str, &'static str);
///
/// impl Item for Task {
///     fn filter_value(&self) -> String {
///         self.0.to_string()
///     }
/// }
///
/// impl DefaultItem for Task {
///     fn title(&self) -> String {
///         self.0.to_string()
///     }
///     fn description(&self) -> String {
///         self.1.to_string()
///     }
/// }
///
/// let l = list::Model::new(
///     vec![Task("Docs", "Write them")],
///     DefaultDelegate::new(),
///     30,
///     10,
/// );
/// let view = strip_ansi(&l.view());
/// let lines: Vec<&str> = view.lines().map(str::trim_end).collect();
/// assert_eq!(lines[4..6], ["│ Docs", "│ Write them"]);
/// ```
pub trait DefaultItem: Item {
    /// The title.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// use tea::bubbles::list::DefaultItem;
    ///
    /// let l = fruits();
    /// assert_eq!(l.items()[0].title(), "apple");
    /// assert!(
    ///     strip_ansi(&l.view()).contains("│ apple "),
    ///     "drawn by the default delegate"
    /// );
    /// ```
    fn title(&self) -> String;
    /// The description.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// use tea::bubbles::list::DefaultItem;
    ///
    /// let l = fruits();
    /// assert_eq!(
    ///     l.items()[0].description(),
    ///     "",
    ///     "drawn on the line under the title"
    /// );
    /// ```
    fn description(&self) -> String;
}

/// Draws [`DefaultItem`]s: Bubbles' `list.DefaultDelegate`. Go's
/// update and help functions are not ported; write a delegate of your
/// own for those.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::list;
/// use tea::style::strip_ansi;
/// # struct Fruit(&'static str);
/// #
/// # impl list::Item for Fruit {
/// #     fn filter_value(&self) -> String {
/// #         self.0.to_string()
/// #     }
/// # }
/// #
/// # impl list::DefaultItem for Fruit {
/// #     fn title(&self) -> String {
/// #         self.0.to_string()
/// #     }
/// #     fn description(&self) -> String {
/// #         String::new()
/// #     }
/// # }
/// #
/// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
/// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
/// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
/// # }
///
/// let mut l = fruits();
/// let mut d = list::DefaultDelegate::new();
/// d.show_description = false;
/// l.set_delegate(d);
/// let view = strip_ansi(&l.view());
/// let lines: Vec<&str> = view.lines().map(str::trim_end).collect();
/// assert_eq!(lines[4..9], ["│ apple", "", "  banana", "", "  cherry"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultDelegate {
    /// Whether to show descriptions under the titles.
    pub show_description: bool,
    /// The styles.
    pub styles: DefaultItemStyles,
    height: usize,
    spacing: usize,
}

impl Default for DefaultDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultDelegate {
    /// Two lines an item, a blank line between: `list.NewDefaultDelegate`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// use tea::bubbles::list::ItemDelegate;
    ///
    /// let d = list::DefaultDelegate::new();
    /// assert!(d.show_description);
    /// assert_eq!(ItemDelegate::<Fruit>::height(&d), 2);
    /// assert_eq!(ItemDelegate::<Fruit>::spacing(&d), 1);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            show_description: true,
            styles: DefaultItemStyles::new(true),
            height: 2,
            spacing: 1,
        }
    }

    /// Lines an item takes with its description.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// use tea::bubbles::list::ItemDelegate;
    ///
    /// let mut d = list::DefaultDelegate::new();
    /// d.set_height(3);
    /// assert_eq!(ItemDelegate::<Fruit>::height(&d), 3);
    /// d.show_description = false;
    /// assert_eq!(ItemDelegate::<Fruit>::height(&d), 1, "only the title");
    /// ```
    pub const fn set_height(&mut self, lines: usize) {
        self.height = lines;
    }

    /// Blank lines between items.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// let mut d = list::DefaultDelegate::new();
    /// d.show_description = false;
    /// d.set_spacing(0);
    /// l.set_delegate(d);
    /// let view = strip_ansi(&l.view());
    /// let lines: Vec<&str> = view.lines().map(str::trim_end).collect();
    /// assert_eq!(lines[4..7], ["│ apple", "  banana", "  cherry"]);
    /// ```
    pub const fn set_spacing(&mut self, lines: usize) {
        self.spacing = lines;
    }
}

impl<I: DefaultItem> ItemDelegate<I> for DefaultDelegate {
    fn height(&self) -> usize {
        if self.show_description {
            self.height
        } else {
            1
        }
    }

    fn spacing(&self) -> usize {
        self.spacing
    }

    fn render(&self, list: &Model<I, Self>, index: usize, item: &I) -> String {
        if list.width() == 0 {
            return String::new();
        }
        let (title, desc) = self.texts(list.width(), item);
        let (title_style, desc_style, highlight) = self.item_styles(list, index);
        let title = if highlight {
            let unmatched = title_style.plain();
            let matched = unmatched.inherit(self.styles.filter_match);
            style_chars(&title, list.matches_for_item(index), matched, unmatched)
        } else {
            title
        };
        let title = title_style.render(&title);
        if self.show_description {
            format!("{title}\n{}", desc_style.render(&desc))
        } else {
            title
        }
    }
}

impl DefaultDelegate {
    /// An item's title and description, cut to `width` less the padding.
    fn texts(&self, width: usize, item: &impl DefaultItem) -> (String, String) {
        let pad = &self.styles.normal_title;
        let room = width.saturating_sub(pad.get_padding_left() + pad.get_padding_right());
        let title = truncate_with(&item.title(), room, "…");
        if !self.show_description {
            return (title, String::new());
        }
        let desc = item.description();
        let lines: Vec<String> = desc
            .split('\n')
            .take(self.height.saturating_sub(1))
            .map(|l| truncate_with(l, room, "…"))
            .collect();
        (title, lines.join("\n"))
    }

    /// The title's and description's styles for item `index`, and
    /// whether the filter's matches are underlined.
    fn item_styles<I: DefaultItem>(
        &self,
        list: &Model<I, Self>,
        index: usize,
    ) -> (Style, Style, bool) {
        let s = &self.styles;
        let state = list.filter_state();
        if state == FilterState::Filtering && list.filter_value().is_empty() {
            return (s.dimmed_title, s.dimmed_desc, false);
        }
        let filtered = state != FilterState::Unfiltered;
        if index == list.index() && state != FilterState::Filtering {
            (s.selected_title, s.selected_desc, filtered)
        } else {
            (s.normal_title, s.normal_desc, filtered)
        }
    }
}

/// `text` with the characters at `indexes` in `matched` and the rest in
/// `unmatched`, a run at a time: Lip Gloss's `StyleRunes`.
fn style_chars(text: &str, indexes: &[usize], matched: Style, unmatched: Style) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let mut run_matched = false;
    for (i, c) in text.chars().enumerate() {
        let is = indexes.contains(&i);
        if is != run_matched && !run.is_empty() {
            out += &(if run_matched { matched } else { unmatched }).paint(&run);
            run.clear();
        }
        run_matched = is;
        run.push(c);
    }
    if !run.is_empty() {
        out += &(if run_matched { matched } else { unmatched }).paint(&run);
    }
    out
}
