//! The runtime: a model, its messages and its commands, in one loop on
//! one thread.
//!
//! The loop owns the model and the renderer. It updates the model with
//! every message that is ready, in order, and draws once the queue is
//! empty, so a burst of messages costs one frame, and at most `fps`
//! frames a second, except that a frame after a key, mouse event or
//! paste is drawn at once. It then sleeps until the next input, task
//! result, timer or held back frame, with no periodic wakeups.

mod run;
mod session;

use crate::Model;
use crate::backend::Backend;
use crate::clock::{Clock, RealClock};
use crate::color::ColorProfile;
use crate::environ::Environment;
use crate::exec::{Event, Executor, Item};
use crate::msg::{Error, Msg};
use crate::renderer::Renderer;
use crate::style::{WidthMethod, set_width_method};
use run::{Loop, Settings};
pub(crate) use session::Session;
use std::fmt;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Frames a second unless [`Program::with_fps`] says otherwise, as in
/// Bubble Tea.
const DEFAULT_FPS: u32 = 60;

/// The most frames a second [`Program::with_fps`] allows, as in Bubble
/// Tea.
const MAX_FPS: u32 = 120;

/// Runs a [`Model`] in a terminal. Bubble Tea's `tea.Program`.
///
/// ```no_run
/// use petitcarnets as tea;
///
/// struct Hello;
///
/// impl tea::Model for Hello {
///     type Msg = ();
///
///     fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
///         match msg {
///             tea::Msg::KeyPress(k) if k == "q" || k == "ctrl+c" => tea::quit(),
///             _ => tea::Cmd::none(),
///         }
///     }
///
///     fn view(&self) -> tea::View {
///         "Hello! Press q to quit.".into()
///     }
/// }
///
/// tea::Program::new(Hello).run()?;
/// # Ok::<(), tea::Error>(())
/// ```
pub struct Program<M: Model> {
    model: M,
    events: mpsc::Sender<Event<M::Msg>>,
    inbox: Receiver<Event<M::Msg>>,
    fps: u32,
    environment: Option<Environment>,
    profile: Option<ColorProfile>,
    suspend_key: bool,
}

impl<M: Model> fmt::Debug for Program<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Program").finish_non_exhaustive()
    }
}

/// Sends messages to a program from other threads: Bubble Tea's
/// `Program.Send`, `Quit` and `Kill`. Sending to a program that has
/// stopped does nothing.
pub struct Sender<M>(mpsc::Sender<Event<M>>);

impl<M> Clone for Sender<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<M> fmt::Debug for Sender<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sender(..)")
    }
}

impl<M> Sender<M> {
    /// Delivers `msg` to the model as [`Msg::App`].
    pub fn send(&self, msg: M) {
        let _ = self.0.send(Event::Send(msg));
    }

    /// Stops the program as [`quit`](crate::quit) does.
    pub fn quit(&self) {
        let _ = self.0.send(Event::Quit);
    }

    /// Stops the program at once, without a final frame;
    /// [`Program::run`] returns [`Error::Killed`].
    pub fn kill(&self) {
        let _ = self.0.send(Event::Kill);
    }
}

/// How the loop ended.
#[derive(Debug)]
pub(crate) enum End {
    Quit,
    Interrupt,
    Kill,
    Panic(String),
}

impl End {
    /// The end an item asks for, if it asks for one.
    fn of<M>(item: Item<M>) -> Option<Self> {
        match item {
            Item::Quit => Some(Self::Quit),
            Item::Interrupt => Some(Self::Interrupt),
            Item::Kill => Some(Self::Kill),
            Item::Panic(message) => Some(Self::Panic(message)),
            _ => None,
        }
    }
}

impl<M: Model> Program<M> {
    /// A program for `model`. Bubble Tea's `tea.NewProgram`.
    pub fn new(model: M) -> Self {
        let (events, inbox) = mpsc::channel();
        Self {
            model,
            events,
            inbox,
            fps: DEFAULT_FPS,
            environment: None,
            profile: None,
            suspend_key: true,
        }
    }

