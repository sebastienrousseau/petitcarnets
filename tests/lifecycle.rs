//! Exec, suspend, release, repaint, the colour profile, and terminal
//! queries, driven headless through `TestProgram`, and a few runs
//! through `TestBackend`.

use petitcarnets as tea;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tea::testing::{TestBackend, TestProgram, TestTerminal};
use tea::{ColorProfile, Input, Key, Msg};

/// Records every message but sizes, as text; keys choose commands.
#[derive(Default)]
struct Life {
    seen: Vec<String>,
    view: tea::View,
}

/// What a key asks for.
fn command(key: &str) -> tea::Cmd<String> {
    match key {
        "e" => tea::sequence([
            tea::exec_command(Recorder::default(), |r| format!("exec {}", r.is_ok())),
            tea::Cmd::task(|| "after exec".to_string()),
        ]),
        "f" => tea::exec(
            std::process::Command::new("/nonexistent/petitcarnets-child"),
            |r| format!("exec failed {}", r.is_err()),
        ),
        "g" => tea::exec_command(std::process::Command::new("false"), |r| match r {
            Ok(()) => "false ok".to_string(),
            Err(err) => format!("false failed: {err}"),
        }),
        "h" => tea::exec(std::process::Command::new("true"), |r| {
            format!("true ok {}", r.is_ok_and(|status| status.success()))
        }),
        "s" => tea::suspend(),
        "r" => tea::sequence([
            tea::release_terminal(),
            tea::println("held back"),
            tea::Cmd::task(|| "while released".to_string()),
            tea::restore_terminal(),
        ]),
        "p" => tea::repaint(),
        "q" => tea::quit(),
        _ => tea::Cmd::none(),
    }
}

impl tea::Model for Life {
    type Msg = String;

    fn update(&mut self, msg: Msg<String>) -> tea::Cmd<String> {
        match msg {
            Msg::WindowSize(_) => tea::Cmd::none(),
            Msg::KeyPress(key) => {
                self.seen.push(format!("key {key}"));
                command(&key.to_string())
            }
            Msg::App(text) => {
                self.seen.push(text);
                tea::Cmd::none()
            }
            other => {
                self.seen.push(format!("{other:?}"));
                tea::Cmd::none()
            }
        }
    }

    fn view(&self) -> tea::View {
        let mut view = self.view.clone();
        view.set_content(format!("seen {}", self.seen.len()));
        view
    }
}

/// A command that records that it ran.
#[derive(Default)]
struct Recorder(Arc<Mutex<bool>>);

impl tea::ExecCommand for Recorder {
    fn run(&mut self) -> io::Result<()> {
        *self.0.lock().unwrap() = true;
        Ok(())
    }
}

fn keys(text: &str) -> Vec<Key> {
    text.chars().map(Key::char).collect()
}

/// A recorder with the mouse on.
fn life() -> Life {
    Life {
        view: tea::View::default().with_mouse_mode(tea::MouseMode::CellMotion),
        ..Life::default()
    }
}

/// Runs [`life`] in a `columns` by 4 terminal with `inputs`, until it
/// ends; the model and all it wrote.
fn run(columns: u16, inputs: impl IntoIterator<Item = Input>) -> (Life, String) {
    let mut test = TestProgram::new(life(), columns, 4);
    for input in inputs {
        test.input(input);
    }
    let raw = String::from_utf8_lossy(test.final_output()).into_owned();
    (test.final_model(), raw)
}

/// Key presses for `text`.
fn typed(text: &str) -> Vec<Input> {
    keys(text).into_iter().map(Input::KeyPress).collect()
}

/// The messages after the colour profile.
fn after_profile(seen: &[String]) -> Vec<&str> {
    seen.iter().skip(1).map(String::as_str).collect()
}

#[test]
fn the_colour_profile_comes_first() {
    let backend = TestBackend::new(20, 4).keys(keys("q"));
    let model = tea::Program::new(life()).run_with(backend).unwrap();
    assert_eq!(model.seen[0], "ColorProfile(TrueColor)");
}

