use super::*;
use crate::bubbles::tests::rows;
use crate::style::strip_ansi;

// Against Bubbles v2, from tests/fixtures/bubbles.tsv.

/// `#rrggbb` as a colour.
fn parse_hex(text: &str) -> Color {
    let value = u32::from_str_radix(text.trim_start_matches('#'), 16).expect("hex");
    let [_, red, green, blue] = value.to_be_bytes();
    Color::Rgb(red, green, blue)
}

#[test]
fn blends_match_lip_gloss() {
    for row in rows("blend") {
        let stops: Vec<Color> = row[1].split(',').map(parse_hex).collect();
        let got = blend::blend_1d(row[0].parse().expect("steps"), &stops);
        let want: Vec<Color> = row[2]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(parse_hex)
            .collect();
        assert_eq!(got, want, "{} steps of {}", row[0], row[1]);
    }
}

#[test]
fn springs_match_harmonica() {
    for row in rows("spring") {
        let (f, d) = (row[0].parse().expect("f"), row[1].parse().expect("d"));
        let mut state = (0.0, 0.0);
        for (i, step) in row[2].split(';').enumerate() {
            state = spring_step(f, d, state);
            let (p, v) = step.split_once(',').expect("pair");
            let close = |a: f64, b: &str| (a - b.parse::<f64>().expect("float")).abs() <= 1e-12;
            assert!(
                close(state.0, p) && close(state.1, v),
                "{f} {d} step {i}: {state:?} vs {step}"
            );
        }
    }
}

#[test]
fn progress_bars_match_bubbles() {
    let bars = [
        ("default", Model::new()),
        ("narrow", Model::new().with_width(12)),
        ("blend", Model::new().with_default_blend().with_width(20)),
        (
            "scaled",
            Model::new()
                .with_scaled(true)
                .with_colors(&[parse_hex("#FF7CCB"), parse_hex("#FDFF8C")])
                .with_width(30),
        ),
        (
            "full-block",
            Model::new()
                .with_fill_characters('█', '·')
                .with_width(16)
                .without_percentage(),
        ),
    ];
    for row in rows("progress") {
        let (_, bar) = bars.iter().find(|(n, _)| *n == row[0]).expect("bar");
        let view = bar.view_as(row[1].parse().expect("percent"));
        assert_eq!(strip_ansi(&view), row[2], "{} at {}", row[0], row[1]);
        assert_eq!(
            cell_colors(&view),
            row[3],
            "{} at {} colours",
            row[0],
            row[1]
        );
    }
}

/// The SGR parameters before each cell of a bar, as the generator
/// writes them: Lip Gloss's for 24-bit colours, which petitcarnets
/// writes the same but for the reset.
fn cell_colors(s: &str) -> String {
    let mut out = Vec::new();
    let mut params = String::new();
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("\x1b[") {
            let end = after.find('m').expect("sgr");
            params = after[..end].to_string();
            if params == "0" {
                params.clear();
            }
            rest = &after[end + 1..];
            continue;
        }
        let c = rest.chars().next().expect("char");
        out.push(params.clone());
        rest = &rest[c.len_utf8()..];
    }
    out.join(" ")
}

/// One step of a 60 frames a second spring to 1.0.
fn spring_step(frequency: f64, damping: f64, (pos, vel): (f64, f64)) -> (f64, f64) {
    Spring::new(frame_seconds(), frequency, damping).update(pos, vel, 1.0)
}

// The bar's own behaviour.

#[test]
fn colours_come_from_a_function_or_one_colour() {
    let f: ColorFn = |_, current| {
        if current < 0.42 {
            Color::RED
        } else {
            Color::BLUE
        }
    };
    let half = Model::new()
        .with_color_fn(f)
        .with_width(10)
        .without_percentage();
    assert_eq!(strip_ansi(&half.view_as(1.0)), "▌".repeat(10));
    assert!(
        half.view_as(1.0).contains("\x1b[31;44m▌"),
        "a half block takes two colours: {:?}",
        half.view_as(1.0)
    );
    let full = half.clone().with_fill_characters('█', ' ').with_color_fn(f);
    assert!(!full.view_as(0.2).contains(";4"), "a full block takes one");
    let solid = Model::default()
        .with_colors(&[Color::GREEN])
        .with_width(5)
        .without_percentage();
    assert!(solid.view_as(1.0).starts_with("\x1b[32m"));
    let back = Model::new().with_default_blend().with_colors(&[]);
    assert_eq!(back.full_color, FULL_COLOR);
    assert!(
        channels(Color::RED).iter().all(|c| *c == 0.0),
        "only 24-bit colours blend"
    );
}

/// The 24-bit colour channels of `c`, for the test above.
fn channels(c: Color) -> [f64; 3] {
    blend::blend_1d(3, &[c, Color::Rgb(0, 0, 0)])
        .first()
        .map_or([1.0; 3], |c| match c {
            Color::Rgb(r, g, b) => [r, g, b].map(|v| f64::from(*v)),
            _ => [1.0; 3],
        })
}

#[test]
fn the_bar_springs_to_its_percentage_on_its_own_frames() {
    let mut bar = Model::new().with_spring_options(18.0, 1.0).with_width(20);
    assert_eq!(bar.width(), 20);
    let _ = bar.set_percent(0.5);
    let _ = bar.decr_percent(0.25);
    assert!((bar.percent() - 0.25).abs() < f64::EPSILON);
    let frame = crate::Msg::App(FrameMsg {
        id: bar.id(),
        tag: bar.tag,
    });
    let other = crate::Msg::App(FrameMsg {
        id: bar.id() + 1,
        tag: bar.tag,
    });
    assert!(bar.update(&other).is_none());
    let mut frames = 0;
    while bar.is_animating() {
        assert!(!bar.update(&frame).is_none());
        frames += 1;
        assert!(frames < 200, "the spring settles");
    }
    assert!(bar.update(&frame).is_none(), "settled, no more frames");
    assert_eq!(strip_ansi(&bar.view()), "▌▌▌▌░░░░░░░░░░░  25%");
    bar.set_width(10);
    assert_eq!(strip_ansi(&bar.view_as(f64::NAN)), "░░░░░   0%");
}
