//! The modes on now, so each frame changes only what differs from the
//! frame before, and the end of a run turns them all off.

use super::TerminalMode;
use crate::backend::Backend;
use crate::color::{ColorProfile, Rgb};
use crate::mouse::MouseMode;
use crate::view::{ProgressBar, ProgressBarState, View};
use std::io;

/// The modes a frame asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Wanted {
    paste: bool,
    focus: bool,
    mouse: MouseMode,
    keyboard: u8,
    title: String,
    foreground: Option<Rgb>,
    background: Option<Rgb>,
    cursor_colour: Option<Rgb>,
    cursor_style: u8,
    progress: Option<ProgressBar>,
}

impl Wanted {
    /// The modes `view` asks for, its colours brought down to `profile`
    /// (and left out when it shows none).
    fn of(view: &View, profile: ColorProfile) -> Self {
        let colour = |c: Option<Rgb>| c.and_then(|c| profile.rgb(c));
        Self {
            paste: !view.disable_bracketed_paste_mode,
            focus: view.report_focus,
            mouse: view.mouse_mode,
            keyboard: view.keyboard_enhancements.flags(),
            title: view.window_title.clone(),
            foreground: colour(view.foreground_color),
            background: colour(view.background_color),
            cursor_colour: colour(view.cursor.and_then(|c| c.color)),
            cursor_style: view.cursor.map_or(0, |c| c.style()),
            progress: view
                .progress_bar
                .filter(|p| p.state != ProgressBarState::None),
        }
    }
}

/// The terminal mode for a mouse mode.
const fn mouse_mode(mode: MouseMode) -> Option<TerminalMode> {
    match mode {
        MouseMode::None => None,
        MouseMode::CellMotion => Some(TerminalMode::MouseCellMotion),
        MouseMode::AllMotion => Some(TerminalMode::MouseAllMotion),
    }
}

/// A change from `was` to `now`, as a mode turned on (with its new
/// value) or off.
fn valued<T: PartialEq>(
    was: Option<T>,
    now: Option<T>,
    mode: fn(T) -> TerminalMode,
) -> Option<(TerminalMode, bool)> {
    if was == now {
        return None;
    }
    match (was, now) {
        (_, Some(v)) => Some((mode(v), true)),
        (Some(v), None) => Some((mode(v), false)),
        (None, None) => None,
    }
}

/// The modes on now.
#[derive(Debug)]
pub(crate) struct Modes {
    now: Option<Wanted>,
    /// Modes on for the whole run rather than per view: autowrap off,
    /// grapheme widths, the alternate screen and the saved title.
    held: Vec<TerminalMode>,
    /// What colours the terminal is sent.
    profile: ColorProfile,
    /// Whether autowrap stays on, for output printed as lines.
    autowrap: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            now: None,
            held: Vec::new(),
            profile: ColorProfile::TrueColor,
            autowrap: false,
        }
    }
}

impl Modes {
    /// Brings the colours of the next frame's modes down to `profile`.
    pub(crate) fn set_color_profile(&mut self, profile: ColorProfile) {
        self.profile = profile;
    }

    /// Leaves autowrap on, so long lines wrap: for output printed as
    /// lines rather than drawn in cells.
    pub(crate) const fn keep_autowrap(&mut self) {
        self.autowrap = true;
    }

    /// Turns every mode off for another program, newest first, as at the
    /// end, but remembers them for [`Modes::resume`].
    pub(crate) fn release(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        let mut off = self
            .now
            .as_ref()
            .map(|now| changes(now, &Wanted::default()))
            .unwrap_or_default();
        off.reverse();
        off.extend(self.held.iter().rev().map(|m| (*m, false)));
        set_all(off, backend)
    }

    /// Turns every mode [`Modes::release`] turned off on again, in the
    /// order they were first turned on, and writes the title again.
    pub(crate) fn resume(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        set_all(self.held.iter().map(|m| (*m, true)).collect(), backend)?;
        let Some(now) = self.now.clone() else {
            return Ok(());
        };
        set_all(changes(&Wanted::default(), &now), backend)?;
        if now.title.is_empty() {
            return Ok(());
        }
        let out = backend.output();
        write!(out, "\x1b]2;{}\x07", now.title)?;
        out.flush()
    }

    /// Turns modes on and off so the terminal matches `view`. The first
    /// call also turns autowrap off, for the renderer, unless it is kept.
    pub(crate) fn apply(&mut self, view: &View, backend: &mut impl Backend) -> io::Result<()> {
        if !self.autowrap {
            self.hold(TerminalMode::NoAutowrap, backend)?;
        }
        let want = Wanted::of(view, self.profile);
        let first = self.now.is_none();
        let was = self.now.take().unwrap_or_default();
        let result = set_all(changes(&was, &want), backend);
        let title = self.title(&was.title, &want.title, first, backend);
        self.now = Some(want);
        result.and(title)
    }

