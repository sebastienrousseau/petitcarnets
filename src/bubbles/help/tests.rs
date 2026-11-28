use super::*;
use crate::style::strip_ansi;

struct Keys(Vec<Vec<Binding>>);

impl KeyMap for Keys {
    fn short_help(&self) -> Vec<Binding> {
        self.0.concat()
    }
    fn full_help(&self) -> Vec<Vec<Binding>> {
        self.0.clone()
    }
}

fn keys() -> Keys {
    let b = |k: &str, d: &str| Binding::new(&[k]).with_help(k, d);
    Keys(vec![
        vec![b("up", "move up"), b("down", "move down")],
        vec![b("?", "help"), Binding::new(&["x"]).with_disabled()],
        vec![Binding::new(&["y"]).with_help("y", "never").with_disabled()],
        vec![b("q", "quit")],
    ])
}

#[test]
fn short_help_joins_the_bindings_that_are_on() {
    let help = Model::default();
    assert_eq!(
        strip_ansi(&help.view(&keys())),
        "up move up • down move down • ? help • q quit"
    );
    assert_eq!(help.short_help_view(&[]), "");
}

#[test]
fn short_help_ends_with_an_ellipsis_where_it_is_cut() {
    let mut help = Model::new();
    help.set_width(30);
    assert_eq!(help.width(), 30);
    assert_eq!(
        strip_ansi(&help.view(&keys())),
        "up move up • down move down …"
    );
    help.set_width(11);
    assert_eq!(
        strip_ansi(&help.view(&keys())),
        "up move up",
        "no room for the ellipsis"
    );
}

#[test]
fn full_help_puts_each_group_in_a_column() {
    let mut help = Model::new();
    help.show_all = true;
    let _ = help.update::<()>(&Msg::Focus);
    assert_eq!(
        strip_ansi(&help.view(&keys())),
        "up   move up      ? help    q quit\ndown move down                    "
    );
    help.set_width(26);
    assert_eq!(
        strip_ansi(&help.view(&keys())),
        "up   move up      ? help\ndown move down          ",
        "no room for the ellipsis"
    );
    help.set_width(27);
    assert_eq!(
        strip_ansi(&help.view(&keys())),
        format!(
            "up   move up      ? help …\ndown move down{}",
            " ".repeat(12)
        )
    );
    assert_eq!(help.full_help_view(&[]), "");
}

#[test]
fn light_styles_differ_from_dark() {
    assert_ne!(Styles::new(false), Styles::new(true));
}

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

use crate::bubbles::table;
use crate::bubbles::tests::rows;

#[test]
fn help_views_match_bubbles() {
    let keys = table::KeyMap::default();
    for row in rows("help") {
        let mut h = Model::new();
        h.set_width(row[0].parse().expect("width"));
        h.show_all = row[1] == "true";
        assert_eq!(
            strip_ansi(&h.view(&keys)),
            row[2],
            "width {} full {}",
            row[0],
            row[1]
        );
    }
}
