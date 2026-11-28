# Migrating from bubbletea-rs

[bubbletea-rs](https://crates.io/crates/bubbletea-rs) is another Rust
take on Bubble Tea, with its components in
[bubbletea-widgets](https://crates.io/crates/bubbletea-widgets). This
guide moves a program from it to petitcarnets. It covers bubbletea-rs
0.0.9 and bubbletea-widgets 0.1.12, the latest versions on crates.io
when it was written (both published on 2025-11-22; their repositories,
per their manifests, are `whit3rabbit/bubbletea-rs` and
`whit3rabbit/bubbles-rs`). Everything said here about bubbletea-rs
comes from reading the source of those versions; where a behaviour
could not be checked, it is left out.

For the Go names, see [docs/go-to-rust.md](go-to-rust.md): petitcarnets
follows Bubble Tea v2 closely, and that guide maps every one of its
names.

## At a glance

| | bubbletea-rs 0.0.9 | petitcarnets 0.0.7 |
| --- | --- | --- |
| Dependencies | tokio, crossterm, futures, async-trait, tokio-util, parking_lot and others: 53 crates in `cargo tree` for the counter below | none: `cargo tree` prints one crate |
| Runtime | async, on tokio (`#[tokio::main]`, `run().await`) | none: `run()` blocks; tasks are closures on threads, timers on the event loop |
| A message | `Msg = Box<dyn Any + Send>`, read with `downcast_ref` | `tea::Msg<A>`, an enum; your own messages are `tea::Msg::App(A)` |
| A command | `Cmd = Pin<Box<dyn Future<Output = Option<Msg>> + Send>>`; `Option<Cmd>` for none | `tea::Cmd<A>`; `Cmd::none()` for none |
| The model | `init()` builds it: `fn init() -> (Self, Option<Cmd>)` | you build it; `fn init(&mut self) -> Cmd<A>`, optional |
| The view | `fn view(&self) -> String` | `fn view(&self) -> tea::View`: the text and the terminal modes |
| Keys | crossterm's `KeyCode` and `KeyModifiers` | `tea::Key`, compared with Bubble Tea's names: `k == "ctrl+c"` |
| Terminal modes | builder options, and commands such as `enter_alt_screen()` | fields of each frame's `View`, set with `with_` methods |
| Testing | `DummyTerminal`, custom input sources, calling `update` directly | `TestProgram`: the real loop on a virtual clock, in an emulated terminal |

## The model

bubbletea-rs's `Model` trait builds the model in an associated
function, so `Program` creates it:

```rust
// bubbletea-rs 0.0.9
pub trait Model: Send + Sized + 'static {
    fn init() -> (Self, Option<Cmd>);
    fn update(&mut self, msg: Msg) -> Option<Cmd>;
    fn view(&self) -> String;
}
```

petitcarnets' `Model` is given an existing model; `init` is a method
with a default, and the model's own messages are an associated type:

```rust
// petitcarnets
pub trait Model {
    type Msg: Send + 'static;
    fn init(&mut self) -> Cmd<Self::Msg> { Cmd::none() }
    fn update(&mut self, msg: Msg<Self::Msg>) -> Cmd<Self::Msg>;
    fn view(&self) -> View;
}
```

To move a model across:

1. Move the body of `init` that builds `Self` into a constructor (or a
   plain struct literal) and pass the result to `tea::Program::new`.
   Keep the command it returned in `fn init(&mut self)`.
2. Declare `type Msg`: an enum of the messages your commands send, or
   `()` if there are none.
3. Replace each `msg.downcast_ref::<T>()` with a `match` arm (see
   [messages](#messages)), each `Some(cmd)` with `cmd`, and each `None`
   with `tea::Cmd::none()`.
4. Return a `tea::View` from `view`: `.into()` turns a `String` into
   one, and the `with_` methods add what the builder or commands used
   to set (the alternate screen, the mouse, the cursor, the title).

petitcarnets does not ask for the model to be `Send`: it stays on the
loop's thread.

## Messages

In bubbletea-rs any `Send + 'static` value can be a message: a command
boxes it, and `update` tries each type in turn.

```rust
// bubbletea-rs 0.0.9
if let Some(key) = msg.downcast_ref::<KeyMsg>() { /* ... */ }
if msg.downcast_ref::<TickMsg>().is_some() { /* ... */ }
```

In petitcarnets the runtime's messages are variants of `tea::Msg`, and
yours are one enum, so `update` is a single `match` that the compiler
checks:

```rust
// petitcarnets
enum Msg {
    Tick,
}

match msg {
    tea::Msg::KeyPress(k) if k == "q" => return tea::quit(),
    tea::Msg::App(Msg::Tick) => self.count += 1,
    _ => {}
}
```

The runtime's own control messages are not delivered to the model.
In bubbletea-rs 0.0.9, a `QuitMsg` (from `quit()`) and an `InterruptMsg`
go through `update` before the loop ends; in petitcarnets `tea::quit()`
ends the program and `update` never sees it.

## The program

```rust
// bubbletea-rs 0.0.9
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::<Counter>::builder().alt_screen(true).build()?;
    let model = program.run().await?;
    Ok(())
}
```

```rust
// petitcarnets
fn main() -> Result<(), tea::Error> {
    let model = tea::Program::new(Counter { count: 0 }).run()?;
    Ok(())
}
```

No runtime is needed. In an async program, run petitcarnets on a
thread of its own (with tokio, `spawn_blocking`) and talk to it through
a `tea::Sender`.

| bubbletea-rs `ProgramBuilder` | petitcarnets |
| --- | --- |
| `Program::<M>::builder()` ... `.build()?` | `tea::Program::new(model)`, then its methods; nothing to build |
| `.alt_screen(true)` | `view.with_alt_screen(true)`, per frame, so a program can go in and out |
| `.mouse_motion(MouseMotion::Cell)`, `MouseMotion::All`, `MouseMotion::None` | `view.with_mouse_mode(tea::MouseMode::CellMotion)`, `AllMotion`, `None` |
| `.report_focus(true)` | `view.with_report_focus(true)` |
| `.bracketed_paste(true)` (off by default) | on by default; `view.with_bracketed_paste(false)` turns it off |
| `.with_fps(n)` | `.with_fps(n)` (1 to 120) |
| `.without_renderer()` | not ported: `TestProgram` for headless runs; `.with_accessible(true)` for plain-line output |
| `.catch_panics(bool)` | not ported: a panic always restores the terminal and `run` returns `Error::Panic` for a task's panic |
| `.signal_handler(bool)` | not ported: signals are always handled |
| `.input_tty()` | not needed: `/dev/tty` is used when standard input or output is not a terminal |
| `.input(reader)`, `.output(writer)` | `run_with(backend)` with a `tea::Backend`; in tests, `TestProgram` |
| `.context(CancellationToken)` | `program.sender()`, then `sender.quit()` or `sender.kill()` |
| `.filter(f)` | not ported: filter at the top of `update` |
| `.event_channel_buffer(n)` | not ported: the channel is unbounded |
| `.memory_monitoring(true)` | not ported |
| `.with_environment(map)`: variables for children of `exec_process` | different meaning: petitcarnets' `.with_environment(env)` is the environment colours and terminal features are detected from (Bubble Tea's `WithEnvironment`); give a child its variables with `Command::env` |
| (none) | `.with_color_profile(p)`, `.without_suspend()`, `.with_accessible(on)` |

| bubbletea-rs `Program` | petitcarnets |
| --- | --- |
| `program.run().await` | `program.run()`, returning `Result<M, tea::Error>` |
| `program.sender()`, an `EventSender` with `send(Msg) -> Result` | `program.sender()`, a `tea::Sender<A>` with `send(a)` |
| `program.send(msg)` | `program.sender().send(a)` |
| `program.quit()`, `program.kill()` | `sender.quit()`, `sender.kill()` |
| `program.wait().await` | not ported: `run` returns when the program ends (in 0.0.9, `wait` only yields, as its own comment says) |
| `program.release_terminal().await`, `restore_terminal().await` | `tea::release_terminal()`, `tea::restore_terminal()`, commands that work while the program runs (bubbletea-rs's take `&mut self`, and `run` takes the program by value) |
| `program.println(s).await`, `printf(s).await` | `tea::println(s)`, `tea::printf!(...)`, commands that print above an inline frame |
| `memory_monitor()`, `memory_health()` | not ported |

| bubbletea-rs `Error` | petitcarnets `tea::Error` |
| --- | --- |
| `ProgramKilled` | `Killed` |
| `Interrupted` | `Interrupted` |
| `ProgramPanic(String)` | `Panic(String)` |
| `Io(io::Error)` | `Io(io::Error)` |
| `Terminal`, `ChannelSend`, `ChannelReceive`, `ChannelFull`, `ChannelClosed`, `SendError`, `Configuration`, `CommandExecution` | no counterpart: a terminal failure is `Io`; sending to a stopped program does nothing |

## Commands

A bubbletea-rs command is a future, run on tokio. A petitcarnets
command is a value the runtime interprets: a closure for a task
(`tea::Cmd::task(|| ...)`, on a thread), a timer on the event loop, or
an instruction to the runtime. Code that `await`ed in a command moves
into a task's closure as blocking code.

| bubbletea-rs 0.0.9 | petitcarnets |
| --- | --- |
| `Box::pin(async { Some(Box::new(MyMsg) as Msg) })` | `tea::Cmd::task(\|\| Msg::Mine)` |
| `None` (no command) | `tea::Cmd::none()` |
| `quit()` | `tea::quit()` |
| `command::kill()` | no command: `sender.kill()` |
| `interrupt()` | `tea::interrupt()` |
| `suspend()` | `tea::suspend()`; Ctrl-Z also suspends by default |
| `batch(vec![a, b])` | `tea::batch([a, b])` |
| `sequence(vec![a, b])`: awaits each in turn, then delivers the messages together | `tea::sequence([a, b])`: each starts once the one before has sent its message |
| `tick(d, \|d: Duration\| ...)` | `tea::tick(d, \|t: SystemTime\| ...)` |
| `every(d, f)`: keeps firing until cancelled | `tea::every(d, f)`: fires once, on the next multiple of `d` on the clock, as in Bubble Tea; return it again to keep going |
| `every_with_id(d, f)`, `cancel_timer(id)`, `cancel_all_timers()` | not needed: a timer that is not returned again stops |
| `window_size()` | `tea::request_window_size()`; `tea::Msg::WindowSize` also arrives at the start and on every resize |
| `exec_process(cmd, \|output\| ...)`: runs the command with its output captured | `tea::Cmd::task(move \|\| Msg::Done(cmd.output()))` for captured output; `tea::exec(cmd, f)` hands the terminal to the child (an editor, a shell) and takes it back |
| `enter_alt_screen()`, `exit_alt_screen()` | `view.with_alt_screen(on)` |
| `enable_mouse_cell_motion()`, `enable_mouse_all_motion()`, `disable_mouse()` | `view.with_mouse_mode(...)` |
| `enable_report_focus()`, `disable_report_focus()` | `view.with_report_focus(on)` |
| `enable_bracketed_paste()`, `disable_bracketed_paste()` | `view.with_bracketed_paste(on)` |
| `show_cursor()`, `hide_cursor()` | `view.with_cursor(tea::Cursor::new(x, y))`; no cursor (the default) hides it |
| `set_window_title(s)` | `view.with_window_title(s)` |
| `clear_screen()` | `tea::clear_screen()` |
| `println(s)`, `printf(s)` | `tea::println(s)`, `tea::printf!(...)` |

A note on the 0.0.9 source: its program loop acts on the messages from
`quit`, `kill`, `interrupt`, `batch`, `sequence`, `every`,
`cancel_timer`, `cancel_all_timers`, `window_size`, `clear_screen`,
`enter_alt_screen` and `exit_alt_screen`. I found no code that acts on
those from `suspend`, `println`, `printf`, `set_window_title`,
`show_cursor`, `hide_cursor`, or the mouse, focus and bracketed paste
commands: they reach `update` like any other message. In petitcarnets
each of these does what its name says.

## Keys, mouse and the rest

bubbletea-rs passes crossterm's events on: a `KeyMsg` holds crossterm's
`KeyCode` and `KeyModifiers` (and no press or release kind), and a
`MouseMsg` holds crossterm's `MouseEventKind`, so a program depends on
crossterm itself. petitcarnets decodes input as Bubble Tea v2 does, and
a key compares with the name Bubble Tea gives it.

| bubbletea-rs 0.0.9 (crossterm) | petitcarnets |
| --- | --- |
| `KeyMsg { key: KeyCode::Char('q'), .. }` | `tea::Msg::KeyPress(k) if k == "q"` |
| `KeyCode::Char('c')` with `KeyModifiers::CONTROL` | `k == "ctrl+c"` |
| `KeyCode::Up`, `Down`, `Left`, `Right` | `"up"`, `"down"`, `"left"`, `"right"` |
| `KeyCode::Enter`, `Esc`, `Backspace`, `Tab` | `"enter"`, `"esc"`, `"backspace"`, `"tab"` |
| `KeyCode::Char(' ')` | `"space"` |
| `KeyCode::PageUp`, `PageDown`, `Home`, `End` | `"pgup"`, `"pgdown"`, `"home"`, `"end"` |
| `KeyCode::F(1)` | `"f1"` |
| (a key's release) | `tea::Msg::KeyRelease(k)`, with `KeyboardEnhancements::new().report_event_types()` |
| several names at once | `k.matches_any(&["q", "esc"])` |
| the parts of a key | `k.code` (`tea::KeyCode`), `k.mods` (`tea::KeyMod`), `k.text` |
| `MouseMsg { button: MouseEventKind::Down(b), .. }` | `tea::Msg::MouseClick(m)` |
| `MouseEventKind::Up(b)` | `tea::Msg::MouseRelease(m)` |
| `MouseEventKind::Drag(b)`, `Moved` | `tea::Msg::MouseMotion(m)` |
| `MouseEventKind::ScrollUp`, `ScrollDown`, `ScrollLeft`, `ScrollRight` | `tea::Msg::MouseWheel(m)`, `m.button` being `MouseButton::WheelUp` and the rest |
| `MouseMsg { x, y, modifiers, .. }` | `tea::Mouse { x, y, button, mods }` |
| `PasteMsg(String)` | `tea::Msg::Paste(String)` |
| `FocusMsg`, `BlurMsg` | `tea::Msg::Focus`, `tea::Msg::Blur` |
| `WindowSizeMsg { width, height }` | `tea::Msg::WindowSize(tea::Size { width, height })` |
| `ResumeMsg` | `tea::Msg::Resume` |

petitcarnets also delivers what bubbletea-rs's messages do not carry:
whether a key was released or repeated, the terminal's colours, cursor position,
clipboard, capabilities and version, mode reports and the colour
profile (see the messages table in [docs/go-to-rust.md](go-to-rust.md#messages)).

## Components

bubbletea-widgets 0.1.12 and `petitcarnets::bubbles` port the same
Bubbles components, with these modules:

| bubbletea-widgets 0.1.12 | petitcarnets |
| --- | --- |
| `cursor` | `tea::bubbles::cursor` |
| `help` | `tea::bubbles::help` |
| `key` | `tea::bubbles::key` |
| `list` | `tea::bubbles::list` |
| `paginator` | `tea::bubbles::paginator` |
| `progress` | `tea::bubbles::progress` |
| `spinner` | `tea::bubbles::spinner` |
| `stopwatch` | `tea::bubbles::stopwatch` |
| `table` | `tea::bubbles::table` |
| `textarea` | `tea::bubbles::textarea` |
| `textinput` | `tea::bubbles::textinput` |
| `timer` | `tea::bubbles::timer` |
| `viewport` | `tea::bubbles::viewport` |
| `filepicker` | not ported |

What changes:

- **Updating.** A bubbletea-widgets component's `update` takes the boxed
  `Msg` and returns `Option<Cmd>`: by value for most (for the list, the
  table and the viewport, through their `bubbletea_rs::Model`
  implementation), `&Msg` for the cursor, `Option<Msg>` for the text
  area; the paginator's takes `&Msg` and returns nothing, and `help`'s
  takes `self` and returns `(Self, Option<Cmd>)`. Because a `Box<dyn Any + Send>` cannot
  be cloned, a message given by value to one component cannot also be
  given to another. Every petitcarnets component has
  `update(&mut self, &tea::Msg<X>) -> tea::Cmd<X>`, where `X` is its own
  message type, so one message can be lent to as many components as
  need it.
- **Composing.** A parent keeps each child's messages in a variant of
  its own enum, maps the child's commands up with
  `cmd.map(Msg::Child)` and picks the child's messages out with
  `msg.route(...)`; the example in
  [docs/go-to-rust.md](go-to-rust.md#composing-components-cmdmap-and-msgroute)
  puts a spinner and a text input in one model.
- **Focus.** bubbletea-widgets has a `Component` trait (`focus`, `blur`,
  `focused`). petitcarnets has the same three as methods on each
  component that takes focus (`textinput`, `textarea`, `cursor`,
  `table`), with no shared trait.
- **Key bindings.** bubbletea-widgets builds a binding from crossterm
  keys (`Binding::new(vec![KeyCode::Enter])`); petitcarnets from Bubble
  Tea's names, `key::Binding::new(&["enter"]).with_help("enter", "confirm")`,
  matched with `binding.matches(&key)`.
- **Some components are models.** In bubbletea-widgets the list, the
  table, the progress bar, the viewport and the cursor implement
  `bubbletea_rs::Model`. petitcarnets components are never models
  themselves; a program wraps one in its own model.
- **Styling.** bubbletea-widgets depends on `lipgloss-extras`;
  petitcarnets has the part of Lip Gloss the components need in
  `tea::style`, with no dependency.

## Testing

bubbletea-rs 0.0.9 offers a `DummyTerminal` whose operations do
nothing, an `InputSource::Custom` that turns each line of an async
reader into key events, and `.without_renderer()`; its own model tests
call `update` and `view` directly. Timers are tokio intervals, so a
test of a one second tick waits a second.

petitcarnets' `tea::testing::TestProgram` runs the model in the real
loop, renderer and terminal modes, in an emulated terminal, on a
virtual clock: `test.advance(Duration::from_secs(10))` fires ten
seconds of ticks at once. Keys go in with `press` and `type_text`, raw
bytes with `bytes`, your messages with `send`; `screen()` is the
emulated screen, `final_model()` the model at the end, and
`assert_golden` compares a screen with a golden file. See the
[README's Testing section](../README.md#testing).

## A counter, side by side

The same counter in both: up and down change it, `q` or Ctrl-C quits,
on the alternate screen, and the count is printed after the program
ends. Both versions build: the first against bubbletea-rs 0.0.9 (with
`crossterm = "0.29"` and `tokio` with `full` as dependencies of the
program too), the second against petitcarnets 0.0.7 with no other
dependency.

bubbletea-rs 0.0.9:

```rust
use bubbletea_rs::{quit, Cmd, KeyMsg, Model, Msg, Program};
use crossterm::event::{KeyCode, KeyModifiers};

struct Counter {
    count: i64,
}

impl Model for Counter {
    fn init() -> (Self, Option<Cmd>) {
        (Self { count: 0 }, None)
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(key) = msg.downcast_ref::<KeyMsg>() {
            match key.key {
                KeyCode::Char('q') => return Some(quit()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Some(quit());
                }
                KeyCode::Up => self.count += 1,
                KeyCode::Down => self.count -= 1,
                _ => {}
            }
        }
        None
    }

    fn view(&self) -> String {
        format!("Count: {}\n\nup/down to change, q to quit", self.count)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::<Counter>::builder().alt_screen(true).build()?;
    let model = program.run().await?;
    println!("final count: {}", model.count);
    Ok(())
}
```

petitcarnets:

```rust
use petitcarnets as tea;

struct Counter {
    count: i64,
}

impl tea::Model for Counter {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        match msg {
            tea::Msg::KeyPress(k) if k == "q" || k == "ctrl+c" => return tea::quit(),
            tea::Msg::KeyPress(k) if k == "up" => self.count += 1,
            tea::Msg::KeyPress(k) if k == "down" => self.count -= 1,
            _ => {}
        }
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        tea::View::new(format!("Count: {}\n\nup/down to change, q to quit", self.count))
            .with_alt_screen(true)
    }
}

fn main() -> Result<(), tea::Error> {
    let model = tea::Program::new(Counter { count: 0 }).run()?;
    println!("final count: {}", model.count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tea::testing::TestProgram;

    #[test]
    fn counts() {
        let mut test = TestProgram::new(Counter { count: 0 }, 40, 10);
        test.press(tea::Key::new(tea::KeyCode::Up));
        test.press(tea::Key::new(tea::KeyCode::Up));
        test.press(tea::Key::new(tea::KeyCode::Down));
        assert!(test.screen().text().starts_with("Count: 1"));
        test.type_text("q");
        assert!(test.wait_finished(Duration::from_secs(1)));
        assert_eq!(test.final_model().count, 1);
    }
}
```

Step by step:

1. `init` built the model; now `main` builds it and `init` goes, since
   it started no command.
2. `type Msg = ()`: the counter sends no messages of its own.
3. The `downcast_ref::<KeyMsg>()` and the match on crossterm's
   `KeyCode` become arms on `tea::Msg::KeyPress`, compared with key
   names; Ctrl-C is `"ctrl+c"`, not a modifier test.
4. `Some(quit())` becomes `tea::quit()`, and `None` becomes
   `tea::Cmd::none()`.
5. The builder's `.alt_screen(true)` moves into the view, as
   `.with_alt_screen(true)`.
6. `#[tokio::main]`, `build()?` and `.await` go; `run()` returns the
   model.

## Names

| bubbletea-rs 0.0.9 | petitcarnets |
| --- | --- |
| `Model` | `tea::Model` |
| `Msg` | `tea::Msg<A>` |
| `Cmd` | `tea::Cmd<A>` |
| `Batch` | not public: `tea::batch` |
| `Program`, `ProgramBuilder`, `ProgramConfig` | `tea::Program` |
| `MouseMotion` | `tea::MouseMode` |
| `Error` | `tea::Error` |
| `EventSender` | `tea::Sender<A>` |
| `EventReceiver` | not public |
| `KeyMsg` | `tea::Msg::KeyPress(tea::Key)` |
| `MouseMsg` | `tea::Msg::MouseClick`, `MouseRelease`, `MouseWheel`, `MouseMotion` with `tea::Mouse` |
| `PasteMsg` | `tea::Msg::Paste` |
| `WindowSizeMsg` | `tea::Msg::WindowSize` |
| `FocusMsg`, `BlurMsg` | `tea::Msg::Focus`, `tea::Msg::Blur` |
| `ResumeMsg` | `tea::Msg::Resume` |
| `QuitMsg`, `KillMsg`, `InterruptMsg`, `SuspendMsg` | not messages: `tea::quit()`, `sender.kill()`, `tea::interrupt()`, `tea::suspend()` |
| `ClearScreenMsg`, `PrintMsg`, `PrintfMsg`, `RequestWindowSizeMsg` | not messages: `tea::clear_screen()`, `tea::println`, `tea::printf!`, `tea::request_window_size()` |
| `EnterAltScreenMsg`, `ExitAltScreenMsg`, `EnableMouseCellMotionMsg`, `EnableMouseAllMotionMsg`, `DisableMouseMsg`, `EnableReportFocusMsg`, `DisableReportFocusMsg`, `EnableBracketedPasteMsg`, `DisableBracketedPasteMsg`, `ShowCursorMsg`, `HideCursorMsg`, `SetWindowTitleMsg` | not messages: fields of `tea::View` |
| `BatchMsgInternal`, `CancelTimerMsg`, `CancelAllTimersMsg` | not needed |
| `quit`, `interrupt`, `suspend`, `batch`, `sequence`, `tick`, `every`, `clear_screen`, `println` | the same names in `tea` |
| `command::kill` (not re-exported at the root) | `Sender::kill` |
| `printf` | `tea::printf!` |
| `window_size` | `tea::request_window_size` |
| `exec_process` | `tea::Cmd::task` with `Command::output`, or `tea::exec` |
| `every_with_id`, `cancel_timer`, `cancel_all_timers` | not needed |
| `enter_alt_screen`, `exit_alt_screen` | `View::with_alt_screen` |
| `enable_mouse_cell_motion`, `enable_mouse_all_motion`, `disable_mouse` | `View::with_mouse_mode` |
| `enable_report_focus`, `disable_report_focus` | `View::with_report_focus` |
| `enable_bracketed_paste`, `disable_bracketed_paste` | `View::with_bracketed_paste` |
| `show_cursor`, `hide_cursor` | `View::with_cursor` |
| `set_window_title` | `View::with_window_title` |
| `Terminal`, `TerminalInterface` | `tea::Terminal`, `tea::Backend` |
| `DummyTerminal` | `tea::testing::TestProgram`, `TestTerminal`, `TestBackend` |
| `InputHandler`, `InputSource` | not public: input is decoded by the runtime; `TestProgram::bytes` feeds raw bytes in tests |
| `MemoryMonitor`, `MemoryHealth`, `MemorySnapshot` | not ported |
| `log_to_file` | not ported |
| `charm_default_gradient`, `lerp_rgb`, `gradient_filled_segment`, `gradient_filled_segment_with_buffer` | not ported as functions: `tea::bubbles::progress::Model::new().with_default_blend()` draws Bubbles' gradient |
