//! The list's update: browsing, filtering, moving, and keeping pages and
//! key bindings in step.

use super::{
    DISABLE_QUIT_KEYBINDINGS, FILTERING_ENABLED, FilterState, Filtered, Item, ItemDelegate, KeyMap,
    Model, Msg, SHOW_FILTER, SHOW_HELP, SHOW_PAGINATION, SHOW_SPINNER, SHOW_STATUS_BAR, SHOW_TITLE,
};
use crate::bubbles::help::KeyMap as HelpKeyMap;
use crate::bubbles::key::Binding;
use crate::bubbles::textinput;
use crate::{Cmd, Key, Msg as TeaMsg, batch, quit};

/// A binding of the key map.
type Field = fn(&KeyMap) -> &Binding;

/// What a key does while browsing; `Some` ends the update there.
type Browse<I, D> = fn(&mut Model<I, D>) -> Option<Cmd<Msg>>;

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// Browses or filters on its keys, spins its spinner, and times out
    /// status messages; Ctrl-C quits.
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        let mut cmds = Vec::new();
        match msg {
            TeaMsg::KeyPress(k) if self.key_map.force_quit.matches(k) => return quit(),
            TeaMsg::App(Msg::Spinner(tick)) => {
                let cmd = self.spinner.update(&TeaMsg::App(*tick)).map(Msg::Spinner);
                if self.flags.get(SHOW_SPINNER) {
                    cmds.push(cmd);
                }
            }
            TeaMsg::App(Msg::StatusTimeout(tag)) if *tag == self.status_tag => {
                self.hide_status_message();
            }
            _ => {}
        }
        if self.filter_state == FilterState::Filtering {
            cmds.push(self.handle_filtering(msg));
        } else {
            cmds.push(self.handle_browsing(msg));
        }
        batch(cmds)
    }

    /// Clears the status message.
    fn hide_status_message(&mut self) {
        self.status_message.clear();
        self.status_tag += 1;
    }

    /// A message while browsing.
    fn handle_browsing(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        if let TeaMsg::KeyPress(key) = msg {
            if let Some(cmd) = self.browse_key(key) {
                return cmd;
            }
        }
        let delegate = self.delegate.clone();
        let cmd = delegate.update(msg, self);
        self.cursor = self.cursor.min(self.max_cursor_index());
        cmd
    }

    /// What each key does while browsing, in the order Bubbles tries
    /// them.
    const BROWSE: [(Field, Browse<I, D>); 11] = [
        (|k| &k.clear_filter, |m| m.then(Self::reset_filtering)),
        (|k| &k.quit, |_| Some(quit())),
        (|k| &k.cursor_up, |m| m.then(Self::cursor_up)),
        (|k| &k.cursor_down, |m| m.then(Self::cursor_down)),
        (|k| &k.prev_page, |m| m.then(|m| m.paginator.prev_page())),
        (|k| &k.next_page, |m| m.then(|m| m.paginator.next_page())),
        (|k| &k.go_to_start, |m| m.then(Self::go_to_start)),
        (|k| &k.go_to_end, |m| m.then(Self::go_to_end)),
        (|k| &k.filter, |m| Some(m.start_filtering())),
        (|k| &k.show_full_help, |m| m.then(Self::toggle_help)),
        (|k| &k.close_full_help, |m| m.then(Self::toggle_help)),
    ];

    /// A key while browsing; `Some` ends the update there.
    fn browse_key(&mut self, key: &Key) -> Option<Cmd<Msg>> {
        let action = Self::BROWSE
            .iter()
            .find(|(b, _)| b(&self.key_map).matches(key));
        action.and_then(|(_, act)| act(self))
    }

    /// Does `f`; the update goes on.
    fn then(&mut self, f: impl FnOnce(&mut Self)) -> Option<Cmd<Msg>> {
        f(self);
        None
    }

    /// Shows or hides the full help.
    fn toggle_help(&mut self) {
        self.help.show_all = !self.help.show_all;
        self.update_pagination();
    }

    /// Starts typing a filter.
    fn start_filtering(&mut self) -> Cmd<Msg> {
        self.hide_status_message();
        if self.filter_input.value().is_empty() {
            self.filtered = self.all_items();
        }
        self.go_to_start();
        self.filter_state = FilterState::Filtering;
        self.filter_input.cursor_end();
        let _ = self.filter_input.focus();
        self.update_keybindings();
        textinput::blink().map(Msg::Input)
    }

    /// A message while the filter is typed.
    fn handle_filtering(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        if let TeaMsg::KeyPress(key) = msg {
            if self.key_map.cancel_while_filtering.matches(key) {
                self.reset_filtering();
                self.key_map.filter.set_enabled(true);
                self.key_map.clear_filter.set_enabled(false);
            } else if self.key_map.accept_while_filtering.matches(key) {
                self.accept_filter();
            }
        }
        let before = self.filter_input.value();
        let routed = msg.route(|m| match m {
            Msg::Input(c) => Some(*c),
            _ => None,
        });
        let cmd = routed.map_or_else(Cmd::none, |m| self.filter_input.update(&m).map(Msg::Input));
        if self.filter_input.value() != before {
            self.filtered = self.filter_items();
            let typed = !self.filter_input.value().is_empty();
            self.key_map.accept_while_filtering.set_enabled(typed);
        }
        self.update_pagination();
        cmd
    }

    /// Keeps the filter typed, if anything matched.
    fn accept_filter(&mut self) {
        self.hide_status_message();
        if self.items.is_empty() {
            return;
        }
        if self.visible_items().is_empty() {
            self.reset_filtering();
            return;
        }
        self.filter_input.blur();
        self.filter_state = FilterState::FilterApplied;
        self.update_keybindings();
        if self.filter_input.value().is_empty() {
            self.reset_filtering();
        }
    }

    /// Sets the filter as if typed and accepted.
    pub fn set_filter_text(&mut self, filter: &str) {
        self.filter_state = FilterState::Filtering;
        self.filter_input.set_value(filter);
        self.filtered = self.filter_items();
        self.filter_state = FilterState::FilterApplied;
        self.go_to_start();
        self.filter_input.cursor_end();
        self.update_pagination();
        self.update_keybindings();
    }

    /// Sets the filter state.
    pub fn set_filter_state(&mut self, state: FilterState) {
        self.go_to_start();
        self.filter_state = state;
        self.filter_input.cursor_end();
        let _ = self.filter_input.focus();
        self.update_keybindings();
    }

    /// Filters again after the items changed.
    pub(super) fn refilter(&mut self) {
        if self.filter_state != FilterState::Unfiltered {
            self.filtered = self.filter_items();
        }
    }

    /// Every item, unfiltered.
    fn all_items(&self) -> Vec<Filtered> {
        (0..self.items.len())
            .map(|index| Filtered {
                index,
                matches: Vec::new(),
            })
            .collect()
    }

    /// The items the filter keeps, best first.
    fn filter_items(&self) -> Vec<Filtered> {
        let term = self.filter_input.value();
        if term.is_empty() || self.filter_state == FilterState::Unfiltered {
            return self.all_items();
        }
        let targets: Vec<String> = self.items.iter().map(Item::filter_value).collect();
        (self.filter)(&term, &targets)
            .into_iter()
            .map(|r| Filtered {
                index: r.index,
                matches: r.matched_indexes,
            })
            .collect()
    }

    /// Drops the filter.
    pub(super) fn reset_filtering(&mut self) {
        if self.filter_state == FilterState::Unfiltered {
            return;
        }
        self.filter_state = FilterState::Unfiltered;
        self.filter_input.reset();
        self.filtered.clear();
        self.update_pagination();
        self.update_keybindings();
    }

    /// The item above, maybe on the page before.
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            return;
        }
        if self.paginator.on_first_page() {
            if self.infinite_scrolling {
                self.go_to_end();
            }
            return;
        }
        self.paginator.prev_page();
        self.cursor = self.max_cursor_index();
    }

    /// The item below, maybe on the page after.
    pub fn cursor_down(&mut self) {
        let most = self.max_cursor_index();
        if self.cursor < most {
            self.cursor += 1;
            return;
        }
        if !self.paginator.on_last_page() {
            self.paginator.next_page();
            self.cursor = 0;
            return;
        }
        self.cursor = most;
        if self.infinite_scrolling {
            self.go_to_start();
        }
    }

    /// The first item.
    pub const fn go_to_start(&mut self) {
        self.paginator.page = 0;
        self.cursor = 0;
    }

    /// The last item.
    pub fn go_to_end(&mut self) {
        self.paginator.page = self.paginator.total_pages.saturating_sub(1);
        self.cursor = self.max_cursor_index();
    }

    /// The page before.
    pub fn prev_page(&mut self) {
        self.paginator.prev_page();
        self.cursor = self.cursor.min(self.max_cursor_index());
    }

    /// The page after.
    pub fn next_page(&mut self) {
        self.paginator.next_page();
        self.cursor = self.cursor.min(self.max_cursor_index());
    }

    /// The last place on the page showing.
    fn max_cursor_index(&self) -> usize {
        self.paginator
            .items_on_page(self.visible_items().len())
            .saturating_sub(1)
    }

    /// Turns bindings on and off for the state.
    pub(super) fn update_keybindings(&mut self) {
        if self.filter_state == FilterState::Filtering {
            self.filtering_keybindings();
            return;
        }
        self.browsing_keybindings();
        let more = self.help.show_all || count_enabled(&self.full_help()) > 1;
        self.key_map.show_full_help.set_enabled(more);
        self.key_map.close_full_help.set_enabled(more);
    }

    /// While browsing, what there are items, pages and a filter for.
    fn browsing_keybindings(&mut self) {
        let items = !self.items.is_empty();
        let pages = self.paginator.total_pages > 1;
        let applied = self.filter_state == FilterState::FilterApplied;
        let quit = !self.flags.get(DISABLE_QUIT_KEYBINDINGS);
        let filter = self.flags.get(FILTERING_ENABLED) && items;
        let k = &mut self.key_map;
        for (b, on) in [
            (&mut k.cursor_up, items),
            (&mut k.cursor_down, items),
            (&mut k.next_page, pages),
            (&mut k.prev_page, pages),
            (&mut k.go_to_start, items),
            (&mut k.go_to_end, items),
            (&mut k.filter, filter),
            (&mut k.clear_filter, applied),
            (&mut k.cancel_while_filtering, false),
            (&mut k.accept_while_filtering, false),
            (&mut k.quit, quit),
        ] {
            b.set_enabled(on);
        }
    }

    /// While filtering, only cancelling and accepting.
    fn filtering_keybindings(&mut self) {
        let typed = !self.filter_input.value().is_empty();
        let k = &mut self.key_map;
        for b in [
            &mut k.cursor_up,
            &mut k.cursor_down,
            &mut k.next_page,
            &mut k.prev_page,
            &mut k.go_to_start,
            &mut k.go_to_end,
            &mut k.filter,
            &mut k.clear_filter,
            &mut k.quit,
            &mut k.show_full_help,
            &mut k.close_full_help,
        ] {
            b.set_enabled(false);
        }
        k.cancel_while_filtering.set_enabled(true);
        k.accept_while_filtering.set_enabled(typed);
    }

    /// Items a page and the page, from the room left by the rest.
    pub(super) fn update_pagination(&mut self) {
        let index = self.index();
        let mut room = signed(self.height);
        if self.flags.get(SHOW_TITLE)
            || (self.flags.get(SHOW_FILTER) && self.flags.get(FILTERING_ENABLED))
        {
            room -= signed(crate::style::height(&self.title_view()));
        }
        if self.flags.get(SHOW_STATUS_BAR) {
            room -= signed(crate::style::height(&self.status_view()));
        }
        if self.flags.get(SHOW_PAGINATION) {
            room -= signed(crate::style::height(&self.pagination_view()));
        }
        if self.flags.get(SHOW_HELP) {
            room -= signed(crate::style::height(&self.help_view()));
        }
        let each = signed(self.delegate.height() + self.delegate.spacing()).max(1);
        self.paginator.per_page = usize::try_from(room / each).unwrap_or(0).max(1);
        let _ = self
            .paginator
            .set_total_pages(self.visible_items().len().max(1));
        self.paginator.page = index / self.paginator.per_page;
        self.cursor = index % self.paginator.per_page;
        let last = self.paginator.total_pages.saturating_sub(1);
        if self.paginator.page >= last {
            self.paginator.page = last;
        }
    }
}

