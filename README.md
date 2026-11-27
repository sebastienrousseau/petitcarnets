# petitcarnets

Terminal user interfaces in the Elm architecture (model, update, view),
for Rust, with no dependencies.

petitcarnets follows [Bubble Tea](https://github.com/charmbracelet/bubbletea)
v2 closely: the same loop, the same command and message names, and the
same key strings. If you have written a Bubble Tea program in Go, the Rust
version should read the way you expect.

*Petits carnets*: little notebooks.

## Status

Early work, not yet published on crates.io. The current code is a working
prototype: raw mode, keys, commands, timers and a line-diff renderer, on
Linux and macOS. The public API is being renamed to Bubble Tea's names
(`Program`, `Model`, `Msg`, `Cmd`, `View`) in v0.0.1, so expect it to
change.

## Why

- **No dependencies.** `cargo tree` prints one crate. The terminal is
  driven through a handful of libc calls in one audited module, the only
  place with `unsafe`.
- **Typed messages.** Your messages are your own enum, matched
  exhaustively; no downcasting.
- **No async runtime required.** A command is a plain closure.
- **Restores your terminal.** Raw mode and every terminal mode are undone
  on exit, on panic and on signals.
- **Testable.** A headless backend runs a model with scripted keys and
  captures the screen, with no pseudo-terminal.

## Example

```sh
cargo run --example counter
```

`examples/` also has `picker` and `spinner`.

## Platforms

Linux and macOS. Windows is planned for a later release.

Minimum supported Rust version: 1.85.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT licence ([LICENSE-MIT](LICENSE-MIT))

at your option.
