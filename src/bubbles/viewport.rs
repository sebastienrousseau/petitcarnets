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

mod render;

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

/// The keys of a viewport: Bubbles' `viewport.KeyMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// A page down.
    pub page_down: Binding,
    /// A page up.
    pub page_up: Binding,
    /// Half a page up.
    pub half_page_up: Binding,
    /// Half a page down.
    pub half_page_down: Binding,
    /// A line down.
    pub down: Binding,
    /// A line up.
    pub up: Binding,
    /// A step left.
    pub left: Binding,
    /// A step right.
    pub right: Binding,
}

impl Default for KeyMap {
    /// Bubbles v2's `viewport.DefaultKeyMap`.
    fn default() -> Self {
        let b = |keys: &[&str], key: &str, desc: &str| Binding::new(keys).with_help(key, desc);
        Self {
            page_down: b(&["pgdown", "space", "f"], "f/pgdn", "page down"),
            page_up: b(&["pgup", "b"], "b/pgup", "page up"),
            half_page_up: b(&["u", "ctrl+u"], "u", "½ page up"),
            half_page_down: b(&["d", "ctrl+d"], "d", "½ page down"),
            up: b(&["up", "k"], "↑/k", "up"),
            down: b(&["down", "j"], "↓/j", "down"),
            left: b(&["left", "h"], "←/h", "move left"),
            right: b(&["right", "l"], "→/l", "move right"),
        }
    }
}

impl KeyMap {
    /// No bindings at all, for a viewport driven by its owner (the text
    /// area's).
    #[must_use]
    pub fn none() -> Self {
        let off = Binding::default;
        Self {
            page_down: off(),
            page_up: off(),
            half_page_up: off(),
            half_page_down: off(),
            down: off(),
            up: off(),
            left: off(),
            right: off(),
        }
    }
}

/// A viewport: Bubbles' `viewport.Model`.
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
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the width.
    pub const fn set_width(&mut self, columns: usize) {
        self.width = columns;
    }

    /// The height.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Sets the height.
    pub const fn set_height(&mut self, rows: usize) {
        self.height = rows;
    }

    /// Whether the top shows.
    #[must_use]
    pub const fn at_top(&self) -> bool {
        self.y_offset == 0
    }

    /// Whether the bottom shows.
    #[must_use]
    pub fn at_bottom(&self) -> bool {
        self.y_offset >= self.max_y_offset()
    }

    /// Whether it scrolled past the bottom (content got shorter).
    #[must_use]
    pub fn past_bottom(&self) -> bool {
        self.y_offset > self.max_y_offset()
    }

    /// How far down it has scrolled, from 0.0 to 1.0.
    #[must_use]
    pub fn scroll_percent(&self) -> f64 {
        let most = self.max_y_offset();
        if most == 0 {
            return 1.0;
        }
        (crate::style::to_f64(self.y_offset) / crate::style::to_f64(most)).clamp(0.0, 1.0)
    }

    /// How far right it has scrolled, from 0.0 to 1.0.
    #[must_use]
    pub fn horizontal_scroll_percent(&self) -> f64 {
        if self.x_offset + self.width >= self.longest {
            return 1.0;
        }
        let f = crate::style::to_f64;
        (f(self.x_offset) / (f(self.longest) - f(self.width))).clamp(0.0, 1.0)
    }

    /// Replaces the content; lines end at `\n` (or `\r\n`).
    pub fn set_content(&mut self, content: &str) {
        let lines: Vec<String> = content
            .replace("\r\n", "\n")
            .split('\n')
            .map(String::from)
            .collect();
        self.set_content_lines(lines);
    }

    /// Replaces the content with these lines.
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
    #[must_use]
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// The first line showing.
    #[must_use]
    pub const fn y_offset(&self) -> usize {
        self.y_offset
    }

    /// Scrolls so line `n` is at the top, as far as the content goes.
    pub fn set_y_offset(&mut self, n: usize) {
        self.y_offset = n.min(self.max_y_offset());
    }

    /// The first column showing.
    #[must_use]
    pub const fn x_offset(&self) -> usize {
        self.x_offset
    }

    /// Scrolls sideways to column `n`; nothing when soft wrapping.
    pub fn set_x_offset(&mut self, n: usize) {
        if !self.soft_wrap {
            self.x_offset = n.min(self.longest.saturating_sub(self.width));
        }
    }

    /// How far a step sideways goes.
    pub const fn set_horizontal_step(&mut self, n: usize) {
        self.horizontal_step = n;
    }

    /// A page down.
    pub fn page_down(&mut self) {
        if !self.at_bottom() {
            self.scroll_down(self.inner_height());
        }
    }

    /// A page up.
    pub fn page_up(&mut self) {
        if !self.at_top() {
            self.scroll_up(self.inner_height());
        }
    }

    /// Half a page down.
    pub fn half_page_down(&mut self) {
        if !self.at_bottom() {
            self.scroll_down(self.inner_height() / 2);
        }
    }

    /// Half a page up.
    pub fn half_page_up(&mut self) {
        if !self.at_top() {
            self.scroll_up(self.inner_height() / 2);
        }
    }

    /// `n` lines down.
    pub fn scroll_down(&mut self, n: usize) {
        if self.at_bottom() || n == 0 || self.lines.is_empty() {
            return;
        }
        self.set_y_offset(self.y_offset + n);
    }

    /// `n` lines up.
    pub fn scroll_up(&mut self, n: usize) {
        if self.at_top() || n == 0 || self.lines.is_empty() {
            return;
        }
        self.set_y_offset(self.y_offset.saturating_sub(n));
    }

    /// `n` columns left.
    pub fn scroll_left(&mut self, n: usize) {
        self.set_x_offset(self.x_offset.saturating_sub(n));
    }

    /// `n` columns right.
    pub fn scroll_right(&mut self, n: usize) {
        self.set_x_offset(self.x_offset + n);
    }

    /// To the top.
    pub const fn goto_top(&mut self) {
        self.y_offset = 0;
    }

    /// To the bottom.
    pub fn goto_bottom(&mut self) {
        self.y_offset = self.max_y_offset();
    }

    /// Lines the content takes, soft wrapped or not.
    #[must_use]
    pub fn total_line_count(&self) -> usize {
        self.locate(0).0
    }

    /// Lines showing.
    #[must_use]
    pub fn visible_line_count(&self) -> usize {
        self.visible_lines().len()
    }

    /// Scrolls on its keys and the mouse wheel.
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
