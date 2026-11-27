//! The runtime, driven headless through `TestBackend`.

use petitcarnets as tea;
use std::time::Duration;
use tea::testing::TestBackend;
use tea::{Input, Key, KeyCode, Size};

/// Records what it saw; its commands are chosen by key.
#[derive(Debug, Default)]
struct Probe {
    count: i32,
    sizes: Vec<Size>,
    messages: Vec<&'static str>,
    keys: Vec<String>,
    events: Vec<String>,
    view: tea::View,
}

impl tea::Model for Probe {
    type Msg = &'static str;

    fn update(&mut self, msg: tea::Msg<&'static str>) -> tea::Cmd<&'static str> {
        match msg {
            tea::Msg::WindowSize(size) => self.sizes.push(size),
            tea::Msg::App(m) => {
                self.messages.push(m);
                if m == "done" {
                    return tea::quit();
                }
            }
            tea::Msg::KeyPress(key) => return self.key(&key),
            other => self.events.push(describe(&other)),
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let mut view = self.view.clone();
        view.set_content(format!(
            "count {}\nmessages {}",
            self.count,
            self.messages.join(",")
        ));
        view
    }
}

/// A message other than a key press or the window size, briefly.
fn describe(msg: &tea::Msg<&'static str>) -> String {
    match msg {
        tea::Msg::KeyRelease(key) => format!("release {key}"),
        tea::Msg::MouseClick(m) => format!("click {m} {},{}", m.x, m.y),
        tea::Msg::MouseRelease(m) => format!("mouse-release {m} {},{}", m.x, m.y),
        tea::Msg::MouseWheel(m) => format!("wheel {m}"),
        tea::Msg::MouseMotion(m) => format!("motion {}", m.motion_string()),
        tea::Msg::Paste(text) => format!("paste {text}"),
        tea::Msg::PasteStart => "paste-start".into(),
        tea::Msg::PasteEnd => "paste-end".into(),
        tea::Msg::Focus => "focus".into(),
        tea::Msg::Blur => "blur".into(),
        other => format!("{other:?}"),
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

impl Probe {
    fn key(&mut self, key: &Key) -> tea::Cmd<&'static str> {
        self.keys.push(key.to_string());
        match key.to_string().as_str() {
            "up" => self.count += 1,
            "down" => self.count -= 1,
            "t" => return tea::Cmd::task(|| "from a task"),
            "k" => return tea::tick(ms(20), |_| "done"),
            "b" => return tea::batch([tea::Cmd::task(|| "one"), tea::Cmd::task(|| "two")]),
            "s" => {
                return tea::sequence([
                    tea::tick(ms(15), |_| "first"),
                    tea::Cmd::task(|| "second"),
                    tea::tick(ms(1), |_| "third"),
                    tea::quit(),
                ]);
            }
            "p" => return tea::Cmd::task(|| panic!("task failed")),
            "i" => return tea::interrupt(),
            "m" => {
                self.view =
                    std::mem::take(&mut self.view).with_mouse_mode(tea::MouseMode::CellMotion);
            }
            "q" => return tea::quit(),
            _ => {}
        }
        tea::Cmd::none()
    }
}

fn run(keys: impl IntoIterator<Item = Key>) -> (Probe, String) {
    let backend = TestBackend::new(40, 10).keys(keys);
    let screen = backend.screen();
    let model = tea::Program::new(Probe::default())
        .run_with(backend)
        .expect("the program runs");
    (model, screen.text())
}

fn run_inputs(inputs: impl IntoIterator<Item = Input>) -> Result<Probe, tea::Error> {
    tea::Program::new(Probe::default()).run_with(TestBackend::new(40, 10).inputs(inputs))
}

#[test]
fn keys_update_the_model_and_the_screen_shows_the_last_view() {
    let (up, down) = (Key::new(KeyCode::Up), Key::new(KeyCode::Down));
    let (model, screen) = run([up.clone(), up.clone(), down, up, Key::char('q')]);
    assert_eq!(model.count, 2);
    // Frames rewrite only the cells that change: "count 0", then "1",
    // "2", "1", "2".
    assert!(screen.trim_start().starts_with("count 0"), "{screen}");
    assert!(screen.trim_end().ends_with("1212"), "{screen}");
}

#[test]
fn the_model_hears_the_window_size_first_and_after_each_resize() {
    let model = run_inputs([
        Input::Resize(Size::new(100, 30)),
        Input::KeyPress(Key::char('q')),
    ])
    .unwrap();
    assert_eq!(model.sizes, [Size::new(40, 10), Size::new(100, 30)]);
}

#[test]
fn a_task_and_a_tick_send_their_messages_back() {
    let (model, screen) = run([Key::char('t'), Key::char('k')]);
    assert!(
        model.messages.contains(&"from a task"),
        "{:?}",
        model.messages
    );
    assert_eq!(model.messages.last(), Some(&"done"), "the tick quit");
    assert!(screen.contains("from a task"), "{screen}");
}

#[test]
fn a_batch_runs_every_command() {
    let (model, _) = run([Key::char('b'), Key::char('k')]);
    let mut seen = model.messages.clone();
    seen.sort_unstable();
    assert_eq!(seen, ["done", "one", "two"]);
}

#[test]
fn a_sequence_delivers_in_order_then_quits() {
    let (model, _) = run([Key::char('s')]);
    assert_eq!(model.messages, ["first", "second", "third"]);
}

#[test]
fn ctrl_c_is_a_key_as_in_bubble_tea() {
    let (model, _) = run([Key::ctrl('c'), Key::new(KeyCode::Up), Key::char('q')]);
    assert_eq!(model.keys, ["ctrl+c", "up", "q"]);
    assert_eq!(model.count, 1);
}

#[test]
fn sigint_interrupts_and_sigterm_quits() {
    assert!(matches!(
        run_inputs([Input::Interrupt]),
        Err(tea::Error::Interrupted)
    ));
    assert!(run_inputs([Input::Terminate]).is_ok());
    assert!(matches!(
        run_inputs([Input::KeyPress(Key::char('i'))]),
        Err(tea::Error::Interrupted)
    ));
}

#[test]
fn a_panicking_task_stops_the_program_with_its_message() {
    match run_inputs([Input::KeyPress(Key::char('p'))]) {
        Err(tea::Error::Panic(message)) => assert_eq!(message, "task failed"),
        other => panic!("expected a panic error, got {other:?}"),
    }
}

#[test]
fn a_sender_delivers_messages_and_can_quit_or_kill() {
    let program = tea::Program::new(Probe::default());
    let sender = program.sender();
    sender.send("before start");
    sender.quit();
    let model = program.run_with(TestBackend::new(10, 3)).unwrap();
    assert_eq!(model.messages, ["before start"]);

    let program = tea::Program::new(Probe::default());
    let sender = program.sender();
    let killer = std::thread::spawn(move || {
        std::thread::sleep(ms(20));
        sender.kill();
    });
    let result = program.run_with(TestBackend::new(10, 3));
    killer.join().unwrap();
    assert!(matches!(result, Err(tea::Error::Killed)));
}

#[test]
fn init_runs_its_command() {
    #[derive(Debug, Default)]
    struct Starter(bool);
    impl tea::Model for Starter {
        type Msg = ();
        fn init(&mut self) -> tea::Cmd<()> {
            tea::tick(ms(5), |_| ())
        }
        fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
            if matches!(msg, tea::Msg::App(())) {
                self.0 = true;
                return tea::quit();
            }
            tea::Cmd::none()
        }
        fn view(&self) -> tea::View {
            tea::View::default()
        }
    }
    let model = tea::Program::new(Starter::default())
        .run_with(TestBackend::new(10, 3))
        .unwrap();
    assert!(model.0);
}

/// Runs a fresh probe on raw terminal bytes, then `q`.
fn run_bytes(view: tea::View, chunks: &[&[u8]]) -> (Probe, String) {
    let mut backend = TestBackend::new(40, 10);
    for chunk in chunks {
        backend = backend.bytes(chunk);
    }
    let backend = backend.keys([Key::char('q')]);
    let screen = backend.screen();
    let probe = Probe {
        view,
        ..Probe::default()
    };
    let model = tea::Program::new(probe).run_with(backend).unwrap();
    let raw = screen.raw();
    // Autowrap goes off before anything else, and back on at the very end.
    let raw = raw.strip_prefix("\x1b[?7l").expect("autowrap off first");
    let raw = raw.strip_suffix("\x1b[?7h").expect("autowrap on last");
    let raw = raw.to_string();
    (model, raw)
}

#[test]
fn raw_bytes_go_through_the_decoder() {
    let (model, _) = run_bytes(tea::View::default(), &[b"\x1b[A\x1b[Ax\x03"]);
    assert_eq!(model.keys, ["up", "up", "x", "ctrl+c", "q"]);
    assert_eq!(model.count, 2);
}

#[test]
fn a_mouse_click_arrives_with_its_cell() {
    let (model, _) = run_bytes(tea::View::default(), &[b"\x1b[<0;10;5M\x1b[<0;10;5m"]);
    assert_eq!(model.events, ["click left 9,4", "mouse-release left 9,4"]);
    let (model, _) = run_bytes(tea::View::default(), &[b"\x1b[<20;300;200M\x1b[<65;1;1M"]);
    assert_eq!(
        model.events,
        ["click ctrl+shift+left 299,199", "wheel wheeldown"]
    );
}

#[test]
fn a_paste_arrives_as_one_message_even_across_reads() {
    let chunks: [&[u8]; 3] = [b"\x1b[200~hello ", b"wor", b"ld\x1b[201~"];
    let (model, _) = run_bytes(tea::View::default(), &chunks);
    assert_eq!(
        model.events,
        ["paste-start", "paste hello world", "paste-end"]
    );
    assert_eq!(model.keys, ["q"], "no key leaks out of the paste");
}

#[test]
fn focus_and_blur_arrive() {
    let (model, _) = run_bytes(tea::View::default(), &[b"\x1b[I", b"\x1b[O"]);
    assert_eq!(model.events, ["focus", "blur"]);
}

#[test]
fn a_kitty_release_arrives_as_a_key_release() {
    let (model, _) = run_bytes(tea::View::default(), &[b"\x1b[97;5:3u\x1b[97u"]);
    assert_eq!(model.events, ["release ctrl+a"]);
    assert_eq!(model.keys, ["a", "q"]);
}

#[test]
fn a_lone_escape_is_the_escape_key() {
    let (model, _) = run_bytes(tea::View::default(), &[b"\x1b"]);
    assert_eq!(model.keys, ["esc", "q"]);
}

#[test]
fn view_modes_are_turned_on_then_off_at_the_end() {
    let enhancements = tea::KeyboardEnhancements::new().report_event_types();
    let view = tea::View::default()
        .with_mouse_mode(tea::MouseMode::AllMotion)
        .with_report_focus(true)
        .with_keyboard_enhancements(enhancements);
    let (_, raw) = run_bytes(view, &[]);
    let on = "\x1b[?2004h\x1b[?1004h\x1b[?1003h\x1b[?1006h\x1b[>4;2m\x1b[>3u";
    let off = "\x1b[>4m\x1b[<u\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1004l\x1b[?2004l";
    assert!(raw.starts_with(on), "{raw:?}");
    assert!(raw.ends_with(off), "{raw:?}");
    assert_eq!(
        raw.matches(on).count(),
        1,
        "modes are set once, not every frame"
    );
}

#[test]
fn defaults_turn_on_paste_and_key_disambiguation_only() {
    let (_, raw) = run_bytes(tea::View::default(), &[]);
    assert!(raw.starts_with("\x1b[?2004h\x1b[>4;2m\x1b[>1u"), "{raw:?}");
    assert!(raw.ends_with("\x1b[>4m\x1b[<u\x1b[?2004l"), "{raw:?}");
    assert!(!raw.contains("1004") && !raw.contains("1002") && !raw.contains("1003"));
    let no_paste = tea::View::default().with_bracketed_paste(false);
    let (_, raw) = run_bytes(no_paste, &[]);
    assert!(!raw.contains("2004"), "{raw:?}");
}

#[test]
fn a_changed_view_changes_only_what_differs() {
    let (_, raw) = run_bytes(tea::View::default(), &[b"m"]);
    let after_m = raw.split("\x1b[?1002h").nth(1).expect("mouse turned on");
    assert!(raw.matches("\x1b[?2004h").count() == 1, "{raw:?}");
    assert!(
        after_m.contains("\x1b[?1002l\x1b[?1003l\x1b[?1006l"),
        "turned off at the end"
    );
}

#[test]
fn changed_keyboard_flags_update_the_entry_in_place() {
    #[derive(Debug, Default)]
    struct Flags(bool);
    impl tea::Model for Flags {
        type Msg = ();
        fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
            match msg {
                tea::Msg::KeyPress(k) if k == "q" => tea::quit(),
                tea::Msg::KeyPress(_) => {
                    self.0 = true;
                    tea::Cmd::none()
                }
                _ => tea::Cmd::none(),
            }
        }
        fn view(&self) -> tea::View {
            tea::View::new("flags").with_keyboard_enhancements(if self.0 {
                tea::KeyboardEnhancements::new().report_alternate_keys()
            } else {
                tea::KeyboardEnhancements::new()
            })
        }
    }
    let backend = TestBackend::new(10, 3).keys([Key::char('x'), Key::char('q')]);
    let screen = backend.screen();
    let _ = tea::Program::new(Flags::default())
        .run_with(backend)
        .unwrap();
    let raw = screen.raw();
    assert_eq!(raw.matches("\x1b[>1u").count(), 1, "{raw:?}");
    assert!(raw.contains("\x1b[>4;2m\x1b[=5;1u"), "{raw:?}");
    assert_eq!(raw.matches("\x1b[<u").count(), 1, "popped once, at the end");
}

/// A model whose keys print, write raw bytes, clear the screen, toggle
/// the alternate screen and quit; its view shows a count of updates.
#[derive(Debug, Default)]
struct Printer {
    alt: bool,
    n: u32,
    ticks: u32,
}

impl tea::Model for Printer {
    type Msg = u32;

