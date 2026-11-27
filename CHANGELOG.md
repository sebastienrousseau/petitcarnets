# Changelog

All notable changes to this project are documented here. Every release is
exactly 0.0.1 above the previous one.

## [Unreleased]

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
