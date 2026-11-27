use super::*;
use crate::bubbles::tests::{messages, rows};
use crate::style::{Border, strip_ansi};

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

/// The table example's rows.
const CITIES: [[&str; 4]; 16] = [
    ["1", "Tokyo", "Japan", "37,274,000"],
    ["2", "Delhi", "India", "32,065,760"],
    ["3", "Shanghai", "China", "28,516,904"],
    ["4", "Dhaka", "Bangladesh", "22,478,116"],
    ["5", "São Paulo", "Brazil", "22,429,800"],
    ["6", "Mexico City", "Mexico", "22,085,140"],
    ["7", "Cairo", "Egypt", "21,750,020"],
    ["8", "Beijing", "China", "21,333,332"],
    ["9", "Mumbai", "India", "20,961,472"],
    ["10", "Osaka", "Japan", "19,059,856"],
    ["11", "Chongqing", "China", "16,874,740"],
    ["12", "Karachi", "Pakistan", "16,839,950"],
    ["13", "Istanbul", "Turkey", "15,636,243"],
    ["14", "Kinshasa", "DR Congo", "15,628,085"],
    ["15", "Lagos", "Nigeria", "15,387,639"],
    ["16", "Buenos Aires", "Argentina", "15,369,919"],
];

#[test]
fn tables_match_bubbles() {
    for row in rows("table") {
        let rows_: Vec<Row> = CITIES
            .iter()
            .map(|r| r.iter().map(|c| (*c).to_string()).collect())
            .collect();
        let cols = vec![
            Column::new("Rank", 4),
            Column::new("City", 10),
            Column::new("Country", 10),
            Column::new("Population", 10),
        ];
        let mut t = Model::new()
            .with_columns(cols)
            .with_rows(rows_)
            .with_focused(true)
            .with_height(7)
            .with_width(42);
        if row[0] == "true" {
            t.set_styles(Styles {
                header: Style::new()
                    .padding(0, 1)
                    .border(Border::NORMAL)
                    .border_sides([false, false, true, false]),
                selected: Style::new().fg(Color::Palette(212)),
                ..Styles::default()
            });
        }
        for msg in messages::<()>(&row[1]) {
            let _ = t.update(&msg);
        }
        let got = (
            t.cursor().to_string(),
            strip_ansi(&t.view()),
            strip_ansi(&t.help_view()),
        );
        assert_eq!(
            got,
            (row[2].clone(), row[3].clone(), row[4].clone()),
            "bordered {} keys {:?}",
            row[0],
            row[1]
        );
    }
}

// The table's own behaviour.

#[test]
fn a_table_from_values_with_its_own_keys_and_styles() {
    let mut t = Model::default()
        .with_columns(vec![
            Column::new("A", 3),
            Column::new("Hidden", 0),
            Column::new("B", 3),
        ])
        .with_styles(Styles::default())
        .with_key_map(KeyMap::default())
        .with_height(4)
        .with_width(12);
    t.from_values("1,x,2\n3,y,4\n5,z,6", ",");
    assert_eq!(t.rows().len(), 3);
    assert_eq!(t.columns().len(), 3);
    assert_eq!((t.height(), t.width()), (3, 12));
    assert_eq!(
        strip_ansi(&t.view()),
        " A    B   \n 1    2     \n 3    4     \n 5    6     "
    );
    t.set_cursor(2);
    assert_eq!(t.cursor(), 2);
    assert!(
        t.update::<()>(&Msg::KeyPress(crate::Key::char('k')))
            .is_none()
    );
    assert_eq!(t.cursor(), 2, "blurred, the keys do nothing");
    t.focus();
    assert!(t.focused());
    let _ = t.update::<()>(&Msg::KeyPress(crate::Key::char('k')));
    assert_eq!(t.cursor(), 1);
    t.set_rows(vec![]);
    assert_eq!(t.selected_row(), None);
    t.set_cursor(5);
    t.blur();
    assert!(!t.focused());
}

#[test]
fn the_selected_style_reaches_every_cell_over_a_coloured_cell_style() {
    let styles = Styles {
        cell: Style::new().fg(Color::Palette(250)).padding(0, 1),
        selected: Style::new().fg(Color::Palette(212)),
        ..Styles::default()
    };
    let t = Model::new()
        .with_columns(vec![Column::new("A", 2), Column::new("B", 2)])
        .with_rows(vec![
            vec!["x".into(), "y".into()],
            vec!["z".into(), "w".into()],
        ])
        .with_styles(styles)
        .with_height(3)
        .with_width(10);
    let view = t.view();
    let selected = view.lines().nth(1).expect("the selected row");
    assert_eq!(selected.matches("38;5;212").count(), 2, "{selected:?}");
    assert!(!selected.contains("38;5;250"));
    assert!(
        view.lines()
            .nth(2)
            .expect("the other row")
            .contains("38;5;250")
    );
}
