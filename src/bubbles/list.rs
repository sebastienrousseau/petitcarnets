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
mod fuzzy;
mod items;
mod keymap;
mod style;
mod update;
mod view;

pub use delegate::{DefaultDelegate, DefaultItem};
pub use filter::{FilterFn, FilterState, Rank, default_filter, unsorted_filter};
pub use keymap::KeyMap;
pub use style::{DefaultItemStyles, Styles};

use super::key::Binding;
use super::{Flags, help, paginator, spinner, textinput};
use crate::{Cmd, Msg as TeaMsg};
use std::time::Duration;

/// Something a list holds: Bubbles' `list.Item`.
pub trait Item {
    /// The text the filter matches against.
    fn filter_value(&self) -> String;
}

/// Draws and updates a list's items: Bubbles' `list.ItemDelegate`. The
/// list holds its delegate; it is cloned for [`ItemDelegate::update`],
/// which gets the list to change.
pub trait ItemDelegate<I>: Clone {
    /// Item `index` (among the visible items), as text without a final
    /// line break, [`ItemDelegate::height`] lines high.
    fn render(&self, list: &Model<I, Self>, index: usize, item: &I) -> String;
    /// The lines each item takes.
    fn height(&self) -> usize;
    /// The blank lines between items.
    fn spacing(&self) -> usize;
    /// Reacts to a message after the list has; nothing by default.
    fn update(&self, _msg: &TeaMsg<Msg>, _list: &mut Model<I, Self>) -> Cmd<Msg> {
        Cmd::none()
    }
    /// More bindings for the short help.
    fn short_help(&self) -> Vec<Binding> {
        Vec::new()
    }
    /// More bindings for the full help.
    fn full_help(&self) -> Vec<Vec<Binding>> {
        Vec::new()
    }
}

