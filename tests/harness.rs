//! The test harness itself: the virtual clock, input, tasks, replies,
//! signals and how a run ends.

use petitcarnets as tea;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tea::testing::{TestProgram, TestTerminal};
use tea::{Input, Key, Msg};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Keys choose commands; everything else is recorded.
#[derive(Debug, Default)]
struct Rig {
    seen: Vec<String>,
    updates: u32,
}

/// A command that records that it ran.
#[derive(Default)]
struct Recorder(Arc<Mutex<u32>>);

impl tea::ExecCommand for Recorder {
    fn run(&mut self) -> io::Result<()> {
        *self.0.lock().unwrap() += 1;
        Ok(())
    }
}

/// What a key asks for.
fn command(key: &str) -> tea::Cmd<String> {
    match key {
        "r" => tea::sequence([
            tea::release_terminal(),
            tea::tick(ms(5000), |_| "slept".to_string()),
            tea::restore_terminal(),
        ]),
        "t" => tea::batch((0..8).map(|i| {
            tea::Cmd::task(move || {
                std::thread::sleep(Duration::from_micros(800 - i * 100));
                format!("task {i}")
            })
        })),
        "k" => tea::tick(ms(100), |_| "tick".to_string()),
        "e" => tea::every(Duration::from_secs(1), |at| {
            let since = at.duration_since(UNIX_EPOCH).unwrap();
            format!("every {}", since.subsec_nanos())
        }),
        "z" => tea::tick(Duration::ZERO, |_| "zero".to_string()),
        "x" => tea::exec_command(Recorder::default(), |r| format!("exec {}", r.is_ok())),
        "p" => tea::Cmd::task(|| panic!("task failed")),
        "s" => tea::suspend(),
        "b" => tea::request_background_color(),
        "c" => tea::sequence([tea::release_terminal(), tea::release_terminal()]),
        "u" => tea::sequence([tea::restore_terminal(), tea::restore_terminal()]),
        "P" => tea::println("held back"),
        "X" => tea::sequence([
            tea::release_terminal(),
            tea::exec_command(Recorder::default(), |r| format!("exec {}", r.is_ok())),
            tea::restore_terminal(),
        ]),
        "Q" => tea::sequence([tea::release_terminal(), tea::println("bye"), tea::quit()]),
        "q" => tea::quit(),
        _ => tea::Cmd::none(),
    }
}

impl tea::Model for Rig {
    type Msg = String;

    fn update(&mut self, msg: Msg<String>) -> tea::Cmd<String> {
        self.updates += 1;
        match msg {
            Msg::KeyPress(key) => {
                self.seen.push(format!("key {key}"));
                command(&key.to_string())
            }
            Msg::App(text) if text == "zero" => tea::tick(Duration::ZERO, |_| "zero".into()),
            Msg::App(text) => {
                self.seen.push(text);
                tea::Cmd::none()
            }
            Msg::WindowSize(size) => {
                self.seen
                    .push(format!("size {}x{}", size.width, size.height));
                tea::Cmd::none()
            }
            Msg::ColorProfile(_) => tea::Cmd::none(),
            other => {
                self.seen.push(format!("{other:?}"));
                tea::Cmd::none()
            }
        }
    }

    fn view(&self) -> tea::View {
        format!("updates {}", self.updates).into()
    }
}

fn rig() -> TestProgram<Rig> {
    TestProgram::new(Rig::default(), 20, 3)
}

/// The messages seen after the first window size.
fn seen(test: &TestProgram<Rig>) -> Vec<&str> {
    test.model()
        .seen
        .iter()
        .skip(1)
        .map(String::as_str)
        .collect()
}

#[test]
fn the_first_frame_is_drawn_before_new_returns() {
    let test = rig();
    assert_eq!(test.screen().text(), "updates 2");
    assert_eq!(test.model().seen, ["size 20x3"]);
    assert_eq!(test.elapsed(), Duration::ZERO);
    assert!(!test.is_finished());
}

