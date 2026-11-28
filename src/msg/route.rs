//! Passing messages down to child components: the runtime's messages as
//! they are, the model's own through a function that picks the child's.

use super::Msg;

impl<M> Msg<M> {
    /// The message for a child component whose messages are `N`: a
    /// runtime message (a key, the mouse, a paste, focus, the window
    /// size, a reply) as it is, and one of the model's own through
    /// `pick`, which gives the child's message if it is one, or `None`
    /// for a message that is not the child's (then this gives `None`
    /// too). The counterpart of [`Cmd::map`](crate::Cmd::map) for
    /// messages going down.
    ///
    /// ```
    /// use petitcarnets as tea;
    ///
    /// #[derive(Debug, Clone, PartialEq)]
    /// enum Child {
    ///     Blink,
    /// }
    ///
    /// enum Parent {
    ///     Child(Child),
    ///     Other,
    /// }
    ///
    /// let pick = |m: &Parent| match m {
    ///     Parent::Child(c) => Some(c.clone()),
    ///     Parent::Other => None,
    /// };
    /// let key = tea::Msg::<Parent>::KeyPress(tea::Key::char('a'));
    /// assert_eq!(key.route(pick), Some(tea::Msg::KeyPress(tea::Key::char('a'))));
    /// assert_eq!(tea::Msg::App(Parent::Child(Child::Blink)).route(pick), Some(tea::Msg::App(Child::Blink)));
    /// assert_eq!(tea::Msg::App(Parent::Other).route(pick), None);
    /// ```
    #[must_use]
    pub fn route<N>(&self, pick: impl FnOnce(&M) -> Option<N>) -> Option<Msg<N>> {
        match self {
            Self::App(m) => pick(m).map(Msg::App),
            other => Some(input(other)),
        }
    }

    /// The same message with the model's own message passed through
    /// `wrap`, and a runtime message as it is.
    ///
    /// ```
    /// use petitcarnets as tea;
    ///
    /// let msg: tea::Msg<u8> = tea::Msg::App(2);
    /// assert_eq!(msg.map(u16::from), tea::Msg::App(2_u16));
    /// ```
    #[must_use]
    pub fn map<N>(self, wrap: impl FnOnce(M) -> N) -> Msg<N> {
        match self {
            Self::App(m) => Msg::App(wrap(m)),
            other => input(&other),
        }
    }
}

/// A runtime message (not [`Msg::App`]) for another message type: keys,
/// mouse, paste and focus here, the rest further down.
fn input<M, N>(msg: &Msg<M>) -> Msg<N> {
    match msg {
        Msg::KeyPress(k) => Msg::KeyPress(k.clone()),
        Msg::KeyRelease(k) => Msg::KeyRelease(k.clone()),
        Msg::MouseClick(m) => Msg::MouseClick(*m),
        Msg::MouseRelease(m) => Msg::MouseRelease(*m),
        Msg::MouseWheel(m) => Msg::MouseWheel(*m),
        Msg::MouseMotion(m) => Msg::MouseMotion(*m),
        Msg::Paste(text) => Msg::Paste(text.clone()),
        other => window(other),
    }
}

/// Paste edges, focus, sizes and the like.
fn window<M, N>(msg: &Msg<M>) -> Msg<N> {
    match msg {
        Msg::PasteStart => Msg::PasteStart,
        Msg::PasteEnd => Msg::PasteEnd,
        Msg::Focus => Msg::Focus,
        Msg::Blur => Msg::Blur,
        Msg::WindowSize(size) => Msg::WindowSize(*size),
        Msg::PixelSize(size) => Msg::PixelSize(*size),
        Msg::CellSize(size) => Msg::CellSize(*size),
        Msg::Resume => Msg::Resume,
        other => replies(other),
    }
}

/// The terminal's replies to queries.
fn replies<M, N>(msg: &Msg<M>) -> Msg<N> {
    match msg {
        Msg::CursorPosition { x, y } => Msg::CursorPosition { x: *x, y: *y },
        Msg::KeyboardEnhancements(report) => Msg::KeyboardEnhancements(*report),
        Msg::ModeReport { mode, value } => Msg::ModeReport {
            mode: *mode,
            value: *value,
        },
        Msg::TerminalVersion(v) => Msg::TerminalVersion(v.clone()),
        Msg::ForegroundColor(c) => Msg::ForegroundColor(*c),
        Msg::BackgroundColor(c) => Msg::BackgroundColor(*c),
        Msg::CursorColor(c) => Msg::CursorColor(*c),
        Msg::ColorProfile(p) => Msg::ColorProfile(*p),
        other => attributes(other),
    }
}

/// Clipboard, capabilities, device attributes and colour scheme; an
/// [`Msg::App`] never reaches here, and is given as a resume.
fn attributes<M, N>(msg: &Msg<M>) -> Msg<N> {
    match msg {
        Msg::Clipboard { content, selection } => Msg::Clipboard {
            content: content.clone(),
            selection: *selection,
        },
        Msg::Capability(c) => Msg::Capability(c.clone()),
        Msg::PrimaryDeviceAttributes(a) => Msg::PrimaryDeviceAttributes(a.clone()),
        Msg::SecondaryDeviceAttributes(a) => Msg::SecondaryDeviceAttributes(a.clone()),
        Msg::TertiaryDeviceAttributes(a) => Msg::TertiaryDeviceAttributes(a.clone()),
        Msg::ColorScheme(s) => Msg::ColorScheme(*s),
        _ => Msg::Resume,
    }
}

#[cfg(test)]
mod tests;
