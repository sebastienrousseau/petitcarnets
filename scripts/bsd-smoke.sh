#!/usr/bin/env bash
# Real-terminal smoke test for FreeBSD and macOS (and Linux, to try it
# locally): the examples run under script(1) in a pseudo-terminal, keys
# and signals reach them, and the terminal mode and screen come back as
# they were after a quit, Ctrl-C, a panic, SIGINT, SIGTERM and a double
# SIGINT. scripts/pty-smoke.sh has the full Linux suite; this is the
# part that needs nothing Linux-only (no /proc, no util-linux script).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q --example counter --example lifecycle
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
fails=0

# The newest process running an example binary.
pid_of() {
  pgrep -n -f "target/debug/examples/$1\$"
}

# How many times the pattern occurs in a file (0 is not an error).
count() {
  { grep -a -o "$1" "$2" || true; } | wc -l | tr -d ' '
}

# Runs a shell command in a pseudo-terminal, logging what it writes:
# BSD script(1) takes the log then the command, util-linux's takes -c.
in_pty() {
  local log="$1" command="$2"
  if [[ "$(uname)" == Linux ]]; then
    script -q -e -c "$command" "$log"
  else
    script -q "$log" sh -c "$command"
  fi
}

# run NAME EXAMPLE EXPECTED_EXIT FEEDER: runs EXAMPLE with FEEDER's
# output as its keyboard, and checks the exit and the terminal after.
run() {
  local name="$1" example="$2" expected="$3" feeder="$4"
  local log="$dir/$name.log"
  "$feeder" "$example" | in_pty "$log" \
    "stty -g > $dir/before; ./target/debug/examples/$example; echo EXIT=\$?; stty -g > $dir/after" \
    > /dev/null 2>&1 || true
  local got entered left
  got="$({ grep -a -o 'EXIT=[0-9]*' "$log" || true; } | tail -1)"
  entered="$(count $'\e\\[?1049h' "$log")"
  left="$(count $'\e\\[?1049l' "$log")"
  if [[ "$got" != "EXIT=$expected" ]]; then
    echo "FAIL $name: ${got:-no exit}, expected EXIT=$expected"; fails=$((fails + 1)); return
  fi
  if ! cmp -s "$dir/before" "$dir/after"; then
    echo "FAIL $name: the terminal mode was not restored"; fails=$((fails + 1)); return
  fi
  if [[ "$entered" != "$left" ]]; then
    echo "FAIL $name: alternate screen entered $entered times, left $left"; fails=$((fails + 1)); return
  fi
  echo "ok   $name: exit $expected, terminal restored"
}

counter_keys() {
  sleep 1; printf '+'; sleep 0.2; printf '\033[A'; sleep 0.2; printf '\033[B'
  sleep 0.2; printf '+'; sleep 0.2; printf 'q'; sleep 1
}
press_q() { sleep 1; printf 'q'; sleep 1; }
press_ctrl_c() { sleep 1; printf '\003'; sleep 1; }
press_p() { sleep 1; printf 'p'; sleep 1; }
sigint() { sleep 1; kill -INT "$(pid_of "$1")" || true; sleep 1; }
double_sigint() {
  sleep 1; local pid; pid="$(pid_of "$1")" || return 0
  kill -INT "$pid" || true; kill -INT "$pid" 2> /dev/null || true; sleep 1
}
sigterm() { sleep 1; kill -TERM "$(pid_of "$1")" || true; sleep 1; }

run counter counter 0 counter_keys
if grep -a -q 'Final count: 2' "$dir/counter.log"; then
  echo "ok   counter: keys reached the model (final count 2)"
else
  echo "FAIL counter: wrong or missing final count"; fails=$((fails + 1))
fi
run quit lifecycle 0 press_q
run ctrl-c-key lifecycle 0 press_ctrl_c
run panic-in-update lifecycle 101 press_p
run sigint lifecycle 130 sigint
run double-sigint lifecycle 130 double_sigint
run sigterm lifecycle 0 sigterm

[[ "$fails" == 0 ]] || { echo "$fails check(s) failed on $(uname)"; exit 1; }
echo "all checks passed on $(uname)"
