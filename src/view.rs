//! What a model shows: its text, and everything else about the terminal
//! a frame decides (screen, cursor, title, colours, progress bar, input
//! modes).

use crate::color::Rgb;
use crate::mode::KeyboardEnhancements;
use crate::mouse::MouseMode;

/// What a model shows: Bubble Tea's `tea.View`. Make one from a string
/// with `.into()` or [`View::new`], and set the rest with the `with_`
/// methods. Everything is declared per frame: after each frame the
/// program changes whatever differs from the frame before, and puts the
/// terminal back when it ends.
///
/// A view is drawn inline, below the shell prompt, unless it sets
/// [`alt_screen`](View::alt_screen), as in Bubble Tea v2.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::TestProgram;
///
/// struct Title;
///
/// impl tea::Model for Title {
///     type Msg = ();
///
///     fn update(&mut self, _: tea::Msg<()>) -> tea::Cmd<()> {
///         tea::Cmd::none()
///     }
///
///     fn view(&self) -> tea::View {
///         tea::View::new("hello")
///             .with_alt_screen(true)
///             .with_window_title("demo")
///     }
/// }
///
/// let test = TestProgram::new(Title, 20, 2);
/// assert_eq!(test.screen().text(), "hello");
/// assert!(test.screen().alt_screen());
/// assert_eq!(test.screen().title(), "demo");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct View {
    /// The text to draw, one line per row; ANSI styles (SGR) and
    /// hyperlinks (OSC 8) are kept, other escape sequences dropped, and
    /// tabs expanded to the next multiple of 8 columns.
    pub content: String,
    /// Draw on the alternate screen, the whole window, instead of
    /// inline. Bubble Tea's `AltScreen`.
    pub alt_screen: bool,
    /// Where to show the cursor, relative to the frame's top left, and
    /// how; `None` hides it. Bubble Tea's `Cursor`.
    pub cursor: Option<Cursor>,
    /// The window title (OSC 2); empty leaves it alone. The title the
    /// terminal had is put back at the end where the terminal keeps a
    /// title stack. Bubble Tea's `WindowTitle`.
    pub window_title: String,
    /// The terminal's default text colour (OSC 10); `None` keeps or
    /// resets the terminal's own. Bubble Tea's `ForegroundColor`.
    pub foreground_color: Option<Rgb>,
    /// The terminal's default background colour (OSC 11); `None` keeps or
    /// resets the terminal's own. Bubble Tea's `BackgroundColor`.
    pub background_color: Option<Rgb>,
    /// A progress bar in the terminal's tab or title bar (OSC 9;4), where
    /// the terminal has one. Bubble Tea's `ProgressBar`.
    pub progress_bar: Option<ProgressBar>,
    /// Which mouse events to receive; none by default.
    pub mouse_mode: MouseMode,
    /// Whether to receive [`Msg::Focus`](crate::Msg::Focus) and
    /// [`Msg::Blur`](crate::Msg::Blur).
    pub report_focus: bool,
    /// Turns bracketed paste off, so pasted text arrives as keys. It is
    /// on by default, as in Bubble Tea.
    pub disable_bracketed_paste_mode: bool,
    /// Keyboard enhancements to ask for. Key disambiguation (the kitty
    /// protocol's first flag) is always asked for, as in Bubble Tea.
    pub keyboard_enhancements: KeyboardEnhancements,
}

impl View {
    /// A view of `content`. Bubble Tea's `tea.NewView`.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    /// Replaces the content. Bubble Tea's `View.SetContent`.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    /// The same view on the alternate screen (`true`) or inline.
    #[must_use]
    pub fn with_alt_screen(mut self, on: bool) -> Self {
        self.alt_screen = on;
        self
    }

    /// The same view with the cursor shown as `cursor`.
    #[must_use]
    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// The same view with this window title.
    #[must_use]
    pub fn with_window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = title.into();
        self
    }

    /// The same view with this default text colour.
    #[must_use]
    pub fn with_foreground_color(mut self, color: Rgb) -> Self {
        self.foreground_color = Some(color);
        self
    }

    /// The same view with this default background colour.
    #[must_use]
    pub fn with_background_color(mut self, color: Rgb) -> Self {
        self.background_color = Some(color);
        self
    }

    /// The same view with this progress bar.
    #[must_use]
    pub fn with_progress_bar(mut self, bar: ProgressBar) -> Self {
        self.progress_bar = Some(bar);
        self
    }

    /// The same view with mouse events of `mode`.
    #[must_use]
    pub fn with_mouse_mode(mut self, mode: MouseMode) -> Self {
        self.mouse_mode = mode;
        self
    }

    /// The same view with focus reports on or off.
    #[must_use]
    pub fn with_report_focus(mut self, on: bool) -> Self {
        self.report_focus = on;
        self
    }

    /// The same view with bracketed paste on (the default) or off.
    #[must_use]
    pub fn with_bracketed_paste(mut self, on: bool) -> Self {
        self.disable_bracketed_paste_mode = !on;
        self
    }

    /// The same view with these keyboard enhancements.
    #[must_use]
    pub fn with_keyboard_enhancements(mut self, enhancements: KeyboardEnhancements) -> Self {
        self.keyboard_enhancements = enhancements;
        self
    }
}

