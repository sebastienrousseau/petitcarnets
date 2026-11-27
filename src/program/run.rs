//! The loop itself: updates, frames, output in program order, and
//! waiting.

mod handover;
mod translate;

use super::End;
use crate::backend::Backend;
use crate::clock::{Clock, Now};
use crate::cmd::{Handover, Output};
use crate::color::ColorProfile;
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

/// How the loop is set up.
pub(super) struct Settings {
    /// The shortest time between frames.
    pub(super) interval: Duration,
    /// The colour profile.
    pub(super) profile: ColorProfile,
    /// Whether the profile was set by the program, so it is kept.
    pub(super) forced: bool,
    /// Whether Ctrl-Z suspends.
    pub(super) suspend_key: bool,
    /// Whether the backend is a real terminal.
    pub(super) terminal: bool,
}

/// The loop's state besides the model.
pub(super) struct Loop<M, C> {
    pub(super) exec: Executor<M>,
    renderer: Renderer,
    pub(super) modes: Modes,
    settings: Settings,
    clock: C,
    dirty: bool,
    urgent: bool,
    last_frame: Option<Instant>,
    frame_due: Option<Instant>,
    /// When a query was last written; its reply may be on its way.
    queried: Option<Instant>,
    /// Whether the terminal is handed over until a restore.
    released: bool,
    /// Output that waits while the terminal is handed over.
    held_back: Vec<Output>,
}

