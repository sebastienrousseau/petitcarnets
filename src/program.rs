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

use crate::Model;
use crate::backend::Backend;
use crate::exec::{Event, Executor, Item};
use crate::mode::Modes;
use crate::msg::{Error, Msg};
use crate::renderer::Renderer;
use crate::style::{WidthMethod, set_width_method};
use run::Loop;
use std::fmt;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

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
enum End {
    Quit,
    Interrupt,
    Kill,
    Panic(String),
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
        }
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
        let mut terminal_modes = Modes::default();
        let result = self.event_loop(&mut backend, &mut terminal_modes);
        let cleared = terminal_modes.clear(&mut backend);
        set_width_method(WidthMethod::WcWidth);
        let left = backend.leave();
        let model = result?;
        cleared?;
        left?;
        Ok(model)
    }

    fn event_loop(self, backend: &mut impl Backend, modes: &mut Modes) -> Result<M, Error> {
        let Self {
            mut model,
            events,
            inbox,
            fps,
        } = self;
        let keys = events.clone();
        backend.start_input(Box::new(move |input| {
            keys.send(Event::Input(input)).is_ok()
        }));
        let size = backend.size()?;
        let mut state = Loop::new(
            Executor::new(events),
            Renderer::new(size),
            modes,
            Duration::from_secs(1) / fps,
        );
        state.query(backend)?;
        state.exec.push(Item::Msg(Msg::WindowSize(size)));
        state.exec.run(model.init(), Instant::now());
        let end = state.run(&mut model, &inbox, backend)?;
        match end {
            End::Quit => Ok(model),
            End::Interrupt => Err(Error::Interrupted),
            End::Kill => Err(Error::Killed),
            End::Panic(message) => Err(Error::Panic(message)),
        }
    }
}

#[cfg(test)]
mod tests;
