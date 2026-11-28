//! Focus, resetting, and the width, the height and the prompt.

use super::{FOCUS, MIN_HEIGHT, Model, Msg, PromptInfo, digits};
use crate::Cmd;
use crate::style::width;

impl Model {
    /// Whether it is focused: only a focused text area takes keys.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// assert!(!area.focused());
    /// let _ = area.focus();
    /// assert!(area.focused());
    /// ```
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.flags.get(FOCUS)
    }

    /// Focuses it; the command starts the cursor blinking.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let blink = area.focus();
    /// assert!(area.focused());
    /// assert!(!blink.is_none(), "the cursor starts blinking");
    /// ```
    pub fn focus(&mut self) -> Cmd<Msg> {
        self.flags.set(FOCUS, true);
        self.virtual_cursor.focus()
    }

    /// Blurs it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// let _ = area.focus();
    /// area.blur();
    /// let _ = area.update(&tea::Msg::KeyPress(tea::Key::char('a')));
    /// assert_eq!(area.value(), "", "a blurred text area takes no keys");
    /// ```
    pub const fn blur(&mut self) {
        self.flags.set(FOCUS, false);
        self.virtual_cursor.blur();
    }

    /// Empties it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_value("a\nb");
    /// area.reset();
    /// assert_eq!((area.value().as_str(), area.line_count()), ("", 1));
    /// ```
    pub fn reset(&mut self) {
        self.value = vec![Vec::new()];
        self.col = 0;
        self.row = 0;
        self.viewport.goto_top();
        self.set_cursor_column(0);
        self.recalculate_height();
        self.clear_selection();
    }

    /// The text's width, in cells, without prompt and line numbers.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.show_line_numbers = false;
    /// area.set_width(20);
    /// assert_eq!(area.width(), 18, "less the prompt");
    /// ```
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Sets the whole width, prompt, line numbers and frame included,
    /// within [`Model::max_width`].
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_width(20);
    /// assert_eq!(area.width(), 14, "less the prompt and the line numbers");
    /// area.set_width(1000);
    /// assert_eq!(area.width(), 494, "no more than max_width");
    /// ```
    pub fn set_width(&mut self, columns: usize) {
        if self.prompt_fn.is_none() {
            self.prompt_width = width(&self.prompt);
        }
        let outer = self.active_style().base.horizontal_frame();
        let mut inner = self.prompt_width;
        if self.show_line_numbers {
            inner += digits(self.max_height) + 2;
        }
        let least = inner + outer + 1;
        let mut input = columns.max(least);
        if self.max_width > 0 {
            input = input.min(self.max_width.max(least));
        }
        self.viewport.set_width(input - outer);
        self.width = input - outer - inner;
        self.recalculate_height();
    }

    /// A prompt drawn by `prompt` for each line, `columns` wide.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.show_line_numbers = false;
    /// area.set_prompt_func(3, |info| if info.focused { "> " } else { "  " }.into());
    /// area.set_width(20);
    /// assert_eq!(area.width(), 17, "less the three prompt columns");
    /// ```
    pub fn set_prompt_func(&mut self, columns: usize, prompt: fn(PromptInfo) -> String) {
        self.prompt_fn = Some(prompt);
        self.prompt_width = columns;
    }

    /// The height, in rows.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// assert_eq!(area.height(), 6);
    /// area.set_height(3);
    /// assert_eq!(area.height(), 3);
    /// ```
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Sets the height, within [`Model::max_height`].
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::textarea;
    ///
    /// let mut area = textarea::Model::new();
    /// area.set_height(0);
    /// assert_eq!(area.height(), 1, "at least one row");
    /// area.set_height(500);
    /// assert_eq!(area.height(), 99, "no more than max_height");
    /// ```
    pub fn set_height(&mut self, rows: usize) {
        self.height = if self.max_height > 0 {
            rows.clamp(MIN_HEIGHT, self.max_height)
        } else {
            rows.max(MIN_HEIGHT)
        };
        self.viewport.set_height(self.height);
        self.reposition_view();
    }
}
