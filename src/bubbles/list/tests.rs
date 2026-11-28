use super::*;
use crate::bubbles::tests::{messages, rows};
use crate::style::strip_ansi;

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

#[test]
fn filter_ranks_match_bubbles() {
    let targets: Vec<String> = rows("targets")[0][0]
        .split('\u{1f}')
        .map(String::from)
        .collect();
    for row in rows("filter") {
        let ranks = if row[0] == "true" {
            default_filter(&row[1], &targets)
        } else {
            unsorted_filter(&row[1], &targets)
        };
        let got: Vec<String> = ranks
            .iter()
            .map(|r| {
                let idx: Vec<String> = r.matched_indexes.iter().map(ToString::to_string).collect();
                format!("{}:{}", r.index, idx.join(","))
            })
            .collect();
        assert_eq!(got.join(";"), row[2], "term {:?} sorted {}", row[1], row[0]);
    }
}

/// A list item with a title and a description.
#[derive(Debug, Clone)]
struct Thing(&'static str, &'static str);

impl Item for Thing {
    fn filter_value(&self) -> String {
        self.0.to_string()
    }
}

impl DefaultItem for Thing {
    fn title(&self) -> String {
        self.0.to_string()
    }
    fn description(&self) -> String {
        self.1.to_string()
    }
}

/// The list example's first items.
fn things() -> Vec<Thing> {
    vec![
        Thing("Raspberry Pi’s", "I have ’em all over my house"),
        Thing("Nutella", "It's good on toast"),
        Thing("Bitter melon", "It cools you down"),
        Thing("Nice socks", "And by that I mean socks without holes"),
        Thing("Eight hours of sleep", "I had this once"),
        Thing("Cats", "Usually"),
        Thing("Plantasia, the album", "My plants love it too"),
        Thing("Pour over coffee", "It takes forever to make though"),
        Thing("VR", "Virtual reality...what is there to say?"),
        Thing("Noguchi Lamps", "Such pleasing organic forms"),
        Thing("Linux", "Pretty much the best OS"),
        Thing("Business school", "Just kidding"),
    ]
}

