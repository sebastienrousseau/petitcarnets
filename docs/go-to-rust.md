# From Bubble Tea (Go) to petitcarnets

This guide is for people who know [Bubble Tea](https://github.com/charmbracelet/bubbletea)
v2 and want to write the same program in Rust. petitcarnets keeps Bubble
Tea's loop, its command and message names, and its key strings, so most
of a port is mechanical. The parts that are not are the ones Rust's type
system changes: messages are an enum instead of `any`, a model is
changed in place instead of returned, and options are methods instead
of functional arguments.

The names below are those of `charm.land/bubbletea/v2` v2.1.1, of
teatest v2 and of Bubbles v2.2.2, against petitcarnets 0.0.7. Import the
crate as `tea` and the code reads like its Go original:

```rust
use petitcarnets as tea;
```

More of Bubble Tea's own examples, ported one by one, are listed in
[docs/examples.md](examples.md).

## The idioms

### The model is a trait, changed in place

Go's `tea.Model` is an interface of three methods on a value; `Update`
returns the next model and a command. In Rust, `tea::Model` is a trait
whose `update` takes `&mut self`, changes the state where it is, and
returns only the command.

```go
type model int

func (m model) Init() tea.Cmd                           { return nil }
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) { return m, nil }
func (m model) View() tea.View                          { return tea.NewView("hi") }
```

```rust
struct Model(i32);

impl tea::Model for Model {
    type Msg = ();

    fn update(&mut self, msg: tea::Msg<()>) -> tea::Cmd<()> {
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        "hi".into()
    }
}
```

- `init` is optional (it returns `Cmd::none()` unless you write one) and
  takes `&mut self`, so it can set state as well as start work.
- You build the first model yourself and hand it to `Program::new`.
  `run` gives it back with its own type when the program ends: there is
  no `finalModel.(model)` assertion.
- There is no way to return a different model from `update`. A program
  that swaps whole screens keeps them in an enum field of one model.

### Messages are a typed enum

Go's `tea.Msg` is `any`, read with a type switch. petitcarnets' message
is `tea::Msg<A>`, an enum of everything the runtime sends (keys, mouse,
paste, focus, the window size, the terminal's replies) plus one variant,
`tea::Msg::App(A)`, for your own messages. `A` is the trait's associated
type `Msg`: your own enum, or `()` when you have none.

```go
switch msg := msg.(type) {
case tea.KeyPressMsg:
    switch msg.String() {
    case "ctrl+c", "q":
        return m, tea.Quit
    }
case tickMsg:
    m--
}
```

```rust
match msg {
    tea::Msg::KeyPress(k) if k == "ctrl+c" || k == "q" => return tea::quit(),
    tea::Msg::App(Msg::Tick) => self.0 -= 1,
    _ => {}
}
```

- The match is checked by the compiler: a message the model does not
  expect cannot arrive, and there is no downcasting. You can no longer
  send a stray `string` as a message, as Go's tests sometimes do.
- `tea::Msg` is `#[non_exhaustive]`: keep a `_` arm, since more of the
  terminal's replies arrive as the runtime grows.
- A key compares with its Bubble Tea name directly (`k == "ctrl+c"`,
  `k.matches_any(&["q", "esc"])`); `k.to_string()` is Go's
  `msg.String()`.
- The runtime's own control messages (`QuitMsg`, `SuspendMsg`,
  `BatchMsg`, `ClearScreenMsg` and the like) never reach the model:
  they are commands, handled by the runtime.

### Commands are values, often closures

A Go `tea.Cmd` is `func() tea.Msg`, run on a goroutine, with `nil` for
nothing. A petitcarnets `tea::Cmd<A>` is an opaque value:

| Go | petitcarnets |
| --- | --- |
| `return m, nil` | `tea::Cmd::none()` |
| `func() tea.Msg { return fetch() }` | `tea::task(\|\| Msg::Fetched(fetch()))`, or `tea::Cmd::task(...)` |
| `return m, tea.Quit` | `return tea::quit()` |
| `tea.Tick(d, func(t time.Time) tea.Msg { return tickMsg(t) })` | `tea::tick(d, Msg::Tick)` with `Tick(SystemTime)` |
| `tea.Batch(a, b)` | `tea::batch([a, b])` |

- Go passes most built-in commands as function values (`tea.Quit`,
  `tea.ClearScreen`, `tea.RequestWindowSize`); in Rust each is a
  function you call (`tea::quit()`, `tea::clear_screen()`,
  `tea::request_window_size()`) that returns the command.
- Go has no `tea.Task`: a Go command is any `func() tea.Msg`, and
  `tea::task(|| ...)` (the same as `tea::Cmd::task(|| ...)`) is the
  equivalent of returning such a function.
- `tea::task` runs its closure on a thread of its own and sends what it
  returns as `tea::Msg::App`. It must return a message: where Go returns
  `nil` from a command, return a variant your `update` ignores.
- `tick` and `every` are timers on the event loop, not sleeping
  goroutines: a thousand of them cost no threads, and in `TestProgram`
  they follow the virtual clock. Prefer them to a task that sleeps.
- `batch` and `sequence` take anything iterable (an array, a `Vec`),
  and drop `Cmd::none()`s as Go drops `nil`s.

### Returning commands from `update`

Go returns the model and the command together, `return m, cmd`. Here
`update` returns the command alone, so an early `return tea::quit()`
ends the arm, and the last expression of the function is the command
for every other case. Several commands go out as one `tea::batch`.

### The view is a struct with builders

Go's `tea.View` is a struct whose fields you set after `tea.NewView`.
petitcarnets' `tea::View` has the same fields under snake case names,
and a `with_` builder for each, so a view is usually one expression:

```go
v := tea.NewView(s)
v.AltScreen = true
v.MouseMode = tea.MouseModeCellMotion
v.WindowTitle = "notes"
return v
```

```rust
tea::View::new(s)
    .with_alt_screen(true)
    .with_mouse_mode(tea::MouseMode::CellMotion)
    .with_window_title("notes")
```

A `String` or `&str` turns into a view with `.into()`. The fields are
public, so `v.alt_screen = true` works too, but `View` is
`#[non_exhaustive]` and cannot be built with a struct literal outside
the crate; start from `View::new` or `View::default()`.

### Composing components: `Cmd::map` and `Msg::route`

In Go, a parent passes its `tea.Msg` straight to each child and returns
each child's `tea.Cmd` as it is, because both are untyped. In Rust each
component has its own message type, so the parent keeps a variant for
each child's messages, maps the child's commands up into its own type
with `Cmd::map`, and routes messages down with `Msg::route`, which
passes the runtime's messages (keys, sizes) through as they are and
picks out the child's own:

```rust
use petitcarnets as tea;
use tea::bubbles::{spinner, textinput};

enum Msg {
    Spinner(spinner::Msg),
    Input(textinput::Msg),
}

struct Model {
    spinner: spinner::Model,
    input: textinput::Model,
}

impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        tea::batch([
            self.spinner.tick().map(Msg::Spinner),
            self.input.focus().map(Msg::Input),
        ])
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        if let tea::Msg::KeyPress(k) = &msg {
            if *k == "esc" || *k == "ctrl+c" {
                return tea::quit();
            }
        }
        let spin = msg.route(|m| match m {
            Msg::Spinner(t) => Some(*t),
            Msg::Input(_) => None,
        });
        let input = msg.route(|m| match m {
            Msg::Input(c) => Some(*c),
            Msg::Spinner(_) => None,
        });
        let mut cmds = Vec::new();
        if let Some(m) = spin {
            cmds.push(self.spinner.update(&m).map(Msg::Spinner));
        }
        if let Some(m) = input {
            cmds.push(self.input.update(&m).map(Msg::Input));
        }
        tea::batch(cmds)
    }

    fn view(&self) -> tea::View {
        format!("{} {}", self.spinner.view(), self.input.view()).into()
    }
}
```

`Cmd::map` keeps quit, interrupt, output and terminal handovers as they
are, and maps every message a batch, sequence, task, tick, `every` or
`exec` sends. A model with a single child can simply use the child's
message type as its own (`type Msg = textinput::Msg;`), as the ported
`textinput` example does, and needs neither.

### Components update in place, from a borrowed message

A Bubbles component in Go is a value: `m.input, cmd = m.input.Update(msg)`.
In petitcarnets it is a struct with

```rust
fn update(&mut self, msg: &tea::Msg<X>) -> tea::Cmd<X>
fn view(&self) -> String
```

where `X` is the component's own message (`spinner::Msg`,
`textinput::Msg`, ...); a component with no messages of its own (the
viewport, the table, the paginator) is generic over `X`. Because the
message is borrowed, one message can go to several children without
cloning. Components that tick carry an id, as in Bubbles, so two
spinners in one parent each take only their own ticks.

