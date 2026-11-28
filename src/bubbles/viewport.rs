//! A pane that shows part of some content and scrolls over it, by line,
//! half page and page, with the keys or the mouse wheel, sideways too,
//! and soft wrapped if asked: Bubbles' `viewport` package. Go's
//! deprecated high performance mode is not ported, nor its search
//! highlights.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::viewport;
//!
//! let mut v = viewport::Model::new(10, 2);
//! v.set_content("one\ntwo\nthree");
//! assert_eq!(v.view(), "one       \ntwo       ");
//! let _ = v.update::<()>(&tea::Msg::KeyPress(tea::Key::char('j')));
//! assert_eq!(v.view(), "two       \nthree     ");
//! assert!(v.at_bottom());
//! ```

mod keymap;
mod render;
mod scroll;

pub use keymap::KeyMap;

use super::key::Binding;
use crate::style::{Style, width};
use crate::{Cmd, KeyMod, MouseButton, Msg};

/// A binding of the key map.
type Field = fn(&KeyMap) -> &Binding;

/// What a binding does.
type Scroll = fn(&mut Model);

/// How far one step sideways scrolls: Bubbles' default.
const HORIZONTAL_STEP: usize = 6;

/// What a gutter function is told about a line: Bubbles'
/// `viewport.GutterContext`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::viewport;
///
/// let mut v = viewport::Model::new(6, 2);
/// v.left_gutter = Some(|g: viewport::GutterContext| format!("{} ", g.index + 1));
/// v.set_content("a\nb");
/// assert_eq!(v.view(), "1 a   \n2 b   ");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GutterContext {
    /// The line's index in the content (past the end for filler lines).
    pub index: usize,
    /// How many lines the content has.
    pub total_lines: usize,
    /// Whether this is the second or a later part of a soft wrapped line.
    pub soft: bool,
}

/// Draws the gutter left of a line: Bubbles' `viewport.GutterFunc`.
pub type GutterFn = fn(GutterContext) -> String;