#[test]
fn an_exec_callback_is_the_next_message_and_modes_come_back() {
    let (model, raw) = run(20, typed("exq"));
    assert_eq!(
        after_profile(&model.seen),
        ["key e", "exec true", "after exec", "key x", "key q"]
    );
    let off = raw.find("\x1b[?1002l").expect("mouse off for the child");
    let on_again = raw[off..]
        .find("\x1b[?1002h")
        .expect("mouse on again after");
    assert!(on_again > 0);
    assert_eq!(
        raw.matches("\x1b[?2004h").count(),
        2,
        "paste on twice: {raw:?}"
    );
    assert_eq!(raw.matches("\x1b[>1u").count(), 2, "keyboard pushed twice");
    assert_eq!(raw.matches("\x1b[<u").count(), 2, "and popped twice");
}

#[test]
fn an_exec_that_cannot_start_reports_the_error() {
    let (model, _) = run(20, typed("fq"));
    assert_eq!(
        after_profile(&model.seen),
        ["key f", "exec failed true", "key q"]
    );
}

#[test]
fn exec_reports_how_the_child_exited() {
    let (model, _) = run(20, typed("ghq"));
    assert_eq!(
        after_profile(&model.seen),
        [
            "key g",
            "false failed: exit status: 1",
            "key h",
            "true ok true",
            "key q"
        ]
    );
}

#[test]
fn suspend_and_resume_from_the_key_the_command_and_signals() {
    let inputs = [
        Input::KeyPress(Key::ctrl('z')),
        Input::KeyPress(Key::char('s')),
        Input::Suspend,
        Input::Continue,
        Input::KeyPress(Key::char('q')),
    ];
    let (model, raw) = run(20, inputs);
    assert_eq!(
        after_profile(&model.seen),
        ["Resume", "key s", "Resume", "Resume", "Resume", "key q"],
        "Ctrl-Z never reaches the model"
    );
    assert_eq!(
        raw.matches("\x1b[?1002h").count(),
        5,
        "modes back after each"
    );
}

#[test]
fn without_suspend_ctrl_z_is_a_key() {
    let program = tea::Program::new(Life::default()).without_suspend();
    let mut test = TestProgram::with_terminal(program, TestTerminal::new(20, 4));
    test.press(Key::ctrl('z'));
    test.press(Key::char('q'));
    let model = test.final_model();
    assert_eq!(after_profile(&model.seen), ["key ctrl+z", "key q"]);
}

#[test]
fn a_released_terminal_holds_output_back_until_it_is_restored() {
    let (model, raw) = run(30, typed("rq"));
    assert_eq!(
        after_profile(&model.seen),
        ["key r", "while released", "key q"]
    );
    let restored = raw.rfind("\x1b[?1002h").expect("modes back");
    let printed = raw.find("held back").expect("printed");
    assert!(printed > restored, "printed after the restore: {raw:?}");
}

#[test]
fn repaint_turns_every_mode_off_and_on_and_draws_again() {
    let (_, raw) = run(20, typed("pq"));
    let repaint = raw.find("\x1b[?1002l").expect("modes off");
    let after = &raw[repaint..];
    assert!(after.contains("\x1b[?1002h"), "{raw:?}");
    assert!(
        after.contains("\x1b[Jseen 2"),
        "drawn again in full: {raw:?}"
    );
}

/// A model whose view is coloured and bold, and which records the
/// colour profiles it hears.
#[derive(Default)]
struct Painter(Vec<ColorProfile>);

impl tea::Model for Painter {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        match msg {
            Msg::ColorProfile(p) => self.0.push(p),
            Msg::KeyPress(k) if k == "q" => {
                return tea::sequence([tea::println("\x1b[31mprinted\x1b[m"), tea::quit()]);
            }
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        tea::View::new("\x1b[1;38;2;10;200;30mgreen\x1b[m")
            .with_background_color(tea::Rgb::new(0x12, 0x34, 0x56))
    }
}

/// Runs `program` in `terminal`, after `bytes` from the terminal, until
/// q; the profiles heard and all it wrote.
fn paint(
    program: tea::Program<Painter>,
    terminal: TestTerminal,
    bytes: &[u8],
) -> (Vec<ColorProfile>, String) {
    let mut test = TestProgram::with_terminal(program, terminal);
    test.bytes(bytes);
    test.press(Key::char('q'));
    test.advance(Duration::from_secs(1));
    let raw = String::from_utf8_lossy(test.final_output()).into_owned();
    (test.final_model().0, raw)
}

/// An RGB capability reply.
const RGB: &[u8] = b"\x1bP1+r524742\x1b\\";

