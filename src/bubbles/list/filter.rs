//! Filtering: the default fuzzy filter and the filter states.

use super::fuzzy;
use std::fmt;

/// Ranks a filter term against the items' filter values, best first:
/// Bubbles' `list.FilterFunc`.
pub type FilterFn = fn(&str, &[String]) -> Vec<Rank>;

/// An item that matched a filter: Bubbles' `list.Rank`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rank {
    /// The item's index among all items.
    pub index: usize,
    /// The matched characters' indexes in its filter value.
    pub matched_indexes: Vec<usize>,
}

/// Bubbles' default filter: fuzzy matching, best match first, ties in
/// item order.
#[must_use]
pub fn default_filter(term: &str, targets: &[String]) -> Vec<Rank> {
    ranks(fuzzy::find(term, targets), targets)
}

/// Fuzzy matching, in item order: Bubbles' `list.UnsortedFilter`.
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