#[test]
fn a_tick_fires_only_when_the_clock_reaches_it() {
    let mut test = rig();
    test.press(Key::char('k'));
    test.advance(ms(99));
    assert_eq!(seen(&test), ["key k"]);
    test.advance(ms(1));
    assert_eq!(seen(&test), ["key k", "tick"]);
    assert_eq!(test.elapsed(), ms(100));
}

#[test]
fn every_lands_on_a_whole_second_of_the_virtual_wall_clock() {
    let mut test = rig();
    test.press(Key::char('e'));
    test.advance(ms(999));
    assert_eq!(seen(&test), ["key e"]);
    test.advance(ms(1));
    assert_eq!(seen(&test), ["key e", "every 0"]);
}

#[test]
fn task_messages_arrive_in_the_order_the_tasks_started() {
    for _ in 0..5 {
        let mut test = rig();
        test.press(Key::char('t'));
        let tasks: Vec<String> = (0..8).map(|i| format!("task {i}")).collect();
        assert_eq!(seen(&test)[1..], tasks);
        assert_eq!(test.elapsed(), Duration::ZERO, "tasks take no virtual time");
    }
}

#[test]
fn a_lone_escape_waits_for_the_escape_timeout_on_the_virtual_clock() {
    let mut test = rig();
    test.bytes(b"\x1b");
    test.advance(ms(49));
    assert_eq!(seen(&test), [""; 0]);
    test.advance(ms(1));
    assert_eq!(seen(&test), ["key esc"]);
    test.bytes(b"\x1b[");
    test.bytes(b"A");
    assert_eq!(seen(&test), ["key esc", "key up"], "joined across reads");
}

#[test]
fn typed_text_and_input_arrive_in_order() {
    let mut test = rig();
    test.type_text("ab");
    test.input(Input::Focus);
    test.press(Key::char('c'));
    assert_eq!(seen(&test), ["key a", "key b", "Focus", "key c"]);
}

#[test]
fn a_resize_reaches_the_model_and_the_screen() {
    let mut test = rig();
    test.resize(30, 5);
    assert_eq!(seen(&test), ["size 30x5"]);
    assert_eq!(test.screen().size(), (30, 5));
}

#[test]
fn messages_from_another_thread_arrive_on_the_next_call() {
    let mut test = rig();
    let sender = test.sender();
    std::thread::spawn(move || sender.send("from a thread".to_string()))
        .join()
        .unwrap();
    test.advance(Duration::ZERO);
    assert_eq!(seen(&test), ["from a thread"]);
}

#[test]
fn quit_ends_with_the_model_and_a_final_frame() {
    let mut test = rig();
    test.send("hello".to_string());
    test.quit();
    assert!(test.is_finished());
    assert!(test.error().is_none());
    assert_eq!(test.final_screen().text(), "updates 3");
    assert!(test.final_output().ends_with(b"\x1b[?7h"));
    let model = test.into_result().unwrap();
    assert_eq!(model.seen, ["size 20x3", "hello"]);
}

#[test]
fn signals_and_kill_end_the_program_as_they_do_a_real_one() {
    let mut test = rig();
    test.interrupt();
    assert!(matches!(test.error(), Some(tea::Error::Interrupted)));
    let mut test = rig();
    test.terminate();
    assert!(test.is_finished() && test.error().is_none());
    let mut test = rig();
    test.kill();
    assert!(matches!(test.into_result(), Err(tea::Error::Killed)));
}

#[test]
fn a_panicking_task_ends_the_program_with_its_message() {
    let mut test = rig();
    test.press(Key::char('p'));
    match test.error() {
        Some(tea::Error::Panic(message)) => assert_eq!(message, "task failed"),
        other => panic!("expected a panic, got {other:?}"),
    }
    assert_eq!(test.final_model().seen, ["size 20x3", "key p"]);
}

#[test]
fn suspend_hands_over_and_continues_at_once() {
    let mut test = rig();
    test.press(Key::char('s'));
    test.suspend();
    test.press(Key::ctrl('z'));
    assert_eq!(test.suspensions(), 3);
    test.resume();
    assert_eq!(
        seen(&test),
        ["key s", "Resume", "Resume", "Resume", "Resume"]
    );
    assert!(!test.is_released());
}

