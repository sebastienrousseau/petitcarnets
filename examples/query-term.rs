//! Asks the terminal about itself and prints every reply: in the spirit
//! of Bubble Tea's `query-term` example, with a key per question instead
//! of typed escape sequences. b, f and c ask for the background, text and
//! cursor colours; p for the cursor position; v for the terminal's name;
//! r for the `RGB` capability; w for the window size; k reads the
//! clipboard and y sets it; d asks for the device attributes. q quits.
//!
//! With `QUERY_TERM_EXIT=1` it asks everything at once and quits at once:
//! the replies still on their way are read before the terminal is given
//! back, so none reach the shell.
//!
//! `cargo run --example query-term`

use petitcarnets as tea;

struct Query;

/// Every question at once.
fn everything() -> Vec<tea::Cmd<()>> {
    vec![
        tea::request_background_color(),
        tea::request_foreground_color(),
        tea::request_cursor_color(),
        tea::request_cursor_position(),
        tea::request_terminal_version(),
        tea::request_capability("RGB"),
        tea::read_clipboard(),
    ]
}

impl tea::Model for Query {
    type Msg = ();

    fn init(&mut self) -> tea::Cmd<()> {
        if std::env::var("QUERY_TERM_EXIT").is_ok_and(|v| v == "1") {
            let mut all = everything();
            all.push(tea::quit());
            return tea::sequence(all);
        }
        tea::Cmd::none()
    }

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        let tea::Msg::KeyPress(key) = msg else {
            return match msg {
                tea::Msg::App(()) => tea::Cmd::none(),
                other => tea::printf!("Received message: {other:?}"),
            };
        };
        match key.to_string().as_str() {
            "b" => tea::request_background_color(),
            "f" => tea::request_foreground_color(),
            "c" => tea::request_cursor_color(),
            "p" => tea::request_cursor_position(),
            "v" => tea::request_terminal_version(),
            "r" => tea::request_capability("RGB"),
            "w" => tea::request_window_size(),
            "k" => tea::read_clipboard(),
            "y" => tea::set_clipboard("hello from petitcarnets"),
            "d" => tea::raw("\x1b[c"),
            "q" | "ctrl+c" => tea::quit(),
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        "b f c: colours, p: cursor, v: version, r: RGB, w: size,\n\
         k/y: read/set clipboard, d: device attributes, q: quit\n"
            .into()
    }
}

fn main() -> Result<(), tea::Error> {
    let _ = tea::Program::new(Query).run()?;
    Ok(())
}