    /// Detects colours and terminal features from `environment` instead
    /// of the process's own: Bubble Tea's `tea.WithEnvironment`, for a
    /// program served over SSH, where the client's `TERM`, `COLORTERM`,
    /// `NO_COLOR` and the rest are what count.
    ///
    /// ```
    /// use petitcarnets as tea;
    ///
    /// # struct M;
    /// # impl tea::Model for M {
    /// #     type Msg = ();
    /// #     fn update(&mut self, _: tea::Msg<()>) -> tea::Cmd<()> { tea::Cmd::none() }
    /// #     fn view(&self) -> tea::View { tea::View::default() }
    /// # }
    /// let client = [("TERM", "xterm-256color"), ("NO_COLOR", "1")];
    /// let program = tea::Program::new(M).with_environment(client.into_iter().collect());
    /// # let _ = program;
    /// ```
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Uses `profile` instead of detecting one: Bubble Tea's
    /// `tea.WithColorProfile`. It is kept even if the terminal later says
    /// it shows 24-bit colour (unlike Bubble Tea, its issue #1773).
    #[must_use]
    pub const fn with_color_profile(mut self, profile: ColorProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Delivers Ctrl-Z to the model as the key `"ctrl+z"` instead of
    /// suspending the program. By default, Ctrl-Z suspends, as it does in
    /// a shell (Bubble Tea leaves it to each program, its issue #500);
    /// [`suspend`](crate::suspend) still suspends, and so does a
    /// `SIGTSTP` sent from outside.
    #[must_use]
    pub const fn without_suspend(mut self) -> Self {
        self.suspend_key = false;
        self
    }

    /// Draws at most `fps` frames a second (60 by default), cut to 1 to
    /// 120. Bubble Tea's `tea.WithFPS`. A frame after a key, mouse event
    /// or paste is drawn at once, whatever the rate.
    #[must_use]
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps.clamp(1, MAX_FPS);
        self
    }

    /// A handle for sending the program messages from other threads,
    /// before or while it runs.
    #[must_use]
    pub fn sender(&self) -> Sender<M::Msg> {
        Sender(self.events.clone())
    }

    /// Runs in the process's terminal until the model quits, and returns
    /// the model. Ctrl-C arrives as the key `"ctrl+c"`, as in Bubble Tea:
    /// quit on it in `update` if the program should stop.
    ///
    /// # Errors
    ///
    /// [`Error::Interrupted`] on `SIGINT`, [`Error::Killed`] after
    /// [`Sender::kill`], [`Error::Panic`] when a task panics, and
    /// [`Error::Io`] when there is no terminal or it fails. The terminal
    /// is restored in every case.
    #[cfg(unix)]
    pub fn run(self) -> Result<M, Error> {
        self.run_with(crate::Terminal::new())
    }

    /// Runs on `backend` until the model quits, and returns the model.
    ///
    /// # Errors
    ///
    /// As [`Program::run`]; the backend is left in every case.
    pub fn run_with(self, mut backend: impl Backend) -> Result<M, Error> {
        backend.enter()?;
        set_width_method(WidthMethod::WcWidth);
        let result = match self.start(&mut backend, RealClock) {
            Ok(mut session) => {
                let end = session.run(&mut backend);
                let (model, ended) = session.close(end, &mut backend);
                ended.map(|()| model)
            }
            Err(err) => Err(err),
        };
        set_width_method(WidthMethod::WcWidth);
        let left = backend.leave();
        let model = result?;
        left?;
        Ok(model)
    }

    /// Starts the loop on `backend`, already entered, with `clock`: input
    /// started, the start queries written and the first messages queued.
    /// Nothing is drawn yet.
    pub(crate) fn start<C: Clock>(
        self,
        backend: &mut impl Backend,
        clock: C,
    ) -> Result<Session<M, C>, Error> {
        let Self {
            mut model,
            events,
            inbox,
            fps,
            environment,
            profile,
            suspend_key,
        } = self;
        let keys = events.clone();
        backend.start_input(Box::new(move |input| {
            keys.send(Event::Input(input)).is_ok()
        }));
        let size = backend.size()?;
        let environment = environment.unwrap_or_else(Environment::current);
        let terminal = backend.is_terminal();
        let settings = Settings {
            interval: Duration::from_secs(1) / fps,
            profile: profile.unwrap_or_else(|| detect(terminal, &environment)),
            forced: profile.is_some(),
            suspend_key,
            terminal,
        };
        let colors = settings.profile;
        let mut state = Loop::new(Executor::new(events), Renderer::new(size), settings, clock);
        state.query(backend, should_query(&environment))?;
        state.exec.push(Item::Msg(Msg::ColorProfile(colors)));
        state.exec.push(Item::Msg(Msg::WindowSize(size)));
        let now = state.now();
        state.exec.run(model.init(), now);
        Ok(Session::new(model, inbox, state))
    }
}

/// The colour profile of a real terminal (`terminal`) in `environment`;
/// anything else shows every colour as it is.
fn detect(terminal: bool, environment: &Environment) -> ColorProfile {
    if terminal {
        ColorProfile::detect(true, environment)
    } else {
        ColorProfile::TrueColor
    }
}

/// Whether to ask the terminal about synchronized output and grapheme
/// widths: Bubble Tea's `shouldQuerySynchronizedOutput`.
fn should_query(env: &Environment) -> bool {
    let term = env.var("TERM");
    let known = ["ghostty", "wezterm", "alacritty", "kitty", "rio"]
        .iter()
        .any(|t| term.contains(t));
    let apple = env.var("TERM_PROGRAM").contains("Apple");
    let ssh = env.get("SSH_TTY").is_some();
    (!ssh && !apple) || env.get("WT_SESSION").is_some() || known
}

#[cfg(test)]
mod tests;
