//! Running a model headless on a virtual clock: Bubble Tea's teatest.

use super::Vt;
use super::terminal::{Harness, TestTerminal};
use crate::backend::Input;
use crate::clock::{Clock, VirtualClock};
use crate::key::Key;
use crate::msg::{Error, Size};
use crate::program::{End, Session};
use crate::style::{WidthMethod, set_width_method};
use crate::{Model, Program, Sender};
use std::fmt;
use std::time::{Duration, Instant};

/// How many wakes in a row at one virtual time mean the program never
/// goes idle.
const SPIN_LIMIT: u32 = 100_000;

/// A model running in a test: the real runtime (the same loop, command
/// executor, renderer and terminal modes as [`Program::run_with`]) on a
/// [`TestTerminal`], with a virtual clock. Bubble Tea's teatest
/// `TestModel`, deterministic.
///
/// Time stands still until the test moves it with
/// [`advance`](TestProgram::advance) or a `wait_for`: then every timer
/// ([`tick`](crate::tick), [`every`](crate::every)), held back frame
/// (the frame rate), escape timeout and query timeout due on the way
/// fires in order, at its own virtual time. Nothing sleeps.
///
/// Each call that gives the program something (a message, a key, bytes,
/// a resize, a signal, time) returns once the program is idle again:
/// every message handled, every frame the frame rate allows drawn, and
/// every [`Cmd::task`](crate::Cmd::task) finished. Tasks still run on
/// their own threads; the harness waits for each, and delivers their
/// messages in the order the tasks started, so they take no virtual
/// time and their order never varies. A task that waits for something
/// only the test can do next would wait forever: after 30 seconds the
/// harness panics instead.
///
/// [`exec`](crate::exec) runs its command for real, with the terminal
/// handed over and taken back as a real terminal would be;
/// [`suspend`](crate::suspend) and Ctrl-Z hand the terminal over and
/// continue at once, counted by [`suspensions`](TestProgram::suspensions).
///
/// ```
/// use petitcarnets as tea;
/// use std::time::Duration;
/// use tea::testing::TestProgram;
///
/// struct Countdown(u32);
///
/// impl tea::Model for Countdown {
///     type Msg = ();
///
///     fn init(&mut self) -> tea::Cmd<()> {
///         tea::tick(Duration::from_secs(1), |_| ())
///     }
///
///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
///         match msg {
///             tea::Msg::App(()) if self.0 > 1 => {
///                 self.0 -= 1;
///                 tea::tick(Duration::from_secs(1), |_| ())
///             }
///             tea::Msg::App(()) => tea::quit(),
///             tea::Msg::KeyPress(k) if k == "q" => tea::quit(),
///             _ => tea::Cmd::none(),
///         }
///     }
///
///     fn view(&self) -> tea::View {
///         format!("{} left", self.0).into()
///     }
/// }
///
/// let mut test = TestProgram::new(Countdown(3), 20, 3);
/// assert_eq!(test.screen().text(), "3 left");
/// test.advance(Duration::from_secs(1));
/// assert_eq!(test.screen().text(), "2 left");
/// assert!(test.wait_finished(Duration::from_secs(5)));
/// assert_eq!(test.elapsed(), Duration::from_secs(3));
/// assert_eq!(test.final_model().0, 1);
/// ```
pub struct TestProgram<M: Model> {
    running: Option<Session<M, VirtualClock>>,
    finished: Option<(M, Result<(), Error>)>,
    harness: Harness,
    clock: VirtualClock,
    sender: Sender<M::Msg>,
}

impl<M: Model> fmt::Debug for TestProgram<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestProgram")
            .field("elapsed", &self.elapsed())
            .field("finished", &self.is_finished())
            .finish_non_exhaustive()
    }
}

impl<M: Model> TestProgram<M> {
    /// Starts `model` in a `columns` by `rows` terminal: teatest's
    /// `NewTestModel` with `WithInitialTermSize`. It returns once the
    /// first frame is drawn and the program is idle.
    #[must_use]
    pub fn new(model: M, columns: u16, rows: u16) -> Self {
        Self::with_terminal(Program::new(model), TestTerminal::new(columns, rows))
    }

    /// Starts `program`, set up as it is (frame rate, colour profile,
    /// environment, suspend key), in `terminal`: teatest's
    /// `WithProgramOptions`.
    ///
    /// # Panics
    ///
    /// Never in practice: the test terminal cannot fail.
    #[must_use]
    pub fn with_terminal(program: Program<M>, terminal: TestTerminal) -> Self {
        let clock = VirtualClock::new();
        let mut harness = terminal.backend(clock.clone());
        let sender = program.sender();
        set_width_method(WidthMethod::WcWidth);
        let session = match program.start(&mut harness, clock.clone()) {
            Ok(session) => session,
            Err(err) => panic!("the test terminal failed: {err}"),
        };
        let mut test = Self {
            running: Some(session),
            finished: None,
            harness,
            clock,
            sender,
        };
        if test.step() {
            test.settle();
        }
        test
    }