#[test]
fn exec_runs_the_command_with_the_terminal_handed_over() {
    let mut test = rig();
    test.press(Key::char('x'));
    assert_eq!(seen(&test), ["key x", "exec true"]);
    assert!(!test.is_released());
}

#[test]
fn a_held_back_frame_does_not_wake_the_loop_while_the_terminal_is_released() {
    let program = tea::Program::new(Rig::default()).with_fps(1);
    let mut test = TestProgram::with_terminal(program, TestTerminal::new(20, 3));
    test.press(Key::char('y'));
    test.send("held".to_string());
    test.press(Key::char('r'));
    assert!(test.is_released());
    test.advance(ms(6000));
    assert!(!test.is_released());
    assert_eq!(seen(&test), ["key y", "held", "key r", "slept"]);
}

#[test]
fn the_frame_rate_holds_frames_back_on_the_virtual_clock() {
    let program = tea::Program::new(Rig::default()).with_fps(10);
    let mut test = TestProgram::with_terminal(program, TestTerminal::new(20, 3));
    test.send("now".to_string());
    assert_eq!(test.screen().text(), "updates 2", "held back");
    test.advance(ms(99));
    assert_eq!(test.screen().text(), "updates 2");
    test.advance(ms(1));
    assert_eq!(
        test.screen().text(),
        "updates 3",
        "drawn 100 ms after the last"
    );
    test.press(Key::char('y'));
    assert_eq!(test.screen().text(), "updates 4", "a key draws at once");
}

#[test]
fn wait_for_stops_when_the_condition_holds_or_the_time_is_up() {
    let mut test = rig();
    test.press(Key::char('k'));
    assert!(test.wait_for(ms(500), |t| t.model().seen.len() == 3));
    assert_eq!(test.elapsed(), ms(100), "stopped at the tick");
    assert!(!test.wait_for(ms(250), TestProgram::is_finished));
    assert_eq!(test.elapsed(), ms(350));
    assert!(!test.wait_finished(ms(10)));
    test.press(Key::char('q'));
    assert!(test.wait_finished(ms(10)));
}

/// Asks for the background colour, then quits.
struct Asker;

