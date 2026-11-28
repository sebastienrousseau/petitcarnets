//! The two traits a list is built on: `Item`, what it holds, and
//! `ItemDelegate`, how each is drawn.

use super::{Model, Msg};
use crate::bubbles::key::Binding;
use crate::{Cmd, Msg as TeaMsg};

/// Something a list holds: Bubbles' `list.Item`.
///
/// ```
/// use petitcarnets::bubbles::list::Item;
///
/// struct Task(String);
///
/// impl Item for Task {
///     fn filter_value(&self) -> String {
///         self.0.clone()
///     }
/// }
///
/// assert_eq!(Task("write docs".into()).filter_value(), "write docs");
/// ```
pub trait Item {
    /// The text the filter matches against.
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
    /// // The filter matches against each fruit's name.
    /// let mut l = fruits();
    /// l.set_filter_text("ch");
    /// assert_eq!(l.visible_items().len(), 1);
    /// assert_eq!(l.selected_item().map(|f| f.0), Some("cherry"));
    /// ```
    fn filter_value(&self) -> String;
}

/// Draws and updates a list's items: Bubbles' `list.ItemDelegate`. The
/// list holds its delegate; it is cloned for [`ItemDelegate::update`],
/// which gets the list to change.
///
/// ```
/// use petitcarnets::bubbles::list::{self, Item, ItemDelegate};
/// use petitcarnets::style::strip_ansi;
///
/// struct Fruit(&'static str);
/// # impl Item for Fruit {
/// #     fn filter_value(&self) -> String {
/// #         self.0.to_string()
/// #     }
/// # }
///
/// #[derive(Clone)]
/// struct Plain;
///
/// impl ItemDelegate<Fruit> for Plain {
///     fn render(&self, _list: &list::Model<Fruit, Self>, _index: usize, item: &Fruit) -> String {
///         item.0.to_string()
///     }
///     fn height(&self) -> usize {
///         1
///     }
///     fn spacing(&self) -> usize {
///         0
///     }
/// }
///
/// let l = list::Model::new(vec![Fruit("apple"), Fruit("pear")], Plain, 20, 10);
/// let view = strip_ansi(&l.view());
/// assert!(view.contains("\napple ") && view.contains("\npear "));
/// ```
pub trait ItemDelegate<I>: Clone {
    /// Item `index` (among the visible items), as text without a final
    /// line break, [`ItemDelegate::height`] lines high.
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
    /// use tea::bubbles::list::ItemDelegate;
    ///
    /// let l = fruits();
    /// let banana = &l.items()[1];
    /// let drawn = l.delegate().render(&l, 1, banana);
    /// assert_eq!(
    ///     strip_ansi(&drawn),
    ///     "  banana\n  ",
    ///     "a title and an empty description"
    /// );
    /// ```
    fn render(&self, list: &Model<I, Self>, index: usize, item: &I) -> String;
    /// The lines each item takes.
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
    /// use tea::bubbles::list::ItemDelegate;
    ///
    /// let mut d = list::DefaultDelegate::new();
    /// assert_eq!(ItemDelegate::<Fruit>::height(&d), 2);
    /// d.show_description = false;
    /// assert_eq!(ItemDelegate::<Fruit>::height(&d), 1);
    /// ```
    fn height(&self) -> usize;
    /// The blank lines between items.
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
    /// use tea::bubbles::list::ItemDelegate;
    ///
    /// let mut d = list::DefaultDelegate::new();
    /// assert_eq!(ItemDelegate::<Fruit>::spacing(&d), 1);
    /// d.set_spacing(0);
    /// assert_eq!(ItemDelegate::<Fruit>::spacing(&d), 0);
    /// ```
    fn spacing(&self) -> usize;
    /// Reacts to a message after the list has; nothing by default.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::list::{self, Item, ItemDelegate};
    ///
    /// # struct Fruit(&'static str);
    /// # impl Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// # #[derive(Clone)]
    /// # struct Deleter;
    /// // A delegate that deletes the selected item on `x`.
    /// impl ItemDelegate<Fruit> for Deleter {
    ///     fn update(
    ///         &self,
    ///         msg: &tea::Msg<list::Msg>,
    ///         l: &mut list::Model<Fruit, Self>,
    ///     ) -> tea::Cmd<list::Msg> {
    ///         if let tea::Msg::KeyPress(k) = msg {
    ///             if k.matches("x") {
    ///                 l.remove_item(l.global_index());
    ///             }
    ///         }
    ///         tea::Cmd::none()
    ///     }
    /// #     fn render(&self, _: &list::Model<Fruit, Self>, _: usize, item: &Fruit) -> String {
    /// #         item.0.to_string()
    /// #     }
    /// #     fn height(&self) -> usize {
    /// #         1
    /// #     }
    /// #     fn spacing(&self) -> usize {
    /// #         0
    /// #     }
    /// }
    ///
    /// let mut l = list::Model::new(vec![Fruit("apple"), Fruit("pear")], Deleter, 20, 10);
    /// let _ = l.update(&tea::Msg::KeyPress(tea::Key::char('x')));
    /// assert_eq!(l.items()[0].0, "pear");
    /// ```
    fn update(&self, _msg: &TeaMsg<Msg>, _list: &mut Model<I, Self>) -> Cmd<Msg> {
        Cmd::none()
    }
    /// More bindings for the short help.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::key::Binding;
    /// use tea::bubbles::list::{self, Item, ItemDelegate};
    /// use tea::style::strip_ansi;
    ///
    /// # struct Fruit(&'static str);
    /// # impl Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// # #[derive(Clone)]
    /// # struct Deleter;
    /// impl ItemDelegate<Fruit> for Deleter {
    ///     fn short_help(&self) -> Vec<Binding> {
    ///         vec![Binding::new(&["x"]).with_help("x", "delete")]
    ///     }
    /// #     fn render(&self, _: &list::Model<Fruit, Self>, _: usize, item: &Fruit) -> String {
    /// #         item.0.to_string()
    /// #     }
    /// #     fn height(&self) -> usize {
    /// #         1
    /// #     }
    /// #     fn spacing(&self) -> usize {
    /// #         0
    /// #     }
    /// }
    ///
    /// let l = list::Model::new(vec![Fruit("apple")], Deleter, 60, 10);
    /// assert!(strip_ansi(&l.view()).contains("x delete"));
    /// ```
    fn short_help(&self) -> Vec<Binding> {
        Vec::new()
    }
    /// More bindings for the full help.
    ///
    /// ```
    /// use petitcarnets as tea;
    /// use tea::bubbles::help::KeyMap as _;
    /// use tea::bubbles::key::Binding;
    /// use tea::bubbles::list::{self, Item, ItemDelegate};
    ///
    /// # struct Fruit(&'static str);
    /// # impl Item for Fruit {
    /// #     fn filter_value(&self) -> String {
    /// #         self.0.to_string()
    /// #     }
    /// # }
    /// # #[derive(Clone)]
    /// # struct Deleter;
    /// impl ItemDelegate<Fruit> for Deleter {
    ///     fn full_help(&self) -> Vec<Vec<Binding>> {
    ///         vec![vec![Binding::new(&["x"]).with_help("x", "delete")]]
    ///     }
    /// #     fn render(&self, _: &list::Model<Fruit, Self>, _: usize, item: &Fruit) -> String {
    /// #         item.0.to_string()
    /// #     }
    /// #     fn height(&self) -> usize {
    /// #         1
    /// #     }
    /// #     fn spacing(&self) -> usize {
    /// #         0
    /// #     }
    /// }
    ///
    /// let l = list::Model::new(vec![Fruit("apple")], Deleter, 60, 10);
    /// assert_eq!(l.full_help()[1][0].keys(), ["x"], "after the cursor keys");
    /// ```
    fn full_help(&self) -> Vec<Vec<Binding>> {
        Vec::new()
    }
}
