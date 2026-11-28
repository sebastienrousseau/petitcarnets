//! Filtering: the default fuzzy filter and the filter states.

use super::fuzzy;
use std::fmt;
use std::sync::Arc;

/// Ranks a filter term against the items' filter values, best first:
/// Bubbles' `list.FilterFunc`. A closure, so it may capture state;
/// [`default_filter`] and [`unsorted_filter`] are plain `fn`s, wrapped
/// with `Arc::new`.
///
/// ```
/// use petitcarnets as tea;
/// use std::sync::Arc;
/// use tea::bubbles::list::{self, FilterFn, Rank};
///
/// let fuzzy: FilterFn = Arc::new(list::default_filter);
/// let at_least = 2;
/// let prefix: FilterFn = Arc::new(move |term: &str, targets: &[String]| {
///     if term.len() < at_least {
///         return Vec::new();
///     }
///     let hits = targets.iter().enumerate().filter(|(_, t)| t.starts_with(term));
///     hits.map(|(index, _)| Rank { index, matched_indexes: (0..term.len()).collect() })
///         .collect()
/// });
/// let fruits = ["apple".to_string(), "apricot".to_string()];
/// assert_eq!(prefix("a", &fruits), []);
/// assert_eq!(prefix("ap", &fruits).len(), 2);
/// assert_eq!(fuzzy("ap", &fruits).len(), 2);
/// ```
pub type FilterFn = Arc<dyn Fn(&str, &[String]) -> Vec<Rank> + Send + Sync>;

/// An item that matched a filter: Bubbles' `list.Rank`.
///
/// ```
/// use petitcarnets::bubbles::list::{self, Rank};
///
/// let fruits = [
///     "apple".to_string(),
///     "banana".to_string(),
///     "cherry".to_string(),
/// ];
/// let ranks = list::default_filter("an", &fruits);
/// assert_eq!(
///     ranks,
///     [Rank {
///         index: 1,
///         matched_indexes: vec![1, 2]
///     }]
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rank {
    /// The item's index among all items.
    pub index: usize,
    /// The matched characters' indexes in its filter value.
    pub matched_indexes: Vec<usize>,
}

/// Bubbles' default filter: fuzzy matching, best match first, ties in
/// item order.
///
/// ```
/// use petitcarnets::bubbles::list;
///
/// let targets = ["papa".to_string(), "apple".to_string()];
/// let ranks = list::default_filter("ap", &targets);
/// let order: Vec<usize> = ranks.iter().map(|r| r.index).collect();
/// assert_eq!(order, [1, 0], "the better match first");
/// ```
#[must_use]
pub fn default_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    ranks(fuzzy::find(term, targets), targets)
}

/// Fuzzy matching, in item order: Bubbles' `list.UnsortedFilter`.
///
/// ```
/// use petitcarnets::bubbles::list;
///
/// let targets = ["papa".to_string(), "apple".to_string()];
/// let ranks = list::unsorted_filter("ap", &targets);
/// let order: Vec<usize> = ranks.iter().map(|r| r.index).collect();
/// assert_eq!(order, [0, 1], "in item order");
/// assert_eq!(ranks[0].matched_indexes, [1, 2]);
/// ```
#[must_use]
pub fn unsorted_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    ranks(fuzzy::find_no_sort(term, targets), targets)
}

/// Matches as ranks, byte offsets made character indexes.
fn ranks(matches: Vec<fuzzy::Match>, targets: &[String]) -> Vec<Rank> {
    matches
        .into_iter()
        .map(|m| {
            let target = &targets[m.index];
            let matched_indexes = m
                .matched
                .iter()
                .map(|&b| target[..b].chars().count())
                .collect();
            Rank {
                index: m.index,
                matched_indexes,
            }
        })
        .collect()
}

/// Whether and how a list is filtered: Bubbles' `list.FilterState`.
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
/// use tea::bubbles::list::FilterState;
///
/// let mut l = fruits();
/// assert_eq!(l.filter_state(), FilterState::Unfiltered);
/// let _ = l.update(&key('/'));
/// assert_eq!(l.filter_state(), FilterState::Filtering);
/// assert_eq!(FilterState::FilterApplied.to_string(), "filter applied");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FilterState {
    /// No filter.
    #[default]
    Unfiltered,
    /// The filter is being typed.
    Filtering,
    /// A filter is set.
    FilterApplied,
}

impl fmt::Display for FilterState {
    /// As Go's `FilterState.String`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unfiltered => "unfiltered",
            Self::Filtering => "filtering",
            Self::FilterApplied => "filter applied",
        })
    }
}
