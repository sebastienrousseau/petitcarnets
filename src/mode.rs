//! Terminal modes a view turns on and off: mouse, focus, bracketed paste
//! and keyboard enhancements, and the replies a terminal sends about
//! modes.

use crate::backend::Backend;
use crate::mouse::MouseMode;
use crate::msg::View;
use std::io;

/// A terminal mode a program can turn on and off. Bubble Tea turns them
/// on and off from each [`View`]'s fields; a [`Backend`] writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TerminalMode {
    /// Bracketed paste (mode 2004): pasted text arrives as one paste.
    BracketedPaste,
    /// Focus reports (mode 1004).
    ReportFocus,
    /// Mouse clicks, wheel and drags (modes 1002 and 1006).
    MouseCellMotion,
    /// Every mouse event, plain motion included (modes 1003 and 1006).
    MouseAllMotion,
    /// xterm's modifyOtherKeys level 2 and a kitty keyboard entry with
    /// these flags pushed on the terminal's stack.
    KeyboardEnhancements(u8),
    /// New flags for the kitty keyboard entry already pushed by
    /// [`TerminalMode::KeyboardEnhancements`], set in place. Turning it
    /// off does nothing: turn off `KeyboardEnhancements` instead.
    KeyboardFlags(u8),
}

impl TerminalMode {
    /// The bytes that turn the mode on (`on`) or off.
    #[must_use]
    pub fn sequence(self, on: bool) -> Vec<u8> {
        match (self, on) {
            (Self::KeyboardEnhancements(flags), true) => {
                format!("\x1b[>4;2m\x1b[>{flags}u").into_bytes()
            }
            (Self::KeyboardFlags(flags), true) => {
                format!("\x1b[>4;2m\x1b[={flags};1u").into_bytes()
            }
            (mode, true) => mode.enable().to_vec(),
            (mode, false) => mode.disable().to_vec(),
        }
    }

    /// The bytes that turn a mode without flags on.
    const fn enable(self) -> &'static [u8] {
        match self {
            Self::BracketedPaste => b"\x1b[?2004h",
            Self::ReportFocus => b"\x1b[?1004h",
            Self::MouseCellMotion => b"\x1b[?1002h\x1b[?1006h",
            Self::MouseAllMotion => b"\x1b[?1003h\x1b[?1006h",
            Self::KeyboardEnhancements(_) | Self::KeyboardFlags(_) => b"",
        }
    }

    /// The bytes that turn the mode off; Bubble Tea turns every mouse
    /// mode off at once.
    pub(crate) const fn disable(self) -> &'static [u8] {
        match self {
            Self::BracketedPaste => b"\x1b[?2004l",
            Self::ReportFocus => b"\x1b[?1004l",
            Self::MouseCellMotion | Self::MouseAllMotion => b"\x1b[?1002l\x1b[?1003l\x1b[?1006l",
            Self::KeyboardEnhancements(_) => b"\x1b[>4m\x1b[<u",
            Self::KeyboardFlags(_) => b"",
        }
    }

    /// The same mode whatever its flags, to tell modes apart.
    pub(crate) const fn kind(self) -> Self {
        match self {
            Self::KeyboardEnhancements(_) => Self::KeyboardEnhancements(0),
            Self::KeyboardFlags(_) => Self::KeyboardFlags(0),
            mode => mode,
        }
    }
}

/// The keyboard enhancements a view asks for, on top of the kitty
/// protocol's key disambiguation, which is always on. Bubble Tea's
/// `KeyboardEnhancements`: where Go sets a field, as in
/// `KeyboardEnhancements{ReportEventTypes: true}`, chain the method of
/// the same name, as in `KeyboardEnhancements::new().report_event_types()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardEnhancements(u8);

impl Default for KeyboardEnhancements {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardEnhancements {
    /// Disambiguation only (kitty flag 1).
    #[must_use]
    pub const fn new() -> Self {
        Self(1)
    }

    /// Also report key repeats and releases
    /// ([`Msg::KeyRelease`](crate::Msg::KeyRelease)). Go's
    /// `ReportEventTypes`.
    #[must_use]
    pub const fn report_event_types(self) -> Self {
        Self(self.0 | 2)
    }

    /// Also report the shifted key and the key on a US layout. Go's
    /// `ReportAlternateKeys`.
    #[must_use]
    pub const fn report_alternate_keys(self) -> Self {
        Self(self.0 | 4)
    }

    /// Report every key as an escape code, plain text included. Go's
    /// `ReportAllKeysAsEscapeCodes`.
    #[must_use]
    pub const fn report_all_keys_as_escape_codes(self) -> Self {
        Self(self.0 | 8)
    }

    /// With all keys as escape codes, report the text keys type. Go's
    /// `ReportAssociatedText`.
    #[must_use]
    pub const fn report_associated_text(self) -> Self {
        Self(self.0 | 16)
    }