### Options are methods

Go's `tea.NewProgram(m, tea.WithFPS(30), tea.WithColorProfile(p))`
becomes a chain of builder methods on `tea::Program`:

```rust
tea::Program::new(model)
    .with_fps(30)
    .with_color_profile(tea::ColorProfile::Ansi256)
    .run()?;
```

### The accessible mode

`Program::with_accessible(true)` prints frames as plain lines, for
screen readers and braille displays, as huh's `WithAccessible` does:
each line that changed is printed once below what is already there,
nothing is redrawn in place, the cursor is not moved and the alternate
screen is not used. Without the call, it is on when the environment has
a non-empty `ACCESSIBLE` and the program runs in a real terminal;
`with_accessible(false)` turns it off whatever the environment says.
Bubble Tea itself has no such mode. Input works as always, so the input
modes (bracketed paste, modifyOtherKeys, the kitty keyboard flags, and
mouse or focus reports if the view asks) are still set at the start and
reset at the end: output captured to a file holds those few sequences
around the plain lines.

### Errors

`Program.Run` in Go returns an `error` that you compare with
`tea.ErrInterrupted` and the others. `run` returns
`Result<M, tea::Error>`, an enum: `Error::Interrupted`,
`Error::Killed`, `Error::Panic(message)` and `Error::Io(err)`. The
terminal is restored in every case.

## Every Bubble Tea name

Every exported identifier of `charm.land/bubbletea/v2` v2.1.1, as
`go doc -all` lists it, with its petitcarnets counterpart. "Not ported"
means there is nothing of that name; the reason says what to do
instead.

