//! A spinner: frames shown one after another while something runs.
//! Bubbles' `spinner` package, with every one of its spinners.
//!
//! ```
//! use petitcarnets as tea;
//! use tea::bubbles::spinner::{self, Spinner};
//!
//! let mut s = spinner::Model::new().with_spinner(Spinner::DOT);
//! assert_eq!(s.view(), "⣾ ");
//! let start: tea::Cmd<spinner::TickMsg> = s.tick();
//! # let _ = start;
//! ```

use super::next_id;
use crate::style::Style;
use crate::{Cmd, Msg as TeaMsg};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A spinner's frames and how fast they turn: Bubbles' `spinner.Spinner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spinner {
    /// The frames, in order.
    pub frames: &'static [&'static str],
    /// How long each frame shows (Go's `FPS`, which is a duration too).
    pub fps: Duration,
}

/// `n` frames a second.
const fn per_second(n: u64) -> Duration {
    Duration::from_nanos(1_000_000_000 / n)
}

impl Spinner {
    /// `| / - \`.
    pub const LINE: Self = Self {
        frames: &["|", "/", "-", "\\"],
        fps: per_second(10),
    };
    /// A braille dot going round, with a space after.
    pub const DOT: Self = Self {
        frames: &["⣾ ", "⣽ ", "⣻ ", "⢿ ", "⡿ ", "⣟ ", "⣯ ", "⣷ "],
        fps: per_second(10),
    };
    /// A small braille dot going round.
    pub const MINI_DOT: Self = Self {
        frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        fps: per_second(12),
    };
    /// A braille dot jumping.
    pub const JUMP: Self = Self {
        frames: &["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"],
        fps: per_second(10),
    };
    /// A block fading.
    pub const PULSE: Self = Self {
        frames: &["█", "▓", "▒", "░"],
        fps: per_second(8),
    };
    /// Three points, one lit.
    pub const POINTS: Self = Self {
        frames: &["∙∙∙", "●∙∙", "∙●∙", "∙∙●"],
        fps: per_second(7),
    };
    /// The globe turning.
    pub const GLOBE: Self = Self {
        frames: &["🌍", "🌎", "🌏"],
        fps: per_second(4),
    };
    /// The moon's phases.
    pub const MOON: Self = Self {
        frames: &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"],
        fps: per_second(8),
    };
    /// The three wise monkeys.
    pub const MONKEY: Self = Self {
        frames: &["🙈", "🙉", "🙊"],
        fps: per_second(3),
    };
    /// A meter filling and emptying.
    pub const METER: Self = Self {
        frames: &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"],
        fps: per_second(7),
    };
    /// A trigram turning.
    pub const HAMBURGER: Self = Self {
        frames: &["☱", "☲", "☴", "☲"],
        fps: per_second(3),
    };
    /// Dots after a word.
    pub const ELLIPSIS: Self = Self {
        frames: &["", ".", "..", "..."],
        fps: per_second(3),
    };
}

/// Time for a spinner to show its next frame: Bubbles' `spinner.TickMsg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickMsg {
    /// When the tick fired, on the program's clock.
    pub time: SystemTime,
    /// The id of the spinner it is for; 0 is for every spinner.
    pub id: u64,
    tag: u64,
}

/// A spinner's messages: only ticks.
pub type Msg = TickMsg;

/// A spinner: Bubbles' `spinner.Model`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The frames and speed.
    pub spinner: Spinner,
    /// The style frames are drawn in.
    pub style: Style,
    frame: usize,
    id: u64,
    tag: u64,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// A [`Spinner::LINE`] spinner with a new id: `spinner.New`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spinner: Spinner::LINE,
            style: Style::new(),
            frame: 0,
            id: next_id(),
            tag: 0,
        }
    }

    /// With these frames: `spinner.WithSpinner`.
    #[must_use]
    pub const fn with_spinner(mut self, spinner: Spinner) -> Self {
        self.spinner = spinner;
        self
    }

    /// With this style: `spinner.WithStyle`.
    #[must_use]
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// The spinner's id, which its ticks carry.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Shows the next frame on its own tick, and asks for the one after;
    /// ticks for other spinners, and its own older ones, are ignored.
    pub fn update(&mut self, msg: &TeaMsg<Msg>) -> Cmd<Msg> {
        match msg {
            TeaMsg::App(tick) if self.is_mine(tick) => {
                self.frame = (self.frame + 1) % self.spinner.frames.len().max(1);
                self.tag += 1;
                let (id, tag) = (self.id, self.tag);
                crate::tick(self.spinner.fps, move |time| TickMsg { time, id, tag })
            }
            _ => Cmd::none(),
        }
    }

    /// Whether `tick` is this spinner's latest (an id or tag of 0 is
    /// anyone's).
    const fn is_mine(&self, tick: &TickMsg) -> bool {
        (tick.id == 0 || tick.id == self.id) && (tick.tag == 0 || tick.tag == self.tag)
    }

    /// The current frame in the style, or `(error)` if there is none.
    #[must_use]
    pub fn view(&self) -> String {
        self.spinner
            .frames
            .get(self.frame)
            .map_or_else(|| "(error)".to_string(), |f| self.style.paint(f))
    }

    /// A command that starts the spinner now: Bubbles' `Model.Tick`.
    #[must_use]
    pub fn tick(&self) -> Cmd<Msg> {
        let (id, tag) = (self.id, self.tag);
        crate::tick(Duration::ZERO, move |time| TickMsg { time, id, tag })
    }

    /// A tick for this spinner at `time`, as the runtime would send.
    #[must_use]
    pub const fn tick_msg(&self, time: SystemTime) -> TickMsg {
        TickMsg {
            time,
            id: self.id,
            tag: self.tag,
        }
    }
}

impl Default for TickMsg {
    /// A tick for every spinner, at the epoch.
    fn default() -> Self {
        Self {
            time: UNIX_EPOCH,
            id: 0,
            tag: 0,
        }
    }
}

#[cfg(test)]
mod tests;
