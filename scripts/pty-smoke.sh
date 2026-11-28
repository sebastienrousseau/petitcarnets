#!/usr/bin/env bash
# Runs the examples in a real pseudo-terminal (util-linux `script`) and
# checks, for every way a program can end, that the exit code is right
# and the terminal comes back byte for byte: same `stty -g`, and the
# alternate screen left as often as it was entered (the examples here
# set `alt_screen` in their views, as Bubble Tea's full-window examples
# do). Also checks that an idle program does not wake up, that mouse and
# paste bytes reach the model, that every input mode is turned off on
# the way out, and that an inline program prints above its frame and
# leaves it in place. Then the lifecycle: a child run with exec gets
# exactly the keys typed while it runs and the program the rest, with
# every mode back after; Ctrl-Z and `kill -TSTP` suspend and `kill
# -CONT` resumes with a fresh frame; NO_COLOR leaves no colour in frames or
# printed lines; replies to queries never reach the shell; a text
# input takes UTF-8 typed in a real terminal; and ACCESSIBLE=1 prints
# plain lines with no redraws. Linux only.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q --example counter --example lifecycle --example mouse --example package-manager \
  --example exec --example suspend --example query-term --example textinput
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

# How many times these exact bytes occur in a file.
count_fixed() {
  { grep -a -o -F -- "$1" "$2" || true; } | wc -l
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

# Each check below gives up after 30 seconds, so a lost key fails it
# rather than hanging it.
# Exec: the child (a shell `read`) gets the line typed while it runs,
# the program the keys typed after it; mouse, paste and keyboard
# enhancements are turned on again after it, and off at the end.
printf 'read line\necho "got:$line"\n' > "$dir/child.sh"
exec_keys() {
  sleep 1; printf 'e'; sleep 0.7; printf 'abc\r'; sleep 0.7; printf 'xyz'; sleep 0.3
  printf 'q'; sleep 1
}
log="$dir/exec.log"
exec_keys | timeout 30 script -q -e -c \
  "stty -g > $dir/before; ./target/debug/examples/exec sh $dir/child.sh; echo EXIT=\$?; stty -g > $dir/after" \
  "$log" > /dev/null || true
on_twice=1
for seq in $'\e[?1002h' $'\e[?2004h' $'\e[>1u'; do
  [[ "$(count_fixed "$seq" "$log")" == 2 ]] || on_twice=0
done
if ! grep -a -q 'EXIT=0' "$log" || ! cmp -s "$dir/before" "$dir/after"; then
  echo "FAIL exec: exit or terminal mode not restored"; fails=$((fails + 1))
elif [[ "$(grep -a -c 'got:abc' "$log")" != 1 ]] || grep -a -q 'got:abcx\|got:.*[xyz]' "$log"; then
  echo "FAIL exec: the child did not get exactly 'abc'"; fails=$((fails + 1))
elif ! grep -a -q 'keys after: x y z' "$log"; then
  echo "FAIL exec: the program did not get exactly x y z after the child"; fails=$((fails + 1))
elif [[ "$on_twice" != 1 ]] || [[ "$(count_fixed $'\e[?1002l' "$log")" != 2 ]]; then
  echo "FAIL exec: modes were not turned off for the child and on again after"; fails=$((fails + 1))
else
  echo "ok   exec: child read 'abc', program got 'x y z' after, modes back on, terminal restored"
fi

# Suspend, by Ctrl-Z and by `kill -TSTP`: the process stops (state T),
# `kill -CONT` continues it, the model hears Resume, the frame is drawn
# again, and the terminal comes back as it was. A small parent puts the
# program in a process group of its own in the foreground, as a shell
# with job control does, but leaves it there while it is stopped, so a
# bare `kill -CONT` resumes it (a shell's `fg` would send the same
# signal after taking the terminal back for it).
cat > "$dir/job.py" <<'JOB'
import os, signal, sys
signal.signal(signal.SIGTTOU, signal.SIG_IGN)
pid = os.fork()
if pid == 0:
    os.setpgid(0, 0)
    os.tcsetpgrp(0, os.getpid())
    signal.signal(signal.SIGTTOU, signal.SIG_DFL)
    os.execv(sys.argv[1], sys.argv[1:])
try:
    os.setpgid(pid, pid)
except OSError:
    pass
_, status = os.waitpid(pid, 0)
os.tcsetpgrp(0, os.getpgrp())
sys.exit(os.waitstatus_to_exitcode(status))
JOB
ctrl_z() { sleep 1; printf '\032'; }
sigtstp() { sleep 1; kill -TSTP "$(pid_of suspend)" || true; }
suspend_check() {
  local name="$1" stopper="$2" log="$dir/$1.log"
  { "$stopper"; sleep 0.5; local pid; pid="$(pid_of suspend)"
    awk '{ print $3 }' /proc/"$pid"/stat > "$dir/state" || true
    kill -CONT "$pid" || true; sleep 0.5; printf 'q'; sleep 1; } |
    timeout 30 script -q -e -c \
      "stty -g > $dir/before; python3 $dir/job.py ./target/debug/examples/suspend; echo EXIT=\$?; stty -g > $dir/after" \
      "$log" > /dev/null || true
  pkill -KILL -f 'target/debug/examples/suspend$' || true
  local frames
  frames="$(count_fixed 'Press ctrl-z to suspend' "$log")"
  if ! grep -a -q 'EXIT=0' "$log" || ! cmp -s "$dir/before" "$dir/after"; then
    echo "FAIL $name: exit or terminal mode not restored"; fails=$((fails + 1))
  elif [[ "$(cat "$dir/state")" != T ]]; then
    echo "FAIL $name: not stopped (state $(cat "$dir/state"))"; fails=$((fails + 1))
  elif ! grep -a -q 'resumed: 1' "$log" || [[ "$frames" -lt 2 ]]; then
    echo "FAIL $name: no Resume, or the frame was not drawn again"; fails=$((fails + 1))
  else
    echo "ok   $name: stopped (state T), kill -CONT resumed it, Resume delivered, frame redrawn, terminal restored"
  fi
}
suspend_check ctrl-z ctrl_z
suspend_check sigtstp sigtstp

# NO_COLOR: no colour parameter in any SGR sequence (frames and printed
# lines alike), while bold still shows; without it, colours do appear.
sgr_counts() {
  python3 - "$1" <<'PY'
import re, sys
d = open(sys.argv[1], "rb").read()
colour = attrs = 0
for m in re.finditer(rb"\x1b\[([0-9;:]*)m", d):
    for p in m.group(1).replace(b":", b";").split(b";"):
        n = int(p) if p else 0
        colour += n in (38, 39, 48, 49, 58, 59) or 30 <= n <= 37 or 40 <= n <= 47 \
            or 90 <= n <= 97 or 100 <= n <= 107
        attrs += 1 <= n <= 9
print(colour, attrs)
PY
}
for no_color in 1 ""; do
  sleep 3 | NO_COLOR="$no_color" TERM=xterm-256color PACKAGE_DELAY_MS=20 script -q -e -c \
    "./target/debug/examples/package-manager" "$dir/colour$no_color.log" > /dev/null || true
done
read -r plain bold <<< "$(sgr_counts "$dir/colour1.log")"
read -r coloured _ <<< "$(sgr_counts "$dir/colour.log")"
if [[ "$plain" == 0 && "$bold" -gt 0 && "$coloured" -gt 0 ]]; then
  echo "ok   no-color: no SGR colour with NO_COLOR=1 ($coloured without), bold still written"
else
  echo "FAIL no-color: $plain colour parameters with NO_COLOR=1, $bold attributes"; fails=$((fails + 1))
fi

# Replies never reach the shell: the program asks seven questions and
# quits at once. This feeder plays a terminal that answers 200 ms after
# it is asked (as over SSH), and answers the program's sentinel query
# (DA1) last if it was sent. Whatever arrives after the program has
# ended is echoed by the shell's terminal, after EXIT.
log="$dir/query.log"
replies() {
  for _ in $(seq 60); do
    grep -a -q -F $'\e]11;?' "$log" 2> /dev/null && break; sleep 0.05
  done
  sleep 0.2
  printf '\033]11;rgb:1111/2222/3333\033\\\033]10;rgb:eeee/eeee/eeee\033\\'
  printf '\033]12;rgb:0000/ffff/0000\033\\\033[5;7R\033P>|fake-term 1.0\033\\'
  printf '\033P1+r524742\033\\\033]52;c;aGVsbG8=\007'
  if grep -a -q -F $'\e[c' "$log"; then printf '\033[?62;22c'; fi
  sleep 1.5
}
replies | QUERY_TERM_EXIT=1 timeout 30 script -f -q -e -c \
  "stty -g > $dir/before; ./target/debug/examples/query-term; echo EXIT=\$?; sleep 1; stty -g > $dir/after" \
  "$log" > /dev/null || true
after="$(python3 -c '
import sys
d = open(sys.argv[1], "rb").read()
d = d[d.rfind(b"EXIT=") + 5:]
print(d[:d.find(b"Script done")].replace(b"\x1b", b"^[").decode(errors="replace"))
' "$log")"
if ! grep -a -q 'EXIT=0' "$log" || ! cmp -s "$dir/before" "$dir/after"; then
  echo "FAIL replies: exit or terminal mode not restored"; fails=$((fails + 1))
elif [[ "$after" == *'^['* || "$after" == *rgb* || "$after" == *fake-term* ]]; then
  echo "FAIL replies: reply bytes reached the shell: $(printf %q "$after" | head -c 300)"; fails=$((fails + 1))
else
  echo "ok   replies: seven queries, quit at once, no reply bytes after exit"
fi

# A text input in a real terminal: UTF-8 typed in one write (a wide
# character and a combining accent among it) reaches the component as
# text, the terminal's own cursor is shown and placed after it, and the
# terminal comes back as it was.
type_name() { sleep 1; printf 'Pik\xc3\xa1chu \xe5\xad\x97e\xcc\x81'; sleep 0.5; printf '\r'; sleep 1; }
log="$dir/textinput.log"
type_name | script -q -e -c \
  "stty -g > $dir/before; ./target/debug/examples/textinput; echo EXIT=\$?; stty -g > $dir/after" \
  "$log" > /dev/null || true
if ! grep -a -q 'EXIT=0' "$log" || ! cmp -s "$dir/before" "$dir/after"; then
  echo "FAIL textinput: exit or terminal mode not restored"; fails=$((fails + 1))
elif ! has "$log" $'Pik\xc3\xa1chu \xe5\xad\x97e\xcc\x81' || ! has "$log" $'\e[?25h'; then
  echo "FAIL textinput: the typed text or the shown cursor is missing"; fails=$((fails + 1))
else
  echo "ok   textinput: UTF-8 typed as text, the terminal's cursor shown, terminal restored"
fi

# Accessible output: with ACCESSIBLE=1 the counter (whose view asks for
# the alternate screen) prints each changed line once, as plain text:
# no alternate screen, no synchronized output, autowrap left on, no
# cursor moves or erases, the cursor shown, and no colour or style.
log="$dir/accessible.log"
counter_keys | ACCESSIBLE=1 TERM=xterm-256color script -q -e -c \
  "stty -g > $dir/before; ./target/debug/examples/counter; echo EXIT=\$?; stty -g > $dir/after" \
  "$log" > /dev/null || true
if ! grep -a -q 'EXIT=0' "$log" || ! cmp -s "$dir/before" "$dir/after"; then
  echo "FAIL accessible: exit or terminal mode not restored"; fails=$((fails + 1))
elif ! grep -a -q 'Final count: 2' "$log"; then
  echo "FAIL accessible: keys did not reach the model"; fails=$((fails + 1))
elif ! python3 - "$log" <<'PY'
import re, sys
d = open(sys.argv[1], "rb").read()
d = d[:d.rindex(b"EXIT=")]
# The terminal hides the cursor when it is entered; the program shows it
# before its first line and never hides it again.
shown = d.find(b"\x1b[?25h")
forbidden = [b"\x1b[?1049", b"\x1b[?2026", b"\x1b[?7l", b"\x1b[?25l", b"\x1b[J", b"\x1b[K", b"\x1b[2J"]
moves = re.findall(rb"\x1b\[[0-9;]*[ABCDGHfm]", d)
counts = [l.strip() for l in re.findall(rb"Count: -?\d+", d)]
want = [b"Count: %d" % n for n in (0, 1, 2, 1, 2)]
ok = 0 <= shown < d.find(b"Count") and not any(f in d[shown:] for f in forbidden)
ok = ok and not moves and counts == want
sys.exit(0 if ok else 1)
PY
then
  echo "FAIL accessible: frames were redrawn or styled, or lines are missing"; fails=$((fails + 1))
else
  echo "ok   accessible: each changed line printed once, plain, no redraws, terminal restored"
fi

[[ "$fails" == 0 ]] || { echo "$fails pty check(s) failed"; exit 1; }
echo "all pty checks passed"