impl<A: Send + 'static, C: Clock> Loop<A, C> {
    /// A loop set up as `settings` say, on `clock`.
    pub(super) fn new(
        exec: Executor<A>,
        mut renderer: Renderer,
        settings: Settings,
        clock: C,
    ) -> Self {
        let mut modes = Modes::default();
        renderer.set_color_profile(settings.profile);
        modes.set_color_profile(settings.profile);
        Self {
            exec,
            renderer,
            modes,
            settings,
            clock,
            dirty: true,
            urgent: false,
            last_frame: None,
            frame_due: None,
            queried: None,
            released: false,
            held_back: Vec::new(),
        }
    }

    /// The time on the loop's clock.
    pub(super) fn now(&self) -> Now {
        self.clock.now()
    }

    /// Asks the terminal about its output modes, if it is one and `ask`
    /// says to. The replies arrive as input; nothing waits for them.
    pub(super) fn query(&mut self, backend: &mut impl Backend, ask: bool) -> Result<(), Error> {
        if self.settings.terminal && ask {
            self.write_query(QUERY, backend)?;
        }
        Ok(())
    }

    /// Writes a query and remembers when.
    fn write_query(&mut self, bytes: &[u8], backend: &mut impl Backend) -> Result<(), Error> {
        let out = backend.output();
        out.write_all(bytes)?;
        out.flush()?;
        self.queried = Some(self.clock.now().at);
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
            if let Some(end) = self.step(model, inbox, backend)? {
                return Ok(end);
            }
            self.wait(inbox);
        }
    }

    /// Updates the model with everything ready and draws, if the frame
    /// rate allows; how the program ended, if it did, after its last
    /// frame.
    pub(super) fn step<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<Option<End>, Error> {
        if let Some(end) = self.update(model, inbox, backend)? {
            return self.end(end, model, inbox, backend).map(Some);
        }
        self.frame(model, backend)?;
        Ok(None)
    }

    /// When the loop next has something to do with no event: the
    /// earliest timer, or a held back frame.
    pub(super) fn next_wake(&self) -> Option<Instant> {
        [self.exec.next_deadline(), self.frame_due]
            .into_iter()
            .flatten()
            .min()
    }

    /// The terminal taken back if it was handed over, with the output
    /// that waited unless killed or panicked; the last frame (likewise),
    /// replies still on their way, and the renderer's goodbye.
    fn end<M: Model<Msg = A>>(
        &mut self,
        end: End,
        model: &M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<End, Error> {
        let graceful = matches!(end, End::Quit | End::Interrupt);
        if self.released {
            self.take_back_at_end(graceful, model, backend)?;
        }
        if graceful {
            self.draw(&model.view(), backend)?;
            self.drain(inbox, backend, false)?;
            self.renderer.finish(backend.output())?;
        } else {
            self.drain(inbox, backend, false)?;
            self.renderer.abandon(backend.output())?;
        }
        Ok(end)
    }

    /// Takes the terminal back for the end: with the output that waited
    /// if the end is `graceful`, without it otherwise.
    fn take_back_at_end<M: Model<Msg = A>>(
        &mut self,
        graceful: bool,
        model: &M,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        if graceful {
            self.restore(model, backend)
        } else {
            self.take_back(backend)
        }
    }

    /// Updates the model with everything ready, in order, and applies
    /// output and hand-overs in the same order; how the program ends,
    /// if it does.
    fn update<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        inbox: &Receiver<Event<A>>,
        backend: &mut impl Backend,
    ) -> Result<Option<End>, Error> {
        while let Some(item) = self.exec.pop() {
            match item {
                Item::Msg(msg) => self.message(model, msg, backend)?,
                Item::Output(output) => self.output(model, output, backend)?,
                Item::Handover(handover) => self.lifecycle(handover, model, inbox, backend)?,
                end => return Ok(End::of(end)),
            }
        }
        Ok(None)
    }

    /// One message for the model; the renderer hears sizes, mode
    /// reports and capabilities first.
    fn message<M: Model<Msg = A>>(
        &mut self,
        model: &mut M,
        msg: Msg<A>,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match &msg {
            Msg::WindowSize(size) => self.renderer.resize(*size),
            Msg::ModeReport { mode, value } => self.mode_report(*mode, *value, backend)?,
            Msg::Capability(capability) => self.capability(capability),
            _ => {}
        }
        let cmd: Cmd<A> = model.update(msg);
        self.exec.run(cmd, self.clock.now());
        self.dirty = true;
        Ok(())
    }

    /// Acts on a reply to the start query, as Bubble Tea does.
    fn mode_report(
        &mut self,
        mode: ReportedMode,
        value: ModeSetting,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        match mode {
            ReportedMode::Dec(2026) => self.renderer.set_sync(value == ModeSetting::Reset),
            ReportedMode::Dec(2027) => {
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

    /// A terminal that says it has `RGB` or `Tc` shows 24-bit colour, as
    /// Bubble Tea decides; unlike Bubble Tea (its issue #1773), a profile
    /// the program set is kept.
    fn capability(&mut self, capability: &str) {
        let name = capability.split('=').next().unwrap_or_default();
        let upgrade = matches!(name, "RGB" | "Tc")
            && !self.settings.forced
            && self.settings.profile != ColorProfile::TrueColor;
        if upgrade {
            self.settings.profile = ColorProfile::TrueColor;
            self.renderer.set_color_profile(ColorProfile::TrueColor);
            self.modes.set_color_profile(ColorProfile::TrueColor);
            let msg = Msg::ColorProfile(ColorProfile::TrueColor);
            self.exec.push(Item::Msg(msg));
        }
    }

    /// Output from a command, applied now, between frames; it waits
    /// while the terminal is handed over.
    fn output<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        output: Output,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        if self.released {
            self.held_back.push(output);
            return Ok(());
        }
        match output {
            Output::Raw(bytes) | Output::Query(bytes) => self.write_query(&bytes, backend),
            Output::Print(text) => self.print(model, &text, backend),
            other => self.redraw(&other, backend),
        }
    }

    /// Clears the screen or repaints, with the modes written again; or
    /// reads the window's size for the model.
    fn redraw(&mut self, output: &Output, backend: &mut impl Backend) -> Result<(), Error> {
        match output {
            Output::ClearScreen => {
                self.renderer.clear_screen();
                self.modes.reassert(backend)?;
            }
            Output::Repaint => {
                self.modes.release(backend)?;
                self.modes.resume(backend)?;
                self.renderer.repaint();
            }
            _ => {
                let size = backend.size()?;
                self.exec.push_front(Item::Msg(Msg::WindowSize(size)));
            }
        }
        self.dirty = true;
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
    /// Nothing is drawn while the terminal is handed over, and a frame
    /// held back then is forgotten (taking the terminal back draws one at
    /// once), so it cannot wake the loop over and over.
    fn frame<M: Model<Msg = A>>(
        &mut self,
        model: &M,
        backend: &mut impl Backend,
    ) -> Result<(), Error> {
        if !self.dirty || self.released {
            self.frame_due = None;
            return Ok(());
        }
        if !self.urgent {
            if let Some(last) = self.last_frame {
                let due = last + self.settings.interval;
                if self.clock.now().at < due {
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
        self.last_frame = Some(self.clock.now().at);
        self.frame_due = None;
        Ok(())
    }

    /// Sleeps until an event arrives, a timer is due or a held back
    /// frame may be drawn, then takes in everything that is ready.
    pub(super) fn wait(&mut self, inbox: &Receiver<Event<A>>) {
        let events = self.clock.wait(inbox, self.next_wake(), self.exec.tasks());
        for event in events {
            self.take(event);
        }
        self.exec.fire_due(self.clock.now());
    }

    /// Queues what one event means for the model. Keys, mouse events and
    /// pastes make the next frame go out at once; Ctrl-Z suspends unless
    /// the program says not to.
    fn take(&mut self, event: Event<A>) {
        let item = match event {
            Event::Input(input) => {
                self.urgent |= translate::is_urgent(&input);
                if self.settings.suspend_key && translate::is_suspend_key(&input) {
                    Item::Handover(Handover::Suspend)
                } else {
                    translate::input_item(input)
                }
            }
            Event::Done(task, msg) => return self.exec.task_done(task, msg, self.clock.now()),
            Event::Panic(_, message) => {
                self.exec.task_panicked();
                Item::Panic(message)
            }
            Event::Send(msg) => Item::Msg(Msg::App(msg)),
            Event::Quit => Item::Quit,
            Event::Kill => Item::Kill,
        };
        self.exec.push(item);
    }
}
