# Changelog

All notable changes to this project are documented here. Every release is
exactly 0.0.1 above the previous one.

## [0.0.7] - 2026-11-28

Docs and examples: sixteen more of Bubble Tea's examples ported (39 of
its 64 now, each run in CI), an accessible output mode for screen
readers, an example on every public item rustdoc counts, and guides for
Bubble Tea and bubbletea-rs users.

### Added

- Accessible output, for screen readers and braille displays (Bubble
  Tea's issue #780): `Program::with_accessible(bool)`, huh's
  `WithAccessible`. Frames are printed as plain lines: each line that
  differs from the line in its place in the frame before is printed
  once, below what is already there, and blank lines are left out.
  Nothing is redrawn, the cursor is never moved or hidden, the
  alternate screen is not used, there is no synchronized output,
  autowrap stays on and lines are not cut to the window; `println`
  prints its lines the same way, and a repaint or a terminal handed
  back prints the whole frame again. Colours and styles are left out
  (the `NoTTY` profile) unless the program sets a profile; input works
  as always. Without the call, a program in a real terminal turns it on
  when `ACCESSIBLE` is set and not empty, the variable huh's
  documentation tells programs to read, so the person at the terminal
  decides. `tests/accessible.rs` checks the bytes written (golden files)
  and that nothing redraws; the real-terminal smoke test runs the
  counter with `ACCESSIBLE=1`.
- Sixteen of Bubble Tea's examples, under their names: `simple`,
  `fullscreen`, `window-size`, `debounce`, `focus-blur`, `sequence`,
  `http` (plain HTTP over `std::net`), `send-msg`, `realtime`,
  `prevent-quit`, `composable-views`, `split-editors`,
  `set-terminal-color`, `views`, `vanish` and `result`, each with golden
  tests in `tests/examples.rs` or `tests/bubbles_examples.rs` (39 golden
  files). `docs/examples.md` lists all 64 of Bubble Tea's examples, and
  why each one not ported yet is not.
- Every example runs in CI: its model in a golden test, or for `exec`
  and `suspend` its binary in the real-terminal smoke test; a test fails
  if an example has neither. The Test job runs the example tests in a
  step of their own.
- A doc example, which asserts what it shows, on every public item
  rustdoc counts for examples (from 8.9% to 100%); `missing_docs` is
  denied crate-wide.
- `docs/go-to-rust.md` (every exported name of Bubble Tea v2.1.1 and its
  equivalent here, the idioms, and Go's `simple` example converted step
  by step), `docs/migrating-from-bubbletea-rs.md`, `docs/architecture.md`
  (the loop, threads, terminal guard, reader, decoder, renderer, output
  queue, colours and test harness) and `docs/issues.md` (the Bubble Tea
  and Bubbles issues petitcarnets addresses, the release and the test
  for each).

### Changed

- Source files that grew past the 500-line ceiling with their doc
  examples are split into child modules; no public path changes.
- The README and the docs call Bubble Tea's #500 and #1773 pull
  requests, which they are, and the crate docs say only Linux and macOS
  are tested so far, as the README does (the FreeBSD code paths are not
  yet run anywhere).

### Fixed

- A `println` taller than the room above an inline frame pushed the
  frame off the screen until the next update; the frame is now drawn
  again with the next frame, with no update needed (Bubble Tea's issue
  #1822, which the 0.0.3 entry counted as fixed by design).

### Fixed, compared with Bubble Tea's examples

- `split-editors`: removing the focused editor focuses the new last one;
  Go's leaves no editor focused.

## [0.0.6] - 2026-11-27

Components: Bubbles v2's, under its names and with its default key
bindings, behind a default feature, `bubbles`, that adds no dependency;
and `Cmd::map` and `Msg::route` to compose them.

### Added

- `bubbles`, a module behind the default feature `bubbles` (the crate
  builds without it with `default-features = false`), with Bubbles v2.2.2's
  components: `key` (`Binding`, `matches`), `help` (short and full help
  from bindings, cut to the width with an ellipsis), `cursor` (a drawn
  cursor that blinks), `spinner` (all twelve of Bubbles' spinners, with
  their frames and speeds), `paginator` (dots or numbers), `textinput`
  (prompt, placeholder, character limit, width and sideways scrolling by
  display width, echo modes for passwords, moves and deletions by
  character and word, paste, suggestions, validation, and the
  terminal's own cursor if asked), `textarea` (soft wrap by display
  width, line numbers, a prompt or a prompt function, limits on
  characters, lines, size and wrapped lines, scrolling, case changes,
  transposition, keyboard and mouse selection, copy with OSC 52),
  `viewport` (keys, mouse wheel, sideways scrolling, soft wrap, a
  gutter, line styles), `list` (items drawn by an `ItemDelegate`, a
  `DefaultDelegate`, fuzzy filtering ranked as `sahilm/fuzzy` ranks it,
  pages, a status bar, a spinner, status messages, help), `table`
  (columns, rows, selection, scrolling, focus), `progress` (solid,
  gradient (blended in CIE L*a*b* as Lip Gloss blends it) or per-cell
  colours, and the spring animation, harmonica's maths ported), `timer`
  and `stopwatch`. Each component is a struct with
  `update(&mut self, &Msg<X>) -> Cmd<X>` and `view(&self) -> String`;
  `X` is its own typed message, and a component with none is generic
  over it. Components that tick carry an id, as in Bubbles. Every key
  map's defaults are Bubbles v2's.
- `Cmd::map`, Elm's `Cmd.map`: a child's commands become the parent's,
  every message passed through a function; quit, interrupt, output and
  terminal handovers stay as they are, batches and sequences map each
  command, and tasks, ticks, `every` and `exec` send the mapped message.
- `Msg::route`, for messages going down: the runtime's messages as they
  are, the model's own through a function that picks a child's; and
  `Msg::map`.
- In `style`, the little of Lip Gloss the components need:
  `Style::render` (each line painted, then padding, height, lines padded
  to one width, the border, margins; text wider than a set width is
  wrapped), `Style` padding, margins, `border` with `Border::NORMAL`,
  `ROUNDED`, `THICK`, `DOUBLE` and `HIDDEN` and chosen sides, border
  colour, width, height, `inherit`, `plain`, and the frame sizes;
  `join_horizontal`, `join_vertical` and `Position`; `height` and
  `block_width` (Lip Gloss's `Height` and `Width`); `truncate_with`,
  `cut` and `wrap` (x/ansi's `Truncate`, `Cut` and `Wrap`), none of which
  splits a wide character.
- Fifteen examples, Bubble Tea's own ported: `textinput`, `textinputs`,
  `textarea`, `spinners`, `list-simple`, `list-default`, `pager`,
  `progress-static`, `progress-animated`, `table`, `help`, `timer`,
  `stopwatch`, `paginator` and `chat`.
- `tests/bubbles.rs`: each of those examples in `TestProgram`, 40 golden
  screens in `tests/golden`, and composition tests: two spinners and a
  text input in one parent each take only their own ticks (even when the
  parent hands every tick to both), and a mapped quit, batch, sequence,
  tick, `every`, task and `exec` keep their meaning.
- `scripts/bubbles-fixture`, a Go program that runs Bubbles v2.2.2
  itself and writes `tests/fixtures/bubbles.tsv`: every default key
  binding with its help, the spinners, help and paginator views, 51 text
  input and 46 text area editing sequences (keys in; value, cursor and
  view out), list filter ranks for 21 terms, colour blends, springs,
  progress bars with each cell's colours, viewports, tables and lists.
  The unit tests check every line.
- The real-terminal smoke test types UTF-8 into the `textinput` example.
- CI's Test job also runs clippy and the tests without the components
  (`--no-default-features`), and builds the documentation with every
  feature.

### Changed

- The test that switches the width method to grapheme widths moved from
  `tests/harness.rs` to `tests/width_method.rs`, a binary of its own:
  the method is shared by the process, and another test starting a
  `TestProgram` at the same moment set it back, so it failed about one
  run in six.
- The Miri job skips the components' unit tests: they are safe Rust with
  no FFI, checked against Go and by a seeded stress test of random keys,
  pastes and resizes in the other jobs.

### Fixed, compared with Bubbles v2

- The text input moves, deletes and scrolls by grapheme cluster, as
  Bubbles' own text area does, instead of by code point: the cursor no
  longer stops inside a cluster, and Backspace and Delete no longer
  leave half of one (Bubbles' issue #1097, Thai; the same for accents,
  ZWJ emoji and skin tones). A placeholder's first cluster stays whole
  under the cursor (#573), and a wide placeholder is cut to the width
  instead of overflowing it with NUL characters.
- The text input's suggestion shows the character after the whole text
  under the cursor; Go takes it at the cursor's place in the visible
  part, the wrong one once the text has scrolled.
- The list filters in `update`, so its matches, pages and dots always
  belong to the filter showing; Go filters on another goroutine and
  pages before the matches arrive (and an older filter's matches can
  land after a newer one's).
- The list's filter input gets the filter styles the list defines; Go
  defines them and never applies them.
- A table's selected style reaches every cell even when the cell style
  has a colour (#729).

### Not ported

- The viewport's deprecated high performance mode and its search
  highlights; the file picker and the tree.
- The default delegate's update and help functions (write a delegate);
  a list's delegate is cloned for its update.
- Timers stop at zero rather than going below it; the stopwatch takes
  its time from the program's clock (so it follows `TestProgram`'s),
  not the wall clock.

## [0.0.5] - 2026-11-27

Testing and speed: a public, deterministic test harness (Bubble Tea's
teatest, which its issue #1654 asks for), golden tests for the examples,
and quality gates for coverage, Miri, ThreadSanitizer, mutation testing
and the bytes the renderer writes.

### Added

- `testing::TestProgram`, Bubble Tea's teatest: runs a model in the real
  runtime (the loop, executor, renderer, terminal modes and output queue
  of `Program::run_with`) on a virtual clock, in an emulated terminal.
  `tick`, `every`, the frame rate, the escape timeout and the wait for
  query replies follow the virtual clock, which moves only with
  `advance`, `wait_for` or `wait_finished`; nothing sleeps. Input goes in
  with `send`, `press`, `type_text`, `bytes` (through the real decoder),
  `input`, `resize`, `interrupt`, `terminate`, `suspend`, `resume`,
  `quit` and `kill`; `screen`, `output`, `model`, `final_screen`,
  `final_output`, `final_model` and `into_result` read the result. Each
  call returns once the program is idle: tasks still run on their own
  threads, and the harness waits for each and delivers their messages in
  the order the tasks started. `exec` runs its command for real;
  `suspend` hands the terminal over and continues at once, counted by
  `suspensions`. A program that never goes idle (a zero `tick` that
  starts itself) is reported with a panic instead of hanging the test.
- `testing::TestTerminal`: the harness's terminal: its size, whether it
  acts as a real terminal, and scripted replies to queries
  (`reply(query, answer)`).
- `testing::Vt`, the terminal emulator the crate's own tests used, now
  public: `feed`, `text`, `row_text`, `cell` (`ScreenCell`: text, width,
  style as SGR, hyperlink), `cursor`, `cursor_visible`, `cursor_style`,
  `alt_screen`, `mode`, `modes`, `title`, `scrollback`, the kitty
  keyboard flags (`keyboard_flags`, now tracked: push, pop and set) and
  the modifyOtherKeys level, and `snapshot`, all of it as stable text for
  golden files.
- `testing::assert_golden(name, actual)` against
  `tests/golden/<name>.golden`, rewritten with
  `PETITCARNETS_UPDATE_GOLDEN=1` (Go's `golden.RequireEqual`), with
  `golden_path` and `escape` (raw output as readable text).
- 15 golden tests (`tests/golden.rs`, 26 golden files) run the examples'
  own models (counter, picker, spinner on virtual ticks, package-manager
  printing above its line, altscreen-toggle, mouse,
  keyboard-enhancements, query-term with scripted replies, cursor-style,
  set-window-title, lifecycle with its process id left out) and check
  screens, cursor, modes and keyboard flags, and the exact bytes for
  three of them. The examples' models are `pub(crate)` so the tests can
  include them.
- CI jobs "Coverage" (cargo-llvm-cov over the unit and integration tests
  and the real-terminal smoke test, failing under 90% of the library's
  lines), "Miri" (every test that makes no FFI call), "ThreadSanitizer"
  (the test suite with `-Zsanitizer=thread` and the standard library
  rebuilt) and "Bytes per frame" (the renderer's bytes for each
  benchmark scenario, against `scripts/bench/expected.tsv`).
- A weekly and on-demand workflow, `mutants.yml`, runs cargo-mutants on
  the core modules (listed in `.cargo/mutants.toml`) and uploads the
  report. It is not a required check.
- `scripts/bench` `--check FILE` and `--bless FILE`, for the CI job.

### Changed

- The runtime reads time from a clock it is given (the real one for
  `Program::run_with`, a virtual one for `TestProgram`); an idle program
  still never wakes.
- Most tests in `tests/runtime.rs`, `tests/lifecycle.rs` and the
  program's unit tests run on `TestProgram`, so they no longer sleep or
  depend on timing; a few stay on `TestBackend` to cover it. The checks
  on timing are now exact: 300 ticks 1 ms apart at 10 fps draw 3 frames
  in 300 ms of virtual time, and an unanswered query delays the end by
  500 ms exactly.
- The `spinner` example waits two seconds with a `tick` before it
  computes on another thread, instead of sleeping on that thread, so the
  wait is the program's and can be tested.
- `docs/benchmarks.md` measured again for v0.0.5: every number is
  unchanged.

### Fixed

- A frame held back by the frame rate while the terminal was handed
  over (`release_terminal`) woke the loop over and over until the
  terminal was taken back: a busy wait. Such a frame is now dropped;
  taking the terminal back draws one at once.
- Quitting while the terminal was released dropped the output that was
  waiting for it (`println`, `raw`); it is now written when the terminal
  is taken back for the last frame. A kill still drops it.

## [0.0.4] - 2026-11-27

Bubble Tea v2's lifecycle and terminal queries: running other programs
in the terminal, suspending, releasing, repainting, the colour profile,
and the terminal's replies, with fixes for the Bubble Tea issues around
each.

### Added

- `exec(Command, done)` (Go's `ExecProcess`), and `exec_command` with the
  `ExecCommand` trait (Go's `Exec`): the terminal is handed over on the
  program's thread, in program order. Input is paused without reading
  (a pause and resume handshake through the reader's wake pipe), every
  mode is turned off, the terminal's own mode is put back at once
  (`TCSANOW`, keeping unread input); afterwards raw mode, every mode and
  the whole frame come back and the callback's message is the next the
  model sees. Keys typed during the child are the child's and the rest
  the program's, none lost or read twice (Bubble Tea #616, #1116,
  #1167); mouse and the other modes come back (#1424); frames resume at
  once (#1778).
- `suspend()` and Ctrl-Z suspend by default (Bubble Tea #497, #500,
  #1673); `Program::without_suspend` delivers Ctrl-Z as a key instead.
  `SIGTSTP` from outside (`kill -TSTP`) suspends the same way, and
  `SIGCONT` after a stop from outside sets the terminal up and draws it
  again. The process group is stopped as Bubble Tea stops it
  (`kill(0, SIGTSTP)` with the default handler, put back after); the
  model gets the new `Msg::Resume`. While another program has the
  terminal, `SIGTSTP` has its default handling, so Ctrl-Z there stops
  both, as in a shell.
- `release_terminal()` and `restore_terminal()` (Go's
  `Program.ReleaseTerminal` and `RestoreTerminal`), as commands run in
  program order on the thread that owns the terminal; output waits while
  the terminal is released.
- `repaint()` (Bubble Tea #1808, pull requests #1809 and #1846): every
  mode turned off and on again (the alternate screen entered again) and
  the whole frame drawn, even unchanged.
- `ColorProfile` (`NoTty`, `Ascii`, `Ansi`, `Ansi256`, `TrueColor`),
  detected at the start from the program's environment as Go's
  colorprofile does (`NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`,
  `COLORTERM`, `TERM` including `dumb`, `TTY_FORCE`, `WT_SESSION`,
  `GOOGLE_CLOUD_SHELL`, and `tmux info` inside tmux), sent as
  `Msg::ColorProfile`, and upgraded to 24-bit on an `RGB` or `Tc`
  capability reply. Every colour written comes down to it in one place,
  the writer every frame and printed line goes through, converted as
  Go's x/ansi converts (`Convert256` with go-colorful's HSLuv distance,
  `Convert16`); the cursor and terminal colours (OSC 10, 11, 12) come
  down too, and are left out with no colours. `Ascii` keeps attributes,
  `NoTty` drops them and hyperlinks, as Go does. The palette and 16-colour
  tables are generated from Go (`src/color/*.in`) by
  `scripts/color-fixture`, which also writes `tests/fixtures/color.tsv`:
  4,096 colours to both palettes, all 256 indexed colours to 16, and 672
  environments (336 sets of variables, terminal or not), all matched
  exactly. The 256-colour conversion was also checked against Go for all
  16,777,216 colours.
- `Program::with_color_profile` (Go's `WithColorProfile`) and
  `Program::with_environment` with the new `Environment` type (Go's
  `WithEnvironment`), which also decides the start queries.
- Requests: `request_window_size`, `request_cursor_position`,
  `request_background_color`, `request_foreground_color`,
  `request_cursor_color`, `request_terminal_version`,
  `request_capability`, `set_clipboard`, `read_clipboard`,
  `set_primary_clipboard` and `read_primary_clipboard`, writing the bytes
  Go writes.
- Replies now delivered: `Msg::Capability`, `Msg::Clipboard` (with
  `ClipboardSelection`), `Msg::PrimaryDeviceAttributes`,
  `Msg::SecondaryDeviceAttributes`, `Msg::TertiaryDeviceAttributes`,
  `Msg::ColorScheme` (with `ColorScheme`), `Msg::PixelSize`,
  `Msg::CellSize`, and window size reports (`CSI 8 ; h ; w t`, and the
  in-band resize report) as `Msg::WindowSize`. Go passes most of these
  on as raw ultraviolet events; here each is a `Msg` variant. The input
  fixture now spells them out, with 21 more lines (1,722 in all).
- `Input` variants for each, and `Input::Suspend` and `Input::Continue`
  for the two new signals; `Backend::is_terminal`, `Backend::release`,
  `Backend::restore` and `Backend::suspend`, and `TestBackend::terminal`.
- Examples named after Bubble Tea's: `exec`, `suspend` and `query-term`.
- Real-terminal checks: a child run with `exec` reads exactly the keys
  typed while it runs and the program the rest, with modes back after;
  Ctrl-Z and `kill -TSTP` stop the process (state T) and `kill -CONT`
  resumes it with a fresh frame; with `NO_COLOR` no colour parameter is
  written; a program that asks seven questions and quits at once leaves
  no reply in the shell.

### Changed

- Ctrl-Z suspends the program by default (see above); it used to arrive
  as the key `"ctrl+z"`.
- The model hears `Msg::ColorProfile` first, before `Msg::WindowSize`,
  as in Bubble Tea.
- `Backend::query_output_modes` is replaced by `Backend::is_terminal`,
  which also turns on colour detection and the reply wait; the gate on
  asking (not over SSH, not in Apple's Terminal) now reads the program's
  environment.
- Replies are waited for with Bubble Tea's sentinel: a primary device
  attributes query (DA1), whose reply comes after every earlier one,
  instead of counting replies; at the end, before an `exec` or a
  suspend, and for every query (any `raw` write counts as one), not just
  the start query. Events that arrive during the wait before an `exec`
  are kept for the model.
- Window size reports that are zero, over 4,096 cells either way or over
  2^20 cells are dropped, as Bubble Tea drops implausible sizes (it lets
  zero through; here zero is dropped too).

### Fixed

- `NO_COLOR` and the colour profile apply to `println` too (Bubble Tea
  #1036 and #1709: Go's `Println` writes styles as they are).
- A profile set with `with_color_profile` is never overridden by a
  capability reply (Bubble Tea #1773).
- A capability reply with a value (`RGB=8/8/8`) upgrades the profile as
  a bare `RGB` does; Go compares the whole reply.
- Replies to queries never reach a child run with `exec` or the shell
  after the program ends (Bubble Tea #1590, #1627, #1771): no query
  blocks start-up, and the wait for replies is bounded at 500 ms.

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
