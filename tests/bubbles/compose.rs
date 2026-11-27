//! Composing components: two spinners and a text input in one parent
//! each take only their own ticks, and a child's commands (quit, batch,
//! sequence, tick, every, task, exec) keep their meaning when mapped.

use super::ms;
use petitcarnets as tea;
use std::time::Duration;
use tea::Key;
use tea::bubbles::spinner::{self, Spinner};
use tea::bubbles::textinput;
use tea::testing::TestProgram;

/// The parent's messages: each child's, wrapped.
#[derive(Debug, Clone, Copy)]
enum Msg {
    Fast(spinner::TickMsg),
    Slow(spinner::TickMsg),
    Input(textinput::Msg),
}

/// Two spinners at different speeds and a text input, with a count of
/// the ticks each spinner took.
struct Parent {
    fast: spinner::Model,
    slow: spinner::Model,
    input: textinput::Model,
    taken: [usize; 2],
    /// Send every spinner tick to both spinners, as a careless parent
    /// would: their ids must keep them apart.
    careless: bool,
}

impl Parent {
    fn new(careless: bool) -> Self {
        Self {
            fast: spinner::Model::new().with_spinner(Spinner::LINE),
            slow: spinner::Model::new().with_spinner(Spinner::PULSE),
            input: textinput::Model::new(),
            taken: [0, 0],
            careless,
        }
    }

    /// The tick in `msg`, for spinner `which` (0 fast, 1 slow).
    fn tick_for(&self, msg: &tea::Msg<Msg>, which: usize) -> Option<tea::Msg<spinner::TickMsg>> {
        msg.route(|m| match (m, which) {
            (Msg::Fast(t), 0) | (Msg::Slow(t), 1) => Some(*t),
            (Msg::Fast(t) | Msg::Slow(t), _) if self.careless => Some(*t),
            _ => None,
        })
    }
}

impl tea::Model for Parent {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        tea::batch([
            self.fast.tick().map(Msg::Fast),
            self.slow.tick().map(Msg::Slow),
            self.input.focus().map(Msg::Input),
        ])
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        let mut cmds = Vec::new();
        if let Some(m) = self.tick_for(&msg, 0) {
            let cmd = self.fast.update(&m);
            self.taken[0] += usize::from(!cmd.is_none());
            cmds.push(cmd.map(Msg::Fast));
        }
        if let Some(m) = self.tick_for(&msg, 1) {
            let cmd = self.slow.update(&m);
            self.taken[1] += usize::from(!cmd.is_none());
            cmds.push(cmd.map(Msg::Slow));
        }
        let input = msg.route(|m| match m {
            Msg::Input(c) => Some(*c),
            _ => None,
        });
        if let Some(m) = input {
            cmds.push(self.input.update(&m).map(Msg::Input));
        }
        tea::batch(cmds)
    }

    fn view(&self) -> tea::View {
        format!(
            "{} {} {}",
            self.fast.view(),
            self.slow.view(),
            self.input.view()
        )
        .into()
    }
}

/// The ticks each spinner takes in a second, and the text typed.
fn run(careless: bool) -> ([usize; 2], String) {
    let mut test = TestProgram::new(Parent::new(careless), 40, 3);
    test.type_text("hi");
    test.advance(Duration::from_secs(1));
    let screen = test.screen().text();
    let m = test.model();
    let (taken, value) = (m.taken, m.input.value());
    assert!(screen.contains("> hi"), "{screen}");
    (taken, value)
}

#[test]
fn each_spinner_takes_only_its_own_ticks() {
    // A tick every 100 ms and every 125 ms, from the start: the first at
    // 0, so 11 and 9 in the first second, the last at 1000 ms.
    assert_eq!(run(false), ([11, 9], "hi".into()));
    assert_eq!(run(true), ([11, 9], "hi".into()), "ids keep them apart");
}

#[test]
fn a_cursor_blinks_on_its_own_ticks_among_the_spinners() {
    let mut test = TestProgram::new(Parent::new(true), 40, 3);
    let shown = |t: &TestProgram<Parent>| t.model().input.view().contains("\x1b[7;");
    assert!(shown(&test), "a focused cursor shows at first");
    test.advance(ms(530));
    assert!(!shown(&test), "it blinks off after 530 ms");
    test.advance(ms(530));
    assert!(shown(&test), "and on again");
    test.press(Key::char('x'));
    assert!(shown(&test), "typing shows it");
}

/// What a child does: messages it sends.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Child {
    Ticked(u32),
    Worked,
    Ran(bool),
}

/// A parent that runs one mapped child command and records what comes
/// back, in order.
struct Recorder {
    child: Option<tea::Cmd<Child>>,
    seen: Vec<String>,
}

impl tea::Model for Recorder {
    type Msg = Option<Child>;

    fn init(&mut self) -> tea::Cmd<Option<Child>> {
        self.child
            .take()
            .map_or_else(tea::Cmd::none, |c| c.map(Some))
    }

    fn update(&mut self, msg: tea::Msg<Option<Child>>) -> tea::Cmd<Option<Child>> {
        if let tea::Msg::App(Some(c)) = msg {
            self.seen.push(format!("{c:?}"));
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        self.seen.join(" ").into()
    }
}

/// The messages a mapped child command brings, and whether the program
/// stopped.
fn record(child: tea::Cmd<Child>, wait: Duration) -> (Vec<String>, bool) {
    let mut test = TestProgram::new(
        Recorder {
            child: Some(child),
            seen: Vec::new(),
        },
        60,
        3,
    );
    test.advance(wait);
    let finished = test.is_finished();
    let seen = if finished {
        test.final_model().seen
    } else {
        test.model().seen.clone()
    };
    (seen, finished)
}

fn tick_after(n: u64, v: u32) -> tea::Cmd<Child> {
    tea::tick(ms(n), move |_| Child::Ticked(v))
}

#[test]
fn a_mapped_quit_still_quits() {
    let (seen, finished) = record(tea::quit(), ms(10));
    assert!(finished && seen.is_empty());
    let (_, finished) = record(tea::Cmd::none(), ms(10));
    assert!(!finished);
}

#[test]
fn a_mapped_sequence_keeps_its_order_and_a_batch_all_its_messages() {
    let seq = tea::sequence([
        tick_after(30, 1),
        tick_after(10, 2),
        tea::Cmd::task(|| Child::Worked),
    ]);
    let (seen, _) = record(seq, ms(100));
    assert_eq!(seen, ["Ticked(1)", "Ticked(2)", "Worked"]);
    let both = tea::batch([tick_after(30, 1), tick_after(10, 2)]);
    let (seen, _) = record(both, ms(100));
    assert_eq!(seen, ["Ticked(2)", "Ticked(1)"], "a batch runs at once");
}

#[test]
fn a_mapped_tick_every_and_quit_after_them() {
    let every = tea::every(Duration::from_secs(1), |_| Child::Ticked(9));
    let (seen, _) = record(every, Duration::from_secs(2));
    assert_eq!(seen, ["Ticked(9)"], "every fires once, on the second");
    let then_quit = tea::sequence([tick_after(5, 3), tea::quit()]);
    let (seen, finished) = record(then_quit, ms(50));
    assert_eq!((seen, finished), (vec!["Ticked(3)".to_string()], true));
}

#[test]
fn a_mapped_exec_sends_its_result() {
    let run = tea::exec(std::process::Command::new("true"), |r| {
        Child::Ran(r.is_ok_and(|s| s.success()))
    });
    let (seen, _) = record(run, ms(50));
    assert_eq!(seen, ["Ran(true)"]);
}
