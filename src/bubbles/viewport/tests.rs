use super::*;
use crate::bubbles::tests::{messages, rows};
use crate::style::strip_ansi;

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

/// The fixture's content: `n` numbered lines.
fn numbered(n: usize) -> String {
    let lines: Vec<String> = (0..n)
        .map(|i| format!("line {} {}", i + 1, "ab字".repeat(i % 4)))
        .collect();
    lines.join("\n")
}

/// The fixture's gutter.
fn gutter(info: GutterContext) -> String {
    if info.soft {
        "   │ ".into()
    } else if info.index >= info.total_lines {
        " ~ │ ".into()
    } else {
        format!("{:2} │ ", info.index + 1)
    }
}

#[test]
fn viewports_match_bubbles() {
    for row in rows("viewport") {
        let n = |i: usize| row[i].parse::<usize>().expect("number");
        let mut v = Model::new(n(1), n(2));
        v.soft_wrap = row[4] == "true";
        if row[5] == "true" {
            v.left_gutter = Some(gutter);
        }
        if n(3) > 0 {
            v.set_content(&numbered(n(3)));
        }
        for msg in messages::<()>(&row[6]) {
            let _ = v.update(&msg);
        }
        let got = (
            v.y_offset(),
            v.x_offset(),
            format!("{:.3}", v.scroll_percent()),
            format!("{:.3}", v.horizontal_scroll_percent()),
            v.total_line_count(),
            v.visible_line_count(),
            strip_ansi(&v.view()),
        );
        let want = (
            n(7),
            n(8),
            row[9].clone(),
            row[10].clone(),
            n(11),
            n(12),
            row[13].clone(),
        );
        assert_eq!(got, want, "case {}", row[0]);
    }
}

// The viewport's own behaviour.

#[test]
fn a_framed_viewport_fills_its_height() {
    let mut v = Model::default();
    assert_eq!(v.view(), "", "no size, nothing");
    v.set_width(10);
    v.set_height(4);
    v.style = Style::new().border(crate::style::Border::NORMAL);
    v.fill_height = true;
    v.style_line = Some(|_| Style::new());
    v.set_content("a\r\nb");
    assert_eq!(v.content(), "a\nb");
    assert_eq!(
        strip_ansi(&v.view()),
        "┌────────┐\n│a       │\n│b       │\n└────────┘"
    );
    v.set_horizontal_step(2);
    v.set_content(&"x".repeat(30));
    v.scroll_right(1);
    let _ = v.update::<()>(&Msg::KeyPress(crate::Key::char('l')));
    assert_eq!(v.x_offset(), 3, "one, then a step of two");
    v.set_content("");
    assert!(!v.past_bottom());
    v.mouse_wheel_enabled = false;
    let wheel = crate::Mouse {
        x: 0,
        y: 0,
        button: MouseButton::WheelDown,
        mods: KeyMod::NONE,
    };
    let _ = v.update::<()>(&Msg::MouseWheel(wheel));
    let other = crate::Mouse {
        button: MouseButton::Left,
        ..wheel
    };
    v.mouse_wheel_enabled = true;
    let _ = v.update::<()>(&Msg::MouseWheel(other));
    assert_eq!(v.y_offset(), 0);
    v.set_height(0);
    assert_eq!(v.visible_line_count(), 0);
}
