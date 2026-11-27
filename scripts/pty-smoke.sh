#!/usr/bin/env bash
# Runs the examples in a real pseudo-terminal (util-linux `script`) and
# checks, for every way a program can end, that the exit code is right
# and the terminal comes back byte for byte: same `stty -g`, and the
# alternate screen left as often as it was entered. Also checks that an
# idle program does not wake up. Linux only.
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

# Context switches of every thread of a process, so far.
switches() {
  cat /proc/"$1"/task/*/status |
    awk '/^(non)?voluntary_ctxt_switches/ { s += $2 } END { print s }'
}

# How many times the pattern occurs in a file (0 is not an error).
count() {
  { grep -a -o "$1" "$2" || true; } | wc -l
}

# run NAME EXAMPLE EXPECTED_EXIT FEEDER: runs EXAMPLE under `script`,
# with FEEDER's output as its keyboard, and checks the result.
run() {
  local name="$1" example="$2" expected="$3" feeder="$4"
  local log="$dir/$name.log"
  "$feeder" "$example" | script -q -e -c \
    "stty -g > $dir/before; ./target/debug/examples/$example; echo EXIT=\$?; stty -g > $dir/after" \
    "$log" > /dev/null || true
  local got entered left
  got="$({ grep -a -o 'EXIT=[0-9]*' "$log" || true; } | tail -1)"
  entered="$(count $'\e\\[?1049h' "$log")"
  left="$(count $'\e\\[?1049l' "$log")"
  if [[ "$got" != "EXIT=$expected" ]]; then
    echo "FAIL $name: $got, expected EXIT=$expected"; fails=$((fails + 1)); return
  fi
  if ! cmp -s "$dir/before" "$dir/after"; then
    echo "FAIL $name: the terminal mode was not restored"; fails=$((fails + 1)); return
  fi
  if [[ "$entered" != 1 || "$left" != 1 ]]; then
    echo "FAIL $name: alternate screen entered $entered, left $left"; fails=$((fails + 1)); return
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
press_t() { sleep 1; printf 't'; sleep 1; }
sigint() { sleep 1; kill -INT "$(pid_of "$1")" || true; sleep 1; }
double_sigint() {
  sleep 1; local pid; pid="$(pid_of "$1")" || return 0
  kill -INT "$pid" || true; kill -INT "$pid" 2> /dev/null || true; sleep 1
}
# Measures context switches over one idle second, then quits.
idle_then_sigterm() {
  sleep 1; local pid a b; pid="$(pid_of "$1")" || return 0
  a="$(switches "$pid")"; sleep 1; b="$(switches "$pid")"
  echo "$((b - a))" > "$dir/wakeups"
  kill -TERM "$pid" || true; sleep 1
}

run counter counter 0 counter_keys
if grep -a -q 'Final count: 2' "$dir/counter.log"; then
  echo "ok   counter: keys reached the model (final count 2)"
else
  echo "FAIL counter: wrong or missing final count"; fails=$((fails + 1))
fi
run quit lifecycle 0 press_q
run ctrl-c-key lifecycle 0 press_ctrl_c
run panic-in-update lifecycle 101 press_p
run panic-in-task lifecycle 1 press_t
run sigint lifecycle 130 sigint
run double-sigint lifecycle 130 double_sigint
run sigterm lifecycle 0 idle_then_sigterm
wakeups="$(cat "$dir/wakeups" 2> /dev/null || echo unmeasured)"
if [[ "$wakeups" == 0 ]]; then
  echo "ok   idle: no wakeups in one idle second"
else
  echo "FAIL idle: $wakeups context switches in one idle second"; fails=$((fails + 1))
fi

[[ "$fails" == 0 ]] || { echo "$fails pty check(s) failed"; exit 1; }
echo "all pty checks passed"
