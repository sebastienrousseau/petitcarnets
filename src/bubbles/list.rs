//! A list of items, drawn by a delegate, browsed by page, filtered with
//! fuzzy matching as you type, with a title, a status bar, a spinner,
//! status messages and help: Bubbles' `list` package.
//!
//! Go filters on another goroutine and sends the matches back as a
//! message; here filtering is done in `update`, so the matches always
//! belong to the filter showing (Go can show an older filter's matches
//! when they arrive late).
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::list::{self, DefaultDelegate, DefaultItem, Item};
//!
//! struct Fruit(&'static str);
//!
//! impl Item for Fruit {
//!     fn filter_value(&self) -> String {
//!         self.0.to_string()
//!     }
//! }
//!
//! impl DefaultItem for Fruit {
//!     fn title(&self) -> String {
//!         self.0.to_string()
//!     }
//!     fn description(&self) -> String {
//!         String::new()
//!     }
//! }
//!
//! let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
//! let mut l = list::Model::new(items, DefaultDelegate::new(), 30, 14);
//! l.set_filter_text("an");
//! assert_eq!(l.visible_items().len(), 1);
//! assert_eq!(l.selected_item().map(|f| f.0), Some("banana"));
//! ```

mod delegate;
mod filter;
mod filtering;
mod fuzzy;
mod items;
mod keymap;
mod show;
mod size;
mod status;
mod style;
mod traits;
mod update;
mod view;

pub use delegate::{DefaultDelegate, DefaultItem};
pub use filter::{FilterFn, FilterState, Rank, default_filter, unsorted_filter};
pub use keymap::KeyMap;
pub use style::{DefaultItemStyles, Styles};
pub use traits::{Item, ItemDelegate};

use super::key::Binding;
use super::{Flags, help, paginator, spinner, textinput};
use std::sync::Arc;
use std::time::Duration;

/// A list's messages.
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
/// let _later = l.new_status_message("Saved");
/// assert!(strip_ansi(&l.view()).contains("Saved"));
/// // The first status message's time is up.
/// let _ = l.update(&tea::Msg::App(list::Msg::StatusTimeout(1)));
/// assert!(!strip_ansi(&l.view()).contains("Saved"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    /// The spinner's tick.
    Spinner(spinner::TickMsg),
    /// The filter input's cursor.
    Input(textinput::Msg),
    /// A status message's time is up (its number, so a newer one stays).
    StatusTimeout(u64),
}

/// An item the filter kept: its index among all items, and the matched
/// characters.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Filtered {
    index: usize,
    matches: Vec<usize>,
}

/// A [`Model`] setting's bit.
const SHOW_TITLE: u16 = 1 << 0;
/// A [`Model`] setting's bit.
const SHOW_FILTER: u16 = 1 << 1;
/// A [`Model`] setting's bit.
const SHOW_STATUS_BAR: u16 = 1 << 2;
/// A [`Model`] setting's bit.
const SHOW_PAGINATION: u16 = 1 << 3;
/// A [`Model`] setting's bit.
const SHOW_HELP: u16 = 1 << 4;
/// A [`Model`] setting's bit.
const FILTERING_ENABLED: u16 = 1 << 5;
/// A [`Model`] setting's bit.
const DISABLE_QUIT_KEYBINDINGS: u16 = 1 << 6;
/// A [`Model`] setting's bit.
const SHOW_SPINNER: u16 = 1 << 7;

