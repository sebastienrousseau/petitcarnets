//! Escape sequences that are not control sequences, and operating
//! system commands (OSC).

use super::Vt;
use std::rc::Rc;

/// OSC numbers understood that only need logging: icon name, notifications
/// and progress, colours and their resets, pointer shape, clipboard.
const LOGGED: [&str; 11] = [
    "1", "9", "10", "11", "12", "22", "52", "104", "110", "111", "112",
];

impl Vt {
    /// Acts on `ESC 7`, `ESC 8`, `ESC M` and `ESC c`; logs anything
    /// else, DCS, APC, PM and SOS strings included.
    pub(super) fn escape(&mut self, s: &str) {
        match s {
            "\x1b7" => self.saved = self.save(),
            "\x1b8" => self.restore(self.saved),
            "\x1bM" => self.reverse_index(),
            "\x1bc" => self.reset(),
            _ => self.log(format!("ESC {}", s[1..].escape_debug())),
        }
    }

    /// Acts on an OSC payload, logged first: 0 and 2 set the title, 8
    /// the hyperlink.
    pub(super) fn osc(&mut self, payload: &str) {
        self.osc_log.push(payload.to_string());
        let (code, rest) = payload.split_once(';').unwrap_or((payload, ""));
        match code {
            "0" | "2" => self.title = rest.to_string(),
            "8" => self.set_link(rest),
            _ if LOGGED.contains(&code) => {}
            _ => self.log(format!("OSC {}", payload.escape_debug())),
        }
    }

    /// OSC 8: `rest` is `params;url`; an empty URL ends the link.
    fn set_link(&mut self, rest: &str) {
        let url = rest.split_once(';').map_or("", |(_, url)| url);
        self.link = if url.is_empty() {
            None
        } else {
            Some(Rc::from(rest))
        };
    }

    /// RIS: the state of a new terminal of the same size. The logs,
    /// scrollback and count of synchronised updates are kept.
    fn reset(&mut self) {
        let fresh = Self::new(self.columns, self.rows).with_width_method(self.method);
        let old = std::mem::replace(self, fresh);
        self.scrollback = old.scrollback;
        self.sync_frames = old.sync_frames;
        self.osc_log = old.osc_log;
        self.unknown = old.unknown;
        self.partial = old.partial;
    }
}
