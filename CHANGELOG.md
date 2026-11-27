# Changelog

All notable changes to this project are documented here. Every release is
exactly 0.0.1 above the previous one.

## [Unreleased]

### Added

- A working prototype: raw mode with restore on exit and panic, a key
  parser, commands (batch, task, tick, quit), a line-diff renderer,
  styles, and a headless test backend.
- Examples: `counter`, `picker`, `spinner`.
- CI on Linux and macOS (stable and 1.85, plus musl), a real-terminal
  smoke test, complexity ceilings, a no-dependencies check, and CodeQL.