/// A list: Bubbles' `list.Model`.
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
/// let key = |c| tea::Msg::KeyPress(tea::Key::char(c));
/// let mut l = fruits();
/// for c in ['/', 'c', 'h'] {
///     let _ = l.update(&key(c));
/// }
/// let _ = l.update(&tea::Msg::KeyPress(tea::Key::new(tea::KeyCode::Enter)));
/// assert!(l.is_filtered());
/// assert_eq!(l.selected_item().map(|f| f.0), Some("cherry"));
/// ```
#[derive(Clone)]
pub struct Model<I, D> {
    /// The title.
    pub title: String,
    /// The styles.
    pub styles: Styles,
    /// Whether moving past either end wraps round.
    pub infinite_scrolling: bool,
    /// The keys.
    pub key_map: KeyMap,
    /// The filter.
    pub filter: FilterFn,
    /// More bindings for the short help: Bubbles'
    /// `AdditionalShortHelpKeys`, a closure that may capture state.
    pub additional_short_help_keys: Option<Arc<dyn Fn() -> Vec<Binding> + Send + Sync>>,
    /// More bindings for the full help: Bubbles'
    /// `AdditionalFullHelpKeys`, a closure that may capture state.
    pub additional_full_help_keys: Option<Arc<dyn Fn() -> Vec<Binding> + Send + Sync>>,
    /// The pages.
    pub paginator: paginator::Model,
    /// The help.
    pub help: help::Model,
    /// The filter input.
    pub filter_input: textinput::Model,
    /// How long a status message shows.
    pub status_message_lifetime: Duration,
    item_name_singular: String,
    item_name_plural: String,
    spinner: spinner::Model,
    width: usize,
    height: usize,
    cursor: usize,
    filter_state: FilterState,
    status_message: String,
    status_tag: u64,
    items: Vec<I>,
    filtered: Vec<Filtered>,
    delegate: D,
    flags: Flags,
}

impl<I: std::fmt::Debug, D: std::fmt::Debug> std::fmt::Debug for Model<I, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("title", &self.title)
            .field("styles", &self.styles)
            .field("infinite_scrolling", &self.infinite_scrolling)
            .field("key_map", &self.key_map)
            .field("filter", &super::Opaque)
            .field(
                "additional_short_help_keys",
                &super::opaque(self.additional_short_help_keys.as_ref()),
            )
            .field(
                "additional_full_help_keys",
                &super::opaque(self.additional_full_help_keys.as_ref()),
            )
            .field("paginator", &self.paginator)
            .field("help", &self.help)
            .field("filter_input", &self.filter_input)
            .field("status_message_lifetime", &self.status_message_lifetime)
            .field("item_name_singular", &self.item_name_singular)
            .field("item_name_plural", &self.item_name_plural)
            .field("spinner", &self.spinner)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("cursor", &self.cursor)
            .field("filter_state", &self.filter_state)
            .field("status_message", &self.status_message)
            .field("status_tag", &self.status_tag)
            .field("items", &self.items)
            .field("filtered", &self.filtered)
            .field("delegate", &self.delegate)
            .field("flags", &self.flags)
            .finish()
    }
}

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// A list of `items` drawn by `delegate`, `width` by `height`:
    /// `list.New`.
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
    /// let l = fruits();
    /// assert_eq!(l.title, "List");
    /// assert_eq!(l.items().len(), 3);
    /// assert_eq!((l.width(), l.height()), (30, 14));
    /// assert!(strip_ansi(&l.view()).contains("3 items"));
    /// ```
    #[must_use]
    pub fn new(items: Vec<I>, delegate: D, width: usize, height: usize) -> Self {
        let styles = Styles::new(true);
        let mut filter_input = textinput::Model::new();
        filter_input.prompt = "Filter: ".into();
        filter_input.char_limit = 64;
        filter_input.set_styles(styles.filter);
        let _ = filter_input.focus();
        let mut paginator = paginator::Model::new();
        paginator.kind = paginator::Type::Dots;
        paginator.active_dot = styles.active_pagination_dot.paint("•");
        paginator.inactive_dot = styles.inactive_pagination_dot.paint("•");
        let mut m = Self {
            title: "List".into(),
            styles,
            infinite_scrolling: false,
            key_map: KeyMap::default(),
            filter: Arc::new(default_filter),
            additional_short_help_keys: None,
            additional_full_help_keys: None,
            paginator,
            help: help::Model::new(),
            filter_input,
            status_message_lifetime: Duration::from_secs(1),
            item_name_singular: "item".into(),
            item_name_plural: "items".into(),
            spinner: spinner::Model::new().with_style(styles.spinner),
            width,
            height,
            cursor: 0,
            filter_state: FilterState::Unfiltered,
            status_message: String::new(),
            status_tag: 0,
            items,
            filtered: Vec::new(),
            delegate,
            flags: Flags(
                SHOW_TITLE
                    | SHOW_FILTER
                    | SHOW_STATUS_BAR
                    | SHOW_PAGINATION
                    | SHOW_HELP
                    | FILTERING_ENABLED,
            ),
        };
        m.update_pagination();
        m.update_keybindings();
        m
    }
}

#[cfg(test)]
mod tests;
