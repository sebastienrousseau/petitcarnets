#!/usr/bin/env bash
# Runs the counter example in a real pseudo-terminal (util-linux
# `script`), presses +, Up, Down, +, q, and checks the final count and
# that the terminal mode is restored byte for byte. Linux only.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --example counter
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
(sleep 1; printf '+'; sleep 0.2; printf '\033[A'; sleep 0.2; printf '\033[B'; sleep 0.2; printf '+'; sleep 0.2; printf 'q'; sleep 1) |
  script -q -e -c "stty -g > $dir/before; ./target/debug/examples/counter; echo EXIT=\$?; stty -g > $dir/after" "$dir/log" > /dev/null
grep -a -q 'Final count: 2' "$dir/log" || { echo "wrong or missing final count"; cat -v "$dir/log"; exit 1; }
grep -a -q 'EXIT=0' "$dir/log" || { echo "the example did not exit 0"; cat -v "$dir/log"; exit 1; }
cmp -s "$dir/before" "$dir/after" || { echo "the terminal mode was not restored"; exit 1; }
echo "pty smoke: final count 2, exit 0, terminal restored"