### The model and the program

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `Model` (interface) | `tea::Model` (trait) |
| `Model.Init() Cmd` | `fn init(&mut self) -> Cmd<Self::Msg>` (optional) |
| `Model.Update(Msg) (Model, Cmd)` | `fn update(&mut self, Msg<Self::Msg>) -> Cmd<Self::Msg>` |
| `Model.View() View` | `fn view(&self) -> View` |
| `Program` | `tea::Program<M>` |
| `NewProgram(m, opts...)` | `tea::Program::new(m)`, then builder methods |
| `ProgramOption` | not ported: options are methods on `Program` |
| `(*Program).Run()` | `program.run()`, returning `Result<M, Error>`; `run_with(backend)` for another `Backend` |
| `(*Program).Send(msg)` | `program.sender().send(msg)`: `Sender<A>` sends your own messages, from any thread |
| `(*Program).Quit()` | `sender.quit()` |
| `(*Program).Kill()` | `sender.kill()`; `run` returns `Error::Killed` |
| `(*Program).Wait()` | not ported: `run` blocks its caller; run it on a thread and join it |
| `(*Program).Println(args...)` | not ported: send a message and return `tea::println(s)` from `update` |
| `(*Program).Printf(t, args...)` | not ported: as `Println`, with `tea::printf!` |
| `(*Program).ReleaseTerminal()` | `tea::release_terminal()`, a command |
| `(*Program).RestoreTerminal()` | `tea::restore_terminal()`, a command |
| `ErrInterrupted` | `tea::Error::Interrupted` |
| `ErrProgramKilled` | `tea::Error::Killed` |
| `ErrProgramPanic` | `tea::Error::Panic(message)` |
| `OpenTTY()` | not ported as a function: `tea::Terminal::new()` reads and writes `/dev/tty` when standard input or output is not a terminal |
| `LogToFile(path, prefix)` | not ported: the crate has no logging dependency; open a file and write to it yourself |
| `LogToFileWith(path, prefix, log)` | not ported, as `LogToFile` |
| `LogOptionsSetter` | not ported, as `LogToFile` |

### Program options

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `WithFPS(n)` | `.with_fps(n)` (1 to 120, 60 by default) |
| `WithColorProfile(p)` | `.with_color_profile(p)` |
| `WithEnvironment(env)` | `.with_environment(env)`, with `tea::Environment` |
| `WithInput(r)` | not ported as a reader: `run_with(backend)` takes a `tea::Backend` for input and output |
| `WithOutput(w)` | not ported as a writer: as `WithInput` |
| `WithContext(ctx)` | not ported: keep a `Sender` and call `quit` or `kill` from where the context would be cancelled |
| `WithFilter(fn)` | not ported: filter at the top of `update`, where the model is to hand |
| `WithWindowSize(w, h)` | not ported: in tests, `TestProgram::new(m, w, h)` or `TestTerminal::new(w, h)` sets the size |
| `WithoutCatchPanics()` | not ported: the terminal is always restored on a panic |
| `WithoutRenderer()` | not ported; `.with_accessible(true)` prints frames as plain lines |
| `WithoutSignalHandler()` | not ported: `SIGINT`, `SIGTERM`, `SIGTSTP`, `SIGCONT` and `SIGWINCH` are always handled |
| `WithoutSignals()` | not ported, as `WithoutSignalHandler` |
| (none) | `.without_suspend()`: Ctrl-Z arrives as a key instead of suspending |
| (none) | `.with_accessible(on)`: plain-line output for screen readers |

### Commands

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `Cmd` (`func() Msg`) | `tea::Cmd<A>`; `tea::task(f)` (or `Cmd::task(f)`) for a closure, `Cmd::none()` for `nil` |
| `Batch(cmds...)` | `tea::batch(cmds)` |
| `BatchMsg` | not ported: batches are handled by the runtime and never reach the model |
| `Sequence(cmds...)` | `tea::sequence(cmds)` |
| `Tick(d, fn)` | `tea::tick(d, f)`; `f` gets a `SystemTime` |
| `Every(d, fn)` | `tea::every(d, f)`; fires once, on the next multiple of `d`, as in Go |
| `Quit()`, `QuitMsg` | `tea::quit()`; the model never sees a quit message |
| `Interrupt()`, `InterruptMsg` | `tea::interrupt()`; `run` returns `Error::Interrupted` |
| `Suspend()`, `SuspendMsg` | `tea::suspend()`; Ctrl-Z also suspends by default |
| `ResumeMsg` | `tea::Msg::Resume` |
| `ClearScreen()` | `tea::clear_screen()` |
| `Println(args...)` | `tea::println(s)` |
| `Printf(template, args...)` | `tea::printf!(template, args...)`, a macro |
| `Raw(r)`, `RawMsg` | `tea::raw(bytes)` |
| `Exec(c, fn)` | `tea::exec_command(c, f)` |
| `ExecProcess(cmd, fn)` | `tea::exec(std::process::Command, f)`; `f` gets `io::Result<ExitStatus>` |
| `ExecCommand` (interface) | `tea::ExecCommand` (trait); only `run`: the command inherits the process's standard streams |
| `ExecCallback` | not ported as a type: any `FnOnce(io::Result<..>) -> A` closure |
| `RequestWindowSize()` | `tea::request_window_size()` |
| `RequestCursorPosition()` | `tea::request_cursor_position()` |
| `RequestBackgroundColor()` | `tea::request_background_color()` |
| `RequestForegroundColor()` | `tea::request_foreground_color()` |
| `RequestCursorColor()` | `tea::request_cursor_color()` |
| `RequestTerminalVersion()` | `tea::request_terminal_version()` |
| `RequestCapability(s)` | `tea::request_capability(name)` |
| `SetClipboard(s)` | `tea::set_clipboard(s)` |
| `SetPrimaryClipboard(s)` | `tea::set_primary_clipboard(s)` |
| `ReadClipboard()` | `tea::read_clipboard()` |
| `ReadPrimaryClipboard()` | `tea::read_primary_clipboard()` |
| `RequestProgramStatusSupport()` | not ported: the Program Status Protocol (OSC 7501) is not implemented |
| (none) | `tea::repaint()`: every mode turned off and on, the whole frame drawn again |