    /// Sends `msg` to the model as [`Msg::App`](crate::Msg::App):
    /// teatest's `Send`.
    pub fn send(&mut self, msg: M::Msg) {
        self.sender.send(msg);
        self.settle();
    }

    /// Sends `input` as the terminal would: a key, a mouse event, a
    /// paste, a reply or a signal.
    pub fn input(&mut self, input: Input) {
        self.harness.wire.keyboard.send(input);
        self.settle();
    }

    /// Presses `key`.
    pub fn press(&mut self, key: Key) {
        self.input(Input::KeyPress(key));
    }

    /// Types `text`, one key press a character: teatest's `Type`.
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.press(Key::char(c));
        }
    }

    /// Sends `bytes` as the terminal would, in one read, through the
    /// same decoder as a real terminal's. A sequence cut off at the end
    /// (a lone ESC among them) waits for more bytes, or until the
    /// 50 ms escape timeout passes on the virtual clock.
    pub fn bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.harness.wire.keyboard.feed(bytes.as_ref());
        self.settle();
    }

    /// Resizes the window, and the emulated screen, to `columns` by
    /// `rows`, and tells the program, as `SIGWINCH` does. While the
    /// terminal is handed over the signal is dropped, as a real
    /// program's reader drops it; the program reads the new size when
    /// it takes the terminal back.
    pub fn resize(&mut self, columns: u16, rows: u16) {
        let size = Size::new(columns, rows);
        self.harness.resize(size);
        if self.harness.released {
            return;
        }
        self.input(Input::Resize(size));
    }

    /// Sends `SIGINT`: the program stops with
    /// [`Error::Interrupted`](crate::Error::Interrupted).
    pub fn interrupt(&mut self) {
        self.input(Input::Interrupt);
    }

    /// Sends `SIGTERM`: the program stops as if the model had quit.
    pub fn terminate(&mut self) {
        self.input(Input::Terminate);
    }

    /// Sends `SIGTSTP`: the program hands the terminal over, is stopped
    /// and continued at once, and takes it back.
    pub fn suspend(&mut self) {
        self.input(Input::Suspend);
    }

    /// Sends `SIGCONT`, as after a stop from outside: the terminal is
    /// set up and drawn again.
    pub fn resume(&mut self) {
        self.input(Input::Continue);
    }

    /// Stops the program as [`quit`](crate::quit) does: teatest's
    /// `Quit`.
    pub fn quit(&mut self) {
        self.sender.quit();
        self.settle();
    }

    /// Stops the program at once, without a final frame, as
    /// [`Sender::kill`] does.
    pub fn kill(&mut self) {
        self.sender.kill();
        self.settle();
    }

    /// Moves the virtual clock on by `by`, firing everything due on the
    /// way in order, each at its own time.
    ///
    /// # Panics
    ///
    /// If the program never goes idle: it wakes a hundred thousand
    /// times in a row with no time passing, as a zero
    /// [`tick`](crate::tick) that starts itself makes it.
    pub fn advance(&mut self, by: Duration) {
        self.advance_to(self.clock.now().at + by);
    }

    /// Moves the clock on until `done` holds or `within` has passed,
    /// checking after each thing that happens: teatest's `WaitFor`.
    /// Whether `done` held.
    ///
    /// # Panics
    ///
    /// As [`advance`](TestProgram::advance).
    pub fn wait_for(&mut self, within: Duration, mut done: impl FnMut(&Self) -> bool) -> bool {
        let until = self.clock.now().at + within;
        while !done(self) {
            let Some(at) = self.next_wake().filter(|at| *at <= until) else {
                self.advance_to(until);
                return done(self);
            };
            self.advance_to(at);
        }
        true
    }

    /// Moves the clock on until the program ends or `within` has passed:
    /// teatest's `WaitFinished`. Whether it ended.
    ///
    /// # Panics
    ///
    /// As [`advance`](TestProgram::advance).
    pub fn wait_finished(&mut self, within: Duration) -> bool {
        self.wait_for(within, Self::is_finished)
    }

    /// How far the virtual clock has moved since the start.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.clock.elapsed()
    }

    /// The screen as a terminal would show it now.
    #[must_use]
    pub const fn screen(&self) -> &Vt {
        &self.harness.wire.vt
    }

    /// Every byte the program has written so far: teatest's `Output`.
    #[must_use]
    pub fn output(&self) -> &[u8] {
        &self.harness.wire.bytes
    }

    /// The model as it is now.
    #[must_use]
    pub fn model(&self) -> &M {
        match (&self.running, &self.finished) {
            (Some(session), _) => session.model(),
            (None, Some((model, _))) => model,
            (None, None) => unreachable!("a test program is running or finished"),
        }
    }

    /// A handle that sends the program messages, from any thread:
    /// teatest's `GetProgram`.
    #[must_use]
    pub fn sender(&self) -> Sender<M::Msg> {
        self.sender.clone()
    }

    /// Whether the program has ended.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    /// The error the program ended with, if it ended with one.
    #[must_use]
    pub fn error(&self) -> Option<&Error> {
        self.finished.as_ref().and_then(|(_, r)| r.as_ref().err())
    }

    /// How many times the program stopped the process to suspend.
    #[must_use]
    pub const fn suspensions(&self) -> usize {
        self.harness.suspensions
    }

    /// Whether the terminal is handed over now, as during
    /// [`release_terminal`](crate::release_terminal).
    #[must_use]
    pub const fn is_released(&self) -> bool {
        self.harness.released
    }

    /// The screen once the program has ended: teatest's `FinalOutput`,
    /// as a terminal shows it.
    ///
    /// # Panics
    ///
    /// If the program is still running.
    #[must_use]
    pub fn final_screen(&self) -> &Vt {
        self.expect_finished();
        self.screen()
    }

    /// Every byte written, once the program has ended: teatest's
    /// `FinalOutput`.
    ///
    /// # Panics
    ///
    /// If the program is still running.
    #[must_use]
    pub fn final_output(&self) -> &[u8] {
        self.expect_finished();
        self.output()
    }

    /// The model once the program has ended, however it ended:
    /// teatest's `FinalModel`.
    ///
    /// # Panics
    ///
    /// If the program is still running.
    #[must_use]
    pub fn final_model(self) -> M {
        self.expect_finished();
        self.finished
            .map_or_else(|| unreachable!(), |(model, _)| model)
    }

    /// What [`Program::run_with`] would have returned, once the program
    /// has ended.
    ///
    /// # Errors
    ///
    /// The error the program ended with.
    ///
    /// # Panics
    ///
    /// If the program is still running.
    pub fn into_result(self) -> Result<M, Error> {
        self.expect_finished();
        self.finished
            .map_or_else(|| unreachable!(), |(model, r)| r.map(|()| model))
    }

    /// Panics if the program is still running.
    fn expect_finished(&self) {
        assert!(
            self.is_finished(),
            "the program is still running: quit it, or wait for it to finish"
        );
    }

    /// Moves the clock on to `at`, waking the program at each time
    /// something is due on the way, and once more at `at`.
    fn advance_to(&mut self, at: Instant) {
        while let Some(due) = self.next_wake().filter(|due| *due <= at) {
            self.clock.move_to(due);
            self.harness.wire.keyboard.expire();
            self.settle();
        }
        self.clock.move_to(at);
        self.settle();
    }

    /// When the program next has something to do with no event: a
    /// timer, a held back frame, or bytes held back for the escape
    /// timeout.
    fn next_wake(&self) -> Option<Instant> {
        let session = self.running.as_ref()?;
        [session.next_wake(), self.harness.wire.keyboard.deadline()]
            .into_iter()
            .flatten()
            .min()
    }

    /// Wakes the program as an event or a deadline would, and lets it
    /// run until it is idle.
    ///
    /// # Panics
    ///
    /// If the program never goes idle: it wakes a hundred thousand times
    /// in a row with something to do and no time passing, as a zero
    /// [`tick`](crate::tick) that starts itself makes it, which would
    /// spin a real program too.
    fn settle(&mut self) {
        let mut wakes = 0_u32;
        while let Some(session) = self.running.as_mut() {
            let woke = session.wake();
            if !self.step() || !woke {
                return;
            }
            wakes += 1;
            assert!(
                wakes < SPIN_LIMIT,
                "the program never goes idle at {:?}",
                self.elapsed()
            );
        }
    }

    /// One update and frame; whether the program still runs.
    fn step(&mut self) -> bool {
        let Some(session) = self.running.as_mut() else {
            return false;
        };
        match session.step(&mut self.harness) {
            Ok(None) => true,
            Ok(Some(end)) => self.finish(Ok(end)),
            Err(err) => self.finish(Err(err)),
        }
    }

    /// Ends the session as [`Program::run_with`] does; `false`.
    fn finish(&mut self, end: Result<End, Error>) -> bool {
        if let Some(session) = self.running.take() {
            self.finished = Some(session.close(end, &mut self.harness));
            set_width_method(WidthMethod::WcWidth);
        }
        false
    }
}