#[test]
fn the_environment_sets_the_profile_and_no_color_keeps_only_attributes() {
    let env = [("TERM", "xterm-256color"), ("NO_COLOR", "1")];
    let program = tea::Program::new(Painter::default()).with_environment(env.into_iter().collect());
    let (profiles, raw) = paint(program, TestTerminal::new(20, 3).terminal(), b"");
    assert_eq!(profiles, [ColorProfile::Ascii]);
    assert!(raw.contains("\x1b[1mgreen"), "bold kept: {raw:?}");
    assert!(!raw.contains("38;") && !raw.contains("[31m"), "{raw:?}");
    assert!(!raw.contains("\x1b]11;"), "no background colour: {raw:?}");
}

#[test]
fn a_256_colour_terminal_gets_palette_colours_until_it_says_rgb() {
    let env = [("TERM", "xterm-256color")];
    let program = tea::Program::new(Painter::default()).with_environment(env.into_iter().collect());
    let (profiles, raw) = paint(program, TestTerminal::new(20, 3).terminal(), RGB);
    assert_eq!(profiles, [ColorProfile::Ansi256, ColorProfile::TrueColor]);
    assert!(raw.contains("38;5;"), "palette first: {raw:?}");
    assert!(
        raw.contains("38;2;10;200;30"),
        "24-bit after the reply: {raw:?}"
    );
    assert!(
        raw.contains("\x1b]11;#005f5f\x07"),
        "palette background: {raw:?}"
    );
}

#[test]
fn a_forced_profile_is_kept_whatever_the_terminal_says() {
    let program = tea::Program::new(Painter::default()).with_color_profile(ColorProfile::Ansi);
    let (profiles, raw) = paint(program, TestTerminal::new(20, 3).terminal(), RGB);
    assert_eq!(profiles, [ColorProfile::Ansi]);
    assert!(!raw.contains("38;2;") && !raw.contains("38;5;"), "{raw:?}");
    assert!(
        raw.contains("\x1b[31m") || raw.contains(";31m"),
        "16 colours: {raw:?}"
    );
}

/// Sends every request at the start, then quits on q.
struct Asker;

impl tea::Model for Asker {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tea::batch([
            tea::request_window_size(),
            tea::request_cursor_position(),
            tea::request_background_color(),
            tea::request_foreground_color(),
            tea::request_cursor_color(),
            tea::request_terminal_version(),
            tea::request_capability("Tc"),
            tea::set_clipboard("hi"),
            tea::set_primary_clipboard(""),
            tea::read_clipboard(),
            tea::read_primary_clipboard(),
        ])
    }

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        match msg {
            Msg::KeyPress(k) if k == "q" => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        tea::View::default()
    }
}

#[test]
fn every_request_writes_what_bubble_tea_writes() {
    let backend = TestBackend::new(20, 3).keys(keys("q"));
    let screen = backend.screen();
    let _ = tea::Program::new(Asker).run_with(backend).unwrap();
    let raw = screen.raw();
    for query in [
        "\x1b[6n",
        "\x1b]11;?\x07",
        "\x1b]10;?\x07",
        "\x1b]12;?\x07",
        "\x1b[>q",
        "\x1bP+q5463\x1b\\",
        "\x1b]52;c;aGk=\x07",
        "\x1b]52;p;\x07",
        "\x1b]52;c;?\x07",
        "\x1b]52;p;?\x07",
    ] {
        assert!(raw.contains(query), "{query:?} missing from {raw:?}");
    }
}

/// Records replies, and quits on q.
#[derive(Default)]
struct Listener(Vec<String>);

impl tea::Model for Listener {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        match msg {
            Msg::KeyPress(k) if k == "q" => return tea::quit(),
            Msg::ColorProfile(_) | Msg::App(()) => {}
            other => self.0.push(format!("{other:?}")),
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        tea::View::default()
    }
}

