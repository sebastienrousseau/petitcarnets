//! One cell of the emulated screen, as a test sees it.

use crate::renderer::cell::{Cell, Pen};

/// One cell of a [`Vt`](super::Vt)'s screen: what it shows, how wide it
/// is, its style and its hyperlink.
#[derive(Debug, Clone, Copy)]
pub struct ScreenCell<'a>(&'a Cell);

impl<'a> ScreenCell<'a> {
    /// The cell as the renderer keeps it.
    pub(super) const fn new(cell: &'a Cell) -> Self {
        Self(cell)
    }

    /// What the cell shows: a grapheme, a space when blank, and nothing
    /// for the right half of a wide character.
    #[must_use]
    pub fn text(&self) -> String {
        let mut text = String::new();
        self.0.push_text(&mut text);
        text
    }

    /// How many columns the cell takes: 1, 2 for a wide character, and 0
    /// for the right half of one.
    #[must_use]
    pub const fn width(&self) -> u8 {
        self.0.width
    }

    /// Whether this is the right half of a wide character.
    #[must_use]
    pub fn is_continuation(&self) -> bool {
        self.0.is_continuation()
    }

    /// The cell's style as the shortest SGR sequence that sets it from
    /// the default (such as `"\x1b[1;31m"`), empty when it has none.
    #[must_use]
    pub fn style(&self) -> String {
        Pen::DEFAULT.transition(&self.0.pen)
    }

    /// The URL of the cell's hyperlink (OSC 8), if it has one.
    #[must_use]
    pub fn link(&self) -> Option<&'a str> {
        let link: &'a str = self.0.link.as_deref()?;
        link.split_once(';').map(|(_, url)| url)
    }
}