    /// Switches to the alternate screen or back. Keyboard enhancements
    /// are popped first and pushed again by the next [`Modes::apply`],
    /// as each screen keeps its own, as Bubble Tea does.
    pub(crate) fn switch_screen(
        &mut self,
        alt: bool,
        backend: &mut impl Backend,
    ) -> io::Result<()> {
        if let Some(now) = self.now.as_mut() {
            if let Some(change) = keyboard_change(now.keyboard, 0) {
                backend.set_mode(change.0, change.1)?;
            }
            now.keyboard = 0;
        }
        if alt {
            self.hold(TerminalMode::AltScreen, backend)
        } else {
            self.unhold(TerminalMode::AltScreen, backend)
        }
    }

    /// Turns grapheme cluster widths (mode 2027) on.
    pub(crate) fn unicode_core(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        self.hold(TerminalMode::UnicodeCore, backend)
    }

    /// Turns a run-wide mode on, once.
    fn hold(&mut self, mode: TerminalMode, backend: &mut impl Backend) -> io::Result<()> {
        if self.held.contains(&mode) {
            return Ok(());
        }
        self.held.push(mode);
        backend.set_mode(mode, true)
    }

    /// Turns a run-wide mode off, if it is on.
    fn unhold(&mut self, mode: TerminalMode, backend: &mut impl Backend) -> io::Result<()> {
        let Some(i) = self.held.iter().position(|m| *m == mode) else {
            return Ok(());
        };
        let _ = self.held.remove(i);
        backend.set_mode(mode, false)
    }

    /// Writes again the modes that are on and can be set twice safely,
    /// after a clear screen.
    pub(crate) fn reassert(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        let Some(now) = self.now.clone() else {
            return Ok(());
        };
        let mut again = changes(&Wanted::default(), &now);
        again.retain(|(mode, _)| !matches!(mode, TerminalMode::KeyboardEnhancements(_)));
        if now.keyboard != 0 {
            again.push((TerminalMode::KeyboardFlags(now.keyboard), true));
        }
        let safe = [TerminalMode::NoAutowrap, TerminalMode::UnicodeCore];
        again.extend(
            self.held
                .iter()
                .filter(|m| safe.contains(m))
                .map(|m| (*m, true)),
        );
        set_all(again, backend)?;
        self.title("", &now.title, true, backend)
    }

    /// Turns every mode off, newest first.
    pub(crate) fn clear(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        let mut off = Vec::new();
        if let Some(was) = self.now.take() {
            off = changes(&was, &Wanted::default());
            off.reverse();
        }
        let held = std::mem::take(&mut self.held);
        off.extend(held.into_iter().rev().map(|m| (m, false)));
        set_all(off, backend)
    }

    /// Writes the window title when it changed, saving the terminal's
    /// own first; an empty title is written only to replace one.
    fn title(
        &mut self,
        was: &str,
        want: &str,
        first: bool,
        backend: &mut impl Backend,
    ) -> io::Result<()> {
        if was == want || (first && want.is_empty()) {
            return Ok(());
        }
        self.hold(TerminalMode::SavedTitle, backend)?;
        let out = backend.output();
        write!(out, "\x1b]2;{want}\x07")?;
        out.flush()
    }
}

/// Turns each mode on or off, in order.
fn set_all(modes: Vec<(TerminalMode, bool)>, backend: &mut impl Backend) -> io::Result<()> {
    modes
        .into_iter()
        .try_for_each(|(mode, on)| backend.set_mode(mode, on))
}

/// The modes to turn on and off to go from `was` to `want`, in the
/// order Bubble Tea writes them.
fn changes(was: &Wanted, want: &Wanted) -> Vec<(TerminalMode, bool)> {
    let toggle = |mode, on: bool, was: bool| (on != was).then_some((mode, on));
    let mut out: Vec<(TerminalMode, bool)> = Vec::new();
    out.extend(toggle(TerminalMode::BracketedPaste, want.paste, was.paste));
    out.extend(toggle(TerminalMode::ReportFocus, want.focus, was.focus));
    if want.mouse != was.mouse {
        out.extend(mouse_mode(was.mouse).map(|m| (m, false)));
        out.extend(mouse_mode(want.mouse).map(|m| (m, true)));
    }
    out.extend(keyboard_change(was.keyboard, want.keyboard));
    out.extend(look_changes(was, want));
    out
}

/// Colours, the cursor's look and the progress bar.
fn look_changes(was: &Wanted, want: &Wanted) -> Vec<(TerminalMode, bool)> {
    let style = |n: u8| (n != 0).then_some(n);
    [
        valued(
            was.cursor_colour,
            want.cursor_colour,
            TerminalMode::CursorColor,
        ),
        valued(
            was.foreground,
            want.foreground,
            TerminalMode::ForegroundColor,
        ),
        valued(
            was.background,
            want.background,
            TerminalMode::BackgroundColor,
        ),
        valued(
            style(was.cursor_style),
            style(want.cursor_style),
            TerminalMode::CursorStyle,
        ),
        valued(was.progress, want.progress, TerminalMode::ProgressBar),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// How the keyboard entry changes from flags `was` to `want` (0 is no
/// entry).
fn keyboard_change(was: u8, want: u8) -> Option<(TerminalMode, bool)> {
    match (was, want) {
        (w, n) if w == n => None,
        (0, n) => Some((TerminalMode::KeyboardEnhancements(n), true)),
        (w, 0) => Some((TerminalMode::KeyboardEnhancements(w), false)),
        (_, n) => Some((TerminalMode::KeyboardFlags(n), true)),
    }
}