### Messages

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `Msg` (`any`) | `tea::Msg<A>`, an enum; your own messages are `tea::Msg::App(a)` |
| `KeyPressMsg` | `tea::Msg::KeyPress(key)` |
| `KeyReleaseMsg` | `tea::Msg::KeyRelease(key)` |
| `KeyMsg` (interface) | not ported: match `KeyPress(k) \| KeyRelease(k)` |
| `KeyPressMsg.Key()`, `KeyReleaseMsg.Key()` | the variant holds the `tea::Key` itself |
| `KeyPressMsg.String()`, `.Keystroke()` | `key.to_string()`, `key.keystroke()` |
| `MouseClickMsg` | `tea::Msg::MouseClick(mouse)` |
| `MouseReleaseMsg` | `tea::Msg::MouseRelease(mouse)` |
| `MouseWheelMsg` | `tea::Msg::MouseWheel(mouse)` |
| `MouseMotionMsg` | `tea::Msg::MouseMotion(mouse)`; its `String()` is `mouse.motion_string()` |
| `MouseMsg` (interface) | not ported: match the four mouse variants |
| `MouseClickMsg.Mouse()` and the others | the variant holds the `tea::Mouse` itself |
| `MouseClickMsg.String()` and the others | `mouse.to_string()` |
| `PasteMsg{Content}`, `PasteMsg.String()` | `tea::Msg::Paste(text)` |
| `PasteStartMsg`, `PasteEndMsg` | `tea::Msg::PasteStart`, `tea::Msg::PasteEnd` |
| `FocusMsg`, `BlurMsg` | `tea::Msg::Focus`, `tea::Msg::Blur` |
| `WindowSizeMsg{Width, Height}` | `tea::Msg::WindowSize(tea::Size { width, height })` |
| `ColorProfileMsg` | `tea::Msg::ColorProfile(profile)` |
| `EnvMsg`, `EnvMsg.Getenv`, `EnvMsg.LookupEnv` | not ported as a message: `tea::Environment::current()`, then `.var(name)` and `.get(name)` |
| `BackgroundColorMsg` | `tea::Msg::BackgroundColor(rgb)` |
| `ForegroundColorMsg` | `tea::Msg::ForegroundColor(rgb)` |
| `CursorColorMsg` | `tea::Msg::CursorColor(rgb)` |
| `.IsDark()` on the three colour messages | `rgb.is_dark()` |
| `.String()` on the three colour messages | not ported: format `rgb.r`, `rgb.g`, `rgb.b` yourself |
| `CursorPositionMsg{X, Y}` | `tea::Msg::CursorPosition { x, y }` |
| `TerminalVersionMsg{Name}`, `.String()` | `tea::Msg::TerminalVersion(name)` |
| `CapabilityMsg{Content}`, `.String()` | `tea::Msg::Capability(content)` |
| `ClipboardMsg{Content, Selection}` | `tea::Msg::Clipboard { content, selection }` |
| `ClipboardMsg.Clipboard()` | `u8::from(selection)`; `selection` is a `tea::ClipboardSelection` |
| `ClipboardMsg.String()` | `content` |
| `ModeReportMsg{Mode, Value}` | `tea::Msg::ModeReport { mode, value }`, with `tea::ReportedMode` and `tea::ModeSetting` |
| `KeyboardEnhancementsMsg{Flags}` | `tea::Msg::KeyboardEnhancements(report)`, a `tea::KeyboardEnhancementsReport` with `flags` |
| `.SupportsKeyDisambiguation()` | `report.supports_key_disambiguation()` |
| `.SupportsEventTypes()` | `report.supports_event_types()` |
| `.SupportsAlternateKeys()` | `report.supports_alternate_keys()` |
| `.SupportsAllKeysAsEscapeCodes()` | `report.supports_all_keys_as_escape_codes()` |
| `.SupportsAssociatedText()` | `report.supports_associated_text()` |
| `ProgramStatusSupportMsg` | not ported: no Program Status Protocol |
| (ultraviolet events Go passes on) | `tea::Msg::PixelSize`, `CellSize`, `PrimaryDeviceAttributes`, `SecondaryDeviceAttributes`, `TertiaryDeviceAttributes`, `ColorScheme` |