    /// The kitty protocol flags: 1 (disambiguate) always, plus 2, 4, 8
    /// and 16 for the enhancements asked for.
    #[must_use]
    pub const fn flags(self) -> u8 {
        self.0
    }
}

/// The terminal's reply about its keyboard enhancements. Bubble Tea's
/// `KeyboardEnhancementsMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardEnhancementsReport {
    /// The kitty protocol flags in force; -1 when the reply had none.
    pub flags: i32,
}

impl KeyboardEnhancementsReport {
    /// Whether keys are disambiguated: any flag at all.
    #[must_use]
    pub const fn supports_key_disambiguation(self) -> bool {
        self.flags > 0
    }

    /// Whether repeats and releases are reported.
    #[must_use]
    pub const fn supports_event_types(self) -> bool {
        self.flags & 2 != 0
    }

    /// Whether alternate keys are reported.
    #[must_use]
    pub const fn supports_alternate_keys(self) -> bool {
        self.flags & 4 != 0
    }

    /// Whether every key is reported as an escape code.
    #[must_use]
    pub const fn supports_all_keys_as_escape_codes(self) -> bool {
        self.flags & 8 != 0
    }

    /// Whether associated text is reported.
    #[must_use]
    pub const fn supports_associated_text(self) -> bool {
        self.flags & 16 != 0
    }
}

/// A mode a terminal reported on: a DEC private mode (`CSI ? n $ y`) or
/// an ANSI one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportedMode {
    /// A DEC private mode, such as 2004 for bracketed paste.
    Dec(u32),
    /// An ANSI mode.
    Ansi(u32),
}

/// What a terminal says about a mode. Go's `ansi.ModeSetting`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeSetting {
    /// The terminal does not know the mode.
    NotRecognized,
    /// On.
    Set,
    /// Off.
    Reset,
    /// On, and cannot be changed.
    PermanentlySet,
    /// Off, and cannot be changed.
    PermanentlyReset,
    /// Any other value.
    Other(u8),
}

impl From<u8> for ModeSetting {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::NotRecognized,
            1 => Self::Set,
            2 => Self::Reset,
            3 => Self::PermanentlySet,
            4 => Self::PermanentlyReset,
            v => Self::Other(v),
        }
    }
}

impl From<ModeSetting> for u8 {
    fn from(setting: ModeSetting) -> Self {
        match setting {
            ModeSetting::NotRecognized => 0,
            ModeSetting::Set => 1,
            ModeSetting::Reset => 2,
            ModeSetting::PermanentlySet => 3,
            ModeSetting::PermanentlyReset => 4,
            ModeSetting::Other(v) => v,
        }
    }
}

/// The modes a frame asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Wanted {
    paste: bool,
    focus: bool,
    mouse: MouseMode,
    keyboard: u8,
}

impl Wanted {
    /// Nothing on.
    const OFF: Self = Self {
        paste: false,
        focus: false,
        mouse: MouseMode::None,
        keyboard: 0,
    };

    fn of(view: &View) -> Self {
        Self {
            paste: !view.disable_bracketed_paste_mode,
            focus: view.report_focus,
            mouse: view.mouse_mode,
            keyboard: view.keyboard_enhancements.flags(),
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

/// The modes on now, so each frame changes only what differs, and the
/// end of a run turns them all off.
#[derive(Debug, Default)]
pub(crate) struct Modes(Option<Wanted>);

impl Modes {
    /// Turns modes on and off so the terminal matches `view`.
    pub(crate) fn apply(&mut self, view: &View, backend: &mut impl Backend) -> io::Result<()> {
        let want = Wanted::of(view);
        self.change(want, backend)?;
        self.0 = Some(want);
        Ok(())
    }

    /// Turns every mode off, newest first.
    pub(crate) fn clear(&mut self, backend: &mut impl Backend) -> io::Result<()> {
        if self.0.is_some() {
            self.change(Wanted::OFF, backend)?;
        }
        self.0 = None;
        Ok(())
    }

    fn change(&self, want: Wanted, backend: &mut impl Backend) -> io::Result<()> {
        let was = self.0.unwrap_or(Wanted::OFF);
        let toggle = |mode, on: bool, was: bool| (on != was).then_some((mode, on));
        let mut changes: Vec<(TerminalMode, bool)> = Vec::new();
        changes.extend(toggle(TerminalMode::BracketedPaste, want.paste, was.paste));
        changes.extend(toggle(TerminalMode::ReportFocus, want.focus, was.focus));
        if want.mouse != was.mouse {
            changes.extend(mouse_mode(was.mouse).map(|m| (m, false)));
            changes.extend(mouse_mode(want.mouse).map(|m| (m, true)));
        }
        changes.extend(keyboard_change(was.keyboard, want.keyboard));
        if want == Wanted::OFF {
            changes.reverse();
        }
        changes
            .into_iter()
            .try_for_each(|(mode, on)| backend.set_mode(mode, on))
    }
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

#[cfg(test)]
mod tests;