#[test]
fn lists_match_bubbles() {
    let mut failed = Vec::new();
    for row in rows("list") {
        let n = |i: usize| row[i].parse::<usize>().expect("number");
        let mut l = Model::new(things(), DefaultDelegate::new(), n(1), n(2));
        l.title = "My Fave Things".into();
        for msg in messages(&row[3]) {
            let _ = l.update(&msg);
        }
        let mut want = row[6].clone();
        if row[0] == "filter-typing" {
            // Go filters on another goroutine and does not page again when
            // the matches arrive: it still shows two pages of dots for the
            // four items left, from the filter before. Here the pages
            // follow the matches: one page, no dots.
            assert!(want.contains("\n  ••  "), "Go still does this");
            want = want.replacen("\n  ••", "\n    ", 1);
        }
        let got = (
            l.index(),
            l.filter_state().to_string(),
            strip_ansi(&l.view()),
        );
        if got != (n(4), row[5].clone(), want) {
            failed.push(format!("case {}: {got:?}\n   want {:?}", row[0], &row[4..]));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

// The list's own behaviour.

use crate::{Cmd, Key, KeyCode};
use std::time::UNIX_EPOCH;

fn press(l: &mut Model<Thing, DefaultDelegate>, k: Key) -> Cmd<Msg> {
    l.update(&crate::Msg::KeyPress(k))
}

#[test]
fn settings_show_and_hide_the_parts() {
    let mut l = Model::new(things(), DefaultDelegate::default(), 40, 20);
    l.set_show_title(false);
    l.set_show_filter(false);
    l.set_show_status_bar(false);
    l.set_show_pagination(false);
    l.set_show_help(false);
    assert!(
        !(l.show_title()
            || l.show_filter()
            || l.show_status_bar()
            || l.show_pagination()
            || l.show_help())
    );
    assert!(strip_ansi(&l.view()).starts_with("│ Raspberry"));
    l.set_status_bar_item_name("thing", "things");
    assert_eq!(l.status_bar_item_name(), ("thing", "things"));
    l.set_height(10);
    assert_eq!((l.width(), l.height()), (40, 10));
    l.set_filtering_enabled(false);
    assert!(!l.filtering_enabled());
    l.disable_quit_keybindings();
    assert!(press(&mut l, Key::char('q')).is_none());
    assert!(press(&mut l, Key::ctrl('c')).is_none());
    assert_eq!(format!("{:?}", FilterState::default()), "Unfiltered");
    assert!(format!("{l:?}").contains("Model"));
    let _ = l.delegate();
}

#[test]
fn items_change_under_a_filter() {
    let mut l = Model::new(things(), DefaultDelegate::new(), 40, 20);
    l.set_filter_text("o");
    assert!(l.is_filtered() && !l.setting_filter());
    let before = l.visible_items().len();
    l.insert_item(0, Thing("Oolong", "A tea"));
    assert_eq!(l.visible_items().len(), before + 1);
    l.set_item(0, Thing("Zzz", "Sleep"));
    assert_eq!(l.visible_items().len(), before);
    l.remove_item(99);
    l.remove_item(1);
    assert_eq!(l.items().len(), things().len());
    l.set_filter_text("Zzz");
    assert_eq!(l.global_index(), 0);
    l.remove_item(0);
    assert_eq!(
        l.filter_state(),
        FilterState::Unfiltered,
        "nothing left, so no filter"
    );
    l.set_items(vec![]);
    assert!(strip_ansi(&l.view()).contains("No items."));
    l.set_filter_state(FilterState::Filtering);
    assert!(l.setting_filter());
    assert_eq!(strip_ansi(&l.view()).matches("Nothing matched").count(), 1);
    l.reset_filter();
}

#[test]
fn the_cursor_crosses_pages_and_wraps_when_asked() {
    let mut l = Model::new(things(), DefaultDelegate::new(), 40, 14);
    l.select(5);
    assert_eq!((l.index(), l.cursor()), (5, 5 % l.paginator.per_page));
    l.cursor_up();
    l.prev_page();
    l.next_page();
    l.reset_selected();
    assert_eq!(l.index(), 0);
    l.infinite_scrolling = true;
    l.cursor_up();
    assert_eq!(l.index(), things().len() - 1, "round to the end");
    l.cursor_down();
    assert_eq!(l.index(), 0, "and back to the start");
    l.go_to_end();
    l.cursor_up();
    assert_eq!(l.index(), things().len() - 2);
    let _ = press(&mut l, Key::char('?'));
    let _ = press(&mut l, Key::char('?'));
    assert!(!l.help.show_all);
    let _ = press(&mut l, Key::new(KeyCode::Esc));
}

#[test]
fn the_spinner_and_status_messages() {
    let mut l = Model::new(things(), DefaultDelegate::new(), 40, 20);
    l.set_spinner(spinner::Spinner::DOT);
    assert!(!l.start_spinner().is_none());
    let tick = spinner::Model::new().tick_msg(UNIX_EPOCH);
    let _ = l.update(&crate::Msg::App(Msg::Spinner(tick)));
    assert!(
        strip_ansi(&l.view()).contains("⣾"),
        "the spinner shows on the right"
    );
    assert!(l.toggle_spinner().is_none());
    assert!(!l.toggle_spinner().is_none());
    l.stop_spinner();
    let _ = l.new_status_message("Saved");
    assert!(strip_ansi(&l.view()).contains("List   Saved"));
    let _ = l.update(&crate::Msg::App(Msg::StatusTimeout(0)));
    assert!(strip_ansi(&l.view()).contains("Saved"), "an old timeout");
    let _ = l.update(&crate::Msg::App(Msg::StatusTimeout(1)));
    assert!(!strip_ansi(&l.view()).contains("Saved"));
    l.styles.title_bar = crate::style::Style::new().padding_sides(0, 0, 1, 4);
    let _ = l.start_spinner();
    assert!(
        strip_ansi(&l.view()).starts_with(" ⣾   List"),
        "the spinner on the left"
    );
}

#[test]
fn narrow_lists_page_with_numbers() {
    let mut d = DefaultDelegate::new();
    d.set_height(1);
    d.set_spacing(0);
    d.show_description = false;
    let mut l = Model::new(things(), d, 4, 8);
    let view = strip_ansi(&l.view());
    assert!(view.contains("1/"), "{view}");
    l.set_width(0);
    assert!(
        !strip_ansi(&l.view()).contains("Raspberry"),
        "nothing drawn at width 0"
    );
    l.additional_short_help_keys = Some(Arc::new(|| {
        vec![Binding::new(&["x"]).with_help("x", "extra")]
    }));
    l.additional_full_help_keys
        .clone_from(&l.additional_short_help_keys);
    l.set_width(80);
    assert!(strip_ansi(&l.view()).contains("x extra"));
    l.help.show_all = true;
    assert!(
        strip_ansi(&l.view()).contains("x extra"),
        "{}",
        strip_ansi(&l.view())
    );
    assert_eq!(unsorted_filter("", &[]), []);
}

#[test]
fn the_filter_and_help_keys_are_closures_that_keep_state() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut l = Model::new(things(), DefaultDelegate::new(), 80, 24);
    let exact = String::from("Cats");
    l.filter = Arc::new(move |term: &str, targets: &[String]| {
        let hits = targets.iter().enumerate();
        hits.filter(|(_, t)| t.as_str() == exact && term.starts_with('c'))
            .map(|(index, _)| Rank {
                index,
                matched_indexes: Vec::new(),
            })
            .collect()
    });
    l.set_filter_text("c");
    let shown: Vec<&str> = l.visible_items().iter().map(|t| t.0).collect();
    assert_eq!(shown, ["Cats"], "the captured string chose");
    let asked = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&asked);
    let label = String::from("extra");
    l.additional_short_help_keys = Some(Arc::new(move || {
        let _ = seen.fetch_add(1, Ordering::Relaxed);
        vec![Binding::new(&["x"]).with_help("x", &label)]
    }));
    l.set_filter_state(FilterState::Unfiltered);
    let copy = l.clone();
    assert!(strip_ansi(&l.view()).contains("x extra"));
    assert!(
        strip_ansi(&copy.view()).contains("x extra"),
        "a clone shares it"
    );
    assert!(asked.load(Ordering::Relaxed) >= 2);
    let debug = format!("{l:?}");
    assert!(
        debug.contains(
            "filter: .., additional_short_help_keys: Some(..), additional_full_help_keys: None"
        ),
        "{debug}"
    );
}
