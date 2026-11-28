# Architecture

How petitcarnets runs a model: the threads, the loop, the terminal, and
the stages a frame goes through on its way out. Each section names the
files to read. Everything here describes the code as it is; where it
follows Bubble Tea, the Go names are given.

## Threads and data flow

A program has one thread that owns everything (the model, the renderer,
the terminal modes) and a few that only send to it.

```text
 terminal bytes        signals (SIGINT, SIGTERM, SIGWINCH, SIGTSTP, SIGCONT)
      |                    |  handler writes the signal number
      |                    v
      |               wake pipe  <-- STOP / PAUSE / RESUME bytes from the loop
      v                    v
 +---------------------------------+
 | reader thread  (src/reader.rs)  |  poll() on the terminal and the pipe
 | decoder        (src/input*)     |  bytes -> Input, 50 ms escape timeout
 +---------------------------------+
      |  Event::Input
      v
 +----------------+   Event::Done / Event::Panic   +---------------------+
 | mpsc channel   | <----------------------------- | task threads        |
 | (Event<M>)     | <-- Event::Send/Quit/Kill ---- | Sender (any thread) |
 +----------------+                                +---------------------+
      |                                                      ^
      v                                                      | Cmd::task
 +-----------------------------------------------------------|---------+
 | event loop, the program's own thread (src/program/run.rs) |         |
 |                                                           |         |
 |   ready queue (Item) --> model.update(msg) --> Cmd --> executor     |
 |        ^                                          (src/exec.rs)     |
 |        |  timers fire from a deadline heap on this thread           |
 |        +--------------------------------------------------+         |
 |                                                                     |
 |   queue empty --> model.view() --> modes (src/mode*)                |
 |                                --> renderer (src/renderer*)         |
 |                                --> colour profile --> Backend output|
 +---------------------------------------------------------------------+
```

Only `Cmd::task` work runs on threads of its own, one thread per task.
Timers, batches, sequences, output and terminal handovers are all
handled on the loop's thread.

## The event loop

Files: `src/program.rs`, `src/program/session.rs`,
`src/program/run.rs`, `src/program/run/translate.rs`,
`src/program/run/handover.rs`.

`Program::run` is `Program::run_with(Terminal::new())`. `run_with`
enters the backend, sets the process's width method to `WcWidth`, and
calls `Program::start`, which:

1. starts input, with a sink that sends `Event::Input` on the program's
   channel;
2. reads the size, the environment (`Environment::current()` unless
   `with_environment` gave one) and whether the backend is a real
   terminal;
