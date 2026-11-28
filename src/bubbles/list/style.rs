//! The list's styles, Bubbles v2's defaults.

use crate::bubbles::textinput;
use crate::style::{Border, Color, Style};

/// The list's styles: Bubbles' `list.Styles`.
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
/// assert!(
///     l.view().starts_with("  \x1b[48;5;62m "),
///     "the title on a purple bar"
/// );
/// l.styles.title = tea::style::Style::new();
/// assert!(l.view().starts_with("  List "));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Styles {
    /// The title bar.
    pub title_bar: Style,
    /// The title.
    pub title: Style,
    /// The spinner.
    pub spinner: Style,
    /// The filter input.
    pub filter: textinput::Styles,
    /// A matched character, for delegates that want it.
    pub default_filter_character_match: Style,
    /// The status bar.
    pub status_bar: Style,
    /// The status bar when there are no items.
    pub status_empty: Style,
    /// The filter in the status bar.
    pub status_bar_active_filter: Style,
    /// The count of filtered out items.
    pub status_bar_filter_count: Style,
    /// "No items."
    pub no_items: Style,
    /// Around the pagination.
    pub pagination_style: Style,
    /// Around the help.
    pub help_style: Style,
    /// The page showing's dot.
    pub active_pagination_dot: Style,
    /// The other pages' dots.
    pub inactive_pagination_dot: Style,
    /// Page numbers, when the dots do not fit.
    pub arabic_pagination: Style,
    /// Between the status and the filtered count.
    pub divider_dot: Style,
}

/// `#rrggbb`.
pub(super) const fn hex(rgb: u32) -> Color {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color::Rgb(r, g, b)
}

/// The dark colour if `is_dark`, else the light one.
pub(super) const fn pick(is_dark: bool, dark: u32, light: u32) -> Color {
    if is_dark { hex(dark) } else { hex(light) }
}

impl Styles {
    /// Bubbles' default styles for a dark or light background:
    /// `list.DefaultStyles`.
    ///
    /// ```
    /// use petitcarnets::bubbles::list::Styles;
    ///
    /// let (dark, light) = (Styles::new(true), Styles::new(false));
    /// assert_eq!(
    ///     dark.status_bar.paint("3 items"),
    ///     "\x1b[38;2;119;119;119m3 items\x1b[0m"
    /// );
    /// assert_ne!(dark.status_bar, light.status_bar);
    /// ```
    #[must_use]
    pub fn new(is_dark: bool) -> Self {
        let very_subdued = pick(is_dark, 0x3C_3C3C, 0xDD_DADA);
        let subdued = pick(is_dark, 0x5C_5C5C, 0x9B_9B9B);
        let prompt = Style::new().fg(pick(is_dark, 0xEC_FD65, 0x04_B575));
        let mut filter = textinput::Styles::new(is_dark);
        filter.cursor.color = hex(0xEE_6FF8);
        filter.focused.prompt = prompt;
        filter.blurred.prompt = prompt;
        Self {
            title_bar: Style::new().padding_sides(0, 0, 1, 2),
            title: Style::new()
                .bg(Color::Palette(62))
                .fg(Color::Palette(230))
                .padding(0, 1),
            spinner: Style::new().fg(pick(is_dark, 0x74_7373, 0x8E_8E8E)),
            filter,
            default_filter_character_match: Style::new().underline(),
            status_bar: Style::new()
                .fg(pick(is_dark, 0x77_7777, 0xA4_9FA5))
                .padding_sides(0, 0, 1, 2),
            status_empty: Style::new().fg(subdued),
            status_bar_active_filter: Style::new().fg(pick(is_dark, 0xDD_DDDD, 0x1A_1A1A)),
            status_bar_filter_count: Style::new().fg(very_subdued),
            no_items: Style::new().fg(pick(is_dark, 0x62_6262, 0x90_9090)),
            arabic_pagination: Style::new().fg(subdued),
            pagination_style: Style::new().padding_left(2),
            help_style: Style::new().padding_sides(1, 0, 0, 2),
            active_pagination_dot: Style::new().fg(pick(is_dark, 0x97_9797, 0x84_7A85)),
            inactive_pagination_dot: Style::new().fg(very_subdued),
            divider_dot: Style::new().fg(very_subdued),
        }
    }
}

/// The styles of the default delegate's items: Bubbles'
/// `list.DefaultItemStyles`.
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
/// let mut d = list::DefaultDelegate::new();
/// d.styles.selected_title = tea::style::Style::new().bold();
/// l.set_delegate(d);
/// assert!(l.view().contains("\x1b[1mapple\x1b[0m"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultItemStyles {
    /// An item's title.
    pub normal_title: Style,
    /// An item's description.
    pub normal_desc: Style,
    /// The selected item's title.
    pub selected_title: Style,
    /// The selected item's description.
    pub selected_desc: Style,
    /// A title while the filter is empty.
    pub dimmed_title: Style,
    /// A description while the filter is empty.
    pub dimmed_desc: Style,
    /// The characters the filter matched.
    pub filter_match: Style,
}

impl DefaultItemStyles {
    /// Bubbles' default item styles: `list.NewDefaultItemStyles`.
    ///
    /// ```
    /// use petitcarnets::bubbles::list::DefaultItemStyles;
    ///
    /// let (dark, light) = (DefaultItemStyles::new(true), DefaultItemStyles::new(false));
    /// assert!(
    ///     dark.selected_title
    ///         .render("x")
    ///         .starts_with("\x1b[38;2;173;88;180m│")
    /// );
    /// assert!(
    ///     light
    ///         .selected_title
    ///         .render("x")
    ///         .starts_with("\x1b[38;2;247;147;255m│")
    /// );
    /// ```
    #[must_use]
    pub fn new(is_dark: bool) -> Self {
        let normal_title = Style::new()
            .fg(pick(is_dark, 0xDD_DDDD, 0x1A_1A1A))
            .padding_sides(0, 0, 0, 2);
        let selected_title = Style::new()
            .border(Border::NORMAL)
            .border_sides([false, false, false, true])
            .border_fg(pick(is_dark, 0xAD_58B4, 0xF7_93FF))
            .fg(hex(0xEE_6FF8))
            .padding_sides(0, 0, 0, 1);
        let dimmed_title = Style::new()
            .fg(pick(is_dark, 0x77_7777, 0xA4_9FA5))
            .padding_sides(0, 0, 0, 2);
        Self {
            normal_title,
            normal_desc: normal_title.fg(pick(is_dark, 0x77_7777, 0xA4_9FA5)),
            selected_title,
            selected_desc: selected_title.fg(pick(is_dark, 0xAD_58B4, 0xF7_93FF)),
            dimmed_title,
            dimmed_desc: dimmed_title.fg(pick(is_dark, 0x4D_4D4D, 0xC2_B8C2)),
            filter_match: Style::new().underline(),
        }
    }
}
