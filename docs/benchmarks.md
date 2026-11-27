# Benchmarks

## Bytes per frame

How many bytes each renderer writes to the terminal to go from one frame
to the next. Fewer bytes means less work for the terminal and less data
over a slow link such as SSH. Byte counts do not depend on the machine:
every run gave the same numbers.

### What was measured

Five scenarios, each a fixed sequence of 101 frames (a first frame, then
100 more) on an 80x24 screen. The frame text is built the same way in
`scripts/bench/src/scenario.rs`, `scripts/bench/go/scenario.go` and
`scripts/bench/verify.py`; all three print the same FNV-1a hash of every
frame's text, so every implementation draws exactly the same frames.

| scenario | each frame |
|---|---|
| counter | `Count: N` and a help line; N goes up by one |
| list | 20 items; a `>` marker and a bold cyan (SGR) row move down one row (wrapping after the last) |
| log | 24 lines of a log; one new line at the bottom, the oldest drops off the top |
| spinner | one line: a braille spinner glyph that changes, then static text |
| full | 24 rows of 79 letters picked by a hash of column, row and frame; capitals on even frames, small letters on odd ones, so every cell changes and nothing can be scrolled or shifted into place |

Each scenario runs inline (below the prompt) and on the alternate screen.

"Mean" is the mean bytes per frame over the 100 frames after the first.
"First" is everything written up to and including the first frame, set-up
sequences included. Bytes written when the program ends are left out.

### How frames were counted

Each step (a key press for petitcarnets, a message for the others) is
sent only after the previous step's frame has been written, so two steps
never share a frame:

- petitcarnets: a `Backend` (public trait) whose output counts bytes and
  flushes. The renderer writes and flushes each frame once. Each step is
  a key press `j` sent through the backend's input sink; the bench waits
  for the flush, then 5 ms more. Key presses are drawn at once, not held
  for the frame rate.
- Bubble Tea v2: `tea.WithOutput` to a writer that counts bytes and
  `Write` calls; steps go through `p.Send`. Bubble Tea draws on a 60 fps
  ticker, so after each step the bench waits for a write, then 40 ms
  more (over two ticks).
- charmed-bubbletea and bubbletea-rs: as for petitcarnets, with the
  program's own message sender and a writer that counts flushes.

In every run each of the 100 steps produced exactly one flush (Rust) or
one `Write` (Go), no step timed out, and the model saw all 100 steps.

Every run's bytes were also replayed through a small 80x24 terminal model
(`scripts/bench/verify.py`): after each of the 101 frames the screen
showed exactly that frame's text, for all 50 runs. Styles are not checked.

### Results: inline

Mean bytes per frame, first frame bytes in brackets. Measured again for
v0.0.5, after v0.0.4 brought every colour down to the colour profile in
the renderer's writer: every number is the same as for v0.0.3, for
petitcarnets and for the others (the bench's backend is not a terminal,
so the profile is 24-bit colour and nothing is converted).

| scenario | petitcarnets | Bubble Tea v2 (tty) | Bubble Tea v2 (no input) | charmed-bubbletea | bubbletea-rs |
|---|---:|---:|---:|---:|---:|
| counter | 4.2 (81) | 2.3 (102) | 4.3 (62) | 57.9 (67) | 53.9 (53) |
| list | 31.7 (301) | 32.4 (285) | 31.4 (227) | 241.0 (250) | 236.0 (236) |
| log | 659.3 (1281) | 1230.4 (1303) | 1207.4 (1241) | 1258.0 (1267) | 1253.0 (1253) |
| spinner | 4.0 (65) | 5.0 (87) | 5.0 (48) | 37.0 (51) | 37.0 (37) |
| full | 1948.0 (1978) | 1948.0 (2000) | 1925.0 (1938) | 1954.0 (1963) | 1949.0 (1949) |

### Results: alternate screen

