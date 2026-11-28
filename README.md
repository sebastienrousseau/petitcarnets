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
prints above an inline frame. From v0.0.4 it has Bubble Tea's
lifecycle: run an editor or a shell in the terminal (`exec`), suspend
with Ctrl-Z, release and restore the terminal, repaint after something
else wrote over it, a colour profile detected from the environment that
every colour written comes down to, and the terminal queries (colours,
cursor, version, capabilities, clipboard). From v0.0.5 it has a public
test harness, `TestProgram`: Bubble Tea's teatest, on a virtual clock.
From v0.0.6 it has the components of [Bubbles](https://github.com/charmbracelet/bubbles)
v2 (text input, text area, list, table, viewport, spinner, progress bar,
help and the rest), with their default key bindings, and `Cmd::map` to
compose them. From v0.0.7 it has an accessible output mode for screen
readers, 39 of Bubble Tea's 64 examples ported, and guides. Expect the
API to grow.

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
  `SIGTERM`, and around an `exec` or a suspend, with no key lost or read
  twice; a real-terminal test checks each one in CI.
- **Testable.** `TestProgram` runs a model in the real runtime on a
  virtual clock and an emulated terminal: no sleeps, no pseudo-terminal,
  the same result every run, with golden files for screens. See
  [Testing](#testing).
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
`spinner` and `lifecycle`, and Bubble Tea's own under their names:
`simple`, `views`, `http`, `split-editors` and the rest).
[docs/examples.md](docs/examples.md) lists every one of Bubble Tea's
examples, ported or not yet; each port runs in CI.

**Accessible output.** For screen readers and braille displays, a
program can print its frames as plain lines, with no redraws, no cursor
movement, no alternate screen and no colour (Bubble Tea's
[issue #780](https://github.com/charmbracelet/bubbletea/issues/780)):
`tea::Program::new(m).with_accessible(true)`, huh's `WithAccessible`.
Without that call, a program in a terminal whose environment sets
`ACCESSIBLE` (to anything but the empty string, as huh's documentation
recommends) prints lines too, so the person at the terminal decides. Input works as
always, so the input modes are still written: bracketed paste,
modifyOtherKeys and the kitty keyboard flags (`\e[?2004h`, `\e[>4;2m`,
`\e[>1u`) before the first line and their resets at exit, plus mouse,
focus reports and the window title if the view asks for them. Output
captured to a file (a `script` log, or a backend given to `run_with`)
holds these few sequences at its start and end.

**Ctrl-Z suspends by default.** Unlike Bubble Tea, where each program
must handle it ([its pull request #500](https://github.com/charmbracelet/bubbletea/pull/500)),
Ctrl-Z suspends a petitcarnets program as it would any shell program:
the terminal is given back, the shell's `fg` brings the program back, and
the model gets `tea::Msg::Resume`. A program that wants Ctrl-Z as a key
(for undo, say) opts out with `tea::Program::new(m).without_suspend()`;
`tea::suspend()` and `kill -TSTP` still suspend it.

Colours come down to what the terminal shows, detected from `TERM`,
`COLORTERM`, `NO_COLOR`, `CLICOLOR` and the rest as Bubble Tea detects
them: with `NO_COLOR` set, no colour is written anywhere (frames and
`println` alike) while bold, italic and underline stay.

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
| `tea.ExecProcess(cmd, fn)`, `tea.Exec(c, fn)` | `tea::exec(cmd, f)`, `tea::exec_command(c, f)` |
| `tea.Suspend`, `case tea.ResumeMsg:` | `tea::suspend()`, `tea::Msg::Resume =>` |
| `p.ReleaseTerminal()`, `p.RestoreTerminal()` | `tea::release_terminal()`, `tea::restore_terminal()` |
| `tea.WithColorProfile(p)`, `tea.WithEnvironment(env)` | `.with_color_profile(p)`, `.with_environment(env)` |
| `tea.RequestBackgroundColor`, `tea.SetClipboard(s)` | `tea::request_background_color()`, `tea::set_clipboard(s)` |
| `case tea.WindowSizeMsg:` | `tea::Msg::WindowSize(size) =>` |
| `return m, tea.Quit` | `return tea::quit()` |
| `tea.Batch(a, b)`, `tea.Sequence(a, b)` | `tea::batch([a, b])`, `tea::sequence([a, b])` |
| `tea.Tick(d, fn)`, `tea.Every(d, fn)` | `tea::tick(d, f)`, `tea::every(d, f)` |
| `p.Send(msg)` | `program.sender().send(msg)` |

Every exported name of Bubble Tea v2, the idioms, and a Go program
converted step by step: [docs/go-to-rust.md](docs/go-to-rust.md).
Coming from the bubbletea-rs crate:
[docs/migrating-from-bubbletea-rs.md](docs/migrating-from-bubbletea-rs.md).
How the crate works inside: [docs/architecture.md](docs/architecture.md).
The Bubble Tea issues it addresses, with the test for each:
[docs/issues.md](docs/issues.md).

## Components

The `bubbles` module (a default feature, no dependencies; turn it off
with `default-features = false`) has Bubbles v2's components under the
names Go uses, with the same default key bindings, checked line by line
against Bubbles itself, and the same behaviour (where Bubbles has a bug,
it is fixed and said so in the CHANGELOG).

| Bubbles (Go) | petitcarnets |
| --- | --- |
| `charm.land/bubbles/v2/key` | `tea::bubbles::key`: `Binding`, `matches` |
| `.../help` | `tea::bubbles::help`: `Model`, `KeyMap` |
| `.../cursor` | `tea::bubbles::cursor` |
| `.../spinner` | `tea::bubbles::spinner`: every spinner, `Spinner::DOT` and the rest |
| `.../paginator` | `tea::bubbles::paginator` |
| `.../textinput` | `tea::bubbles::textinput`: passwords, suggestions, validation |
| `.../textarea` | `tea::bubbles::textarea`: soft wrap, line numbers, selection |
| `.../viewport` | `tea::bubbles::viewport`: keys, mouse wheel, soft wrap, gutter |
| `.../list` | `tea::bubbles::list`: delegates, fuzzy filtering, pages, help |
| `.../table` | `tea::bubbles::table` |
| `.../progress` | `tea::bubbles::progress`: gradients, spring animation |
| `.../timer`, `.../stopwatch` | `tea::bubbles::timer`, `tea::bubbles::stopwatch` |

A component is a struct with `update(&mut self, &tea::Msg<X>) ->
tea::Cmd<X>` and `view(&self) -> String`, where `X` is its own typed
message. A parent keeps its children's messages in its own enum, maps
their commands up and routes messages down:

| Go | petitcarnets |
| --- | --- |
| `ti := textinput.New(); ti.Focus()` | `let mut ti = textinput::Model::new(); let cmd = ti.focus();` |
| `m.input, cmd = m.input.Update(msg)` | `let cmd = self.input.update(&msg);` |
| `case spinner.TickMsg:` | `tea::Msg::App(Msg::Spinner(tick)) =>` |
| a child's `tea.Cmd` returned as is | `child_cmd.map(Msg::Spinner)` |
| the parent's `msg` passed down as is | `msg.route(\|m\| ...)` gives the child's `tea::Msg<X>` |
| `key.Matches(msg, m.keys.Up)` | `m.keys.up.matches(&key)` or `key::matches(&key, &[&up])` |

Components that tick (spinners, the cursor's blink, the progress bar's
animation, timers) carry an id, as in Bubbles, so several in one parent
take only their own ticks. Text is measured by display width with the
program's width method: no component splits a grapheme cluster or a
wide character. The examples `textinput`, `textinputs`, `textarea`,
`spinners`, `list-simple`, `list-default`, `pager`, `progress-static`,
`progress-animated`, `table`, `help`, `timer`, `stopwatch`, `paginator`
and `chat` are Bubble Tea's own, ported.

## Testing

`petitcarnets::testing::TestProgram` is Bubble Tea's teatest: it runs a
model in the same loop, renderer and terminal modes as a real program,
in an emulated terminal, on a virtual clock. Timers, the frame rate and
the escape timeout move only when the test moves time, so a test that
waits ten seconds of program time takes microseconds, and gives the same
result every run. Tasks still run on their own threads; the harness waits
for each, and delivers their messages in the order they started.

```rust
use petitcarnets as tea;
use std::time::Duration;
use tea::testing::{TestProgram, assert_golden};

// `Counter` as in the example above.

#[test]
fn counts_up() {
    let mut test = TestProgram::new(Counter(0), 40, 10);
    test.press(tea::Key::new(tea::KeyCode::Up));
    assert!(test.screen().text().starts_with("Count: 1"));
    // tests/golden/counter.golden: the screen, the cursor and the modes.
    assert_golden("counter", &test.screen().snapshot());
    test.type_text("q");
    assert!(test.wait_finished(Duration::from_secs(1)));
    assert_eq!(test.final_model().0, 1);
}
```

| teatest (Go) | petitcarnets |
| --- | --- |
| `teatest.NewTestModel(t, m, teatest.WithInitialTermSize(w, h))` | `TestProgram::new(m, w, h)` |
| `teatest.WithProgramOptions(...)` | `TestProgram::with_terminal(program, TestTerminal::new(w, h))` |
| `tm.Send(msg)`, `tm.Type(s)` | `test.send(msg)`, `test.type_text(s)`, `test.press(key)`, `test.bytes(b)` |
| `teatest.WaitFor(t, tm.Output(), cond)` | `test.wait_for(within, \|t\| cond)`, `test.advance(d)` |
| `tm.Quit()`, `tm.WaitFinished(t)` | `test.quit()`, `test.wait_finished(within)` |
| `tm.FinalModel(t)`, `tm.FinalOutput(t)` | `test.final_model()`, `test.final_output()`, `test.final_screen()` |
| `teatest.RequireEqualOutput(t, out)` | `assert_golden(name, &escape(test.final_output()))` |

Set `PETITCARNETS_UPDATE_GOLDEN=1` to write golden files instead of
comparing with them. `TestTerminal` sets the terminal up: its size,
whether it acts as a real terminal, and replies to the queries a program
writes. `test.screen()` is a small terminal emulator (`Vt`) with the
text of each row, each cell's style and link, the cursor, and the modes.

## Platforms

Linux and macOS. Windows is planned for a later release.

Minimum supported Rust version: 1.85.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT licence ([LICENSE-MIT](LICENSE-MIT))

at your option.
