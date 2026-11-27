//! Terminal modes a view turns on and off (the alternate screen, mouse,
//! focus, bracketed paste, keyboard enhancements, the cursor's look, the
//! window title, colours, the progress bar), and the replies a terminal
//! sends about modes.

mod active;

pub(crate) use active::Modes;

use crate::color::Rgb;
use crate::view::{ProgressBar, ProgressBarState};

/// A terminal mode a program can turn on and off. Bubble Tea turns them
/// on and off from each [`View`](crate::View)'s fields; a
/// [`Backend`](crate::Backend) writes them.
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
    /// The alternate screen (mode 1049), saving the main screen's cursor.
    AltScreen,
    /// Autowrap turned off (DECAWM, mode 7): on writes `CSI ?7l`, off
    /// `CSI ?7h`. On while a program draws, so writing the last column
    /// never wraps or scrolls.
    NoAutowrap,
    /// Grapheme cluster widths (mode 2027), where the terminal has them.
    UnicodeCore,
    /// The window title saved on the terminal's title stack (XTWINOPS 22
    /// and 23): off clears the title, then pops the saved one back.
    SavedTitle,
    /// A cursor shape (DECSCUSR 1 to 6); off is the terminal's default.
    CursorStyle(u8),
    /// The terminal's default text colour (OSC 10); off resets it.
    ForegroundColor(Rgb),
    /// The terminal's default background colour (OSC 11); off resets it.
    BackgroundColor(Rgb),
    /// The cursor's colour (OSC 12); off resets it.
    CursorColor(Rgb),
    /// A progress bar (OSC 9;4); off removes it.
    ProgressBar(ProgressBar),
}

impl TerminalMode {
    /// The bytes that turn the mode on (`on`) or off.
    #[must_use]
    pub fn sequence(self, on: bool) -> Vec<u8> {
        if on {
            self.enter_sequence()
        } else {
            self.disable().to_vec()
        }
    }

    /// The bytes that turn the mode on.
    fn enter_sequence(self) -> Vec<u8> {
        match self {
            Self::KeyboardEnhancements(flags) => format!("\x1b[>4;2m\x1b[>{flags}u").into_bytes(),
            Self::KeyboardFlags(flags) => format!("\x1b[>4;2m\x1b[={flags};1u").into_bytes(),
            Self::CursorStyle(n) => format!("\x1b[{n} q").into_bytes(),
            Self::ForegroundColor(c) => osc_colour(10, c),
            Self::BackgroundColor(c) => osc_colour(11, c),
            Self::CursorColor(c) => osc_colour(12, c),
            Self::ProgressBar(bar) => bar.sequence().into_bytes(),
            mode => mode.entry().1.to_vec(),
        }
    }

    /// The mode's row in [`SEQUENCES`].
    fn entry(self) -> &'static (TerminalMode, &'static [u8], &'static [u8]) {
        let kind = self.kind();
        SEQUENCES
            .iter()
            .find(|e| e.0 == kind)
            .unwrap_or(&SEQUENCES[0])
    }

    /// The bytes that turn the mode off; Bubble Tea turns every mouse
    /// mode off at once.
    pub(crate) fn disable(self) -> &'static [u8] {
        self.entry().2
    }

    /// The same mode whatever its flags, to tell modes apart.
    pub(crate) const fn kind(self) -> Self {
        const BLACK: Rgb = Rgb::new(0, 0, 0);
        match self {
            Self::KeyboardEnhancements(_) => Self::KeyboardEnhancements(0),
            Self::KeyboardFlags(_) => Self::KeyboardFlags(0),
            Self::CursorStyle(_) => Self::CursorStyle(0),
            Self::ForegroundColor(_) => Self::ForegroundColor(BLACK),
            Self::BackgroundColor(_) => Self::BackgroundColor(BLACK),
            Self::CursorColor(_) => Self::CursorColor(BLACK),
            Self::ProgressBar(_) => Self::ProgressBar(ProgressBar {
                state: ProgressBarState::None,
                value: 0,
            }),
            mode => mode,
        }
    }
}

/// Each kind of mode, the bytes that turn it on (when they do not
/// depend on its value) and the bytes that turn it off.
const SEQUENCES: [(TerminalMode, &[u8], &[u8]); 15] = {
    use TerminalMode as M;
    const BLACK: Rgb = Rgb::new(0, 0, 0);
    const NO_BAR: ProgressBar = ProgressBar {
        state: ProgressBarState::None,
        value: 0,
    };
    const MOUSE_OFF: &[u8] = b"\x1b[?1002l\x1b[?1003l\x1b[?1006l";
    [
        (M::BracketedPaste, b"\x1b[?2004h", b"\x1b[?2004l"),
        (M::ReportFocus, b"\x1b[?1004h", b"\x1b[?1004l"),
        (M::MouseCellMotion, b"\x1b[?1002h\x1b[?1006h", MOUSE_OFF),
        (M::MouseAllMotion, b"\x1b[?1003h\x1b[?1006h", MOUSE_OFF),
        (M::KeyboardEnhancements(0), b"", b"\x1b[>4m\x1b[<u"),
        (M::KeyboardFlags(0), b"", b""),
        (M::AltScreen, b"\x1b[?1049h", b"\x1b[?1049l"),
        (M::NoAutowrap, b"\x1b[?7l", b"\x1b[?7h"),
        (M::UnicodeCore, b"\x1b[?2027h", b"\x1b[?2027l"),
        (M::SavedTitle, b"\x1b[22;2t", b"\x1b]2;\x07\x1b[23;2t"),
        (M::CursorStyle(0), b"", b"\x1b[0 q"),
        (M::ForegroundColor(BLACK), b"", b"\x1b]110\x07"),
        (M::BackgroundColor(BLACK), b"", b"\x1b]111\x07"),
        (M::CursorColor(BLACK), b"", b"\x1b]112\x07"),
        (M::ProgressBar(NO_BAR), b"", b"\x1b]9;4;0\x07"),
    ]
};

/// An OSC that sets colour `n` (10, 11 or 12), as Go writes it.
fn osc_colour(n: u8, c: Rgb) -> Vec<u8> {
    format!("\x1b]{n};#{:02x}{:02x}{:02x}\x07", c.r, c.g, c.b).into_bytes()
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

#[cfg(test)]
mod tests;