| scenario | petitcarnets | Bubble Tea v2 (tty) | Bubble Tea v2 (no input) | charmed-bubbletea | bubbletea-rs |
|---|---:|---:|---:|---:|---:|
| counter | 4.2 (92) | 2.2 (118) | 4.2 (73) | 18.9 (94) | 53.9 (53) |
| list | 31.4 (312) | 31.4 (301) | 30.4 (238) | 50.1 (424) | 236.0 (236) |
| log | 54.0 (1292) | 52.0 (1319) | 52.0 (1252) | 1455.0 (1477) | 1253.0 (1253) |
| spinner | 4.0 (76) | 5.0 (103) | 5.0 (59) | 40.0 (62) | 37.0 (37) |
| full | 1945.0 (1988) | 1945.0 (2015) | 1922.0 (1948) | 2151.0 (2173) | 1949.0 (1949) |

### What is and is not apples to apples

- **Bubble Tea v2 (tty)** is the fair comparison with petitcarnets: input
  is a terminal (a pty from `script`), so Bubble Tea sets raw mode, as in
  real use. It then reads the pty's tab and backspace settings (the usual
  `TAB0` and `BS0`), which let it move the cursor with tabs and
  backspaces where shorter; in these runs only the counter used them
  (backspaces). That is why its counter costs 2 bytes a frame where
  petitcarnets writes 4: one column left is `BS` (1 byte) in Bubble Tea
  and `CSI D` (3 bytes) in petitcarnets.
- **Bubble Tea v2 (no input)** uses `tea.WithInput(nil)`. Without a
  terminal on input, Bubble Tea does not set raw mode and moves down with
  a bare `LF`, relying on the terminal turning it into `CR LF`; that saves
  one byte per row moved (23 bytes on the full redraw). It also skips the
  keyboard set-up. It is shown because it is the easy way to run Bubble
  Tea headless, not as the reference.
- **Log, inline**: petitcarnets rewrites only the cells that changed in
  each row (659 bytes); Bubble Tea rewrites each row from its first
  changed column (1230). On the alternate screen both scroll the screen
  up one line and draw the new line (54 and 52 bytes).
- **Spinner**: Bubble Tea writes the glyph and the space after it
  (5 bytes); petitcarnets writes the glyph only (4).
- **First frame** contents differ. Bubble Tea asks the terminal about
  modes 2026 and 2027 (16 bytes) in both of its runs, and with a tty also
  sets up keyboard enhancements and asks about them. petitcarnets turns
  off autowrap and sets up bracketed paste and keyboard enhancements; it
  does not ask about modes 2026 and 2027 here, because the bench's
  backend keeps `Backend::is_terminal` at its default (no); the
  real terminal backend asks.
- **Synchronised output** (mode 2026) was off in every run, because no
  terminal answered the query. In a terminal that has it, each frame is
  also wrapped in `CSI ? 2026 h` and `CSI ? 2026 l`. Not measured here.
- **Bubble Tea environment**: pinned in the program with
  `tea.WithEnvironment([]string{"TERM=xterm-256color"})` (no
  `TERM_PROGRAM`, no `SSH_TTY`, so it always asks about 2026 and 2027),
  `tea.WithColorProfile(colorprofile.TrueColor)` (so the list's SGR is
  kept although the output is not a terminal) and
  `tea.WithWindowSize(80, 24)`. The caller's environment does not change
  the output; with `-tty`, the pty's tab and backspace settings do.
- **charmed-bubbletea 0.3.0** runs headless: `Program::with_output` turns
  on its custom I/O mode (no raw mode, no event polling) and
  `Program::start` returns a `ProgramHandle` for sending messages. It
  rewrites every changed line in full (`CUP`, `EL`, the line) on the
  alternate screen and the whole view every frame inline. It does not
  use the window size, so 80x24 only bounds what the frames contain.
- **bubbletea-rs 0.0.9** can render to a writer
  (`ProgramBuilder::output`, a tokio `AsyncWrite`), but `Program::run`
  always calls `Terminal::enter_raw_mode`, which calls crossterm's
  `terminal::enable_raw_mode` and fails without a terminal
  ("No such device or address"). It was measured under `script`. Its
  `Terminal::render` writes `CSI H CSI 2J` and the whole view every frame,
  inline or not; the alternate screen and cursor sequences go to the
  process's stdout, not to the writer, so they are missing from its first
  frame. It draws only after a message, so the bench sends one no-op
  message before the first frame.