    fn update(&mut self, msg: tea::Msg<u32>) -> tea::Cmd<u32> {
        self.n += 1;
        match msg {
            tea::Msg::KeyPress(k) => match k.to_string().as_str() {
                "p" => tea::printf!("printed {}", self.n),
                "r" => tea::raw("<RAW>"),
                "c" => tea::clear_screen(),
                "a" => {
                    self.alt = !self.alt;
                    tea::Cmd::none()
                }
                "q" => tea::sequence([tea::println("bye"), tea::raw("<LAST>"), tea::quit()]),
                _ => tea::Cmd::none(),
            },
            tea::Msg::App(left) if left > 0 => {
                self.ticks += 1;
                tea::tick(ms(1), move |_| left - 1)
            }
            tea::Msg::App(_) => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        tea::View::new(format!("frame {} ticks {}\n", self.n, self.ticks)).with_alt_screen(self.alt)
    }
}

fn run_printer(backend: TestBackend) -> String {
    let screen = backend.screen();
    let _ = tea::Program::new(Printer::default())
        .run_with(backend)
        .unwrap();
    screen.raw()
}

#[test]
fn views_are_inline_by_default() {
    let raw = run_printer(TestBackend::new(20, 5).keys([Key::char('q')]));
    assert!(!raw.contains("1049"), "{raw:?}");
    assert!(raw.contains("frame 1"), "{raw:?}");
}

#[test]
fn the_view_switches_the_alternate_screen_on_and_off() {
    let keys = [
        Key::char('a'),
        Key::char('x'),
        Key::char('a'),
        Key::char('q'),
    ];
    let raw = run_printer(TestBackend::new(20, 5).keys(keys));
    let on = raw.find("\x1b[?1049h").expect("entered");
    let off = raw.find("\x1b[?1049l").expect("left");
    assert!(on < off);
    assert_eq!(raw.matches("\x1b[?1049h").count(), 1);
    assert_eq!(raw.matches("\x1b[?1049l").count(), 1);
}

#[test]
fn the_alternate_screen_is_left_at_the_end() {
    let raw = run_printer(TestBackend::new(20, 5).keys([Key::char('a'), Key::char('q')]));
    let off = raw.rfind("\x1b[?1049l").expect("left at the end");
    assert!(raw[off..].ends_with("\x1b[?7h"), "{raw:?}");
}

#[test]
fn println_comes_after_the_frame_before_it_and_before_quitting() {
    let raw = run_printer(TestBackend::new(30, 5).keys([Key::char('p'), Key::char('q')]));
    let frame = raw.find("frame 1").expect("first frame");
    let printed = raw.find("printed 2").expect("printed");
    let bye = raw.find("bye").expect("bye");
    let last = raw.find("<LAST>").expect("raw");
    let restore = raw.rfind("\x1b[?2004l").expect("modes turned off");
    assert!(frame < printed && printed < bye && bye < last && last < restore);
}

#[test]
fn raw_bytes_are_written_before_the_terminal_is_restored() {
    let raw = run_printer(TestBackend::new(20, 5).keys([Key::char('r'), Key::char('q')]));
    let at = raw.find("<RAW>").expect("written");
    assert!(at < raw.rfind("\x1b[?7h").unwrap());
}

#[test]
fn clear_screen_clears_and_writes_the_modes_again() {
    let raw = run_printer(TestBackend::new(20, 5).keys([Key::char('c'), Key::char('q')]));
    let clear = raw.find("\x1b[H\x1b[2J").expect("cleared");
    assert_eq!(raw.matches("\x1b[?2004h").count(), 2, "{raw:?}");
    assert!(raw.rfind("\x1b[?2004h").unwrap() < clear);
}

/// Runs a printer that ticks `n` times, 1 ms apart, at `fps`, with
/// synchronized output on so frames can be counted, and `keys` keys
/// 20 ms apart.
fn ticking(n: u32, fps: u32, keys: usize) -> (String, Printer) {
    let mut backend = TestBackend::new(30, 5).inputs([Input::ModeReport {
        mode: tea::ReportedMode::Dec(2026),
        value: tea::ModeSetting::Reset,
    }]);
    if keys > 0 {
        backend = backend
            .gap(ms(20))
            .keys(std::iter::repeat_n(Key::char('x'), keys))
            .keys([Key::char('q')]);
    }
    let screen = backend.screen();
    let program = tea::Program::new(Printer::default()).with_fps(fps);
    if n > 0 {
        program.sender().send(n);
    }
    let model = program.run_with(backend).unwrap();
    (screen.raw(), model)
}

#[test]
fn with_fps_limits_how_often_frames_are_drawn() {
    let started = std::time::Instant::now();
    let (raw, model) = ticking(300, 10, 0);
    let tenths = usize::try_from(started.elapsed().as_millis() / 100).unwrap();
    let frames = raw.matches("\x1b[?2026h").count();
    assert!(model.ticks >= 300);
    assert!(
        frames <= tenths + 3,
        "{frames} frames in {tenths} tenths at 10 fps"
    );
    assert!(frames >= 2, "{frames}");
}

#[test]
fn a_frame_after_a_key_is_drawn_at_once() {
    // At 1 fps, five keys 20 ms apart still each get a frame at once.
    let started = std::time::Instant::now();
    let (raw, _) = ticking(0, 1, 5);
    let frames = raw.matches("\x1b[?2026h").count();
    assert!(started.elapsed() < ms(900), "never held back for a second");
    assert!(frames >= 6, "{frames} frames: {raw:?}");
}

/// Shows one line, without a trailing newline, until `x` empties it.
#[derive(Debug, Default)]
struct OneLine(bool);

impl tea::Model for OneLine {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k == "x" => self.0 = true,
            tea::Msg::KeyPress(k) if k == "q" => return tea::quit(),
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        tea::View::new(if self.0 { "" } else { "last line" })
    }
}

fn run_one_line(keys: &str) -> String {
    let backend = TestBackend::new(20, 5).keys(keys.chars().map(Key::char));
    let screen = backend.screen();
    let _ = tea::Program::new(OneLine::default())
        .run_with(backend)
        .unwrap();
    screen.raw()
}

#[test]
fn quitting_inline_keeps_the_last_line_and_moves_below_it() {
    let raw = run_one_line("q");
    let line = raw.rfind("last line").expect("drawn");
    let below = raw.rfind("\r\n\x1b[J").expect("a fresh line, erased below");
    assert!(line < below, "{raw:?}");
    assert!(
        !raw[line..below].contains('\r'),
        "not erased first: {raw:?}"
    );
}

#[test]
fn an_empty_last_view_erases_the_frame_and_stays_where_it_started() {
    let raw = run_one_line("xq");
    assert!(!raw.contains("\r\n"), "never moves below: {raw:?}");
    let erase = raw.rfind("\x1b[J").expect("erased");
    assert!(erase > raw.rfind("last line").unwrap());
}
