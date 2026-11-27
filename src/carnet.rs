//! The runtime: a model, its messages and its commands, in a loop.

use crate::backend::Backend;
use crate::cmd::{Cmd, Inner};
use crate::key::Key;
use crate::renderer::Renderer;
use crate::{Event, Model};
use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// How often the loop wakes without an event, to notice a resize.
const POLL: Duration = Duration::from_millis(50);

/// A running program: a [`Model`] in a terminal.
///
/// ```no_run
/// # use petitcarnets::{Carnet, Cmd, Event, Key, Model};
/// struct Hello;
/// impl Model for Hello {
///     type Msg = ();
///     fn update(&mut self, event: Event<()>) -> Cmd<()> {
///         match event {
///             Event::Key(Key::Char('q')) => Cmd::quit(),
///             _ => Cmd::none(),
///         }
///     }
///     fn view(&self) -> String {
///         "Hello! Press q to quit.".into()
///     }
/// }
/// Carnet::new(Hello).run()?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct Carnet<M: Model> {
    model: M,
    quit_on_ctrl_c: bool,
}

impl<M: Model> Carnet<M> {
    /// A program for `model`. Ctrl-C quits unless
    /// [`Carnet::keep_ctrl_c`] hands it to the model instead.
    pub fn new(model: M) -> Self {
        Self {
            model,
            quit_on_ctrl_c: true,
        }
    }

    /// Delivers Ctrl-C to the model as a key instead of quitting.
    #[must_use]
    pub const fn keep_ctrl_c(mut self) -> Self {
        self.quit_on_ctrl_c = false;
        self
    }

    /// Runs in the process's terminal until the model quits, and returns
    /// the model.
    ///
    /// # Errors
    ///
    /// If standard input is not a terminal, or the terminal fails.
    #[cfg(unix)]
    pub fn run(self) -> io::Result<M> {
        self.run_with(crate::Terminal::new())
    }

    /// Runs on `backend` until the model quits, and returns the model.
    ///
    /// # Errors
    ///
    /// If the backend fails; the backend is left in either case.
    pub fn run_with(self, mut backend: impl Backend) -> io::Result<M> {
        backend.enter()?;
        let result = self.event_loop(&mut backend);
        let left = backend.leave();
        let model = result?;
        left.map(|()| model)
    }

    /// The loop: deliver an event, run its command, redraw.
    fn event_loop(mut self, backend: &mut impl Backend) -> io::Result<M> {
        let (tx, rx) = mpsc::channel::<Event<M::Msg>>();
        let keys = tx.clone();
        backend.start_input(Box::new(move |key| keys.send(Event::Key(key)).is_ok()));
        let mut size = backend.size()?;
        let mut renderer = Renderer::new(size);
        let init = self.model.init();
        let mut quit = run(init, &tx);
        quit |= self.resized(size, &mut renderer, backend, &tx)?;
        while !quit {
            quit = self.next(&rx, &tx);
            let now = backend.size().unwrap_or(size);
            if now != size {
                size = now;
                quit |= self.resized(size, &mut renderer, backend, &tx)?;
            }
            renderer.render(&self.model.view(), backend.output())?;
        }
        Ok(self.model)
    }

    /// Clears for a new size, tells the model, and draws; whether the model
    /// asked to quit.
    fn resized(
        &mut self,
        size: (u16, u16),
        renderer: &mut Renderer,
        backend: &mut impl Backend,
        tx: &Sender<Event<M::Msg>>,
    ) -> io::Result<bool> {
        renderer.resize(size, backend.output())?;
        let quit = self.deliver(
            Event::Resize {
                columns: size.0,
                rows: size.1,
            },
            tx,
        );
        renderer.render(&self.model.view(), backend.output())?;
        Ok(quit)
    }

    /// Waits up to [`POLL`] for events and delivers every one that is
    /// ready; whether the model asked to quit.
    fn next(&mut self, rx: &Receiver<Event<M::Msg>>, tx: &Sender<Event<M::Msg>>) -> bool {
        let first = match rx.recv_timeout(POLL) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return false,
        };
        let mut quit = self.deliver(first, tx);
        while !quit {
            match rx.try_recv() {
                Ok(event) => quit = self.deliver(event, tx),
                Err(_) => break,
            }
        }
        quit
    }

    /// Updates the model with one event and runs the command it returns.
    fn deliver(&mut self, event: Event<M::Msg>, tx: &Sender<Event<M::Msg>>) -> bool {
        if self.quit_on_ctrl_c && matches!(event, Event::Key(Key::Ctrl('c'))) {
            return true;
        }
        let cmd = self.model.update(event);
        run(cmd, tx)
    }
}

/// Starts a command; whether it asks to quit.
fn run<Msg: Send + 'static>(cmd: Cmd<Msg>, tx: &Sender<Event<Msg>>) -> bool {
    match cmd.0 {
        Inner::None => false,
        Inner::Quit => true,
        Inner::Batch(cmds) => cmds.into_iter().fold(false, |quit, c| run(c, tx) | quit),
        Inner::Task(work) => {
            let tx = tx.clone();
            drop(std::thread::spawn(move || tx.send(Event::Msg(work()))));
            false
        }
        Inner::Tick(delay, message) => {
            let tx = tx.clone();
            drop(std::thread::spawn(move || {
                std::thread::sleep(delay);
                tx.send(Event::Msg(message()))
            }));
            false
        }
    }
}
