//! The spinner, turning off the quit bindings, and status messages.

use super::{DISABLE_QUIT_KEYBINDINGS, Item, ItemDelegate, Model, Msg, SHOW_SPINNER, spinner};
use crate::Cmd;

impl<I: Item, D: ItemDelegate<I>> Model<I, D> {
    /// Replaces the spinner's frames.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// use tea::bubbles::spinner::Spinner;
    ///
    /// let mut l = fruits();
    /// l.set_spinner(Spinner::MOON);
    /// let _ticks = l.start_spinner();
    /// assert!(l.view().contains("🌑"));
    /// ```
    pub const fn set_spinner(&mut self, spinner: spinner::Spinner) {
        self.spinner.spinner = spinner;
    }

    /// Starts the spinner if it is stopped, else stops it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// assert!(!l.toggle_spinner().is_none(), "started: a tick");
    /// assert!(strip_ansi(&l.view()).starts_with("|  List"));
    /// assert!(l.toggle_spinner().is_none(), "stopped");
    /// assert!(!strip_ansi(&l.view()).starts_with("|  List"));
    /// ```
    pub fn toggle_spinner(&mut self) -> Cmd<Msg> {
        if self.flags.get(SHOW_SPINNER) {
            self.stop_spinner();
            Cmd::none()
        } else {
            self.start_spinner()
        }
    }

    /// Shows the spinner; the command starts it.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// let _ticks = l.start_spinner();
    /// assert!(strip_ansi(&l.view()).starts_with("|  List"));
    /// ```
    pub fn start_spinner(&mut self) -> Cmd<Msg> {
        self.flags.set(SHOW_SPINNER, true);
        self.spinner.tick().map(Msg::Spinner)
    }

    /// Hides the spinner.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// let _ticks = l.start_spinner();
    /// l.stop_spinner();
    /// assert!(!strip_ansi(&l.view()).starts_with("|  List"));
    /// ```
    pub const fn stop_spinner(&mut self) {
        self.flags.set(SHOW_SPINNER, false);
    }

    /// Turns the quit keys off.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let key = |c| tea::Msg::KeyPress(tea::Key::char(c));
    /// let mut l = fruits();
    /// assert!(!l.update(&key('q')).is_none(), "quits");
    /// l.disable_quit_keybindings();
    /// assert!(l.update(&key('q')).is_none());
    /// assert!(!l.key_map.quit.enabled());
    /// ```
    pub const fn disable_quit_keybindings(&mut self) {
        self.flags.set(DISABLE_QUIT_KEYBINDINGS, true);
        self.key_map.quit.set_enabled(false);
        self.key_map.force_quit.set_enabled(false);
    }

    /// Shows `message` by the title for [`Model::status_message_lifetime`].
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list;
    /// use tea::style::strip_ansi;
    /// # struct Fruit(&'static str);
    /// #
    /// # impl list::Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// #
    /// # impl list::DefaultItem for Fruit {
    /// #     fn title(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// #     fn description(&self) -> String {
    /// #         String::new()
    /// #     }
    /// # }
    /// #
    /// # fn fruits() -> list::Model<Fruit, list::DefaultDelegate> {
    /// #     let items = vec![Fruit("apple"), Fruit("banana"), Fruit("cherry")];
    /// #     list::Model::new(items, list::DefaultDelegate::new(), 30, 14)
    /// # }
    ///
    /// let mut l = fruits();
    /// let timeout = l.new_status_message("Saved");
    /// assert!(!timeout.is_none(), "it goes after a second");
    /// assert!(strip_ansi(&l.view()).contains("Saved"));
    /// ```
    pub fn new_status_message(&mut self, message: &str) -> Cmd<Msg> {
        message.clone_into(&mut self.status_message);
        self.status_tag += 1;
        let tag = self.status_tag;
        crate::tick(self.status_message_lifetime, move |_| {
            Msg::StatusTimeout(tag)
        })
    }
}
