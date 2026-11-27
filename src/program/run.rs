//! The loop itself: updates, frames, output in program order, and
//! waiting.

use super::End;
use crate::backend::{Backend, Input};
use crate::cmd::Output;
use crate::exec::{Event, Executor, Item};
use crate::mode::{ModeSetting, Modes, ReportedMode};
use crate::msg::{Error, Msg};
use crate::renderer::Renderer;
use crate::style::{WidthMethod, set_width_method};
use crate::view::View;
use crate::{Cmd, Model};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Asks whether the terminal has synchronized output (mode 2026) and
/// grapheme widths (mode 2027), as Bubble Tea does at the start.
const QUERY: &[u8] = b"\x1b[?2026$p\x1b[?2027$p";

/// Replies to the start query this young are waited for at the end, so
/// they do not reach the shell; as Bubble Tea's `queryReplyWindow`.
const REPLY_WINDOW: Duration = Duration::from_secs(1);

/// The longest wait for those replies; Bubble Tea's `queryDrainTimeout`.
const DRAIN_TIMEOUT: Duration = Duration::from_millis(500);

/// The loop's state besides the model.
pub(super) struct Loop<'m, M> {
    pub(super) exec: Executor<M>,
    renderer: Renderer,
    modes: &'m mut Modes,
    dirty: bool,
    urgent: bool,
    interval: Duration,
    last_frame: Option<Instant>,
    frame_due: Option<Instant>,
    queried: Option<Instant>,
    replies: u8,
}

