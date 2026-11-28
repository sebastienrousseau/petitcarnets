//! Drawing the list: the title bar (or the filter input), the status
//! bar, the items on the page, the pagination and the help.

use super::{
    FILTERING_ENABLED, FilterState, Item, ItemDelegate, Model, SHOW_FILTER, SHOW_HELP,
    SHOW_PAGINATION, SHOW_SPINNER, SHOW_STATUS_BAR, SHOW_TITLE,
};
use crate::bubbles::paginator;
use crate::style::{Position, Style, block_width, height, join_vertical, truncate_with, width};

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// The list.
    #[must_use]
    pub fn view(&self) -> String {
        let mut sections = Vec::new();
        let mut room = self.height;
        if self.flags.get(SHOW_TITLE)
            || (self.flags.get(SHOW_FILTER) && self.flags.get(FILTERING_ENABLED))
        {
            let v = self.title_view();
            room = room.saturating_sub(height(&v));
            sections.push(v);
        }
        if self.flags.get(SHOW_STATUS_BAR) {
            let v = self.status_view();
            room = room.saturating_sub(height(&v));
            sections.push(v);
        }
        let pagination = self.pagination_view();
        if self.flags.get(SHOW_PAGINATION) {
            room = room.saturating_sub(height(&pagination));
        }
        let help = self.help_view();
        if self.flags.get(SHOW_HELP) {
            room = room.saturating_sub(height(&help));
        }
        let rows = u16::try_from(room).unwrap_or(u16::MAX);
        sections.push(Style::new().height(rows).render(&self.populated_view()));
        if self.flags.get(SHOW_PAGINATION) {
            sections.push(pagination);
        }
        if self.flags.get(SHOW_HELP) {
            sections.push(help);
        }
        let blocks: Vec<&str> = sections.iter().map(String::as_str).collect();
        join_vertical(Position::LEFT, &blocks)
    }

    /// The title, the spinner and any status message, or the filter
    /// input while filtering.
    pub(super) fn title_view(&self) -> String {
        let spinner = self.spinner.view();
        let room = self
            .styles
            .title_bar
            .get_padding_left()
            .checked_sub(width(&spinner) + 1);
        let on_left = self.flags.get(SHOW_SPINNER) && room.is_some();
        let (mut view, bar) = self.title_body(&spinner, room.filter(|_| on_left));
        if self.flags.get(SHOW_SPINNER) && !on_left {
            view += &self.spinner_on_right(&view, &spinner);
        }
        if view.is_empty() {
            view
        } else {
            bar.render(&view)
        }
    }

    /// The filter input or the title, and the title bar's style, its
    /// left padding made `left` when the spinner takes the rest.
    fn title_body(&self, spinner: &str, left: Option<usize>) -> (String, Style) {
        let bar = self.styles.title_bar;
        if self.flags.get(SHOW_FILTER) && self.filter_state == FilterState::Filtering {
            return (self.filter_input.view(), bar);
        }
        if !self.flags.get(SHOW_TITLE) {
            return (String::new(), bar);
        }
        let bar = left.map_or(bar, |l| bar.padding_left(to16(l)));
        (self.title_text(left.map(|_| spinner)), bar)
    }

    /// The title, after the spinner if it goes on the left, and the
    /// status message, cut to the width.
    fn title_text(&self, spinner: Option<&str>) -> String {
        let mut view = spinner.map_or_else(String::new, |s| format!("{s} "));
        view += &self.styles.title.render(&self.title);
        if !self.status_message.is_empty() {
            view += "  ";
            view += &self.status_message;
        }
        let spinner_width = width(&self.spinner.view());
        truncate_with(&view, self.width.saturating_sub(spinner_width), "…")
    }

    /// The spinner at the right of the title bar, if there is room.
    fn spinner_on_right(&self, view: &str, spinner: &str) -> String {
        let room = self
            .width
            .saturating_sub(block_width(&self.styles.title_bar.render(view)));
        let spinner_width = width(spinner);
        if room > spinner_width {
            " ".repeat(room - spinner_width) + spinner
        } else {
            String::new()
        }
    }

    /// How many items, how many filtered out, and the filter.
    pub(super) fn status_view(&self) -> String {
        let total = self.items.len();
        let visible = self.visible_items().len();
        let name = if visible == 1 {
            &self.item_name_singular
        } else {
            &self.item_name_plural
        };
        let shown = format!("{visible} {name}");
        let mut status = if self.filter_state == FilterState::Filtering {
            if visible == 0 {
                self.styles.status_empty.render("Nothing matched")
            } else {
                shown
            }
        } else if total == 0 {
            self.styles
                .status_empty
                .render(&format!("No {}", self.item_name_plural))
        } else if self.filter_state == FilterState::FilterApplied {
            let f = truncate_with(self.filter_input.value().trim(), 10, "…");
            format!("“{f}” {shown}")
        } else {
            shown
        };
        if total > visible {
            status += &self.styles.divider_dot.paint(" • ");
            status += &self
                .styles
                .status_bar_filter_count
                .render(&format!("{} filtered", total - visible));
        }
        self.styles.status_bar.render(&status)
    }

    /// The pages, as dots, or numbers when the dots do not fit.
    pub(super) fn pagination_view(&self) -> String {
        if self.paginator.total_pages < 2 {
            return String::new();
        }
        let mut s = self.paginator.view();
        if width(&s) > self.width {
            let mut numbers = self.paginator.clone();
            numbers.kind = paginator::Type::Arabic;
            s = self.styles.arabic_pagination.render(&numbers.view());
        }
        let mut style = self.styles.pagination_style;
        if self.delegate.spacing() == 0 && style.get_margin_top() == 0 {
            style = style.margin_top(1);
        }
        style.render(&s)
    }

    /// The items on the page, padded to the page's height.
    fn populated_view(&self) -> String {
        let items = self.visible_items();
        if items.is_empty() {
            if self.filter_state == FilterState::Filtering {
                return String::new();
            }
            return self
                .styles
                .no_items
                .render(&format!("No {}.", self.item_name_plural));
        }
        let (start, end) = self.paginator.slice_bounds(items.len());
        let gap = "\n".repeat(self.delegate.spacing() + 1);
        let rendered: Vec<String> = (start..end)
            .map(|i| self.delegate.render(self, i, items[i]))
            .collect();
        let mut out = rendered.join(&gap);
        let on_page = self.paginator.items_on_page(items.len());
        if on_page < self.paginator.per_page {
            let missing = self.paginator.per_page - on_page;
            out += &"\n".repeat(missing * (self.delegate.height() + self.delegate.spacing()));
        }
        out
    }

    /// The help, in its style.
    pub(super) fn help_view(&self) -> String {
        self.styles.help_style.render(&self.help.view(self))
    }
}

/// `n` as a style size.
fn to16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}