impl<I: Item, D: ItemDelegate<I>> HelpKeyMap for Model<I, D> {
    fn short_help(&self) -> Vec<Binding> {
        let k = &self.key_map;
        let filtering = self.filter_state == FilterState::Filtering;
        let mut kb = vec![k.cursor_up.clone(), k.cursor_down.clone()];
        if !filtering {
            kb.extend(self.delegate.short_help());
        }
        kb.extend([
            k.filter.clone(),
            k.clear_filter.clone(),
            k.accept_while_filtering.clone(),
            k.cancel_while_filtering.clone(),
        ]);
        if let (false, Some(more)) = (filtering, self.additional_short_help_keys) {
            kb.extend(more());
        }
        kb.extend([k.quit.clone(), k.show_full_help.clone()]);
        kb
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        let k = &self.key_map;
        let filtering = self.filter_state == FilterState::Filtering;
        let mut groups = vec![vec![
            k.cursor_up.clone(),
            k.cursor_down.clone(),
            k.next_page.clone(),
            k.prev_page.clone(),
            k.go_to_start.clone(),
            k.go_to_end.clone(),
        ]];
        if !filtering {
            groups.extend(self.delegate.full_help());
        }
        let mut list_level = vec![
            k.filter.clone(),
            k.clear_filter.clone(),
            k.accept_while_filtering.clone(),
            k.cancel_while_filtering.clone(),
        ];
        if let (false, Some(more)) = (filtering, self.additional_full_help_keys) {
            list_level.extend(more());
        }
        groups.push(list_level);
        groups.push(vec![k.quit.clone(), k.close_full_help.clone()]);
        groups
    }
}

/// How many bindings are on.
fn count_enabled(groups: &[Vec<Binding>]) -> usize {
    groups.iter().flatten().filter(|b| b.enabled()).count()
}

/// `n` as a signed count.
fn signed(n: usize) -> isize {
    isize::try_from(n).unwrap_or(isize::MAX)
}