impl tea::Model for Asker {
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
fn an_unanswered_query_holds_the_end_for_half_a_second_of_virtual_time() {
    let terminal = TestTerminal::new(20, 3).terminal();
    let test = TestProgram::with_terminal(tea::Program::new(Asker), terminal);
    assert!(test.is_finished());
    assert_eq!(test.elapsed(), ms(500));
    assert!(test.output().windows(3).any(|w| w == b"\x1b[c"));
}

#[test]
fn an_answered_sentinel_ends_the_wait_at_once() {
    let terminal = TestTerminal::new(20, 3)
        .terminal()
        .reply("\x1b[c", "\x1b[?62c");
    let test = TestProgram::with_terminal(tea::Program::new(Asker), terminal);
    assert!(test.is_finished());
    assert_eq!(test.elapsed(), Duration::ZERO);
}

#[test]
fn a_scripted_reply_arrives_as_its_message() {
    let terminal =
        TestTerminal::new(20, 3).reply("\x1b]11;?\x07", "\x1b]11;rgb:1111/2222/3333\x07");
    let mut test = TestProgram::with_terminal(tea::Program::new(Rig::default()), terminal);
    test.press(Key::char('b'));
    test.press(Key::char('b'));
    assert_eq!(
        seen(&test),
        [
            "key b",
            "BackgroundColor(Rgb { r: 17, g: 34, b: 51 })",
            "key b",
            "BackgroundColor(Rgb { r: 17, g: 34, b: 51 })"
        ]
    );
}

#[test]
#[should_panic(expected = "still running")]
fn the_final_screen_needs_a_finished_program() {
    let _ = rig().final_screen();
}

#[test]
#[should_panic(expected = "never goes idle")]
fn a_program_that_never_idles_is_reported() {
    let mut test = rig();
    test.press(Key::char('z'));
    test.advance(ms(1));
}

#[test]
fn release_and_restore_happen_once_each_however_often_they_are_asked() {
    let mut test = rig();
    test.press(Key::char('c'));
    assert!(test.is_released());
    test.resume();
    assert!(
        test.is_released(),
        "a continue changes nothing while released"
    );
    test.press(Key::char('u'));
    assert!(!test.is_released());
    test.resume();
    assert_eq!(seen(&test), ["key c", "key u", "Resume"]);
}

#[test]
fn exec_while_released_runs_without_handing_over_again() {
    let mut test = rig();
    test.press(Key::char('X'));
    assert_eq!(seen(&test), ["key X", "exec true"]);
    assert!(!test.is_released());
}

#[test]
fn quitting_while_released_takes_the_terminal_back_first() {
    let mut test = rig();
    test.press(Key::char('Q'));
    assert!(test.is_finished() && test.error().is_none());
    let screen = test.final_screen();
    assert!(screen.text().contains("bye"), "{}", screen.text());
    assert!(!screen.mode(2004) && screen.autowrap());
}

#[test]
fn a_resize_while_released_reaches_the_model_when_it_takes_the_terminal_back() {
    let mut test = rig();
    test.press(Key::char('c'));
    test.resize(30, 6);
    assert_eq!(
        seen(&test),
        ["key c"],
        "the signal is dropped while released"
    );
    test.press(Key::char('u'));
    assert_eq!(seen(&test), ["key c", "key u", "size 30x6"]);
}

#[test]
fn a_test_program_describes_itself() {
    let test = rig();
    let described = format!("{test:?}");
    assert!(described.starts_with("TestProgram {"), "{described}");
    assert!(described.contains("finished: false"), "{described}");
}

#[test]
fn killing_while_released_drops_the_output_that_waited() {
    let mut test = rig();
    test.press(Key::char('c'));
    test.press(Key::char('P'));
    test.kill();
    let output = String::from_utf8_lossy(test.final_output()).into_owned();
    assert!(!output.contains("held back"), "{output:?}");
    assert!(
        output.ends_with("\x1b[?7h"),
        "the terminal is set back: {output:?}"
    );
}

/// Prints more lines than fit above its frame when a key is pressed.
struct TallPrint;

impl tea::Model for TallPrint {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        match msg {
            Msg::KeyPress(k) if k == "p" => {
                let lines: Vec<String> = (1..=12).map(|n| format!("line {n}")).collect();
                tea::println(lines.join("\n"))
            }
            Msg::KeyPress(k) if k == "q" => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        "first row of the view\nsecond row of the view".into()
    }
}

/// Bubble Tea's issue #1822: one print taller than the room above an
/// inline frame used to leave the frame off the screen (its rows pushed
/// into the scrollback) until the next update.
#[test]
fn a_print_taller_than_the_window_leaves_the_frame_on_screen() {
    let mut test = TestProgram::new(TallPrint, 40, 10);
    test.type_text("p");
    test.advance(ms(20));
    let rows: Vec<String> = (0..10).map(|y| test.screen().row_text(y)).collect();
    assert_eq!(rows[8], "first row of the view", "{rows:#?}");
    assert_eq!(rows[9], "second row of the view", "{rows:#?}");
    assert_eq!(rows[7], "line 12", "{rows:#?}");
}

/// Starts a task on Enter; the task's message changes the view.
#[derive(Default)]
struct Loader {
    loaded: bool,
}

impl tea::Model for Loader {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        match msg {
            Msg::KeyPress(k) if k == "enter" => tea::Cmd::task(|| ()),
            Msg::App(()) => {
                self.loaded = true;
                tea::Cmd::none()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        if self.loaded { "loaded" } else { "loading" }.into()
    }
}

/// A call that gives the program something returns with the screen
/// showing the model as it is: here a task's message arrives after the
/// frame for the key that started it, and the frame rate holds the next
/// frame back, which the still clock would never release on its own.
#[test]
fn a_tasks_message_is_on_screen_when_the_call_returns() {
    let mut test = TestProgram::new(Loader::default(), 20, 3);
    let start = test.elapsed();
    test.press(Key::new(tea::KeyCode::Enter));
    assert!(test.model().loaded);
    assert_eq!(test.screen().row_text(0), "loaded");
    assert_eq!(test.elapsed(), start, "and the clock has not moved");
}
