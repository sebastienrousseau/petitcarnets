# petitcarnets

Terminal user interfaces in the Elm architecture (model, update, view),
for Rust, with no dependencies.

petitcarnets follows [Bubble Tea](https://github.com/charmbracelet/bubbletea)
v2 closely: the same loop, the same command and message names, and the
same key strings. If you have written a Bubble Tea program in Go, the Rust
version should read the way you expect.

*Petits carnets*: little notebooks.

## Status

Early work, on [crates.io](https://crates.io/crates/petitcarnets) since
v0.0.1. It has the runtime (Bubble Tea's names, commands, window size,
signals, a terminal restored however the program ends) and, from v0.0.2,
Bubble Tea's input: keys with modifiers and releases, mouse, paste,
focus, and the terminal's replies. From v0.0.3 it has Bubble Tea v2's
renderer: views draw inline by default (or on the alternate screen when
they ask), on a cell buffer that writes only the cells that change, with
wide characters, emoji and tabs measured as Go measures them; `println`
prints above an inline frame. Expect the API to grow.

## Why

- **No dependencies.** `cargo tree` prints one crate. The terminal is
  driven through a handful of libc calls in one audited module, the only
  place with `unsafe`.
- **Typed messages.** Your messages are your own enum, matched
  exhaustively; no downcasting.
- **No async runtime required.** A command is a plain closure. Timers
  run on the event loop, not on threads, and an idle program does not
  wake up at all.
- **Restores your terminal.** Raw mode and every terminal mode are undone
  on quit, on panic (in `update` or in a task), on `SIGINT` and
  `SIGTERM`; a real-terminal test checks each one in CI.
- **Testable.** A headless backend runs a model with scripted keys and
  captures the screen, with no pseudo-terminal.
- **Small, prompt frames.** Frames are diffed cell by cell; an unchanged
  frame writes nothing, and the frame after a key is drawn at once
  rather than on the next tick. See [docs/benchmarks.md](docs/benchmarks.md).

## Example

```rust
use petitcarnets as tea;

struct Counter(i64);

impl tea::Model for Counter {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k == "q" || k == "ctrl+c" => return tea::quit(),
            tea::Msg::KeyPress(k) if k == "up" => self.0 += 1,
            tea::Msg::KeyPress(k) if k == "down" => self.0 -= 1,
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        format!("Count: {}\n\nup/down to change, q to quit", self.0).into()
    }
}

fn main() -> Result<(), tea::Error> {
    tea::Program::new(Counter(0)).run()?;
    Ok(())
}
```

Run the examples with `cargo run --example counter` (also `picker`,
`spinner`, `lifecycle`, `mouse`, `keyboard-enhancements`, and, named
after Bubble Tea's, `altscreen-toggle`, `cursor-style`,
`set-window-title` and `package-manager`).

As in Bubble Tea v2, a view is drawn inline unless it sets
`alt_screen`. When an inline program quits, its last frame stays on
screen, every row of it, and the shell prompt starts on the line below;
a program that wants nothing left behind returns an empty view last.

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `tea.NewProgram(m).Run()` | `tea::Program::new(m).run()` |
| `case tea.KeyPressMsg:` | `tea::Msg::KeyPress(k) =>` |
| `msg.String() == "ctrl+c"` | `k == "ctrl+c"` |
| `msg.Keystroke()`, `msg.MatchString("q", "esc")` | `k.keystroke()`, `k.matches_any(&["q", "esc"])` |
| `case tea.MouseClickMsg:` | `tea::Msg::MouseClick(m) =>` |
| `case tea.PasteMsg:` | `tea::Msg::Paste(text) =>` |
| `v.MouseMode = tea.MouseModeAllMotion` | `view.with_mouse_mode(tea::MouseMode::AllMotion)` |
| `v.AltScreen = true` | `view.with_alt_screen(true)` |
| `v.Cursor = tea.NewCursor(x, y)` | `view.with_cursor(tea::Cursor::new(x, y))` |
| `v.WindowTitle = "t"` | `view.with_window_title("t")` |
| `tea.Println(s)`, `tea.Printf(f, a)` | `tea::println(s)`, `tea::printf!(f, a)` |
| `tea.Raw(s)`, `tea.ClearScreen` | `tea::raw(s)`, `tea::clear_screen()` |
| `tea.WithFPS(n)` | `Program::new(m).with_fps(n)` |
| `case tea.WindowSizeMsg:` | `tea::Msg::WindowSize(size) =>` |
| `return m, tea.Quit` | `return tea::quit()` |
| `tea.Batch(a, b)`, `tea.Sequence(a, b)` | `tea::batch([a, b])`, `tea::sequence([a, b])` |
| `tea.Tick(d, fn)`, `tea.Every(d, fn)` | `tea::tick(d, f)`, `tea::every(d, f)` |
| `p.Send(msg)` | `program.sender().send(msg)` |

## Platforms

Linux and macOS. Windows is planned for a later release.

Minimum supported Rust version: 1.85.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT licence ([LICENSE-MIT](LICENSE-MIT))

at your option.