impl From<String> for View {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl From<&str> for View {
    fn from(content: &str) -> Self {
        Self::new(content)
    }
}

/// The cursor's shape. Bubble Tea's `CursorShape`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::{Cursor, CursorShape};
///
/// assert_eq!(Cursor::new(0, 0).with_shape(CursorShape::Bar).style(), 5);
/// assert_eq!(
///     Cursor::new(0, 0).with_shape(CursorShape::Underline).style(),
///     3
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CursorShape {
    /// A block.
    #[default]
    Block,
    /// An underline.
    Underline,
    /// A vertical bar.
    Bar,
}

/// The cursor: where it is, relative to the frame's top left, and how
/// it looks. Bubble Tea's `tea.Cursor`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::testing::TestProgram;
/// use tea::{Cursor, CursorShape};
///
/// struct Prompt;
///
/// impl tea::Model for Prompt {
///     type Msg = ();
///
///     fn update(&mut self, _: tea::Msg<()>) -> tea::Cmd<()> {
///         tea::Cmd::none()
///     }
///
///     fn view(&self) -> tea::View {
///         let cursor = Cursor::new(2, 0)
///             .with_shape(CursorShape::Bar)
///             .with_blink(false);
///         tea::View::new("> ").with_cursor(cursor)
///     }
/// }
///
/// let test = TestProgram::new(Prompt, 20, 2);
/// assert_eq!(test.screen().cursor(), (2, 0));
/// assert!(test.screen().cursor_visible());
/// assert_eq!(test.screen().cursor_style(), 6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cursor {
    /// The column, from 0.
    pub x: u16,
    /// The row, from 0.
    pub y: u16,
    /// Its shape.
    pub shape: CursorShape,
    /// Whether it blinks.
    pub blink: bool,
    /// Its colour (OSC 12); `None` keeps the terminal's.
    pub color: Option<Rgb>,
}

impl Cursor {
    /// A blinking block at column `x` of row `y`, in the terminal's
    /// colour. Bubble Tea's `tea.NewCursor`.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            shape: CursorShape::Block,
            blink: true,
            color: None,
        }
    }

    /// The same cursor with this shape.
    #[must_use]
    pub const fn with_shape(mut self, shape: CursorShape) -> Self {
        self.shape = shape;
        self
    }

    /// The same cursor, blinking or steady.
    #[must_use]
    pub const fn with_blink(mut self, blink: bool) -> Self {
        self.blink = blink;
        self
    }

    /// The same cursor in this colour.
    #[must_use]
    pub const fn with_color(mut self, color: Rgb) -> Self {
        self.color = Some(color);
        self
    }

    /// The DECSCUSR value for the shape: 1 and 2 a blinking or steady
    /// block, 3 and 4 an underline, 5 and 6 a bar. Go's
    /// `encodeCursorStyle`.
    #[must_use]
    pub const fn style(&self) -> u8 {
        let base = match self.shape {
            CursorShape::Block => 1,
            CursorShape::Underline => 3,
            CursorShape::Bar => 5,
        };
        if self.blink { base } else { base + 1 }
    }
}

/// What a progress bar shows. Bubble Tea's `ProgressBarState`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::{ProgressBar, ProgressBarState, TerminalMode};
///
/// let busy = ProgressBar::new(ProgressBarState::Indeterminate, 0);
/// assert_eq!(
///     TerminalMode::ProgressBar(busy).sequence(true),
///     b"\x1b]9;4;3\x07"
/// );
/// assert_eq!(ProgressBarState::default(), ProgressBarState::None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProgressBarState {
    /// No progress bar.
    #[default]
    None,
    /// Progress, as a percentage.
    Default,
    /// An error, at a percentage.
    Error,
    /// Busy, with no percentage.
    Indeterminate,
    /// A warning, at a percentage.
    Warning,
}

/// A progress bar the terminal shows in its tab or title bar (OSC 9;4),
/// where it has one. Bubble Tea's `tea.ProgressBar`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::{ProgressBar, ProgressBarState, TerminalMode};
///
/// let bar = ProgressBar::new(ProgressBarState::Default, 150);
/// assert_eq!(bar.value, 100);
/// assert_eq!(
///     TerminalMode::ProgressBar(bar).sequence(true),
///     b"\x1b]9;4;1;100\x07"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProgressBar {
    /// What it shows.
    pub state: ProgressBarState,
    /// The percentage, 0 to 100.
    pub value: u8,
}

impl ProgressBar {
    /// A bar in `state` at `value` percent, cut to 100. Bubble Tea's
    /// `tea.NewProgressBar`.
    #[must_use]
    pub fn new(state: ProgressBarState, value: u8) -> Self {
        Self {
            state,
            value: value.min(100),
        }
    }

    /// The OSC 9;4 sequence that shows the bar.
    pub(crate) fn sequence(self) -> String {
        let v = self.value.min(100);
        match self.state {
            ProgressBarState::None => "\x1b]9;4;0\x07".to_string(),
            ProgressBarState::Default => format!("\x1b]9;4;1;{v}\x07"),
            ProgressBarState::Error => format!("\x1b]9;4;2;{v}\x07"),
            ProgressBarState::Indeterminate => "\x1b]9;4;3\x07".to_string(),
            ProgressBarState::Warning => format!("\x1b]9;4;4;{v}\x07"),
        }
    }
}

#[cfg(test)]
mod tests;