- Steps reach petitcarnets as key presses and the others as messages;
  what is written depends only on the frame text, which is the same.

### Environment

- 2026-11-27, Linux 7.3.5-2-cachyos, AMD Ryzen AI 9 HX 370 w/ Radeon 890M
- rustc 1.100.0 (0ef1bc386 2026-11-09), petitcarnets 0.0.5 (working tree
  of `feat/v0.0.5`), charmed-bubbletea 0.3.0, bubbletea-rs 0.0.9
- go1.27.4 linux/amd64, Bubble Tea v2.2.0 (`charm.land/bubbletea/v2`,
  tag `v2.2.0`, commit f510168f)

### Reproduce

The Rust bench is its own Cargo project (an empty `[workspace]` keeps it
out of the petitcarnets package):

```sh
cd scripts/bench
cargo run --release                                  # all three Rust ports
cargo run --release --no-default-features            # petitcarnets only
cargo run --release -- --frames 100 --only petitcarnets --dump /tmp/frames
```

bubbletea-rs needs a terminal and writes its cursor and alternate screen
sequences to stdout; without one (CI, an agent) it reports why and is
skipped. To run it without a terminal:

```sh
script -qec "cargo run --release" /dev/null > rust.txt
```

The Go bench is its own module. `go.sum` has every hash except Bubble
Tea's, which `-mod=mod` adds on the first online run:

```sh
cd scripts/bench/go
export GOFLAGS=-mod=mod
go run .                                             # no input
script -qec "go run . -tty" /dev/null > go-tty.txt   # tty
```

Run the `-tty` case with `script` and its output sent to a file, as
above, so no terminal answers Bubble Tea's queries: an answer would turn
on synchronised output and change the numbers.

Offline, from a Bubble Tea v2.2.0 checkout, point a copy of `go.mod` at
it:

```sh
cd scripts/bench/go
{ cat go.mod; echo "replace charm.land/bubbletea/v2 => /path/to/bubbletea"; } > /tmp/offline.mod
GOFLAGS="-mod=mod -modfile=/tmp/offline.mod" GOPROXY=off go run .
```

To check that every run drew the right screens:

```sh
cd scripts/bench
cargo run --release -- --dump /tmp/frames
(cd go && go run . -dump /tmp/frames)
python3 verify.py /tmp/frames
```

### Checked in CI

The "Bytes per frame" job runs petitcarnets alone (no other port, no
Go, nothing to download beyond the bench's own lockfile) and fails if any
scenario writes a different number of bytes than
`scripts/bench/expected.tsv` records: the first frame, and the 100
frames after it together.

```sh
cd scripts/bench
cargo run --release --no-default-features -- --check expected.tsv
cargo run --release --no-default-features -- --bless expected.tsv  # after a change meant to alter them
```

A renderer change that writes more (or fewer) bytes fails the job until
`expected.tsv` and the tables above are updated with it. The comparison
with Bubble Tea and the other ports stays a manual run, as above.

## Keystroke latency

Time from writing a key to the first byte of the frame that answers it,
in a real pseudo-terminal: `scripts/latency.py` runs the `counter`
example (release build, alternate screen), presses `+` 300 times and
waits for each frame to finish (20 ms of quiet) before the next press.
The frame after a key is drawn at once, not on the next frame tick.

```
cargo build --release --example counter
python3 scripts/latency.py 300
```

Four runs on the machine above, 2026-11-27 (the spread between runs is
the CPU's power state, not the program):

| run | median | p99 | max |
| --- | ---: | ---: | ---: |
| 1 | 0.478 ms | 0.591 ms | 1.295 ms |
| 2 | 0.486 ms | 0.579 ms | 0.708 ms |
| 3 | 0.101 ms | 0.538 ms | 0.903 ms |
| 4 | 0.115 ms | 0.546 ms | 1.496 ms |

Every run is under the 2 ms target at the median and at p99. Bubble Tea
was not measured this way.