3. picks the colour profile (the one `with_color_profile` set, else
   detected; see [Colour](#colour));
4. writes the start query (`CSI ? 2026 $ p` and `CSI ? 2027 $ p`) if the
   backend is a terminal and `should_query` allows it (not over SSH, not
   in Apple's Terminal, as Bubble Tea gates it);
5. queues `Msg::ColorProfile`, then `Msg::WindowSize`, then runs the
   model's `init` command.

The result is a `Session` (model, channel, `Loop`). `Session::run` is the
whole loop; the test harness drives the same session one step at a time
with `Session::step` and `Session::wake`.

Each turn of `Loop::run` is a `step` then a `wait`:

- `step` pops every ready `Item` in order. A `Msg` goes to the model (the
  renderer first hears window sizes, mode reports and capabilities); the
  returned `Cmd` goes to the executor. An `Output` is applied at once,
  between frames (see [The output queue](#the-output-queue)). A
  `Handover` hands the terminal over or takes it back. `Quit`,
  `Interrupt`, `Kill` and `Panic` end the loop.
- When the queue is empty, `frame` draws if anything changed. A frame is
  drawn at once after a key, mouse event or paste (`translate::is_urgent`);
  otherwise no sooner than one interval (`1 s / fps`, 60 by default, cut
  to 1..=120) after the last, and a frame held back sets `frame_due`.
- `wait` asks the clock to sleep until an event arrives, the earliest
  timer is due, or `frame_due`, whichever is first; then it takes every
  event that is ready and fires the timers that are due. With no timer
  and no held back frame it sleeps in `recv` with no timeout: an idle
  program does not wake.

`translate.rs` turns each `Input` into an `Item`, as Bubble Tea's
`translateInputEvent` does, plus the signals: `SIGINT` is `Interrupt`,
`SIGTERM` is `Quit`, `SIGTSTP` is a suspend, and `SIGCONT` after a stop
from outside is `Handover::Continue`. Ctrl-Z becomes a suspend unless
the program was built with `without_suspend`.

A graceful end (quit or interrupt) draws the last frame, reads the
replies still due, and calls `Renderer::finish`; a kill or a panic calls
`Renderer::abandon` instead. `Session::close` then turns every mode off
and maps the end to `Ok(model)`, `Error::Interrupted`, `Error::Killed` or
`Error::Panic`.

### Handing the terminal over

`handover.rs` serves `exec`, `suspend`, `release_terminal` and
`restore_terminal`, and a continue after a stop from outside. Handing
over (`hand_over`) reads the replies to recent queries first (`drain`),
erases an inline frame, turns every mode off, then calls
`Backend::release`. Taking back (`take_back`) calls `Backend::restore`,
turns the modes on again, sends a changed window size to the model, and
marks the next frame as due at once, drawn in full.

`exec` runs its command on the loop's thread between the two, so the
callback's message is pushed to the front of the queue and is the next
the model sees. A suspend calls `Backend::suspend` between the two and
then delivers `Msg::Resume`.

`drain` only waits for a real terminal, and only for queries written in
the last second (Bubble Tea's `queryReplyWindow`). It writes a primary
device attributes query (`CSI c`) as a sentinel, since terminals answer
in order, and reads until that reply comes or 500 ms pass (Bubble Tea's
`queryDrainTimeout`). Before a handover, events read meanwhile are kept
for the model; at the end they are dropped.

## The executor and the clock

Files: `src/exec.rs`, `src/clock.rs`, `src/cmd.rs`, `src/cmd/*.rs`.

A `Cmd` is a tree (`Inner`): none, quit, interrupt, batch, sequence,
task, tick, every, output, or handover. The `Executor` keeps:

- `ready`, the queue of `Item`s the loop pops;
- a `BinaryHeap` of timers keyed by deadline, so a thousand ticks are a
  thousand heap entries and no threads; `every` is due at the next whole
  multiple of its period on the system clock, as in Bubble Tea;
- bookkeeping for batches (a count of children left) and sequences (the
  commands left), driven by `settle`: a sequence starts its next command
  only when the one before reports done;
- a count of running tasks. `spawn` starts a `std::thread` per task,
  runs the work under `catch_unwind`, and sends `Event::Done` or
  `Event::Panic`.

`Cmd::map` (`src/cmd/map.rs`) wraps every message a command will send in
a function, leaving quit, interrupt, output and handovers as they are.

The loop never reads `Instant::now()` itself; it asks a `Clock`.
`RealClock` waits in the channel (`recv` or `recv_timeout`).
`VirtualClock` never sleeps: `wait` takes the events already there,
then waits only for tasks still running (up to 30 seconds), and returns
them sorted by task id, so task results arrive in the order the tasks
started. Time on a `VirtualClock` moves only when a test moves it; its
wall clock starts at midnight UTC on a whole day, so `every` lands on
whole seconds, minutes and hours.

## The Backend trait and the real terminal

Files: `src/backend.rs`, `src/terminal.rs`.

`Backend` is public: `enter`, `leave`, `size`, `start_input`, `output`,
and defaults for `set_mode` (write the mode's sequence), `is_terminal`
(false), `release`, `restore` and `suspend` (nothing). A backend that is
not a terminal is never queried, never waited for, and gets every colour
as it is unless the program set a profile.

`Terminal` is the Unix implementation. It reads from standard input and
draws on standard output when they are terminals, else opens
`/dev/tty` for whichever is not, so `my-tui | grep x` still draws on the
screen. `enter` puts the terminal in raw mode through the guard, hides
the cursor, makes the wake pipe and installs the signal handlers.
`start_input` spawns the reader thread. `set_mode` flushes any buffered
frame, then writes the mode through the guard, which records it.
`release` pauses the reader and waits for its acknowledgement (at most a
second), shows the cursor, gives `SIGTSTP` its default handling and puts
the saved terminal mode back at once (`TCSANOW`, keeping unread input).
`restore` does the reverse. `suspend` stops the process group with
`kill(0, SIGTSTP)`; once the process runs again it waits, at most
500 ms, for the `SIGCONT` handler to have counted the continue. `leave`
stops the reader, flushes, restores the terminal, and only then gives
the signals their default handling back, so a second Ctrl-C during
shutdown cannot leave the terminal raw.

## The terminal guard

Files: `src/guard.rs`.

`TerminalGuard::enter` saves the terminal's mode (`tcgetattr`) in a
process-wide `Mutex`, installs a panic hook once, and sets raw mode
(`cfmakeraw`). Every mode turned on is recorded with its leave sequence,
in order, the leave sequence first so a failed write is still undone.
Restoring writes the leave sequences newest first and puts the saved
mode back. It runs:

- from `Terminal::leave`, on every normal end and error;
- when the guard drops, including while a panic unwinds through `run`;
- from the panic hook, before the panic message prints, for a panic on
  any thread (a task, or `update`) and under `panic = "abort"`. The hook
  uses `try_lock`, in case the panic happened while the lock was held.

Restoring twice is harmless: the second time finds nothing saved.
`cook` and `raw` switch the saved mode and raw mode at once around a
handover, keeping the recorded modes.

## sys.rs: the only unsafe code

Files: `src/sys.rs`, and `src/sys/pty.rs`, a pseudo-terminal for the
tests only.

The crate has no dependencies. `Cargo.toml` sets `unsafe_code = "deny"`,
and `src/sys.rs` is the only module that allows it, with a `SAFETY`
argument at each call. It declares by hand the libc functions it needs:
`tcgetattr`, `tcsetattr`, `cfmakeraw`, `ioctl` (window size), `isatty`,
`pipe`, `read`, `write`, `close`, `fcntl`, `poll`, `signal` and `kill`.

- `termios` differs between platforms, so it is never read: it is an
  opaque, over-sized, 8-byte-aligned buffer that `tcgetattr` fills,
  `cfmakeraw` edits and `tcsetattr` reads back.
- Signals use `signal(2)`, not `sigaction`, whose struct differs between
  libcs. The handler only writes the signal number to the wake pipe (a
  non-blocking write end) and counts `SIGCONT` in an atomic.
- Variadic functions (`ioctl`, `fcntl`) are declared variadic, for Apple
  Silicon's calling convention.
- Signal numbers for `SIGTSTP` and `SIGCONT` are per platform.

## The reader thread

Files: `src/reader.rs`.

The reader waits in `poll` on the terminal and the wake pipe, not in
`read`, so stopping or pausing never swallows a byte meant for whatever
runs next. Its only timeout is the escape timeout while a lone ESC or a
cut off sequence waits. The pipe carries:

- a signal number, turned into `Input::Resize` (with the size read
  again), `Interrupt`, `Terminate`, `Suspend` or `Continue`;
- `STOP` (0), to end the thread;
- `PAUSE` (0xF0): the reader flushes what it holds, acknowledges, and
  waits on the pipe alone, dropping signals, until `RESUME` (0xF1) or
  `STOP`. The terminal's bytes stay unread for the program it was handed
  to.

## The input decoder

Files: `src/input.rs`, `src/input/*.rs`, `src/key.rs`,
`src/key/code.rs`, `src/mouse.rs`.

A port of ultraviolet's `EventDecoder` (the Unix paths, Bubble Tea's
zero-value legacy flags, no terminfo) and of the scanner in its
`TerminalReader`.

- `decode` is a pure function from bytes to one event and its length,
  dispatching on the first byte: ESC and the 8-bit introducers go to the
  CSI (`csi.rs`, with `params.rs`), SS3 (`keys.rs`), OSC, DCS, APC, PM
  and SOS (`strings.rs`) parsers; control bytes and text to
  `control.rs`. Kitty keyboard reports are in `kitty.rs`, SGR and X10
  mouse in `mouse.rs`. A run of ESCs is unrolled into Alt modifiers
  rather than recursed.
- `Scanner` (`scanner.rs`) first looks a whole read up in Go's legacy
  key table (`table.rs`), holds back a sequence that may be cut off,
  gathers a bracketed paste across reads into one `Paste`, and reads
  what it held as it is after `ESC_TIMEOUT` (50 ms, Go's
  `DefaultEscTimeout`).
- Unknown sequences are read and dropped, never turned into keys.

`Key` is Bubble Tea's struct (text, `KeyMod`, `KeyCode`, shifted and
base codes, repeat); it prints as Go's `Key.String()`, so `k == "ctrl+c"`
works. Grapheme clusters are split as `rivo/uniseg` splits them
(`src/unicode/grapheme.rs`), so a typed emoji is one key. The decoder is
checked against a fixture written by Go (`scripts/input-fixture`,
`tests/fixtures/input.tsv`) and by a deterministic fuzz test.

## Modes

Files: `src/mode.rs`, `src/mode/active.rs`, `src/view.rs`.

A `View` carries its content and the terminal state it wants: alternate
screen, cursor (position, shape, colour), window title, foreground and
background colours, progress bar, mouse mode, focus reports, bracketed
paste, keyboard enhancements. `Modes` keeps what is on and, for each
frame, `apply` turns on and off only what differs from the last, in the
order Bubble Tea writes them. Some modes are held for the whole run
rather than per view: autowrap off (from the first frame), grapheme
widths (mode 2027, once the terminal says it has them), the alternate
screen, and the saved title (pushed on the title stack the first time a
title is set). `release` turns everything off newest first and
remembers it; `resume` turns it back on in the first order and writes
the title again; `clear` turns everything off at the end. Switching
screens pops the kitty keyboard entry first, since each screen keeps its
own.

Every mode goes through `Backend::set_mode`, so on the real terminal the
guard records it and undoes it however the program ends.

## The renderer

Files: `src/renderer.rs`, `src/renderer/*.rs`.

Bubble Tea v2's cursed renderer, on a cell buffer.

- **Layout** (`layout.rs`, `cell.rs`): the view's text is read token by
  token (`src/ansi.rs`, split where Go's `ansi.DecodeSequence` splits),
  into rows of cells: a grapheme, its width, its SGR style (`Pen`) and
  its OSC 8 hyperlink. Tabs expand to every eight columns, wide
  characters are never split, and each row is cut at the window's edge.
  Each row has a hash. Inline, a frame taller than the window keeps its
  bottom rows; on the alternate screen it is cut to the window.
- **Diff** (`diff.rs`): unchanged rows are skipped by hash; a changed row
  rewrites only the spans of cells that differ, and erases a shorter
  line's old tail with one sequence when that is shorter. An unchanged
  frame with the cursor in the same place writes nothing.
- **Scroll** (`scroll.rs`): on the alternate screen, a frame that is the
  last one moved up or down is scrolled by the terminal (SU and SD,
  inside a scroll region when only part of the screen moved). The
  renderer computes both the plain diff and the scroll and keeps the
  shorter.
- **Insert above** (`insert.rs`): `println` makes room above an inline
  frame and writes its lines there, counting wrapped rows as the
  terminal wraps them, wide characters included. On the alternate
  screen it prints nothing, as in Bubble Tea.
- **Writer** (`writer.rs`): writes moves and styles while tracking the
  terminal's cursor and pen, choosing the shortest move. It never moves
  with tabs or backspaces and never counts on `\n` becoming `\r\n`.
  Inline it uses relative moves only; on the alternate screen, absolute
  ones too. Every cell's style and link go through the colour profile
  here.
- **Synchronized output**: when the terminal answers the start query
  with mode 2026 known and reset (available, not yet on), each frame
  that writes anything is wrapped in `CSI ? 2026 h` and `l`. Without it,
  a frame that moves a shown cursor hides it first and shows it after.

`finish` (quit or interrupt) leaves every row of an inline frame on
screen and the cursor at the start of a fresh line below it; an empty
last view leaves nothing. This differs on purpose from Bubble Tea
v2.2.0. `abandon` (kill or panic) ends as Bubble Tea does. `release`
erases an inline frame before a handover, and the next frame is drawn in
full. `repaint` and `clear_screen` force a full frame.

### Accessible output

Files: `src/renderer/lines.rs`, `Loop::view_of` in `src/program/run.rs`,
`Modes::keep_autowrap` in `src/mode/active.rs`.

`Program::with_accessible(true)`, or a non-empty `ACCESSIBLE` variable
on a real terminal, swaps the cell-diff renderer for a line printer, for
screen readers and braille displays (Bubble Tea's issue #780). Each
frame prints, once and as plain text ended with `\r\n`, every non-blank
line that differs from the line in the same place in the frame before.
There is no cursor movement, no alternate screen, no synchronized
output, and autowrap is left on. The colour profile is `NoTty` unless
the program set one. `println` prints every line, blank ones included.
The renderer keeps its cell-diff state but hands `render` and
`insert_above` to the line printer; `Loop::view_of` takes the alternate
screen and the cursor out of every view, so the modes never ask for
them, and the start query (modes 2026 and 2027) is not written. A
repaint or a terminal taken back after a handover prints the whole
frame again. On a real terminal the cursor, hidden by the terminal guard
when it is entered, is shown before the first line.

The input modes are not taken out: input works as it always does, so
bracketed paste (`\e[?2004h`), modifyOtherKeys (`\e[>4;2m`) and the
kitty keyboard flags (`\e[>1u`) are set before the first line and reset
at exit (`\e[>4m`, `\e[<u`, `\e[?2004l`), and mouse tracking, focus
reports and the window title are written when the view asks for them.
On a real terminal the guard's `\e[?25l` on entry and `\e[?25h` on exit
are written too. Output captured to a file (a `script` log, or a
`Backend` given to `run_with`) therefore holds these few sequences at
its start and end, around the plain lines.

## The output queue

Files: `src/program/run.rs` (`output`, `print`, `redraw`),
`src/program/run/handover.rs` (`restore`), `src/cmd.rs` (`Output`).

`println`, `printf!`, `raw`, `clear_screen`, `repaint`, the terminal
queries and `request_window_size` are commands whose result is an
`Output` item in the same ready queue as messages, so they are applied in
program order, between frames:

- `Print` first draws the frame for the state the lines follow (unless
  that frame enters the alternate screen), then inserts the lines above
  it. A `println` returned from the update that leaves the alternate
  screen is therefore printed on the normal screen, after the switch.
  When the lines are taller than the room above the frame, they push it
  off the screen, and the frame is drawn again in the next frame, with
  no update needed (Bubble Tea's
  issue #1822).
- `Raw` and `Query` are written as they are; the time is noted, so the
  replies are read before the terminal is handed over or the program
  ends.
- `ClearScreen` and `Repaint` force a full frame and write the modes
  again; `WindowSize` reads the size and sends it to the model.

While the terminal is released, output is held back in `held_back` and
no frame is drawn (a frame held back by the frame rate is dropped rather
than waking the loop). `restore_terminal` writes the held output in
order. Quitting while released takes the terminal back first and writes
it; a kill drops it.

## Colour

Files: `src/color.rs`, `src/color/profile.rs`, `src/color/convert.rs`,
`src/environ.rs`.

`ColorProfile` is `NoTty`, `Ascii`, `Ansi`, `Ansi256` or `TrueColor`.
For a real terminal it is detected from the program's `Environment` as
Go's colorprofile detects it (`NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`,
`COLORTERM`, `TERM` including `dumb`, `TTY_FORCE`, `WT_SESSION`,
`GOOGLE_CLOUD_SHELL`, and `tmux info` inside tmux). A backend that is
not a terminal gets `TrueColor`. A reply to a capability query naming
`RGB` or `Tc` upgrades the profile to `TrueColor` and sends
`Msg::ColorProfile`, unless the program set the profile.

Every colour written comes down to the profile in one place, the
renderer's writer, which frames and `println` lines both go through:
24-bit colours to the 256-colour palette by go-colorful's HSLuv
distance (`Convert256`), then to 16 by a table (`Convert16`). `Ascii`
keeps attributes and drops colours; `NoTty` drops attributes and
hyperlinks too. The view's terminal colours (OSC 10, 11, 12) come down
in `Modes`, and are left out when the profile shows no colour. The
tables (`src/color/*.in`) and `tests/fixtures/color.tsv` are generated
from Go by `scripts/color-fixture`.

## Unicode widths

Files: `src/unicode.rs`, `src/unicode/width.rs`,
`src/unicode/width/*.rs`, `src/unicode/grapheme.rs`, `src/style.rs`
(`width_method`).

Text width is measured as Go's `charmbracelet/x/ansi` v0.12.0 measures
it: `WcWidth` (each code point on its own, the default) or
`GraphemeWidth` (each grapheme cluster as a whole), chosen when the
terminal reports mode 2027. The method is process-wide (an atomic in
`src/style.rs`), set back to `WcWidth` at the start and end of each run,
and also held by the renderer. The tables are generated from the Go
packages by `scripts/gen-width-tables.py`; `tests/fixtures/width.tsv`
(from `scripts/width-fixture`) holds Go's answers for a few thousand
strings.

## Style

Files: `src/style.rs`, `src/style/*.rs`.

ANSI styles, display width (`width`, `block_width`, `height`), and the
small part of Lip Gloss the components need: `Style::render` (each line
painted, wrapping at a set width, padding, height, lines brought to one
width, border, margins), borders, `join_horizontal`, `join_vertical`,
and `truncate_with`, `cut` and `wrap`, none of which splits a wide
character.

## Components

Files: `src/bubbles.rs`, `src/bubbles/*` (behind the default `bubbles`
feature).

Bubbles v2's components under Go's names: `key`, `help`, `cursor`,
`spinner`, `paginator`, `textinput`, `textarea`, `viewport`, `list`,
`table`, `progress`, `timer`, `stopwatch`. Each is a struct with
`update(&mut self, &Msg<X>) -> Cmd<X>` and `view(&self) -> String`, where
`X` is its own typed message. A parent maps a child's commands up with
`Cmd::map` and routes messages down with `Msg::route`
(`src/msg/route.rs`). Components that tick carry an id, so several in
one parent take only their own ticks. Text is measured with
`style::width`, which follows the program's width method. The unit tests
check every line of `tests/fixtures/bubbles.tsv`, written by
`scripts/bubbles-fixture` from Bubbles itself.

## Testing harness

Files: `src/testing.rs`, `src/testing/program.rs`,
`src/testing/terminal.rs`, `src/testing/vt.rs`, `src/testing/vt/*.rs`,
`src/testing/golden.rs`; `TestBackend` in `src/backend.rs`.

`TestProgram` is Bubble Tea's teatest. It calls the same
`Program::start` as a real run, with a `VirtualClock`, on the harness's
backend, and keeps the `Session`. Each call (`send`, `press`,
`type_text`, `bytes`, `resize`, `interrupt`, `advance` and the rest)
moves the session on with `wake` and `step` until it is idle, then
returns. `advance` moves the clock to each deadline on the way (timers,
held back frames, the escape timeout), so timers fire in order with no
sleeping. A program that is still busy after 100,000 wakes with no time
passing (a zero `tick` that starts itself) is reported with a panic.

`TestTerminal` sets the terminal up: its size, whether it acts as a real
terminal (`terminal()`), and scripted replies (`reply(query, answer)`).
It becomes an internal `Harness` backend whose output records every
byte, feeds a `Vt`, and answers each scripted query through the real
decoder on the virtual clock. `release`, `restore` and `suspend` only
record state.

`Vt` is a small terminal emulator following xterm's rules: rows, cells
with style and link, cursor, modes, title, scrollback, kitty keyboard
flags, and `snapshot` as stable text. It logs anything it does not
understand in `unknown`, including tabs and backspaces, which the
renderer never sends.

`assert_golden(name, actual)` compares with `tests/golden/<name>.golden`,
or writes it with `PETITCARNETS_UPDATE_GOLDEN=1`; `escape` turns raw
output into readable text for one.

`TestBackend` is the older way: scripted keys, inputs or bytes on the
real clock, for `Program::run_with`.

Outside the crate, `scripts/pty-smoke.sh` runs the examples in a real
pseudo-terminal (Linux, in CI) and checks that the terminal comes back
byte for byte for every way a program ends.
