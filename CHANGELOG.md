# Changelog

All notable changes to this project are documented here. Every release is
exactly 0.0.1 above the previous one.

## [0.0.2] - Unreleased

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
