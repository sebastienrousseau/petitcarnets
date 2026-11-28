//! Golden tests for the ports of Bubble Tea's examples that need no
//! components: each runs in `TestProgram` on a virtual clock, its
//! screens compared with `tests/golden/*.golden`. The ports that use
//! components are in `tests/bubbles_examples.rs`. Rewrite the files with
//! `PETITCARNETS_UPDATE_GOLDEN=1 cargo test --test examples`.

use petitcarnets as tea;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;
use tea::testing::{TestProgram, assert_golden, escape};
use tea::{Key, KeyCode};

#[path = "../examples/debounce.rs"]
mod debounce;
#[path = "../examples/focus-blur.rs"]
mod focus_blur;
#[path = "../examples/fullscreen.rs"]
mod fullscreen;
#[path = "../examples/http.rs"]
mod http;
#[path = "../examples/result.rs"]
mod result;
#[path = "../examples/sequence.rs"]
mod sequence;
#[path = "../examples/simple.rs"]
mod simple;
#[path = "../examples/vanish.rs"]
mod vanish;
#[path = "../examples/views.rs"]
mod views;
#[path = "../examples/window-size.rs"]
mod window_size;

/// Naming each example's `main` keeps it from counting as unused here.
#[test]
fn every_included_example_keeps_its_main() {
    let mains: [fn() -> Result<(), tea::Error>; 10] = [
        debounce::main,
        focus_blur::main,
        fullscreen::main,
        http::main,
        result::main,
        sequence::main,
        simple::main,
        vanish::main,
        views::main,
        window_size::main,
    ];
    assert_eq!(mains.len(), 10);
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn check<M: tea::Model>(name: &str, test: &TestProgram<M>) {
    assert_golden(name, &test.screen().snapshot());
}

#[test]
fn simple_counts_down_and_quits() {
    let mut test = TestProgram::new(simple::Model(5), 80, 6);
    check("simple-start", &test);
    test.advance(Duration::from_secs(3));
    check("simple-two", &test);
    assert!(test.wait_finished(Duration::from_secs(3)));
    assert_eq!(test.elapsed(), Duration::from_secs(5));
    assert_eq!(test.final_model().0, 0);
}

#[test]
fn simple_quits_sooner_on_q_with_the_same_bytes_every_run() {
    let mut test = TestProgram::new(simple::Model(5), 80, 6);
    test.advance(Duration::from_secs(1));
    test.type_text("q");
    assert_golden("simple-output", &escape(test.final_output()));
    assert_eq!(test.final_model().0, 4);
}

#[test]
fn fullscreen_counts_down_on_the_alternate_screen() {
    let mut test = TestProgram::new(fullscreen::Model(5), 60, 5);
    check("fullscreen-start", &test);
    assert!(test.screen().alt_screen());
    test.advance(Duration::from_secs(4));
    check("fullscreen-one", &test);
    test.press(Key::new(KeyCode::Esc));
    assert!(
        !test.final_screen().alt_screen(),
        "the screen is given back"
    );
    assert_eq!(test.final_model().0, 1);
}

#[test]
fn window_size_prints_the_size_on_any_key() {
    let mut test = TestProgram::new(window_size::Model, 60, 8);
    test.type_text("x");
    test.resize(50, 8);
    test.press(Key::new(KeyCode::Space));
    check("window-size-printed", &test);
    test.type_text("q");
    assert!(test.wait_finished(ms(100)));
}

#[test]
fn debounce_quits_one_second_after_the_last_key() {
    let mut test = TestProgram::new(debounce::Model::default(), 80, 4);
    check("debounce-start", &test);
    test.type_text("a");
    test.advance(ms(600));
    test.type_text("bc");
    test.advance(ms(900));
    check("debounce-three", &test);
    assert!(!test.is_finished(), "the first tick's tag is old");
    assert!(test.wait_finished(ms(200)));
    assert_eq!(test.elapsed(), ms(1600));
    assert_eq!(test.final_model().tag, 3);
}

#[test]
fn focus_blur_follows_focus_reports_and_toggles_them() {
    let model = focus_blur::Model {
        focused: true,
        reporting: true,
    };
    let mut test = TestProgram::new(model, 70, 7);
    check("focus-blur-focused", &test);
    assert!(test.screen().mode(1004), "focus reports on");
    test.bytes(b"\x1b[O");
    // Not input that hurries a frame: the next one is a frame later.
    test.advance(ms(20));
    check("focus-blur-blurred", &test);
    test.type_text("t");
    check("focus-blur-disabled", &test);
    assert!(!test.screen().mode(1004), "focus reports off");
    test.type_text("q");
    assert!(!test.final_model().focused);
}

#[test]
fn sequence_prints_in_order_then_quits() {
    let mut test = TestProgram::new(sequence::Model, 20, 12);
    assert!(test.wait_finished(Duration::from_secs(10)));
    assert_eq!(test.elapsed(), ms(2000 + 1000 + 750 + 500));
    assert_golden("sequence-end", &test.final_screen().snapshot());
}

/// A one-shot HTTP server on a free local port that answers every
/// request with `status`; its URL.
fn fake_server(status: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let _ = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0; 1024];
            let _ = stream.read(&mut buf).unwrap();
            assert!(buf.starts_with(b"GET /status HTTP/1.1\r\nHost: 127.0.0.1\r\n"));
            let reply = format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\n\r\n");
            stream.write_all(reply.as_bytes()).unwrap();
        }
    });
    format!("http://127.0.0.1:{port}/status")
}

