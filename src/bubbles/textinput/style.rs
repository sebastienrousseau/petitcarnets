//! The styles a text input is drawn with, focused and blurred, and
//! its cursor's.

use crate::CursorShape;
use crate::style::{Color, Style};
use std::time::Duration;

/// The styles for one state, focused or blurred: Bubbles'
/// `textinput.StyleState`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// let styles = textinput::Styles::new(true);
/// assert_eq!(styles.focused.prompt, styles.blurred.prompt);
/// assert_ne!(
///     styles.focused.text, styles.blurred.text,
///     "blurred text is dimmed"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StyleState {
    /// The text.
    pub text: Style,
    /// The placeholder.
    pub placeholder: Style,
    /// The rest of a suggestion.
    pub suggestion: Style,
    /// The prompt.
    pub prompt: Style,
}

/// How the cursor looks: Bubbles' `textinput.CursorStyle`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// let cursor = textinput::Styles::new(true).cursor;
/// assert_eq!(cursor.shape, tea::CursorShape::Block);
/// assert!(cursor.blink);
/// assert!(cursor.blink_speed.is_zero(), "the cursor's own speed");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorStyle {
    /// Its colour. A real cursor (see [`Model::set_virtual_cursor`](super::Model::set_virtual_cursor))
    /// takes it only if it is [`Color::Rgb`].
    pub color: Color,
    /// The real cursor's shape.
    pub shape: CursorShape,
    /// Whether it blinks.
    pub blink: bool,
    /// How fast it blinks; zero keeps the cursor's default.
    pub blink_speed: Duration,
}

/// A text input's styles: Bubbles' `textinput.Styles`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::textinput;
///
/// let (dark, light) = (textinput::Styles::new(true), textinput::Styles::new(false));
/// assert_eq!(dark.focused, light.focused);
/// assert_ne!(dark.blurred.text, light.blurred.text);
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
    /// `textinput.DefaultStyles`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textinput;
    ///
    /// use tea::style::{Color, Style};
    ///
    /// let styles = textinput::Styles::new(true);
    /// assert_eq!(
    ///     styles.focused.placeholder,
    ///     Style::new().fg(Color::Palette(240))
    /// );
    /// assert_eq!(styles.blurred.text, Style::new().fg(Color::Ansi(7)));
    /// ```
    #[must_use]
    pub const fn new(is_dark: bool) -> Self {
        let subdued = Style::new().fg(Color::Palette(240));
        let prompt = Style::new().fg(Color::Ansi(7));
        let state = StyleState {
            text: Style::new(),
            placeholder: subdued,
            suggestion: subdued,
            prompt,
        };
        let blurred_text = if is_dark {
            Color::Ansi(7)
        } else {
            Color::Palette(245)
        };
        Self {
            focused: state,
            blurred: StyleState {
                text: Style::new().fg(blurred_text),
                ..state
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
