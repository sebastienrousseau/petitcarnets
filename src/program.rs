//! The runtime: a model, its messages and its commands, in one loop on
//! one thread.
//!
//! The loop owns the model and the renderer. It updates the model with
//! every message that is ready, in order, and draws once the queue is
//! empty, so a burst of messages costs one frame. It then sleeps until
//! the next input, task result or timer, with no periodic wakeups.

use crate::backend::{Backend, Input};
use crate::exec::{Event, Executor, Item};
use crate::msg::{Error, Msg};
use crate::renderer::Renderer;
use crate::{Cmd, Model};
use std::fmt;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

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
        }
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
        let result = self.event_loop(&mut backend);
        let left = backend.leave();
        let model = result?;
        left?;
        Ok(model)
    }

    fn event_loop(self, backend: &mut impl Backend) -> Result<M, Error> {
        let Self {
            mut model,
            events,
            inbox,
        } = self;
        let keys = events.clone();
        backend.start_input(Box::new(move |input| {
            keys.send(Event::Input(input)).is_ok()
        }));
        let size = backend.size()?;
        let mut state = Loop {
            exec: Executor::new(events),
            renderer: Renderer::new(size),
            dirty: true,
        };
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

/// The loop's state besides the model.
struct Loop<M> {
    exec: Executor<M>,
    renderer: Renderer,
    dirty: bool,
}

impl<A: Send + 'static> Loop<A> {
    /// Updates, draws and waits until something ends the program.
    fn run<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<End, Error> {
        loop {
            if let Some(end) = self.update(model, backend)? {
                if !matches!(end, End::Kill | End::Panic(_)) {
                    self.draw(model, backend)?;
                }
                return Ok(end);
            }
            self.draw(model, backend)?;
            self.wait(inbox);
        }
    }

    /// Updates the model with everything ready, in order; how the program
    /// ends, if it does.
    fn update<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        backend: &mut impl Backend,
    ) -> Result<Option<End>, Error> {
        while let Some(item) = self.exec.pop() {
            let msg = match item {
                Item::Msg(msg) => msg,
                Item::Quit => return Ok(Some(End::Quit)),
                Item::Interrupt => return Ok(Some(End::Interrupt)),
                Item::Kill => return Ok(Some(End::Kill)),
                Item::Panic(message) => return Ok(Some(End::Panic(message))),
            };
            if let Msg::WindowSize(size) = msg {
                self.renderer.resize(size, backend.output())?;
            }
            let cmd: Cmd<A> = model.update(msg);
            self.exec.run(cmd, Instant::now());
            self.dirty = true;
        }
        Ok(None)
    }

    fn draw<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        if self.dirty {
            self.renderer
                .render(&model.view().content, backend.output())?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Sleeps until an event arrives or a timer is due, then takes in
    /// everything that is ready.
    fn wait(&mut self, inbox: &Receiver<Event<A>>) {
        let first = match self.exec.next_deadline() {
            None => inbox.recv().ok(),
            Some(at) => inbox
                .recv_timeout(at.saturating_duration_since(Instant::now()))
                .ok(),
        };
        for event in first
            .into_iter()
            .chain(std::iter::from_fn(|| inbox.try_recv().ok()))
        {
            self.take(event);
        }
        self.exec.fire_due(Instant::now());
    }

    /// Queues what one event means for the model.
    fn take(&mut self, event: Event<A>) {
        let item = match event {
            Event::Input(input) => input_item(input),
            Event::Done(parent, msg) => return self.exec.task_done(parent, msg, Instant::now()),
            Event::Panic(message) => Item::Panic(message),
            Event::Send(msg) => Item::Msg(Msg::App(msg)),
            Event::Quit => Item::Quit,
            Event::Kill => Item::Kill,
        };
        self.exec.push(item);
    }
}

/// What a piece of terminal input means for the model.
fn input_item<M>(input: Input) -> Item<M> {
    match input {
        Input::Key(key) => Item::Msg(Msg::KeyPress(key)),
        Input::Resize(size) => Item::Msg(Msg::WindowSize(size)),
        Input::Interrupt => Item::Interrupt,
        Input::Terminate => Item::Quit,
    }
}
