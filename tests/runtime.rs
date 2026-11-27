//! The runtime, driven headless through `TestBackend`.

use petitcarnets::testing::TestBackend;
use petitcarnets::{Carnet, Cmd, Event, Key, Model};
use std::time::Duration;

/// Records what it saw; its commands are chosen by key.
#[derive(Debug, Default)]
struct Probe {
    count: i32,
    sizes: Vec<(u16, u16)>,
    messages: Vec<&'static str>,
    keys: Vec<Key>,
}

impl Model for Probe {
    type Msg = &'static str;

    fn update(&mut self, event: Event<&'static str>) -> Cmd<&'static str> {
        match event {
            Event::Resize { columns, rows } => self.sizes.push((columns, rows)),
            Event::Msg(m) => {
                self.messages.push(m);
                if m == "done" {
                    return Cmd::quit();
                }
            }
            Event::Key(key) => return self.key(key),
        }
        Cmd::none()
    }

    fn view(&self) -> String {
        format!("count {}\nmessages {}", self.count, self.messages.join(","))
    }
}

impl Probe {
    fn key(&mut self, key: Key) -> Cmd<&'static str> {
        self.keys.push(key);
        match key {
            Key::Up => self.count += 1,
            Key::Down => self.count -= 1,
            Key::Char('t') => return Cmd::task(|| "from a task"),
            Key::Char('k') => return Cmd::tick(Duration::from_millis(20), || "done"),
            Key::Char('b') => return Cmd::batch([Cmd::task(|| "one"), Cmd::task(|| "two")]),
            Key::Char('q') => return Cmd::quit(),
            _ => {}
        }
        Cmd::none()
    }
}

fn run(keys: impl IntoIterator<Item = Key>) -> (Probe, String) {
    let backend = TestBackend::new(40, 10).keys(keys);
    let screen = backend.screen();
    let model = Carnet::new(Probe::default())
        .run_with(backend)
        .expect("the program runs");
    (model, screen.text())
}

#[test]
fn keys_update_the_model_and_the_screen_shows_the_last_view() {
    let (model, screen) = run([Key::Up, Key::Up, Key::Down, Key::Up, Key::Char('q')]);
    assert_eq!(model.count, 2);
    assert!(screen.contains("count 2"), "{screen}");
}

#[test]
fn the_model_hears_the_window_size_first() {
    let (model, _) = run([Key::Char('q')]);
    assert_eq!(model.sizes, vec![(40, 10)]);
}

#[test]
fn a_task_and_a_tick_send_their_messages_back() {
    let (model, screen) = run([Key::Char('t'), Key::Char('k')]);
    assert!(
        model.messages.contains(&"from a task"),
        "{:?}",
        model.messages
    );
    assert_eq!(
        model.messages.last(),
        Some(&"done"),
        "the tick quit the program"
    );
    assert!(screen.contains("from a task"), "{screen}");
}

#[test]
fn a_batch_runs_every_command() {
    let (model, _) = run([Key::Char('b'), Key::Char('k')]);
    let mut seen = model.messages.clone();
    seen.sort_unstable();
    assert_eq!(seen, vec!["done", "one", "two"]);
}

#[test]
fn ctrl_c_quits_unless_the_model_keeps_it() {
    let (model, _) = run([Key::Up, Key::Ctrl('c'), Key::Up]);
    assert_eq!(model.count, 1, "nothing after Ctrl-C");
    assert!(!model.keys.contains(&Key::Ctrl('c')));

    let backend = TestBackend::new(40, 10).keys([Key::Ctrl('c'), Key::Char('q')]);
    let model = Carnet::new(Probe::default())
        .keep_ctrl_c()
        .run_with(backend)
        .unwrap();
    assert_eq!(model.keys, vec![Key::Ctrl('c'), Key::Char('q')]);
}

#[test]
fn init_runs_its_command() {
    #[derive(Debug, Default)]
    struct Starter(bool);
    impl Model for Starter {
        type Msg = ();
        fn init(&mut self) -> Cmd<()> {
            Cmd::tick(Duration::from_millis(5), || ())
        }
        fn update(&mut self, event: Event<()>) -> Cmd<()> {
            if matches!(event, Event::Msg(())) {
                self.0 = true;
                return Cmd::quit();
            }
            Cmd::none()
        }
        fn view(&self) -> String {
            String::new()
        }
    }
    assert!(
        Carnet::new(Starter::default())
            .run_with(TestBackend::new(10, 3))
            .unwrap()
            .0
    );
}