impl<'m, A: Send + 'static> Loop<'m, A> {
    /// A loop drawing at most one frame every `interval`.
    pub(super) fn new(
        exec: Executor<A>,
        renderer: Renderer,
        modes: &'m mut Modes,
        interval: Duration,
    ) -> Self {
        Self {
            exec,
            renderer,
            modes,
            dirty: true,
            urgent: false,
            interval,
            last_frame: None,
            frame_due: None,
            queried: None,
            replies: 0,
        }
    }

    /// Asks the terminal about its output modes, if the backend says to.
    /// The replies arrive as input; nothing waits for them.
    pub(super) fn query(&mut self, backend: &mut impl Backend) -> Result<(), Error> {
        if backend.query_output_modes() {
            let out = backend.output();
            out.write_all(QUERY)?;
            out.flush()?;
            self.queried = Some(Instant::now());
            self.replies = 2;
        }
        Ok(())
    }

    /// Updates, draws and waits until something ends the program.
    pub(super) fn run<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<End, Error> {
        loop {
            if let Some(end) = self.update(model, backend)? {
                return self.end(end, model, inbox, backend);
            }
            self.frame(model, backend)?;
            self.wait(inbox);
        }
    }

    /// The last frame (unless killed or panicked), replies still on
    /// their way, and the renderer's goodbye.
    fn end<M: Model<Msg = A>>(
        &mut self,
        end: End,
        model: &M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<End, Error> {
        if matches!(end, End::Quit | End::Interrupt) {
            self.draw(&model.view(), backend)?;
            self.drain(inbox);
            self.renderer.finish(backend.output())?;
        } else {
            self.renderer.abandon(backend.output())?;
        }
        Ok(end)
    }

    /// Updates the model with everything ready, in order, and applies
    /// output in the same order; how the program ends, if it does.
    fn update<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        backend: &mut impl Backend,
    ) -> Result<Option<End>, Error> {
        while let Some(item) = self.exec.pop() {
            match item {
                Item::Msg(msg) => self.message(model, msg, backend)?,
                Item::Output(output) => self.output(model, output, backend)?,
                Item::Quit => return Ok(Some(End::Quit)),
                Item::Interrupt => return Ok(Some(End::Interrupt)),
                Item::Kill => return Ok(Some(End::Kill)),
                Item::Panic(message) => return Ok(Some(End::Panic(message))),
            }
        }
        Ok(None)
    }

    /// One message for the model; the renderer hears sizes and mode
    /// reports first.
    fn message<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        msg: Msg<A>,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match msg {
            Msg::WindowSize(size) => self.renderer.resize(size),
            Msg::ModeReport { mode, value } => self.capability(mode, value, backend)?,
            _ => {}
        }
        let cmd: Cmd<A> = model.update(msg);
        self.exec.run(cmd, Instant::now());
        self.dirty = true;
        Ok(())
    }

    /// Acts on a reply to the start query, as Bubble Tea does.
    fn capability(
        &mut self,
        mode: ReportedMode,
        value: ModeSetting,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match mode {
            ReportedMode::Dec(2026) => {
                self.replies = self.replies.saturating_sub(1);
                self.renderer.set_sync(value == ModeSetting::Reset);
            }
            ReportedMode::Dec(2027) => {
                self.replies = self.replies.saturating_sub(1);
                let set = [
                    ModeSetting::Reset,
                    ModeSetting::Set,
                    ModeSetting::PermanentlySet,
                ];
                if set.contains(&value) {
                    self.renderer.set_width_method(WidthMethod::GraphemeWidth);
                    set_width_method(WidthMethod::GraphemeWidth);
                    self.modes.unicode_core(backend)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Output from a command, applied now, between frames.
    fn output<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        output: Output,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match output {
            Output::Raw(bytes) => {
                let out = backend.output();
                out.write_all(&bytes)?;
                out.flush()?;
            }
            Output::ClearScreen => {
                self.renderer.clear_screen();
                self.modes.reassert(backend)?;
                self.dirty = true;
            }
            Output::Print(text) => self.print(model, &text, backend)?,
        }
        Ok(())
    }

    /// Prints above the frame. The frame for the state the lines follow
    /// goes on screen first, unless it enters the alternate screen: then
    /// the lines go on the normal screen, where they stay.
    fn print<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        text: &str,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        let view = model.view();
        if self.dirty && !view.alt_screen {
            self.draw(&view, backend)?;
        }
        Ok(self.renderer.insert_above(text, backend.output())?)
    }

    /// Draws if something changed and the frame rate allows: at once
    /// after input, else no sooner than one interval after the last.
    fn frame<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        if !self.dirty {
            return Ok(());
        }
        if !self.urgent {
            if let Some(last) = self.last_frame {
                let due = last + self.interval;
                if Instant::now() < due {
                    self.frame_due = Some(due);
                    return Ok(());
                }
            }
        }
        self.draw(&model.view(), backend)
    }

    /// Draws `view`: screen switch, modes, then content.
    fn draw(&mut self, view: &View, backend: &mut impl Backend) -> Result<(), Error> {
        if view.alt_screen != self.renderer.is_alt() {
            self.modes.switch_screen(view.alt_screen, backend)?;
            self.renderer.switch_screen(view.alt_screen);
        }
        self.modes.apply(view, backend)?;
        self.renderer.render(view, backend.output())?;
        self.dirty = false;
        self.urgent = false;
        self.last_frame = Some(Instant::now());
        self.frame_due = None;
        Ok(())
    }

    /// Sleeps until an event arrives, a timer is due or a held back
    /// frame may be drawn, then takes in everything that is ready.
    fn wait(&mut self, inbox: &Receiver<Event<A>>) {
        let deadline = [self.exec.next_deadline(), self.frame_due]
            .into_iter()
            .flatten()
            .min();
        let first = match deadline {
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

    /// Waits a little for replies to the start query that have not come
    /// yet, so the terminal does not type them into the shell.
    fn drain(&mut self, inbox: &Receiver<Event<A>>) {
        let Some(asked) = self.queried else { return };
        if self.replies == 0 || asked.elapsed() > REPLY_WINDOW {
            return;
        }
        let until = Instant::now() + DRAIN_TIMEOUT;
        while self.replies > 0 {
            let left = until.saturating_duration_since(Instant::now());
            match inbox.recv_timeout(left) {
                Ok(Event::Input(Input::ModeReport {
                    mode: ReportedMode::Dec(2026 | 2027),
                    ..
                })) => self.replies -= 1,
                Ok(_) => {}
                Err(_) => return,
            }
        }
    }

    /// Queues what one event means for the model. Keys, mouse events and
    /// pastes make the next frame go out at once.
    fn take(&mut self, event: Event<A>) {
        let item = match event {
            Event::Input(input) => {
                self.urgent |= is_urgent(&input);
                input_item(input)
            }
            Event::Done(parent, msg) => return self.exec.task_done(parent, msg, Instant::now()),
            Event::Panic(message) => Item::Panic(message),
            Event::Send(msg) => Item::Msg(Msg::App(msg)),
            Event::Quit => Item::Quit,
            Event::Kill => Item::Kill,
        };
        self.exec.push(item);
    }
}

/// Whether input comes from the user, whose frame should not wait.
const fn is_urgent(input: &Input) -> bool {
    matches!(
        input,
        Input::KeyPress(_)
            | Input::KeyRelease(_)
            | Input::MouseClick(_)
            | Input::MouseRelease(_)
            | Input::MouseWheel(_)
            | Input::MouseMotion(_)
            | Input::Paste(_)
    )
}

/// What a piece of terminal input means for the model: keys, mouse and
/// paste here, the rest below.
fn input_item<M>(input: Input) -> Item<M> {
    let msg = match input {
        Input::KeyPress(key) => Msg::KeyPress(key),
        Input::KeyRelease(key) => Msg::KeyRelease(key),
        Input::MouseClick(mouse) => Msg::MouseClick(mouse),
        Input::MouseRelease(mouse) => Msg::MouseRelease(mouse),
        Input::MouseWheel(mouse) => Msg::MouseWheel(mouse),
        Input::MouseMotion(mouse) => Msg::MouseMotion(mouse),
        Input::Paste(text) => Msg::Paste(text),
        input => return focus_or_reply_item(input),
    };
    Item::Msg(msg)
}

/// Paste marks, focus, and the terminal's replies.
fn focus_or_reply_item<M>(input: Input) -> Item<M> {
    let msg = match input {
        Input::PasteStart => Msg::PasteStart,
        Input::PasteEnd => Msg::PasteEnd,
        Input::Focus => Msg::Focus,
        Input::Blur => Msg::Blur,
        Input::CursorPosition { x, y } => Msg::CursorPosition { x, y },
        Input::KeyboardEnhancements(report) => Msg::KeyboardEnhancements(report),
        Input::ModeReport { mode, value } => Msg::ModeReport { mode, value },
        input => return window_item(input),
    };
    Item::Msg(msg)
}

/// The terminal's name and colours, resizes, and signals.
fn window_item<M>(input: Input) -> Item<M> {
    Item::Msg(match input {
        Input::TerminalVersion(name) => Msg::TerminalVersion(name),
        Input::ForegroundColor(c) => Msg::ForegroundColor(c),
        Input::BackgroundColor(c) => Msg::BackgroundColor(c),
        Input::CursorColor(c) => Msg::CursorColor(c),
        Input::Resize(size) => Msg::WindowSize(size),
        Input::Interrupt => return Item::Interrupt,
        Input::Terminate => return Item::Quit,
        input => return input_item(input),
    })
}
