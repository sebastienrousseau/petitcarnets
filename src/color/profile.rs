//! How many colours a terminal shows, found from the environment as Go's
//! colorprofile finds it, and every colour written brought down to it.

use super::Rgb;
use super::convert::{palette, to_16, to_256};
use crate::environ::Environment;
use crate::renderer::cell::{Colour, Link, Pen};
use std::fmt;

/// How many colours a terminal shows. Bubble Tea's
/// `colorprofile.Profile`, from fewest to most, so profiles compare.
///
/// Every colour the program writes (frames, printed lines, the cursor's
/// colour, the terminal's default colours) is brought down to the
/// program's profile, as Bubble Tea's renderer does: a 24-bit colour
/// becomes the nearest of the 256 or 16 palette colours, and with
/// [`Ascii`](ColorProfile::Ascii) (as `NO_COLOR` asks) colours go but
/// bold, italic, underline and the rest stay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorProfile {
    /// Not a terminal: no colours, no attributes, no hyperlinks.
    NoTty,
    /// Attributes only, no colours.
    Ascii,
    /// The 16 standard colours.
    Ansi,
    /// The 256 palette colours.
    Ansi256,
    /// 24-bit colours.
    TrueColor,
}

impl fmt::Display for ColorProfile {
    /// The profile's name, as Go writes it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoTty => "NoTTY",
            Self::Ascii => "Ascii",
            Self::Ansi => "ANSI",
            Self::Ansi256 => "ANSI256",
            Self::TrueColor => "TrueColor",
        })
    }
}

impl ColorProfile {
    /// The profile for a terminal (`is_terminal`) or anything else, in
    /// `env`: Go's `colorprofile.Detect`. It reads `NO_COLOR`,
    /// `CLICOLOR`, `CLICOLOR_FORCE`, `COLORTERM`, `TERM`, `TTY_FORCE`,
    /// `WT_SESSION`, `GOOGLE_CLOUD_SHELL` and `TMUX`, and inside tmux
    /// asks `tmux info` whether it passes 24-bit colour on. Unlike Go it
    /// reads no terminfo database: a terminal whose entry alone claims
    /// 24-bit colour (the `RGB` or `Tc` capability) is upgraded when it
    /// answers [`request_capability`](crate::request_capability) for
    /// `RGB` or `Tc` instead.
    #[must_use]
    pub fn detect(is_terminal: bool, env: &Environment) -> Self {
        Self::detect_with(is_terminal, env, tmux_info)
    }

    /// [`ColorProfile::detect`] with `tmux` standing for `tmux info`.
    pub(crate) fn detect_with(
        is_terminal: bool,
        env: &Environment,
        tmux: impl FnOnce() -> Option<String>,
    ) -> Self {
        let tty = env.flag("TTY_FORCE") || is_terminal;
        let term = env.get("TERM");
        let dumb = term.is_none_or(|t| t == "dumb");
        let profile = from_env(tty, env);
        if profile == Self::TrueColor || env.flag("NO_COLOR") {
            return profile;
        }
        if tty && !dumb {
            let terminfo = if term.is_some_and(|t| !t.is_empty()) {
                Self::Ansi
            } else {
                Self::NoTty
            };
            return profile.max(terminfo).max(in_tmux(env, tmux));
        }
        profile
    }

    /// `c` as this profile shows it; [`Colour::Default`] when it shows
    /// no colours. Go's `Profile.Convert`.
    pub(crate) fn colour(self, c: Colour) -> Colour {
        match (self, c) {
            (Self::NoTty | Self::Ascii, _) | (_, Colour::Default) => Colour::Default,
            (Self::TrueColor, c)
            | (_, c @ Colour::Basic(_))
            | (Self::Ansi256, c @ Colour::Indexed(_)) => c,
            (_, Colour::Indexed(n)) => Colour::Basic(to_16(n)),
            (Self::Ansi256, Colour::Rgb(r, g, b)) => Colour::Indexed(to_256(Rgb::new(r, g, b))),
            (_, Colour::Rgb(r, g, b)) => Colour::Basic(to_16(to_256(Rgb::new(r, g, b)))),
        }
    }

