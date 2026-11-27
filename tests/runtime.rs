//! The runtime, driven headless: through `TestProgram` on a virtual
//! clock, and a few runs through `TestBackend` on the real one, which
//! is public too.

use petitcarnets as tea;
use std::time::Duration;
use tea::testing::{TestBackend, TestProgram, TestTerminal};
use tea::{Input, Key, KeyCode, Size};

/// Records what it saw; its commands are chosen by key.
#[derive(Debug, Default)]
struct Probe {
    count: i32,
    sizes: Vec<Size>,
    messages: Vec<&'static str>,
    keys: Vec<String>,
    events: Vec<String>,
    profiles: Vec<tea::ColorProfile>,
    view: tea::View,
}

impl tea::Model for Probe {
    type Msg = &'static str;

    fn update(&mut self, msg: tea::Msg<&'static str>) -> tea::Cmd<&'static str> {
        match msg {
            tea::Msg::WindowSize(size) => self.sizes.push(size),
            tea::Msg::ColorProfile(profile) => self.profiles.push(profile),
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

/// Runs a probe with `keys`, then lets it finish; the model, and the
/// text written without escape sequences.
fn run(keys: impl IntoIterator<Item = Key>) -> (Probe, String) {
    let mut test = TestProgram::new(Probe::default(), 40, 10);
    for key in keys {
        test.press(key);
    }
    assert!(test.wait_finished(Duration::from_secs(1)), "it ends");
    let written = tea::style::strip_ansi(&String::from_utf8_lossy(test.output()));
    (test.into_result().expect("the program runs"), written)
}

/// Runs a probe with `inputs` until it ends.
fn run_inputs(inputs: impl IntoIterator<Item = Input>) -> Result<Probe, tea::Error> {
    let mut test = TestProgram::new(Probe::default(), 40, 10);
    for input in inputs {
        test.input(input);
    }
    test.into_result()
}

#[test]
fn a_test_backend_plays_keys_on_the_real_clock() {
    let (up, down) = (Key::new(KeyCode::Up), Key::new(KeyCode::Down));
    let backend = TestBackend::new(40, 10).keys([up.clone(), up.clone(), down, up, Key::char('q')]);
    let screen = backend.screen();
    let model = tea::Program::new(Probe::default())
        .run_with(backend)
        .expect("the program runs");
    assert_eq!(model.count, 2);
    // On the real clock, how keys group into frames depends on timing
    // (a slow machine, or Miri, can draw two keys in one frame), so the
    // check is the screen the bytes leave behind, not the bytes.
    let mut vt = tea::testing::Vt::new(40, 10);
    vt.feed(screen.raw().as_bytes());
    assert!(vt.text().contains("count 2"), "{}", vt.text());
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

    let mut test = TestProgram::new(Probe::default(), 10, 3);
    let sender = test.sender();
    std::thread::spawn(move || sender.kill()).join().unwrap();
    test.advance(Duration::ZERO);
    assert!(matches!(test.into_result(), Err(tea::Error::Killed)));
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
    let mut test = TestProgram::new(Starter::default(), 10, 3);
    test.advance(ms(4));
    assert!(!test.model().0);
    test.advance(ms(1));
    assert!(test.final_model().0);
}

/// Runs a fresh probe on raw terminal bytes, one read each, then `q`;
/// the output, minus autowrap turned off first and on last.
fn run_bytes(view: tea::View, chunks: &[&[u8]]) -> (Probe, String) {
    let probe = Probe {
        view,
        ..Probe::default()
    };
    let mut test = TestProgram::new(probe, 40, 10);
    for chunk in chunks {
        test.bytes(chunk);
    }
    test.advance(ms(50));
    test.press(Key::char('q'));
    let raw = String::from_utf8_lossy(test.final_output()).into_owned();
    // Autowrap goes off before anything else, and back on at the very end.
    let raw = raw.strip_prefix("\x1b[?7l").expect("autowrap off first");
    let raw = raw.strip_suffix("\x1b[?7h").expect("autowrap on last");
    let raw = raw.to_string();
    (test.final_model(), raw)
}

#[test]
fn a_test_backend_sends_bytes_through_the_decoder() {
    let backend = TestBackend::new(40, 10)
        .bytes(b"\x1b[A\x1b[Ax\x03")
        .keys([Key::char('q')]);
    let model = tea::Program::new(Probe::default())
        .run_with(backend)
        .unwrap();
    assert_eq!(model.keys, ["up", "up", "x", "ctrl+c", "q"]);
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
    let mut test = TestProgram::new(Flags::default(), 10, 3);
    test.type_text("xq");
    let raw = String::from_utf8_lossy(test.final_output()).into_owned();
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
        if matches!(msg, tea::Msg::ColorProfile(_)) {
            return tea::Cmd::none();
        }
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

/// Runs a printer in a `columns` by 5 terminal with `keys`; all it
/// wrote.
fn run_printer(columns: u16, keys: &str) -> String {
    let mut test = TestProgram::new(Printer::default(), columns, 5);
    test.type_text(keys);
    String::from_utf8_lossy(test.final_output()).into_owned()
}

#[test]
fn views_are_inline_by_default() {
    let raw = run_printer(20, "q");
    assert!(!raw.contains("1049"), "{raw:?}");
    assert!(raw.contains("frame 1"), "{raw:?}");
}

#[test]
fn the_view_switches_the_alternate_screen_on_and_off() {
    let raw = run_printer(20, "axaq");
    let on = raw.find("\x1b[?1049h").expect("entered");
    let off = raw.find("\x1b[?1049l").expect("left");
    assert!(on < off);
    assert_eq!(raw.matches("\x1b[?1049h").count(), 1);
    assert_eq!(raw.matches("\x1b[?1049l").count(), 1);
}

#[test]
fn the_alternate_screen_is_left_at_the_end() {
    let raw = run_printer(20, "aq");
    let off = raw.rfind("\x1b[?1049l").expect("left at the end");
    assert!(raw[off..].ends_with("\x1b[?7h"), "{raw:?}");
}

#[test]
fn println_comes_after_the_frame_before_it_and_before_quitting() {
    let raw = run_printer(30, "pq");
    let frame = raw.find("frame 1").expect("first frame");
    let printed = raw.find("printed 2").expect("printed");
    let bye = raw.find("bye").expect("bye");
    let last = raw.find("<LAST>").expect("raw");
    let restore = raw.rfind("\x1b[?2004l").expect("modes turned off");
    assert!(frame < printed && printed < bye && bye < last && last < restore);
}

#[test]
fn raw_bytes_are_written_before_the_terminal_is_restored() {
    let raw = run_printer(20, "rq");
    let at = raw.find("<RAW>").expect("written");
    assert!(at < raw.rfind("\x1b[?7h").unwrap());
}

#[test]
fn clear_screen_clears_and_writes_the_modes_again() {
    let raw = run_printer(20, "cq");
    let clear = raw.find("\x1b[H\x1b[2J").expect("cleared");
    assert_eq!(raw.matches("\x1b[?2004h").count(), 2, "{raw:?}");
    assert!(raw.rfind("\x1b[?2004h").unwrap() < clear);
}

/// Runs a printer at `fps` that ticks `n` times, 1 ms apart, with
/// synchronized output on so frames can be counted, then gets `keys`
/// keys 20 ms apart and `q`; all it wrote, and the test.
fn ticking(n: u32, fps: u32, keys: usize) -> (String, TestProgram<Printer>) {
    let program = tea::Program::new(Printer::default()).with_fps(fps);
    let mut test = TestProgram::with_terminal(program, TestTerminal::new(30, 5));
    test.input(Input::ModeReport {
        mode: tea::ReportedMode::Dec(2026),
        value: tea::ModeSetting::Reset,
    });
    if n > 0 {
        test.send(n);
        test.advance(ms(u64::from(n)));
    }
    for _ in 0..keys {
        test.advance(ms(20));
        test.press(Key::char('x'));
    }
    test.press(Key::char('q'));
    let raw = String::from_utf8_lossy(test.final_output()).into_owned();
    (raw, test)
}

#[test]
fn with_fps_limits_how_often_frames_are_drawn() {
    let (raw, test) = ticking(300, 10, 0);
    assert_eq!(test.elapsed(), ms(300));
    let frames = raw.matches("\x1b[?2026h").count();
    // One frame each 100 ms of the 300 the ticks take; the last tick
    // quits, with its frame.
    assert_eq!(frames, 3, "{raw:?}");
    assert_eq!(test.final_model().ticks, 300);
}

#[test]
fn a_frame_after_a_key_is_drawn_at_once() {
    // At 1 fps, five keys 20 ms apart still each get a frame at once,
    // and so does the q that ends the run.
    let (raw, test) = ticking(0, 1, 5);
    assert_eq!(test.elapsed(), ms(100), "never held back for a second");
    assert_eq!(raw.matches("\x1b[?2026h").count(), 6, "{raw:?}");
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
    let mut test = TestProgram::new(OneLine::default(), 20, 5);
    test.type_text(keys);
    String::from_utf8_lossy(test.final_output()).into_owned()
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
