//! The items: setting, inserting and removing them, which show, and
//! which is selected.

use super::{FilterState, Item, ItemDelegate, Model};

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// The items.
    #[must_use]
    pub fn items(&self) -> &[I] {
        &self.items
    }

    /// Replaces the items, filtered again if a filter is set.
    pub fn set_items(&mut self, items: Vec<I>) {
        self.items = items;
        self.refilter();
        self.update_pagination();
        self.update_keybindings();
    }

    /// Replaces item `index`.
    pub fn set_item(&mut self, index: usize, item: I) {
        if let Some(slot) = self.items.get_mut(index) {
            *slot = item;
        }
        self.refilter();
        self.update_pagination();
    }

    /// Inserts `item` at `index`, or at the end.
    pub fn insert_item(&mut self, index: usize, item: I) {
        self.items.insert(index.min(self.items.len()), item);
        self.refilter();
        self.update_pagination();
        self.update_keybindings();
    }

    /// Removes item `index`, if there is one.
    pub fn remove_item(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        let _ = self.items.remove(index);
        if self.filter_state != FilterState::Unfiltered {
            self.filtered.retain(|f| f.index != index);
            for f in &mut self.filtered {
                if f.index > index {
                    f.index -= 1;
                }
            }
            if self.filtered.is_empty() {
                self.reset_filtering();
            }
        }
        self.update_pagination();
    }

    /// Replaces the delegate.
    pub fn set_delegate(&mut self, delegate: D) {
        self.delegate = delegate;
        self.update_pagination();
    }

    /// The delegate.
    #[must_use]
    pub const fn delegate(&self) -> &D {
        &self.delegate
    }

    /// The items showing: the filter's matches, or all of them.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&I> {
        if self.filter_state == FilterState::Unfiltered {
            return self.items.iter().collect();
        }
        self.filtered.iter().map(|f| &self.items[f.index]).collect()
    }

    /// The selected item.
    #[must_use]
    pub fn selected_item(&self) -> Option<&I> {
        self.visible_items().get(self.index()).copied()
    }

    /// The characters the filter matched in visible item `index`.
    #[must_use]
    pub fn matches_for_item(&self, index: usize) -> &[usize] {
        self.filtered.get(index).map_or(&[], |f| &f.matches)
    }

    /// The selected item's index among the visible items.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.paginator.page * self.paginator.per_page + self.cursor
    }

    /// The selected item's index among all items.
    #[must_use]
    pub fn global_index(&self) -> usize {
        let index = self.index();
        match self.filtered.get(index) {
            Some(f) if self.filter_state != FilterState::Unfiltered => f.index,
            _ => index,
        }
    }

    /// The selected item's place on the page.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Selects visible item `index`.
    pub const fn select(&mut self, index: usize) {
        self.paginator.page = index / self.paginator.per_page;
        self.cursor = index % self.paginator.per_page;
    }

    /// Selects the first item.
    pub const fn reset_selected(&mut self) {
        self.select(0);
    }
}
