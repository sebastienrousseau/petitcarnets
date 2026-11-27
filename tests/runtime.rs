//! The runtime, driven headless through `TestBackend`.

use petitcarnets as tea;
use std::time::Duration;
use tea::testing::TestBackend;
use tea::{Input, Key, Size};

/// Records what it saw; its commands are chosen by key.
#[derive(Debug, Default)]
struct Probe {
    count: i32,
    sizes: Vec<Size>,
    messages: Vec<&'static str>,
    keys: Vec<String>,
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
            tea::Msg::KeyPress(key) => return self.key(key),
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        format!("count {}\nmessages {}", self.count, self.messages.join(",")).into()
    }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

impl Probe {
    fn key(&mut self, key: Key) -> tea::Cmd<&'static str> {
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
    let (model, screen) = run([Key::Up, Key::Up, Key::Down, Key::Up, Key::Char('q')]);
    assert_eq!(model.count, 2);
    assert!(screen.contains("count 2"), "{screen}");
}

#[test]
fn the_model_hears_the_window_size_first_and_after_each_resize() {
    let model = run_inputs([
        Input::Resize(Size::new(100, 30)),
        Input::Key(Key::Char('q')),
    ])
    .unwrap();
    assert_eq!(model.sizes, [Size::new(40, 10), Size::new(100, 30)]);
}

#[test]
fn a_task_and_a_tick_send_their_messages_back() {
    let (model, screen) = run([Key::Char('t'), Key::Char('k')]);
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
    let (model, _) = run([Key::Char('b'), Key::Char('k')]);
    let mut seen = model.messages.clone();
    seen.sort_unstable();
    assert_eq!(seen, ["done", "one", "two"]);
}

#[test]
fn a_sequence_delivers_in_order_then_quits() {
    let (model, _) = run([Key::Char('s')]);
    assert_eq!(model.messages, ["first", "second", "third"]);
}

#[test]
fn ctrl_c_is_a_key_as_in_bubble_tea() {
    let (model, _) = run([Key::Ctrl('c'), Key::Up, Key::Char('q')]);
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
        run_inputs([Input::Key(Key::Char('i'))]),
        Err(tea::Error::Interrupted)
    ));
}

#[test]
fn a_panicking_task_stops_the_program_with_its_message() {
    match run_inputs([Input::Key(Key::Char('p'))]) {
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
