#!/usr/bin/env bash
# Runs the examples in a real pseudo-terminal (util-linux `script`) and
# checks, for every way a program can end, that the exit code is right
# and the terminal comes back byte for byte: same `stty -g`, and the
# alternate screen left as often as it was entered (the examples here
# set `alt_screen` in their views, as Bubble Tea's full-window examples
# do). Also checks that an idle program does not wake up, that mouse and
# paste bytes reach the model, that every input mode is turned off on
# the way out, and that an inline program prints above its frame and
# leaves it in place. Linux only.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q --example counter --example lifecycle --example mouse --example package-manager
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

# Whether a log holds these exact bytes.
has() {
  grep -a -q -F -- "$2" "$1"
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
# An SGR mouse press and release at column 10, row 5, then q.
click() {
  sleep 1; printf '\033[<0;10;5M'; sleep 0.2; printf '\033[<0;10;5m'; sleep 0.2
  printf 'q'; sleep 1
}
# A bracketed paste in one write, then q.
paste_hello() {
  sleep 1; printf '\033[200~hello world\033[201~'; sleep 0.2; printf 'q'; sleep 1
}
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
# Standard input from /dev/null and standard output into a pipe: keys
# must come from /dev/tty, frames go there too, and the pipe gets only
# what the program prints after it ends.
counter_keys | script -q -e -c \
  "stty -g > $dir/before; ./target/debug/examples/counter < /dev/null | cat > $dir/piped; stty -g > $dir/after" \
  "$dir/tty.log" > /dev/null || true
if [[ "$(cat "$dir/piped")" == "Final count: 2" ]] && cmp -s "$dir/before" "$dir/after" &&
  grep -a -q 'Count:' "$dir/tty.log"; then
  echo "ok   dev-tty: drew on /dev/tty, read keys there, piped only the result"
else
  echo "FAIL dev-tty: piped '$(cat "$dir/piped")'"; fails=$((fails + 1))
fi
run quit lifecycle 0 press_q
run ctrl-c-key lifecycle 0 press_ctrl_c
run panic-in-update lifecycle 101 press_p
run panic-in-task lifecycle 1 press_t
run sigint lifecycle 130 sigint
run double-sigint lifecycle 130 double_sigint
run sigterm lifecycle 0 idle_then_sigterm
run mouse mouse 0 click
if grep -a -q 'Last click: (X: 9, Y: 4) left' "$dir/mouse.log"; then
  echo "ok   mouse: the SGR click reached the model at cell 9,4"
else
  echo "FAIL mouse: no click at 9,4"; fails=$((fails + 1))
fi
missing=""
for seq in $'\e[?7h' $'\e[?1002l' $'\e[?1003l' $'\e[?1006l' $'\e[?1004l' $'\e[?2004l' $'\e[>4m' $'\e[<u'; do
  has "$dir/mouse.log" "$seq" || missing+=" ${seq#$'\e'}"
done
if [[ -z "$missing" ]]; then
  echo "ok   modes: autowrap back on; mouse, focus, paste and keyboard enhancements off on exit"
else
  echo "FAIL modes: not turned off:$missing"; fails=$((fails + 1))
fi
run paste lifecycle 0 paste_hello
if grep -a -q 'pasted: hello world' "$dir/paste.log"; then
  echo "ok   paste: bracketed paste arrived as one paste"
else
  echo "FAIL paste: no 'hello world' paste"; fails=$((fails + 1))
fi
wakeups="$(cat "$dir/wakeups" 2> /dev/null || echo unmeasured)"
if [[ "$wakeups" == 0 ]]; then
  echo "ok   idle: no wakeups in one idle second"
else
  echo "FAIL idle: $wakeups context switches in one idle second"; fails=$((fails + 1))
fi

# Inline: the package manager prints each package above its progress
# line, never touches the alternate screen, and when it ends keeps its
# whole last frame (two rows: "Done!" and a blank one) with the cursor on
# a fresh line just below it: after the last print leaves the cursor on
# the frame's first row, the program only moves down two rows (CR LF or
# CUD) and erases below.
log="$dir/inline.log"
sleep 3 | PACKAGE_DELAY_MS=20 script -q -e -c \
  "stty -g > $dir/before; ./target/debug/examples/package-manager; echo EXIT=\$?; stty -g > $dir/after" \
  "$log" > /dev/null || true
printed="$(grep -a -o '✓.\{0,8\} [a-z-]*' "$log" | sed 's/.* //' | tr '\n' ' ')"
want="vegeutils libgardening currykit spicerack fullenglish eggy bad-kitty chai hojicha libtacos babys-monads libpurring "
if ! grep -a -q 'EXIT=0' "$log" || ! cmp -s "$dir/before" "$dir/after"; then
  echo "FAIL inline: exit or terminal mode not restored"; fails=$((fails + 1))
elif [[ "$(count $'\e\\[?1049' "$log")" != 0 ]]; then
  echo "FAIL inline: the alternate screen was used"; fails=$((fails + 1))
elif [[ "$printed" != "$want" ]]; then
  echo "FAIL inline: printed '$printed'"; fails=$((fails + 1))
elif [[ "$(count $'\e\\[1L' "$log")" != 12 ]]; then
  echo "FAIL inline: lines were not inserted above the frame"; fails=$((fails + 1))
elif ! python3 - "$log" <<'PY'
import re, sys
d = open(sys.argv[1], "rb").read()
tail = d[d.rfind(b"\x1b[?7l") + 5:]
tail = tail[:tail.index(b"\x1b[J")]
moves = re.fullmatch(rb"(?:\r\n|\x1b\[\d*B)*", tail)
down = sum(1 if m == b"\r\n" else int(m[2:-1] or 1)
           for m in re.findall(rb"\r\n|\x1b\[\d*B", tail))
sys.exit(0 if moves and down == 2 else 1)
PY
then
  echo "FAIL inline: the last frame was not kept with the cursor below it"; fails=$((fails + 1))
else
  echo "ok   inline: 12 lines printed above the frame, last frame kept, cursor below it, terminal restored"
fi

[[ "$fails" == 0 ]] || { echo "$fails pty check(s) failed"; exit 1; }
echo "all pty checks passed"
