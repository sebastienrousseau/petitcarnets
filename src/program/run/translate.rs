//! What each piece of terminal input means for the model: Bubble Tea's
//! `translateInputEvent`, plus the signals.

use crate::backend::Input;
use crate::cmd::Handover;
use crate::exec::Item;
use crate::msg::Msg;

/// Whether input comes from the user, whose frame should not wait.
pub(super) const fn is_urgent(input: &Input) -> bool {
    matches!(
        input,
        Input::KeyPress(_)
            | Input::KeyRelease(_)
            | Input::MouseClick(_)
            | Input::MouseRelease(_)
            | Input::MouseWheel(_)
            | Input::MouseMotion(_)
            | Input::Paste(_)
    )
}

/// Whether input is Ctrl-Z, which suspends by default.
pub(super) fn is_suspend_key(input: &Input) -> bool {
    matches!(input, Input::KeyPress(key) if *key == "ctrl+z")
}

/// What a piece of terminal input means for the model: keys, mouse and
/// paste here, the rest below.
pub(super) fn input_item<M>(input: Input) -> Item<M> {
    let msg = match input {
        Input::KeyPress(key) => Msg::KeyPress(key),
        Input::KeyRelease(key) => Msg::KeyRelease(key),
        Input::MouseClick(mouse) => Msg::MouseClick(mouse),
        Input::MouseRelease(mouse) => Msg::MouseRelease(mouse),
        Input::MouseWheel(mouse) => Msg::MouseWheel(mouse),
        Input::MouseMotion(mouse) => Msg::MouseMotion(mouse),
        Input::Paste(text) => Msg::Paste(text),
        input => return focus_item(input),
    };
    Item::Msg(msg)
}

/// Paste marks, focus, and the cursor and keyboard replies.
fn focus_item<M>(input: Input) -> Item<M> {
    let msg = match input {
        Input::PasteStart => Msg::PasteStart,
        Input::PasteEnd => Msg::PasteEnd,
        Input::Focus => Msg::Focus,
        Input::Blur => Msg::Blur,
        Input::CursorPosition { x, y } => Msg::CursorPosition { x, y },
        Input::KeyboardEnhancements(report) => Msg::KeyboardEnhancements(report),
        Input::ModeReport { mode, value } => Msg::ModeReport { mode, value },
        input => return colour_item(input),
    };
    Item::Msg(msg)
}

/// The terminal's name and colours, and sizes.
fn colour_item<M>(input: Input) -> Item<M> {
    let msg = match input {
        Input::TerminalVersion(name) => Msg::TerminalVersion(name),
        Input::ForegroundColor(c) => Msg::ForegroundColor(c),
        Input::BackgroundColor(c) => Msg::BackgroundColor(c),
        Input::CursorColor(c) => Msg::CursorColor(c),
        Input::ColorScheme(scheme) => Msg::ColorScheme(scheme),
        Input::Resize(size) => Msg::WindowSize(size),
        Input::PixelSize(size) => Msg::PixelSize(size),
        Input::CellSize(size) => Msg::CellSize(size),
        input => return reply_item(input),
    };
    Item::Msg(msg)
}

/// Other replies: clipboard, capabilities, device attributes.
fn reply_item<M>(input: Input) -> Item<M> {
    let msg = match input {
        Input::Clipboard { content, selection } => Msg::Clipboard { content, selection },
        Input::Capability(capability) => Msg::Capability(capability),
        Input::PrimaryDeviceAttributes(attrs) => Msg::PrimaryDeviceAttributes(attrs),
        Input::SecondaryDeviceAttributes(attrs) => Msg::SecondaryDeviceAttributes(attrs),
        Input::TertiaryDeviceAttributes(id) => Msg::TertiaryDeviceAttributes(id),
        input => return signal_item(input),
    };
    Item::Msg(msg)
}

/// Signals.
fn signal_item<M>(input: Input) -> Item<M> {
    match input {
        Input::Interrupt => Item::Interrupt,
        Input::Terminate => Item::Quit,
        Input::Suspend => Item::Handover(Handover::Suspend),
        Input::Continue => Item::Handover(Handover::Continue),
        input => input_item(input),
    }
}
