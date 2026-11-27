//! An inline progress line with finished packages printed above it,
//! where they stay in the scrollback: Bubble Tea's `package-manager`
//! example. The terminal's own progress bar (OSC 9;4) follows along
//! where it has one. Set `PACKAGE_DELAY_MS` to change the pace.
//!
//! `cargo run --example package-manager`

use petitcarnets as tea;
use std::time::Duration;
use tea::style::{Color, Style};

const PACKAGES: [&str; 12] = [
    "vegeutils",
    "libgardening",
    "currykit",
    "spicerack",
    "fullenglish",
    "eggy",
    "bad-kitty",
    "chai",
    "hojicha",
    "libtacos",
    "babys-monads",
    "libpurring",
];

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

enum Event {
    Installed,
    Spin,
}

struct Installer {
    index: usize,
    frame: usize,
    width: usize,
    delay: Duration,
    done: bool,
}

impl Installer {
    /// Installing a package takes a while; pretend.
    fn install(&self) -> tea::Cmd<Event> {
        let step = u64::try_from(self.index % 5).unwrap_or(0);
        tea::tick(self.delay + Duration::from_millis(step * 37), |_| {
            Event::Installed
        })
    }

    fn installed(&mut self) -> tea::Cmd<Event> {
        let check = Style::new().fg(Color::GREEN).paint("✓");
        let print = tea::printf!("{check} {}", PACKAGES[self.index]);
        if self.index + 1 == PACKAGES.len() {
            self.done = true;
            return tea::sequence([print, tea::quit()]);
        }
        self.index += 1;
        tea::batch([print, self.install()])
    }
}

impl tea::Model for Installer {
    type Msg = Event;

    fn init(&mut self) -> tea::Cmd<Event> {
        tea::batch([
            self.install(),
            tea::tick(Duration::from_millis(80), |_| Event::Spin),
        ])
    }

    fn update(&mut self, msg: tea::Msg<Event>) -> tea::Cmd<Event> {
        match msg {
            tea::Msg::WindowSize(size) => self.width = usize::from(size.width),
            tea::Msg::KeyPress(k) if k == "q" || k == "esc" || k == "ctrl+c" => {
                return tea::quit();
            }
            tea::Msg::App(Event::Installed) => return self.installed(),
            tea::Msg::App(Event::Spin) if !self.done => {
                self.frame = (self.frame + 1) % SPINNER.len();
                return tea::tick(Duration::from_millis(80), |_| Event::Spin);
            }
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let n = PACKAGES.len();
        if self.done {
            let done = Style::new()
                .bold()
                .paint(&format!("Done! Installed {n} packages."));
            return tea::View::new(format!("{done}\n"));
        }
        let spin = Style::new()
            .fg(Color::Palette(63))
            .paint(SPINNER[self.frame]);
        let name = Style::new()
            .fg(Color::Palette(211))
            .paint(PACKAGES[self.index]);
        let filled = 30 * self.index / n;
        let bar = "█".repeat(filled) + &"░".repeat(30 - filled);
        let count = format!(" {:>2}/{n}", self.index);
        let info = format!("{spin} Installing {name}");
        let used = tea::style::width(&info) + 30 + count.len();
        let gap = " ".repeat(self.width.saturating_sub(used));
        let percent = u8::try_from(100 * self.index / n).unwrap_or(100);
        tea::View::new(format!("{info}{gap}{bar}{count}")).with_progress_bar(tea::ProgressBar::new(
            tea::ProgressBarState::Default,
            percent,
        ))
    }
}

fn main() -> Result<(), tea::Error> {
    let delay = std::env::var("PACKAGE_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(250);
    let installer = Installer {
        index: 0,
        frame: 0,
        width: 80,
        delay: Duration::from_millis(delay),
        done: false,
    };
    let _ = tea::Program::new(installer).run()?;
    Ok(())
}