/// A viewport: Bubbles' `viewport.Model`.
///
/// ```
/// use petitcarnets as tea;
/// use tea::bubbles::viewport;
///
/// let mut v = viewport::Model::new(3, 2);
/// v.soft_wrap = true;
/// v.set_content("abcdef");
/// assert_eq!(v.view(), "abc\ndef");
/// ```
#[derive(Debug, Clone)]
pub struct Model {
    width: usize,
    height: usize,
    /// The keys.
    pub key_map: KeyMap,
    /// Whether long lines wrap rather than scroll sideways.
    pub soft_wrap: bool,
    /// Whether short content is padded with blank lines to the height.
    pub fill_height: bool,
    /// Whether the mouse wheel scrolls.
    pub mouse_wheel_enabled: bool,
    /// How many lines a wheel turn scrolls.
    pub mouse_wheel_delta: usize,
    /// The row the viewport starts at on the screen, for the owner's use.
    pub y_position: usize,
    /// The style around the content (padding, border).
    pub style: Style,
    /// Draws a gutter left of each line.
    pub left_gutter: Option<GutterFn>,
    /// A style for each line, by index.
    pub style_line: Option<fn(usize) -> Style>,
    y_offset: usize,
    x_offset: usize,
    horizontal_step: usize,
    lines: Vec<String>,
    longest: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl Model {
    /// An empty viewport `width` by `height` cells: `viewport.New` with
    /// `WithWidth` and `WithHeight`.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let v = viewport::Model::new(10, 2);
    /// assert_eq!((v.width(), v.height()), (10, 2));
    /// assert_eq!(v.content(), "");
    /// ```
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            key_map: KeyMap::default(),
            soft_wrap: false,
            fill_height: false,
            mouse_wheel_enabled: true,
            mouse_wheel_delta: 3,
            y_position: 0,
            style: Style::new(),
            left_gutter: None,
            style_line: None,
            y_offset: 0,
            x_offset: 0,
            horizontal_step: HORIZONTAL_STEP,
            lines: Vec::new(),
            longest: 0,
        }
    }

    /// The width.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(10, 2);
    /// assert_eq!(v.width(), 10);
    /// v.set_width(4);
    /// assert_eq!(v.width(), 4);
    /// ```
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the width.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(10, 1);
    /// v.set_content("abcdef");
    /// v.set_width(3);
    /// assert_eq!(v.view(), "abc");
    /// ```
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// The height.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(10, 2);
    /// assert_eq!(v.height(), 2);
    /// v.set_height(5);
    /// assert_eq!(v.height(), 5);
    /// ```
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Sets the height.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 1);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.set_height(3);
    /// assert_eq!(v.visible_line_count(), 3);
    /// ```
    pub const fn set_height(&mut self, rows: usize) {
        self.height = rows;
    }

    /// Whether the top shows.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// assert!(v.at_top());
    /// v.scroll_down(1);
    /// assert!(!v.at_top());
    /// ```
    #[must_use]
    pub const fn at_top(&self) -> bool {
        self.y_offset == 0
    }

    /// Whether the bottom shows.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// assert!(!v.at_bottom());
    /// v.goto_bottom();
    /// assert!(v.at_bottom());
    /// ```
    #[must_use]
    pub fn at_bottom(&self) -> bool {
        self.y_offset >= self.max_y_offset()
    }

    /// Whether it scrolled past the bottom (content got shorter).
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// v.goto_bottom();
    /// assert!(!v.past_bottom());
    /// v.set_height(5);
    /// assert!(v.past_bottom(), "taller now, so scrolled too far");
    /// ```
    #[must_use]
    pub fn past_bottom(&self) -> bool {
        self.y_offset > self.max_y_offset()
    }

    /// How far down it has scrolled, from 0.0 to 1.0.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 3);
    /// v.set_content("1\n2\n3\n4\n5");
    /// assert_eq!(v.scroll_percent(), 0.0);
    /// v.scroll_down(1);
    /// assert_eq!(v.scroll_percent(), 0.5);
    /// ```
    #[must_use]
    pub fn scroll_percent(&self) -> f64 {
        let most = self.max_y_offset();
        if most == 0 {
            return 1.0;
        }
        (crate::style::to_f64(self.y_offset) / crate::style::to_f64(most)).clamp(0.0, 1.0)
    }

    /// How far right it has scrolled, from 0.0 to 1.0.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(2, 1);
    /// v.set_content("abcdef");
    /// assert_eq!(v.horizontal_scroll_percent(), 0.0);
    /// v.set_x_offset(2);
    /// assert_eq!(v.horizontal_scroll_percent(), 0.5);
    /// ```
    #[must_use]
    pub fn horizontal_scroll_percent(&self) -> f64 {
        if self.x_offset + self.width >= self.longest {
            return 1.0;
        }
        let f = crate::style::to_f64;
        (f(self.x_offset) / (f(self.longest) - f(self.width))).clamp(0.0, 1.0)
    }

    /// Replaces the content; lines end at `\n` (or `\r\n`).
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(3, 2);
    /// v.set_content("a\r\nb");
    /// assert_eq!(v.content(), "a\nb");
    /// assert_eq!(v.view(), "a  \nb  ");
    /// ```
    pub fn set_content(&mut self, content: &str) {
        let lines: Vec<String> = content
            .replace("\r\n", "\n")
            .split('\n')
            .map(String::from)
            .collect();
        self.set_content_lines(lines);
    }

    /// Replaces the content with these lines.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(3, 2);
    /// v.set_content_lines(vec!["x".into(), "y".into()]);
    /// assert_eq!(v.view(), "x  \ny  ");
    /// ```
    pub fn set_content_lines(&mut self, lines: Vec<String>) {
        self.lines = if lines.len() == 1 && width(&lines[0]) == 0 {
            Vec::new()
        } else {
            lines
        };
        self.longest = self.lines.iter().map(|l| width(l)).max().unwrap_or(0);
        if self.y_offset > self.max_y_offset() {
            self.goto_bottom();
        }
    }

    /// The content.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(3, 1);
    /// v.set_content_lines(vec!["x".into(), "y".into()]);
    /// assert_eq!(v.content(), "x\ny");
    /// ```
    #[must_use]
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Lines the content takes, soft wrapped or not.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(3, 1);
    /// v.set_content("abcdef\ng");
    /// assert_eq!(v.total_line_count(), 2);
    /// v.soft_wrap = true;
    /// assert_eq!(v.total_line_count(), 3, "the long line wraps in two");
    /// ```
    #[must_use]
    pub fn total_line_count(&self) -> usize {
        self.locate(0).0
    }

    /// Lines showing.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// assert_eq!(v.visible_line_count(), 2);
    /// v.set_content("1");
    /// assert_eq!(v.visible_line_count(), 1);
    /// ```
    #[must_use]
    pub fn visible_line_count(&self) -> usize {
        self.visible_lines().len()
    }

    /// Scrolls on its keys and the mouse wheel.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::viewport;
    ///
    /// let mut v = viewport::Model::new(1, 2);
    /// v.set_content("1\n2\n3\n4\n5");
    /// let _ = v.update::<()>(&tea::Msg::KeyPress(tea::Key::char('j')));
    /// assert_eq!(v.y_offset(), 1, "j is a line down");
    /// let _ = v.update::<()>(&tea::Msg::KeyPress(tea::Key::char('k')));
    /// assert_eq!(v.y_offset(), 0);
    /// ```
    pub fn update<M>(&mut self, msg: &Msg<M>) -> Cmd<M> {
        match msg {
            Msg::KeyPress(key) => self.press(key),
            Msg::MouseWheel(mouse) if self.mouse_wheel_enabled => {
                let shift = mouse.mods.contains(KeyMod::SHIFT);
                self.wheel(mouse.button, shift);
            }
            _ => {}
        }
        Cmd::none()
    }

    /// What each key does, in the order Bubbles tries them.
    const KEYS: [(Field, Scroll); 8] = [
        (|k| &k.page_down, Self::page_down),
        (|k| &k.page_up, Self::page_up),
        (|k| &k.half_page_down, Self::half_page_down),
        (|k| &k.half_page_up, Self::half_page_up),
        (|k| &k.down, |m| m.scroll_down(1)),
        (|k| &k.up, |m| m.scroll_up(1)),
        (|k| &k.left, |m| m.scroll_left(m.horizontal_step)),
        (|k| &k.right, |m| m.scroll_right(m.horizontal_step)),
    ];

    /// A key.
    fn press(&mut self, key: &crate::Key) {
        if let Some((_, act)) = Self::KEYS
            .iter()
            .find(|(b, _)| b(&self.key_map).matches(key))
        {
            act(self);
        }
    }

    /// A wheel turn; Shift turns up and down into left and right.
    fn wheel(&mut self, button: MouseButton, shift: bool) {
        match (button, shift) {
            (MouseButton::WheelDown, false) => self.scroll_down(self.mouse_wheel_delta),
            (MouseButton::WheelUp, false) => self.scroll_up(self.mouse_wheel_delta),
            (MouseButton::WheelDown | MouseButton::WheelRight, _) => {
                self.scroll_right(self.horizontal_step);
            }
            (MouseButton::WheelUp | MouseButton::WheelLeft, _) => {
                self.scroll_left(self.horizontal_step);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