#[test]
fn every_reply_arrives_as_its_message() {
    let replies: &[&[u8]] = &[
        b"\x1b[5;7R",
        b"\x1b]11;rgb:1111/2222/3333\x1b\\",
        b"\x1b]10;#eeeeee\x07",
        b"\x1b]12;rgb:00/ff/00\x07",
        b"\x1bP>|fake 1.0\x1b\\",
        b"\x1bP1+r5463\x1b\\",
        b"\x1b]52;p;aGk=\x07",
        b"\x1b[?62;22c",
        b"\x1b[>41;354;0c",
        b"\x1bP!|7E565445\x1b\\",
        b"\x1b[?997;1n",
        b"\x1b[8;30;100t",
        b"\x1b[4;600;800t",
        b"\x1b[6;16;8t",
    ];
    let mut test = TestProgram::new(Listener::default(), 20, 3);
    for reply in replies {
        test.bytes(reply);
    }
    test.press(Key::char('q'));
    let model = test.final_model();
    assert_eq!(
        model.0,
        [
            "WindowSize(Size { width: 20, height: 3 })",
            "CursorPosition { x: 6, y: 4 }",
            "BackgroundColor(Rgb { r: 17, g: 34, b: 51 })",
            "ForegroundColor(Rgb { r: 238, g: 238, b: 238 })",
            "CursorColor(Rgb { r: 0, g: 255, b: 0 })",
            "TerminalVersion(\"fake 1.0\")",
            "Capability(\"Tc\")",
            "Clipboard { content: \"hi\", selection: Primary }",
            "PrimaryDeviceAttributes([62, 22])",
            "SecondaryDeviceAttributes([41, 354, 0])",
            "TertiaryDeviceAttributes(\"~VTE\")",
            "ColorScheme(Dark)",
            "WindowSize(Size { width: 100, height: 30 })",
            "PixelSize(Size { width: 800, height: 600 })",
            "CellSize(Size { width: 8, height: 16 })",
        ]
    );
}

#[test]
fn request_window_size_sends_the_size_again() {
    struct Sizes(u32);
    impl tea::Model for Sizes {
        type Msg = ();
        fn init(&mut self) -> tea::Cmd<()> {
            tea::sequence([tea::request_window_size(), tea::quit()])
        }
        fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
            if matches!(msg, Msg::WindowSize(_)) {
                self.0 += 1;
            }
            tea::Cmd::none()
        }
        fn view(&self) -> tea::View {
            tea::View::default()
        }
    }
    let sizes = TestProgram::new(Sizes(0), 20, 3).final_model();
    assert_eq!(sizes.0, 2, "once at the start, once asked");
}

/// Asks one question at the start and quits at once.
struct Quick;

impl tea::Model for Quick {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        tea::sequence([tea::request_background_color(), tea::quit()])
    }

    fn update(&mut self, _: Msg<()>) -> tea::Cmd<()> {
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        tea::View::default()
    }
}

#[test]
fn an_unanswered_query_delays_the_end_by_half_a_second() {
    let terminal = TestTerminal::new(20, 3).terminal();
    let test = TestProgram::with_terminal(tea::Program::new(Quick), terminal);
    assert_eq!(
        test.elapsed(),
        Duration::from_millis(500),
        "waited for the reply"
    );
    assert!(test.final_output().ends_with(b"\x1b[?7h"));
    let raw = String::from_utf8_lossy(test.output()).into_owned();
    assert!(raw.contains("\x1b[c"), "the sentinel was asked");
}

#[test]
fn an_answered_sentinel_ends_the_wait_at_once() {
    let terminal = TestTerminal::new(20, 3)
        .terminal()
        .reply("\x1b[c", "\x1b]11;rgb:0000/0000/0000\x07\x1b[?62c");
    let test = TestProgram::with_terminal(tea::Program::new(Quick), terminal);
    assert!(test.is_finished());
    assert_eq!(test.elapsed(), Duration::ZERO);
}

#[test]
fn a_backend_that_is_not_a_terminal_is_never_waited_for() {
    let test = TestProgram::new(Quick, 20, 3);
    assert!(test.is_finished());
    assert_eq!(test.elapsed(), Duration::ZERO);
    let raw = String::from_utf8_lossy(test.output()).into_owned();
    assert!(!raw.contains("\x1b[c"));
}

#[test]
fn a_test_backend_that_is_a_terminal_waits_for_replies_on_the_real_clock() {
    let backend = TestBackend::new(20, 3)
        .terminal()
        .gap(Duration::from_millis(50))
        .bytes(b"\x1b]11;rgb:0000/0000/0000\x07\x1b[?62c");
    let screen = backend.screen();
    let _ = tea::Program::new(Quick).run_with(backend).unwrap();
    assert!(screen.raw().contains("\x1b[c"), "the sentinel was asked");
}
