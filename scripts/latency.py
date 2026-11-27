#!/usr/bin/env python3
"""Keystroke-to-echo latency in a real pseudo-terminal.

Runs the counter example (release build) in a pty, presses `+` N times,
and times each press from the write to the first byte of the frame that
answers it, then reads until the frame is complete (no more bytes for
20 ms) before the next press. Prints the median, p99 and max.

Usage, from the repository root:

    cargo build --release --example counter
    python3 scripts/latency.py [N]
"""

import os
import pty
import select
import statistics
import sys
import time

BINARY = "target/release/examples/counter"


def drain(fd, quiet):
    """Reads until nothing arrives for `quiet` seconds."""
    try:
        while select.select([fd], [], [], quiet)[0]:
            if not os.read(fd, 65536):
                return
    except OSError:
        return  # the program has ended and closed the terminal


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    pid, fd = pty.fork()
    if pid == 0:
        os.execv(BINARY, [BINARY])
    drain(fd, 0.5)
    times = []
    for _ in range(n):
        start = time.perf_counter()
        os.write(fd, b"+")
        if not select.select([fd], [], [], 2.0)[0]:
            sys.exit("no frame within 2 s")
        os.read(fd, 65536)
        times.append((time.perf_counter() - start) * 1000)
        drain(fd, 0.02)
    os.write(fd, b"q")
    drain(fd, 0.5)
    os.waitpid(pid, 0)
    times.sort()
    p99 = times[min(len(times) - 1, int(len(times) * 0.99))]
    print(f"keys {n}: median {statistics.median(times):.3f} ms, "
          f"p99 {p99:.3f} ms, max {times[-1]:.3f} ms")


if __name__ == "__main__":
    main()
