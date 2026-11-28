//! The text area's styles, Bubbles v2's defaults, and how each part's
//! style is worked out from the base.

use crate::CursorShape;
use crate::style::{Color, Style};
use std::time::Duration;

/// The styles for one state, focused or blurred: Bubbles'
/// `textarea.StyleState`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let styles = textarea::Styles::new(true);
/// assert_eq!(styles.focused.line_number, styles.blurred.line_number);
/// assert_ne!(styles.focused.cursor_line, styles.blurred.cursor_line);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StyleState {
    /// Around the whole text area; its padding and border take room.
    pub base: Style,
    /// The text.
    pub text: Style,
    /// Line numbers.
    pub line_number: Style,
    /// The cursor line's number.
    pub cursor_line_number: Style,
    /// The line the cursor is on.
    pub cursor_line: Style,
    /// The lines after the end.
    pub end_of_buffer: Style,
    /// The placeholder.
    pub placeholder: Style,
    /// The prompt.
    pub prompt: Style,
    /// Selected text.
    pub selection: Style,
}

impl StyleState {
    /// The cursor line's style over the base.
    pub(super) const fn cursor_line(&self) -> Style {
        self.cursor_line.plain().inherit(self.base.plain())
    }

    /// The cursor line number's style.
    pub(super) const fn cursor_line_number(&self) -> Style {
        self.cursor_line_number
            .plain()
            .inherit(self.cursor_line.plain())
            .inherit(self.base.plain())
    }

    /// The end of buffer's style.
    pub(super) const fn end_of_buffer(&self) -> Style {
        self.end_of_buffer.plain().inherit(self.base.plain())
    }

    /// The line numbers' style.
    pub(super) const fn line_number(&self) -> Style {
        self.line_number.plain().inherit(self.base.plain())
    }

    /// The placeholder's style.
    pub(super) const fn placeholder(&self) -> Style {
        self.placeholder.plain().inherit(self.base.plain())
    }

    /// The prompt's style.
    pub(super) const fn prompt(&self) -> Style {
        self.prompt.plain().inherit(self.base.plain())
    }

    /// The text's style.
    pub(super) const fn text(&self) -> Style {
        self.text.plain().inherit(self.base.plain())
    }

    /// The selection's style.
    pub(super) const fn selection(&self) -> Style {
        self.selection
            .plain()
            .inherit(self.text.plain())
            .inherit(self.base.plain())
    }
}

/// How the cursor looks: Bubbles' `textarea.CursorStyle`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let cursor = textarea::Styles::new(true).cursor;
/// assert_eq!(cursor.shape, tea::CursorShape::Block);
/// assert!(cursor.blink);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorStyle {
    /// Its colour; a real cursor takes it only if it is [`Color::Rgb`].
    pub color: Color,
    /// The real cursor's shape.
    pub shape: CursorShape,
    /// Whether it blinks.
    pub blink: bool,
    /// How fast it blinks; zero keeps the cursor's default.
    pub blink_speed: Duration,
}

/// A text area's styles: Bubbles' `textarea.Styles`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textarea;
///
/// let mut area = textarea::Model::new();
/// let mut styles = textarea::Styles::new(true);
/// styles.cursor.blink = false;
/// area.set_styles(styles);
/// assert!(!area.styles().cursor.blink);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Styles {
    /// When focused.
    pub focused: StyleState,
    /// When blurred.
    pub blurred: StyleState,
    /// The cursor.
    pub cursor: CursorStyle,
}

impl Styles {
    /// Bubbles' default styles for a dark or light background:
    /// `textarea.DefaultStyles`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// use tea::style::{Color, Style};
    ///
    /// let (dark, light) = (textarea::Styles::new(true), textarea::Styles::new(false));
    /// assert_eq!(dark.focused.cursor_line, Style::new().bg(Color::Ansi(0)));
    /// assert_eq!(
    ///     light.focused.cursor_line,
    ///     Style::new().bg(Color::Palette(255))
    /// );
    /// ```
    #[must_use]
    pub fn new(is_dark: bool) -> Self {
        let pick = |dark: u8, light: u8| if is_dark { color(dark) } else { color(light) };
        let fg = |c: Color| Style::new().fg(c);
        let common = StyleState {
            base: Style::new(),
            text: Style::new(),
            line_number: fg(pick(7, 249)),
            cursor_line_number: fg(color(240)),
            cursor_line: Style::new().bg(pick(0, 255)),
            end_of_buffer: fg(pick(0, 254)),
            placeholder: fg(color(240)),
            prompt: fg(color(7)),
            selection: Style::new().bg(pick(8, 253)),
        };
        Self {
            focused: common,
            blurred: StyleState {
                cursor_line: fg(pick(7, 245)),
                cursor_line_number: fg(pick(7, 249)),
                text: fg(pick(7, 245)),
                ..common
            },
            cursor: CursorStyle {
                color: Color::Ansi(7),
                shape: CursorShape::Block,
                blink: true,
                blink_speed: Duration::ZERO,
            },
        }
    }
}

/// Lip Gloss's `Color("n")`: one of the 16 colours under 16, else a
/// palette colour.
const fn color(n: u8) -> Color {
    if n < 16 {
        Color::Ansi(n)
    } else {
        Color::Palette(n)
    }
}