#[test]
fn http_shows_the_status_of_a_local_server_and_quits() {
    let url = fake_server("200 OK");
    let mut test = TestProgram::new(http::Model::new(&url), 70, 4);
    assert!(test.wait_finished(Duration::from_secs(1)));
    let screen = test.final_screen().snapshot().replace(&url, "<url>");
    assert_golden("http-ok", &screen);
    assert_eq!(test.final_model().status, 200);
}

#[test]
fn http_names_the_status_go_names() {
    let url = fake_server("404 Not Found");
    let mut test = TestProgram::new(http::Model::new(&url), 70, 4);
    assert!(test.wait_finished(Duration::from_secs(1)));
    let screen = test.final_screen().text();
    assert_eq!(screen, format!("Checking {url}...404 Not Found"));
}

#[test]
fn http_shows_an_error_and_waits_for_a_key() {
    let mut test = TestProgram::new(http::Model::new("https://charm.sh/"), 120, 4);
    assert!(test.wait_for(ms(100), |t| t.screen().text().contains("wrong")));
    check("http-https-refused", &test);
    assert!(!test.is_finished(), "an error does not quit");
    test.type_text("q");
    assert!(test.final_model().err.is_some());
}

#[test]
fn views_counts_down_while_choosing() {
    let mut test = TestProgram::new(views::initial_model(), 80, 14);
    check("views-choices", &test);
    test.type_text("jjjjk");
    test.advance(Duration::from_secs(3));
    check("views-third", &test);
    assert!(test.wait_finished(Duration::from_secs(8)));
    assert_eq!(
        test.elapsed(),
        Duration::from_secs(11),
        "ten ticks, then the quitting one"
    );
    assert_golden("views-bye", &test.final_screen().snapshot());
}

#[test]
fn views_fills_the_bar_with_a_bounce_then_quits() {
    let mut test = TestProgram::new(views::initial_model(), 80, 14);
    test.press(Key::new(KeyCode::Enter));
    test.advance(Duration::from_secs(1) / 60 * 50);
    check("views-loading", &test);
    assert!(test.wait_for(Duration::from_secs(2), |t| t.model().loaded));
    assert_eq!(test.model().frames, 90, "OutBounce(0.9) is 1");
    check("views-loaded", &test);
    assert!(test.wait_finished(Duration::from_secs(5)));
}

/// The bar's colours are go-colorful's blend, as Go prints them: cells
/// 1, 2, 69 and 70 of the 71 (`#af53fd`, `#ae58fb`, `#20fba6`,
/// `#14fda4`); all 70 from cell 1 on match Go's. Cell 0 is left out: it
/// is the start colour after a Luv round trip, which lands within a few
/// bits of whole numbers, so truncating it (as Go does) gives a byte
/// that depends on the C library's `cbrt` and `powf` on the machine.
/// Go's own is noise too (`#b14ffe` for `#B14FFF`).
#[test]
fn views_blends_the_bar_as_go_does() {
    let mut test = TestProgram::new(views::initial_model(), 80, 14);
    test.press(Key::new(KeyCode::Enter));
    assert!(test.wait_for(Duration::from_secs(2), |t| t.model().loaded));
    let out = String::from_utf8_lossy(test.output()).into_owned();
    for rgb in ["175;83;253", "174;88;251", "32;251;166", "20;253;164"] {
        assert!(out.contains(&format!("38;2;{rgb}m")), "{rgb}");
    }
}

#[test]
fn vanish_leaves_nothing_behind() {
    let mut test = TestProgram::new(vanish::Model::default(), 70, 4);
    check("vanish-start", &test);
    test.type_text("x");
    assert_eq!(
        test.final_screen().text(),
        "",
        "erased, cursor where it began"
    );
}

#[test]
fn result_returns_the_chosen_tea() {
    let mut test = TestProgram::new(result::Model::default(), 60, 8);
    test.press(Key::new(KeyCode::Up));
    test.type_text("j");
    test.press(Key::new(KeyCode::Down));
    check("result-coffee", &test);
    test.press(Key::new(KeyCode::Enter));
    assert_eq!(
        test.final_model().choice,
        Some("Coffee"),
        "up wraps to the last, j back to the first"
    );
}

#[test]
fn result_returns_no_choice_on_q() {
    let mut test = TestProgram::new(result::Model::default(), 60, 8);
    test.type_text("q");
    assert_eq!(test.final_model().choice, None);
}

/// Every example runs in CI: its model in one of the test files that
/// include it (these run in the Test job), or its binary in the
/// real-terminal smoke test (the `pty` job), for those that hand the
/// terminal to another process.
#[test]
fn every_example_runs_in_a_test() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let tests: String = [
        "golden",
        "bubbles",
        "examples",
        "bubbles_examples",
        "accessible",
    ]
    .map(|t| read(&format!("tests/{t}.rs")))
    .concat();
    let smoke = read("scripts/pty-smoke.sh");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let Some(stem) = name.strip_suffix(".rs") else {
            continue;
        };
        let included = tests.contains(&format!("#[path = \"../examples/{name}\"]"));
        let smoked = smoke.contains(&format!("target/debug/examples/{stem} "))
            || smoke.contains(&format!("target/debug/examples/{stem};"));
        if !included && !smoked {
            missing.push(stem.to_owned());
        }
    }
    assert!(missing.is_empty(), "examples no test runs: {missing:?}");
}
