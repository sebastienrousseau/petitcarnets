# Changelog

All notable changes to this project are documented here. Every release is
exactly 0.0.1 above the previous one.

## [0.0.3] - 2026-11-27

Bubble Tea v2's renderer: a cell buffer diffed frame to frame, inline
drawing by default, and every pure-output `View` field.

### Added

- A cell-buffer renderer (Bubble Tea's cursed renderer): views are laid
  out in cells (grapheme, width, SGR style, OSC 8 hyperlink), unchanged
  rows are skipped by hash, changed rows rewrite only the cells that
  differ, a shorter line's old tail is erased with one sequence when that
  is shorter, and on the alternate screen a frame that moved is scrolled
  by the terminal (SU and SD, inside a scroll region when only part of
  the screen moved). An unchanged frame writes nothing.
- Inline drawing, the default as in Bubble Tea v2: relative cursor moves
  only, and an inline frame taller than the window keeps its bottom
  rows.
- `View` fields `alt_screen`, `cursor` (`Cursor`, `CursorShape`, with
  `Cursor::new` as Go's `NewCursor`), `window_title`, `foreground_color`,
  `background_color` and `progress_bar` (`ProgressBar`,
  `ProgressBarState`), with `with_` builders. The window title is saved
  on the terminal's title stack and given back on exit; the cursor shape,
  colours and progress bar are reset on exit.
- Commands `println`, `printf!`, `raw` and `clear_screen`, applied in
  program order with frames. `println` prints above an inline frame, its
  lines in the scrollback, wrapped rows counted as the terminal wraps
  them; on the alternate screen it prints nothing, as in Bubble Tea.
- `Program::with_fps` (default 60, cut to 1..=120).
- `style::WidthMethod`, `style::width_method`, and text width measured as
  Go's `charmbracelet/x/ansi` v0.12.0 measures it: `WcWidth` (default)
  or `GraphemeWidth`, chosen when the terminal reports mode 2027. Tables
  are generated from the same Go packages (go-runewidth, displaywidth,
  uax29, Go's `unicode`) by `scripts/gen-width-tables.py`, and
  `scripts/width-fixture` writes `tests/fixtures/width.tsv`: 8,033
  strings whose widths and cell splits the Rust code matches exactly.
- Synchronized output (mode 2026) and grapheme widths (mode 2027) are
  queried at the start, gated as Bubble Tea gates it (not over SSH, not
  in Apple's Terminal); replies reach the renderer as well as the model.
  Startup never waits; replies still due at the end are waited for, at
  most 500 ms, so they do not reach the shell.
- `TerminalMode` variants `AltScreen`, `NoAutowrap`, `UnicodeCore`,
  `SavedTitle`, `CursorStyle`, `ForegroundColor`, `BackgroundColor`,
  `CursorColor` and `ProgressBar`, all recorded by the terminal guard;
  `Backend::query_output_modes`.
- Examples named after Bubble Tea's: `altscreen-toggle`, `cursor-style`,
  `set-window-title` and `package-manager` (inline, printing above its
  progress line).
- A test terminal emulator and golden frame tests through it, a
  real-terminal check of an inline program, `docs/benchmarks.md` with a
  bytes-per-frame comparison, and `scripts/latency.py` for keystroke to
  frame latency in a pseudo-terminal.
- Releases publish to crates.io from GitHub Actions through crates.io
  trusted publishing, so no crates.io token is stored anywhere. Pushing
  a version tag starts `.github/workflows/release.yml`, which waits for
  the maintainer's approval in the `crates-io` environment and checks
  that the tag names the crate's version, sits on `main` and carries a
  signature GitHub has verified.

### Changed

- Views are inline unless they set `alt_screen`; the program no longer
  enters the alternate screen on its own. The `counter`, `picker`,
  `lifecycle` and `mouse` examples set it; `spinner` and
  `keyboard-enhancements` are inline, as their Go counterparts.
- Frames are drawn when the queue is empty but at most `fps` times a
  second, except that the frame after a key, mouse event or paste is
  drawn at once. Idle programs still never wake up.
- `style::width` and `style::truncate` measure display width (wide
  characters count two) with the program's width method, instead of
  counting characters; `truncate` never splits a wide character.
- Autowrap is off while a program runs and back on when it ends.
- A resize redraws in place instead of clearing the whole screen.
- `View` moved to its own module; it is still `petitcarnets::View`.
- When an inline program quits or is interrupted, every row of its last
  frame stays on screen, a last line without a trailing newline
  included, and the cursor ends at the start of a fresh line just below
  it, with anything further down erased. An empty last view leaves
  nothing, the cursor where the frame started. This differs on purpose
  from Bubble Tea v2.2.0, which moves to the start of the frame's last
  row and erases from there, losing that line (Bubble Tea issue #1869,
  pull request #1798). A killed or panicked program still ends as
  Bubble Tea's does; the alternate screen is unchanged.

### Fixed

- The 0.0.2 changelog heading now carries its release date.
- Bubble Tea issues fixed by design: frames are not on a ticker, so
  there is no ticker race (#1780, #1857, #1873); `println` is ordered
  with frames and written before the terminal is restored (#1736,
  #1881, #1909, #1822); the last frame is always drawn on quit and
  interrupt (#1898); tabs are expanded (#1838, #1541); widths follow one
  method per program, wide characters included when counting wrapped
  `println` rows (#1777, #1737, #1802, #1906); the cursor is placed
  where the view says, for input methods (#918, #874); the frame after
  input is drawn at once (#1481, #1724).

### Not done

- Bubble Tea's `ProgramStatus` (OSC 7501), `ProgramResume` (OSC 7577)
  and `OnMouse` view fields.

## [0.0.2] - 2026-11-27

Input, decoded as Bubble Tea v2 decodes it: a port of ultraviolet's
input decoder (the Unix paths), checked line for line against Go on a
corpus of 1,701 byte sequences.

### Added

- `Key` as Bubble Tea's struct: `text`, `mods` (`KeyMod`), `code`
  (`KeyCode`, every named key Go has), `shifted_code`, `base_code`,
  `is_repeat`. It prints as Go's `Key.String()`, with `keystroke()` for
  Go's `Keystroke()` and `matches` / `matches_any` for `MatchString`.
- Messages: `KeyRelease`, `MouseClick`, `MouseRelease`, `MouseWheel`,
  `MouseMotion`, `Paste`, `PasteStart`, `PasteEnd`, `Focus`, `Blur`, and
  the replies `CursorPosition`, `KeyboardEnhancements`, `ModeReport`,
  `TerminalVersion`, `ForegroundColor`, `BackgroundColor`, `CursorColor`
  (`Rgb`, with Go's `IsDark`).
- Decoding of legacy, xterm, rxvt-unicode, kitty keyboard protocol and
  modifyOtherKeys keys, SGR and X10 mouse, focus, bracketed paste,
  grapheme clusters as one key, and the replies above. Unknown sequences
  are read and dropped, never turned into keys.
- A lone ESC, or a sequence cut off at the end of a read, waits 50 ms
  for the rest, as in Bubble Tea.
- `View` fields `mouse_mode`, `report_focus`,
  `disable_bracketed_paste_mode` and `keyboard_enhancements`, with `with_`
  builders. The program turns modes on and off as views change, and off
  on every way out; bracketed paste and kitty key disambiguation are on
  by default, as in Bubble Tea. `KeyboardEnhancements` takes methods
  named after Go's fields: `KeyboardEnhancements::new().report_event_types()`.
- `Backend::set_mode` and `TerminalMode`; the terminal guard keeps the
  modes that are on and turns them off newest first.
- `TestBackend::bytes`, to script raw terminal bytes through the real
  decoder.
- Examples: `mouse` and `keyboard-enhancements`.
- A Go program (`scripts/input-fixture`) that writes the decoder
  fixture, a deterministic fuzz test (100,000 inputs per test run, ten
  million in the `Fuzz (10M)` CI job), and real-terminal checks for mouse
  clicks, paste and modes being turned off.

### Changed

- `Key` is a struct instead of an enum, and `Input::Key` is now
  `Input::KeyPress`; `TestBackend::keys` and `inputs` add to the script
  instead of replacing it.
- Control characters follow Go's mapping: BS (0x08) is `"ctrl+h"`, LF is
  `"ctrl+j"`, NUL is `"ctrl+space"`.

### Removed

- `parse_keys`; the decoder is internal.

### Fixed

- The 0.0.1 changelog heading now carries its release date, and the
  README no longer says the crate is unpublished.
- The published crate includes the decoder fixture, so its unit tests
  build from the crates.io archive.

## [0.0.1] - 2026-11-27

### Added

- Bubble Tea's names: `Program`, `Model`, `Msg`, `Cmd`, `View`, and the
  commands `quit`, `interrupt`, `batch`, `sequence`, `tick` and `every`.
  Keys print and compare as Bubble Tea writes them (`k == "ctrl+c"`).
- `Msg::WindowSize` at the start and after every resize (`SIGWINCH`).
- `Sender` to send messages, quit or kill from other threads.
- `Error`: `Interrupted` on `SIGINT`, `Killed`, `Panic` when a task
  panics, and `Io`.
- Timers on the event loop: a thousand ticks cost no threads, and an
  idle program sleeps with no wakeups.
- A terminal guard that restores raw mode and every terminal mode on
  quit, panic and signals; input from `/dev/tty` when standard input or
  output is not a terminal.
- Examples: `counter`, `picker`, `spinner`, `lifecycle`.
- CI on Linux and macOS (stable and 1.85, plus musl), real-terminal
  checks for every way a program ends, complexity ceilings, a
  no-dependencies check, and CodeQL.

### Changed

- Ctrl-C is now a key (`"ctrl+c"`), as in Bubble Tea, instead of
  quitting on its own; quit on it in `update`.