### Keys

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `Key` | `tea::Key` |
| `Key.Text` | `key.text` |
| `Key.Mod` | `key.mods` |
| `Key.Code` (a rune) | `key.code`, a `tea::KeyCode`: `KeyCode::Char('a')` or a named key |
| `Key.ShiftedCode`, `Key.BaseCode` | `key.shifted_code`, `key.base_code` (`Option<KeyCode>`) |
| `Key.IsRepeat` | `key.is_repeat` |
| `Key.String()` | `key.to_string()`, or compare: `key == "ctrl+c"` |
| `Key.Keystroke()` | `key.keystroke()` |
| `KeyPressMsg{Code: tea.KeyEnter}` (building one) | `tea::Key::new(tea::KeyCode::Enter)`; also `Key::char('a')`, `Key::ctrl('c')`, `.with_mods(m)` |
| `KeyMod` | `tea::KeyMod` |
| `ModShift`, `ModAlt`, `ModCtrl`, `ModMeta` | `KeyMod::SHIFT`, `KeyMod::ALT`, `KeyMod::CTRL`, `KeyMod::META` |
| `ModHyper`, `ModSuper` | `KeyMod::HYPER`, `KeyMod::SUPER` |
| `ModCapsLock`, `ModNumLock`, `ModScrollLock` | `KeyMod::CAPS_LOCK`, `KeyMod::NUM_LOCK`, `KeyMod::SCROLL_LOCK` |
| `KeyUp`, `KeyDown`, `KeyRight`, `KeyLeft` | `KeyCode::Up`, `Down`, `Right`, `Left` |
| `KeyBegin`, `KeyFind`, `KeyInsert`, `KeyDelete`, `KeySelect` | `KeyCode::Begin`, `Find`, `Insert`, `Delete`, `Select` |
| `KeyPgUp`, `KeyPgDown`, `KeyHome`, `KeyEnd` | `KeyCode::PgUp`, `PgDown`, `Home`, `End` |
| `KeyKpEnter`, `KeyKpEqual`, `KeyKpMultiply`, `KeyKpPlus`, `KeyKpComma`, `KeyKpMinus`, `KeyKpDecimal`, `KeyKpDivide` | `KeyCode::KpEnter`, `KpEqual`, `KpMultiply`, `KpPlus`, `KpComma`, `KpMinus`, `KpDecimal`, `KpDivide` |
| `KeyKp0` to `KeyKp9` | `KeyCode::Kp0` to `KeyCode::Kp9` |
| `KeyKpSep`, `KeyKpUp`, `KeyKpDown`, `KeyKpLeft`, `KeyKpRight`, `KeyKpPgUp`, `KeyKpPgDown`, `KeyKpHome`, `KeyKpEnd`, `KeyKpInsert`, `KeyKpDelete`, `KeyKpBegin` | `KeyCode::KpSep`, `KpUp`, `KpDown`, `KpLeft`, `KpRight`, `KpPgUp`, `KpPgDown`, `KpHome`, `KpEnd`, `KpInsert`, `KpDelete`, `KpBegin` |
| `KeyF1` to `KeyF63` | `KeyCode::F(1)` to `KeyCode::F(63)` |
| `KeyCapsLock`, `KeyScrollLock`, `KeyNumLock`, `KeyPrintScreen`, `KeyPause`, `KeyMenu` | `KeyCode::CapsLock`, `ScrollLock`, `NumLock`, `PrintScreen`, `Pause`, `Menu` |
| `KeyMediaPlay`, `KeyMediaPause`, `KeyMediaPlayPause`, `KeyMediaReverse`, `KeyMediaStop`, `KeyMediaFastForward`, `KeyMediaRewind`, `KeyMediaNext`, `KeyMediaPrev`, `KeyMediaRecord` | `KeyCode::MediaPlay`, `MediaPause`, `MediaPlayPause`, `MediaReverse`, `MediaStop`, `MediaFastForward`, `MediaRewind`, `MediaNext`, `MediaPrev`, `MediaRecord` |
| `KeyLowerVol`, `KeyRaiseVol`, `KeyMute` | `KeyCode::LowerVol`, `RaiseVol`, `Mute` |
| `KeyLeftShift`, `KeyLeftAlt`, `KeyLeftCtrl`, `KeyLeftSuper`, `KeyLeftHyper`, `KeyLeftMeta` | `KeyCode::LeftShift`, `LeftAlt`, `LeftCtrl`, `LeftSuper`, `LeftHyper`, `LeftMeta` |
| `KeyRightShift`, `KeyRightAlt`, `KeyRightCtrl`, `KeyRightSuper`, `KeyRightHyper`, `KeyRightMeta` | `KeyCode::RightShift`, `RightAlt`, `RightCtrl`, `RightSuper`, `RightHyper`, `RightMeta` |
| `KeyIsoLevel3Shift`, `KeyIsoLevel5Shift` | `KeyCode::IsoLevel3Shift`, `IsoLevel5Shift` |
| `KeyBackspace`, `KeyTab`, `KeySpace` | `KeyCode::Backspace`, `Tab`, `Space` |
| `KeyEnter`, `KeyReturn` | `KeyCode::Enter` (Go's two names are one key) |
| `KeyEscape`, `KeyEsc` | `KeyCode::Esc` (likewise) |
| `KeyExtended` | `KeyCode::Extended` |

### Mouse

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `Mouse{X, Y, Button, Mod}` | `tea::Mouse { x, y, button, mods }` (`u16` coordinates) |
| `Mouse.String()` | `mouse.to_string()` |
| `MouseButton` | `tea::MouseButton`; `button.name()` is its Go name |
| `MouseNone`, `MouseLeft`, `MouseMiddle`, `MouseRight` | `MouseButton::None`, `Left`, `Middle`, `Right` |
| `MouseWheelUp`, `MouseWheelDown`, `MouseWheelLeft`, `MouseWheelRight` | `MouseButton::WheelUp`, `WheelDown`, `WheelLeft`, `WheelRight` |
| `MouseBackward`, `MouseForward`, `MouseButton10`, `MouseButton11` | `MouseButton::Backward`, `Forward`, `Button10`, `Button11` |
| `MouseMode` | `tea::MouseMode` |
| `MouseModeNone`, `MouseModeCellMotion`, `MouseModeAllMotion` | `MouseMode::None`, `CellMotion`, `AllMotion` |

### The view

| Bubble Tea (Go) | petitcarnets |
| --- | --- |
| `View` | `tea::View` |
| `NewView(s)` | `tea::View::new(s)`, or `s.into()` |
| `(*View).SetContent(s)` | `view.set_content(s)` |
| `View.Content` | `view.content` |
| `View.AltScreen` | `.with_alt_screen(on)` |
| `View.Cursor` | `.with_cursor(cursor)`; `None` (the default) hides it |
| `View.WindowTitle` | `.with_window_title(t)` |
| `View.ForegroundColor`, `View.BackgroundColor` | `.with_foreground_color(rgb)`, `.with_background_color(rgb)` |
| `View.ProgressBar` | `.with_progress_bar(bar)` |
| `View.MouseMode` | `.with_mouse_mode(mode)` |
| `View.ReportFocus` | `.with_report_focus(on)` |
| `View.DisableBracketedPasteMode` | `.with_bracketed_paste(false)` |
| `View.KeyboardEnhancements` | `.with_keyboard_enhancements(e)` |
| `View.OnMouse` | not ported: hit-test mouse messages in `update` against the state the view was drawn from |
| `View.ProgramStatus` | not ported: no Program Status Protocol (OSC 7501) |
| `KeyboardEnhancements` | `tea::KeyboardEnhancements::new()` |
| `.ReportEventTypes`, `.ReportAlternateKeys`, `.ReportAllKeysAsEscapeCodes`, `.ReportAssociatedText` | `.report_event_types()`, `.report_alternate_keys()`, `.report_all_keys_as_escape_codes()`, `.report_associated_text()` |
| `Cursor` | `tea::Cursor` |
| `NewCursor(x, y)` | `tea::Cursor::new(x, y)` |
| `Cursor.Position` (`Position{X, Y}`) | `cursor.x`, `cursor.y`; there is no separate `Position` type |
| `Position` | not ported: the cursor has `x` and `y` fields |
| `Cursor.Shape`, `Cursor.Blink`, `Cursor.Color` | `.with_shape(s)`, `.with_blink(on)`, `.with_color(rgb)` |
| `CursorShape` | `tea::CursorShape` |
| `CursorBlock`, `CursorUnderline`, `CursorBar` | `CursorShape::Block`, `Underline`, `Bar` |
| `ProgressBar{State, Value}` | `tea::ProgressBar { state, value }` (`value` a `u8`, at most 100) |
| `NewProgressBar(state, value)` | `tea::ProgressBar::new(state, value)` |
| `ProgressBarState` | `tea::ProgressBarState` |
| `ProgressBarNone`, `ProgressBarDefault`, `ProgressBarError`, `ProgressBarIndeterminate`, `ProgressBarWarning` | `ProgressBarState::None`, `Default`, `Error`, `Indeterminate`, `Warning` |
| `ProgressBarState.String()` | not ported: `{:?}` gives the variant's name |
| `ProgramStatus`, `ProgramStatusKind`, `ProgramState` | not ported: no Program Status Protocol |
| `ProgramStateIdle`, `ProgramStateWorking`, `ProgramStateDone`, `ProgramStateBlocked`, `ProgramStateError`, `ProgramStateClear` | not ported, as `ProgramState` |
| `ProgramStatusKindPermission`, `ProgramStatusKindQuestion`, `ProgramStatusKindAuth` | not ported, as `ProgramStatusKind` |

### teatest

| teatest v2 (Go) | petitcarnets |
| --- | --- |
| `NewTestModel(t, m, opts...)` | `tea::testing::TestProgram::new(m, w, h)` |
| `WithInitialTermSize(w, h)` | the `w, h` of `TestProgram::new`, or `TestTerminal::new(w, h)` |
| `WithProgramOptions(opts...)` | `TestProgram::with_terminal(program, TestTerminal::new(w, h))`, with a `Program` built with its methods |
| `TestOption`, `TestModelOptions` | not ported: the arguments above |
| `(*TestModel).Send(msg)` | `test.send(msg)`; keys with `test.press(key)`, bytes with `test.bytes(b)` |
| `(*TestModel).Type(s)` | `test.type_text(s)` |
| `(*TestModel).Output()` | `test.output()`, or the emulated screen with `test.screen()` |
| `WaitFor(t, r, cond, opts...)` | `test.wait_for(within, \|t\| cond)` |
| `WithDuration(d)` | the `within` of `wait_for` |
| `WithCheckInterval(d)`, `WaitForOption`, `WaitingForContext` | not ported: the condition is checked after each step of the virtual clock, nothing polls |
| (time passing) | `test.advance(d)`: moves the virtual clock, so ticks fire without waiting |
| `(*TestModel).Quit()` | `test.quit()` |
| `(*TestModel).WaitFinished(t, opts...)` | `test.wait_finished(within)`, which returns whether it finished |
| `WithFinalTimeout(d)` | the `within` of `wait_finished` |
| `WithTimeoutFn(fn)`, `FinalOpt`, `FinalOpts` | not ported: `wait_finished` returns `false`, and the test decides |
| `(*TestModel).FinalModel(t)` | `test.final_model()`, with the model's own type |
| `(*TestModel).FinalOutput(t)` | `test.final_output()`, `test.final_screen()` |
| `(*TestModel).GetProgram()` | not ported: `test.sender()` for sending from elsewhere |
| `Program` (interface) | not ported, as `GetProgram` |
| `RequireEqualOutput(t, out)` | `assert_golden(name, &escape(test.final_output()))`, or a screen with `assert_golden(name, &test.screen().snapshot())` |

### Bubbles

The components live in `tea::bubbles`, behind the default feature
`bubbles`. Each is a struct named `Model` in a module named after the
Go package.

| Bubbles v2 (Go) | petitcarnets |
| --- | --- |
| `key.NewBinding(key.WithKeys("k", "up"), key.WithHelp("↑/k", "up"))` | `key::Binding::new(&["k", "up"]).with_help("↑/k", "up")` |
| `key.WithDisabled()`, `b.SetEnabled(on)` | `.with_disabled()`, `b.set_enabled(on)` |
| `key.Matches(msg, m.keys.Up)` | `m.keys.up.matches(&key)`, or `key::matches(&key, &[&up, &down])` |
| `help.New()`, `m.help.View(m.keys)` | `help::Model::new()`, `self.help.view(&self.keys)`; the keys implement `help::KeyMap` |
| `spinner.New(spinner.WithSpinner(spinner.Dot))` | `spinner::Model::new().with_spinner(spinner::Spinner::DOT)` |
| `m.spinner.Tick` (as a command) | `self.spinner.tick()` |
| `case spinner.TickMsg:` | `tea::Msg::App(Msg::Spinner(tick))`, with `spinner::Msg` (`TickMsg`) |
| `textinput.New()`, `ti.Focus()` | `textinput::Model::new()`, `let cmd = ti.focus();` |
| `ti.Placeholder = "x"`, `ti.CharLimit = 156`, `ti.SetWidth(20)` | `ti.placeholder = "x".into()`, `ti.char_limit = 156`, `ti.set_width(20)` |
| `ti.Value()`, `ti.SetValue(s)`, `ti.Reset()` | `ti.value()`, `ti.set_value(s)`, `ti.reset()` |
| `textinput.Blink` | `textinput::blink()` |
| `textarea.New()` | `textarea::Model::new()` |
| `viewport.New(viewport.WithWidth(w), viewport.WithHeight(h))` | `viewport::Model::new(w, h)` |
| `vp.SetContent(s)`, `vp.GotoBottom()` | `vp.set_content(s)`, `vp.goto_bottom()` |
| `list.New(items, list.NewDefaultDelegate(), w, h)` | `list::Model::new(items, list::DefaultDelegate::new(), w, h)`; items implement `list::Item` |
| `l.SelectedItem()` | `l.selected_item()`, an `Option<&I>` |
| `table.New(table.WithColumns(c), table.WithRows(r), table.WithFocused(true))` | `table::Model::new().with_columns(c).with_rows(r).with_focused(true)` |
| `table.Column{Title, Width}` | `table::Column::new(title, width)` |
| `progress.New(progress.WithDefaultBlend())` | `progress::Model::new().with_default_blend()` |
| `p.SetPercent(0.5)` | `p.set_percent(0.5)`, a command for the animation |
| `paginator.New()` | `paginator::Model::new()` |
| `timer.New(d)`, `t.Init()` | `timer::Model::new(d)`, `t.init()` |
| `stopwatch.New()`, `s.Init()` | `stopwatch::Model::new()`, `s.init()` |
| `cursor.New()` | `cursor::Model::new()` |
| `m.input, cmd = m.input.Update(msg)` | `let cmd = self.input.update(&msg);` (see [composing](#composing-components-cmdmap-and-msgroute)) |
| `filepicker`, `tree` | not ported |

## A worked example: `examples/simple`

Bubble Tea's `examples/simple` counts down from 5, one second at a
time, and quits at zero or on `q` or Ctrl-C. Here it is in Go:

```go
package main

// A simple program that counts down from 5 and then exits.

import (
	"fmt"
	"log"
	"os"
	"time"

	tea "charm.land/bubbletea/v2"
)

func main() {
	// Log to a file. Useful in debugging since you can't really log to stdout.
	// Not required.
	logfilePath := os.Getenv("BUBBLETEA_LOG")
	if logfilePath != "" {
		if _, err := tea.LogToFile(logfilePath, "simple"); err != nil {
			log.Fatal(err)
		}
	}

	// Initialize our program
	p := tea.NewProgram(model(5))
	if _, err := p.Run(); err != nil {
		log.Fatal(err)
	}
}

type model int

func (m model) Init() tea.Cmd {
	return tick
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyPressMsg:
		switch msg.String() {
		case "ctrl+c", "q":
			return m, tea.Quit
		case "ctrl+z":
			return m, tea.Suspend
		}

	case tickMsg:
		m--
		if m <= 0 {
			return m, tea.Quit
		}
		return m, tick
	}
	return m, nil
}

func (m model) View() tea.View {
	return tea.NewView(fmt.Sprintf("Hi. This program will exit in %d seconds.\n\nTo quit sooner press ctrl-c, or press ctrl-z to suspend...\n", m))
}

type tickMsg time.Time

func tick() tea.Msg {
	time.Sleep(time.Second)
	return tickMsg{}
}
```

### Step 1: the model and its messages

`type model int` becomes a tuple struct. The program's one message of
its own, `tickMsg`, becomes a variant of an enum, which is the model's
`Msg` type. Go's `tickMsg` carries a `time.Time` it never fills in;
the Rust variant carries nothing.

```rust
use petitcarnets as tea;
use std::time::Duration;

struct Model(i32);

enum Msg {
    Tick,
}
```

### Step 2: the command

Go's `tick` is itself a command: a function that sleeps a second on its
goroutine and returns the message. The literal port is a task that
sleeps on its thread:

```rust
fn tick() -> tea::Cmd<Msg> {
    tea::Cmd::task(|| {
        std::thread::sleep(Duration::from_secs(1));
        Msg::Tick
    })
}
```

That works, but `tea::tick` says the same thing without a thread: the
timer runs on the event loop, and in tests it follows the virtual
clock instead of sleeping for real.

```rust
fn tick() -> tea::Cmd<Msg> {
    tea::tick(Duration::from_secs(1), |_| Msg::Tick)
}
```

### Step 3: `Init`, `Update` and `View`

`Init` returns the first tick. `Update`'s type switch becomes one
`match`: the key arms compare the key with its name, and the tick arm
matches `tea::Msg::App(Msg::Tick)`. Each `return m, cmd` becomes
`return cmd`, and `return m, nil` becomes `tea::Cmd::none()`. `View`
formats the same text.

The `ctrl+z` arm can go. In Bubble Tea a program suspends only if it
returns `tea.Suspend` itself; petitcarnets suspends on Ctrl-Z by default,
before `update` sees the key, and sends `tea::Msg::Resume` when the
shell brings the program back. (A program that wants the key builds its
`Program` with `.without_suspend()`, and can then return
`tea::suspend()` from a `"ctrl+z"` arm as Go does.)

```rust
impl tea::Model for Model {
    type Msg = Msg;

    fn init(&mut self) -> tea::Cmd<Msg> {
        tick()
    }

    fn update(&mut self, msg: tea::Msg<Msg>) -> tea::Cmd<Msg> {
        match msg {
            tea::Msg::KeyPress(k) if k == "ctrl+c" || k == "q" => tea::quit(),
            tea::Msg::App(Msg::Tick) => {
                self.0 -= 1;
                if self.0 <= 0 {
                    return tea::quit();
                }
                tick()
            }
            _ => tea::Cmd::none(),
        }
    }

    fn view(&self) -> tea::View {
        tea::View::new(format!(
            "Hi. This program will exit in {} seconds.\n\nTo quit sooner press ctrl-c, or press ctrl-z to suspend...\n",
            self.0
        ))
    }
}
```

### Step 4: `main`

`tea.NewProgram(model(5)).Run()` becomes `tea::Program::new(Model(5)).run()`.
`log.Fatal(err)` prints the error and exits with status 1. petitcarnets
has no `LogToFile`; a program that wants a debug log opens a file and
writes to it.

```rust
fn main() {
    if let Err(err) = tea::Program::new(Model(5)).run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
```

The view is inline, as in Bubble Tea v2: when the program quits, its
last frame stays on screen and the prompt starts on the line below.

### Step 5: the test

Bubble Tea's `examples/simple/main_test.go` has a teatest test that is
disabled: it sleeps 1.2 seconds of real time and compares raw output,
and the comments call it flaky. On `TestProgram`'s virtual clock the
same test is exact and takes no time at all:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tea::testing::TestProgram;

    #[test]
    fn counts_down_and_ignores_keys() {
        let mut test = TestProgram::new(Model(10), 70, 30);
        test.advance(Duration::from_millis(1200));
        test.type_text("I'm typing things, but it'll be ignored by my program");
        test.press(tea::Key::new(tea::KeyCode::Enter));
        assert!(test.screen().text().contains("This program will exit in 9 seconds"));
        test.quit();
        assert!(test.wait_finished(Duration::from_secs(1)));
        assert_eq!(test.final_model().0, 9);
    }

    #[test]
    fn quits_at_zero() {
        let mut test = TestProgram::new(Model(3), 70, 30);
        assert!(test.wait_finished(Duration::from_secs(5)));
        assert_eq!(test.elapsed(), Duration::from_secs(3));
        assert_eq!(test.final_model().0, 0);
    }
}
```

Go's test also sends `"ignored msg"`, a string, as a message. A
petitcarnets model's messages are its own enum, so there is nothing
to port: a message of another type does not compile.

With the step 2 task that sleeps, the second test would wait three real
seconds; with `tea::tick` it finishes at once, at exactly three seconds
of virtual time.

The whole program, steps 1 to 4 together, is the code above in one
file: put it in `examples/simple.rs` of a crate that depends on
petitcarnets and run `cargo run --example simple`.