    /// A cell's style as this profile shows it: Go's `uv.ConvertStyle`.
    pub(crate) fn pen(self, pen: Pen) -> Pen {
        if self == Self::NoTty {
            return Pen::DEFAULT;
        }
        Pen {
            fg: self.colour(pen.fg),
            bg: self.colour(pen.bg),
            underline_colour: self.colour(pen.underline_colour),
            ..pen
        }
    }

    /// A hyperlink as this profile shows it: none when it is not a
    /// terminal. Go's `uv.ConvertLink`.
    pub(crate) fn link(self, link: &Link) -> Link {
        if self == Self::NoTty {
            None
        } else {
            link.clone()
        }
    }

    /// A 24-bit colour for OSC 10, 11 or 12 as this profile shows it:
    /// the palette colour it comes down to, or `None` when the profile
    /// shows no colours.
    pub(crate) fn rgb(self, c: Rgb) -> Option<Rgb> {
        match self {
            Self::NoTty | Self::Ascii => None,
            Self::Ansi => Some(palette(to_16(to_256(c)))),
            Self::Ansi256 => Some(palette(to_256(c))),
            Self::TrueColor => Some(c),
        }
    }
}

/// Go's `colorProfile`: the profile from the environment alone.
fn from_env(tty: bool, env: &Environment) -> ColorProfile {
    let term = env.get("TERM");
    let dumb = term.is_none_or(|t| t == "dumb");
    let env_profile = term_profile(env);
    let mut p = if !tty || dumb {
        ColorProfile::NoTty
    } else {
        env_profile
    };
    if env.flag("NO_COLOR") && tty {
        return p.min(ColorProfile::Ascii);
    }
    if env.flag("CLICOLOR_FORCE") {
        return p.max(ColorProfile::Ansi).max(env_profile);
    }
    if env.flag("CLICOLOR") && tty && !dumb {
        p = p.max(ColorProfile::Ansi);
    }
    p
}

/// Terminals known to show 24-bit colour, by a part of their `TERM`.
const TRUE_COLOUR_TERMS: [&str; 8] = [
    "alacritty",
    "contour",
    "foot",
    "ghostty",
    "kitty",
    "rio",
    "st",
    "wezterm",
];

/// Go's `envColorProfile`: the profile `TERM`, `COLORTERM` and a few
/// terminals' own variables say.
fn term_profile(env: &Environment) -> ColorProfile {
    let term = env.var("TERM");
    if TRUE_COLOUR_TERMS.iter().any(|t| term.contains(t)) || says_true_colour(env, term) {
        return ColorProfile::TrueColor;
    }
    let mut p = if term.is_empty() || term == "dumb" {
        ColorProfile::NoTty
    } else {
        ColorProfile::Ansi
    };
    if is_multiplexer(term) || term.ends_with("256color") {
        p = p.max(ColorProfile::Ansi256);
    }
    p
}

/// Whether `TERM` names tmux or screen.
fn is_multiplexer(term: &str) -> bool {
    term.starts_with("tmux") || term.starts_with("screen")
}

/// Whether Windows Terminal, Google Cloud Shell, `COLORTERM` (outside
/// tmux and screen) or a `-direct` `TERM` says 24-bit colour.
fn says_true_colour(env: &Environment, term: &str) -> bool {
    let colorterm = env.var("COLORTERM").to_lowercase();
    let says = ["truecolor", "24bit", "yes", "true"].contains(&colorterm.as_str());
    !env.var("WT_SESSION").is_empty()
        || env.flag("GOOGLE_CLOUD_SHELL")
        || (says && !is_multiplexer(term))
        || term.ends_with("direct")
}

/// Go's `tmux`: inside tmux, 256 colours, or 24-bit ones when `tmux
/// info` (here `info`) says `Tc` or `RGB` is on.
fn in_tmux(env: &Environment, info: impl FnOnce() -> Option<String>) -> ColorProfile {
    if env.var("TMUX").is_empty() {
        return ColorProfile::NoTty;
    }
    let says_rgb =
        |line: &str| (line.contains("Tc") || line.contains("RGB")) && line.contains("true");
    match info() {
        Some(out) if out.lines().any(says_rgb) => ColorProfile::TrueColor,
        _ => ColorProfile::Ansi256,
    }
}

/// What `tmux info` prints, if it runs.
fn tmux_info() -> Option<String> {
    let out = std::process::Command::new("tmux")
        .arg("info")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}
