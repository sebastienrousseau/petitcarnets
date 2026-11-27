use super::*;
use crate::{Key, KeyCode};

#[test]
fn pages_turn_on_their_keys_within_bounds() {
    let mut p = Model {
        per_page: 3,
        ..Model::default()
    };
    assert_eq!(p.set_total_pages(0), 1, "no items change nothing");
    assert_eq!(p.set_total_pages(7), 3);
    assert!(p.on_first_page() && !p.on_last_page());
    let _ = p.update::<()>(&Msg::KeyPress(Key::char('l')));
    let _ = p.update::<()>(&Msg::KeyPress(Key::new(KeyCode::PgDown)));
    let _ = p.update::<()>(&Msg::KeyPress(Key::char('l')));
    assert_eq!(p.page, 2);
    assert!(p.on_last_page());
    assert_eq!(p.items_on_page(7), 1);
    assert_eq!(p.slice_bounds(7), (6, 7));
    let _ = p.update::<()>(&Msg::KeyPress(Key::new(KeyCode::Left)));
    assert_eq!(p.page, 1);
    for _ in 0..3 {
        p.prev_page();
    }
    assert_eq!(p.page, 0);
    let _ = p.update::<()>(&Msg::KeyPress(Key::char('x')));
    assert_eq!(p.page, 0);
    assert_eq!(p.view(), "1/3");
    p.kind = Type::Dots;
    assert_eq!(p.view(), "•○○");
    assert_eq!(Model::new().items_on_page(0), 0);
}

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

use crate::bubbles::tests::rows;

#[test]
fn paginators_match_bubbles() {
    for row in rows("paginator") {
        let n = |i: usize| row[i].parse::<usize>().expect("number");
        let mut p = Model::new();
        p.kind = if row[0] == "1" {
            Type::Dots
        } else {
            Type::Arabic
        };
        p.per_page = 3;
        let _ = p.set_total_pages(n(1));
        p.next_page();
        p.next_page();
        let got = (
            p.page,
            p.total_pages,
            p.slice_bounds(n(1)),
            p.items_on_page(n(1)),
            p.view(),
        );
        assert_eq!(
            got,
            (n(2), n(3), (n(4), n(5)), n(6), row[7].clone()),
            "{row:?}"
        );
    }
}