/// A list's messages.
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
#[derive(Debug, Clone)]
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
    /// More bindings for the short help.
    pub additional_short_help_keys: Option<fn() -> Vec<Binding>>,
    /// More bindings for the full help.
    pub additional_full_help_keys: Option<fn() -> Vec<Binding>>,
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

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// A list of `items` drawn by `delegate`, `width` by `height`:
    /// `list.New`.
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
            filter: default_filter,
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

    /// Turns filtering on or off; off drops any filter.
    pub fn set_filtering_enabled(&mut self, on: bool) {
        self.flags.set(FILTERING_ENABLED, on);
        if !on {
            self.reset_filtering();
        }
        self.update_keybindings();
    }

    /// Whether filtering is on.
    #[must_use]
    pub const fn filtering_enabled(&self) -> bool {
        self.flags.get(FILTERING_ENABLED)
    }

    /// Shows or hides the title.
    pub fn set_show_title(&mut self, on: bool) {
        self.flags.set(SHOW_TITLE, on);
        self.update_pagination();
    }

    /// Whether the title shows.
    #[must_use]
    pub const fn show_title(&self) -> bool {
        self.flags.get(SHOW_TITLE)
    }

    /// Shows or hides the filter input.
    pub fn set_show_filter(&mut self, on: bool) {
        self.flags.set(SHOW_FILTER, on);
        self.update_pagination();
    }

    /// Whether the filter input shows.
    #[must_use]
    pub const fn show_filter(&self) -> bool {
        self.flags.get(SHOW_FILTER)
    }

    /// Shows or hides the status bar.
    pub fn set_show_status_bar(&mut self, on: bool) {
        self.flags.set(SHOW_STATUS_BAR, on);
        self.update_pagination();
    }

    /// Whether the status bar shows.
    #[must_use]
    pub const fn show_status_bar(&self) -> bool {
        self.flags.get(SHOW_STATUS_BAR)
    }

    /// The status bar's name for one item and for several.
    pub fn set_status_bar_item_name(&mut self, singular: &str, plural: &str) {
        singular.clone_into(&mut self.item_name_singular);
        plural.clone_into(&mut self.item_name_plural);
    }

    /// The status bar's names for items.
    #[must_use]
    pub fn status_bar_item_name(&self) -> (&str, &str) {
        (&self.item_name_singular, &self.item_name_plural)
    }

    /// Shows or hides the pagination.
    pub fn set_show_pagination(&mut self, on: bool) {
        self.flags.set(SHOW_PAGINATION, on);
        self.update_pagination();
    }

    /// Whether the pagination shows.
    #[must_use]
    pub const fn show_pagination(&self) -> bool {
        self.flags.get(SHOW_PAGINATION)
    }

    /// Shows or hides the help.
    pub fn set_show_help(&mut self, on: bool) {
        self.flags.set(SHOW_HELP, on);
        self.update_pagination();
    }

    /// Whether the help shows.
    #[must_use]
    pub const fn show_help(&self) -> bool {
        self.flags.get(SHOW_HELP)
    }

    /// Drops the filter.
    pub fn reset_filter(&mut self) {
        self.reset_filtering();
    }

    /// Whether and how it is filtered.
    #[must_use]
    pub const fn filter_state(&self) -> FilterState {
        self.filter_state
    }

    /// The filter.
    #[must_use]
    pub fn filter_value(&self) -> String {
        self.filter_input.value()
    }

    /// Whether the filter is being typed.
    #[must_use]
    pub fn setting_filter(&self) -> bool {
        self.filter_state == FilterState::Filtering
    }

    /// Whether a filter is set.
    #[must_use]
    pub fn is_filtered(&self) -> bool {
        self.filter_state == FilterState::FilterApplied
    }

    /// The width.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// The height.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Sets the width.
    pub fn set_width(&mut self, columns: usize) {
        self.set_size(columns, self.height);
    }

    /// Sets the height.
    pub fn set_height(&mut self, rows: usize) {
        self.set_size(self.width, rows);
    }

    /// Sets the size.
    pub fn set_size(&mut self, width: usize, height: usize) {
        let prompt = crate::style::width(&self.styles.title.render(&self.filter_input.prompt));
        self.width = width;
        self.height = height;
        self.help.set_width(width);
        let spinner = crate::style::width(&self.spinner.view());
        self.filter_input
            .set_width(width.saturating_sub(prompt + spinner));
        self.update_pagination();
        self.update_keybindings();
    }

    /// Replaces the spinner's frames.
    pub const fn set_spinner(&mut self, spinner: spinner::Spinner) {
        self.spinner.spinner = spinner;
    }

    /// Starts the spinner if it is stopped, else stops it.
    pub fn toggle_spinner(&mut self) -> Cmd<Msg> {
        if self.flags.get(SHOW_SPINNER) {
            self.stop_spinner();
            Cmd::none()
        } else {
            self.start_spinner()
        }
    }

    /// Shows the spinner; the command starts it.
    pub fn start_spinner(&mut self) -> Cmd<Msg> {
        self.flags.set(SHOW_SPINNER, true);
        self.spinner.tick().map(Msg::Spinner)
    }

    /// Hides the spinner.
    pub const fn stop_spinner(&mut self) {
        self.flags.set(SHOW_SPINNER, false);
    }

    /// Turns the quit keys off.
    pub const fn disable_quit_keybindings(&mut self) {
        self.flags.set(DISABLE_QUIT_KEYBINDINGS, true);
        self.key_map.quit.set_enabled(false);
        self.key_map.force_quit.set_enabled(false);
    }

    /// Shows `message` by the title for [`Model::status_message_lifetime`].
    pub fn new_status_message(&mut self, message: &str) -> Cmd<Msg> {
        message.clone_into(&mut self.status_message);
        self.status_tag += 1;
        let tag = self.status_tag;
        crate::tick(self.status_message_lifetime, move |_| {
            Msg::StatusTimeout(tag)
        })
    }
}

#[cfg(test)]
mod tests;
