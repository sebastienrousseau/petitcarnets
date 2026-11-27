//! Shows the mouse events the terminal sends: clicks, releases, the
//! wheel and motion, with their cells. Bubble Tea's `mouse` example.
//!
//! `cargo run --example mouse`

use petitcarnets as tea;
use tea::{Mouse, MouseMode, Msg};

#[derive(Default)]
struct Mice {
    events: Vec<String>,
    last_click: Option<String>,
}

/// An event as `(X: 9, Y: 4) left`.
fn describe(mouse: &Mouse, name: &str) -> String {
    format!("(X: {}, Y: {}) {name}", mouse.x, mouse.y)
}

impl tea::Model for Mice {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> tea::Cmd<()> {
        let line = match msg {
            Msg::KeyPress(key) if key.matches_any(&["q", "esc", "ctrl+c"]) => return tea::quit(),
            Msg::MouseClick(m) => {
                let line = describe(&m, &m.to_string());
                self.last_click = Some(line.clone());
                format!("click   {line}")
            }
            Msg::MouseRelease(m) => format!("release {}", describe(&m, &m.to_string())),
            Msg::MouseWheel(m) => format!("wheel   {}", describe(&m, &m.to_string())),
            Msg::MouseMotion(m) => format!("motion  {}", describe(&m, &m.motion_string())),
            Msg::Focus => "focus".to_string(),
            Msg::Blur => "blur".to_string(),
            _ => return tea::Cmd::none(),
        };
        self.events.push(line);
        let excess = self.events.len().saturating_sub(10);
        drop(self.events.drain(..excess));
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        let content = format!(
            "\n  Do mouse stuff. When you're done press q to quit.\n\n  {}",
            self.events.join("\n  ")
        );
        tea::View::new(content)
            .with_mouse_mode(MouseMode::AllMotion)
            .with_report_focus(true)
    }
}

fn main() -> Result<(), tea::Error> {
    let mice = tea::Program::new(Mice::default()).run()?;
    match mice.last_click {
        Some(click) => println!("Last click: {click}"),
        None => println!("No clicks"),
    }
    Ok(())
}
