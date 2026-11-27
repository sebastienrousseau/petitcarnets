//! Mouse events and the mouse modes a view asks for.

use crate::key::KeyMod;
use std::fmt;

/// A mouse button, as X11 numbers them. Bubble Tea's `MouseButton`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MouseButton {
    /// No button: motion with nothing held, or a release in the X10
    /// encoding, which does not say which button.
    #[default]
    None,
    /// The left button.
    Left,
    /// The middle button (pressing the wheel).
    Middle,
    /// The right button.
    Right,
    /// Wheel turned up.
    WheelUp,
    /// Wheel turned down.
    WheelDown,
    /// Wheel pushed left.
    WheelLeft,
    /// Wheel pushed right.
    WheelRight,
    /// The fourth button, browser back.
    Backward,
    /// The fifth button, browser forward.
    Forward,
    /// Button 10.
    Button10,
    /// Button 11.
    Button11,
}

/// Each button's name, as Bubble Tea writes it.
const BUTTON_NAMES: [(MouseButton, &str); 12] = {
    use MouseButton as B;
    [
        (B::None, "none"),
        (B::Left, "left"),
        (B::Middle, "middle"),
        (B::Right, "right"),
        (B::WheelUp, "wheelup"),
        (B::WheelDown, "wheeldown"),
        (B::WheelLeft, "wheelleft"),
        (B::WheelRight, "wheelright"),
        (B::Backward, "backward"),
        (B::Forward, "forward"),
        (B::Button10, "button10"),
        (B::Button11, "button11"),
    ]
};

impl MouseButton {
    /// The name Bubble Tea writes: `"left"`, `"wheelup"`, `"none"`.
    #[must_use]
    pub fn name(self) -> &'static str {
        BUTTON_NAMES
            .iter()
            .find(|(b, _)| *b == self)
            .map_or("", |(_, name)| name)
    }

    /// Whether this is one of the four wheel directions.
    #[must_use]
    pub const fn is_wheel(self) -> bool {
        matches!(
            self,
            Self::WheelUp | Self::WheelDown | Self::WheelLeft | Self::WheelRight
        )
    }
}

impl fmt::Display for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Where the mouse was and what was held: Bubble Tea's `Mouse`. The
/// message it comes in says whether it is a click, a release, a wheel
/// turn or motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mouse {
    /// The column, from 0 at the left.
    pub x: u16,
    /// The row, from 0 at the top.
    pub y: u16,
    /// The button.
    pub button: MouseButton,
    /// Shift, Alt and Ctrl, if held.
    pub mods: KeyMod,
}

impl Mouse {
    /// How Bubble Tea writes a motion event: `"left+motion"` while a
    /// button is held, `"motion"` with none.
    #[must_use]
    pub fn motion_string(&self) -> String {
        let sep = if self.button == MouseButton::None {
            ""
        } else {
            "+"
        };
        format!("{self}{sep}motion")
    }
}

impl fmt::Display for Mouse {
    /// As Bubble Tea's `Mouse.String`: modifiers, then the button, such
    /// as `"ctrl+left"` or `"wheelup"`; nothing for no button.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (m, name) in [
            (KeyMod::CTRL, "ctrl+"),
            (KeyMod::ALT, "alt+"),
            (KeyMod::SHIFT, "shift+"),
        ] {
            if self.mods.contains(m) {
                f.write_str(name)?;
            }
        }
        if self.button != MouseButton::None {
            f.write_str(self.button.name())?;
        }
        Ok(())
    }
}

/// Which mouse events a view asks the terminal for. Bubble Tea's
/// `MouseMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MouseMode {
    /// No mouse events (the default).
    #[default]
    None,
    /// Clicks, releases, wheel turns, and motion while a button is held.
    CellMotion,
    /// Every event, motion with no button held included.
    AllMotion,
}

#[cfg(test)]
mod tests;
