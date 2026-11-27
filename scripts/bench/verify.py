#!/usr/bin/env python3
"""Replays the bytes a benchmark run kept (`--dump DIR` / `-dump DIR`)
through a small 80x24 terminal model and checks that after every frame
the screen shows that frame's text (styles are not checked).

    python3 verify.py DIR

The terminal model knows the sequences these renderers write: cursor
moves, erases, scrolls, insert and delete characters, CR, LF, BS and
tab. Other sequences (modes, SGR, queries) change no text and are
skipped. Files named `go-noinput-*` come from a program whose output is
not a terminal, so LF is read as CR LF, as a terminal's ONLCR setting
would; everything else is read as raw-mode output.
"""

import pathlib
import re
import sys

W, H = 80, 24
SPINNER = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
LETTERS = ["ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz"]
CSI = re.compile(r"\x1b\[([?>=<]?)([0-9;:]*)([$ ]?)([@-~])")


def view(scenario, n):
    """Frame n's text, as scenario.rs and scenario.go build it."""
    if scenario == "counter":
        return f"Count: {n}\n\n+/- or arrows to change, q to quit"
    if scenario == "list":
        return "\n".join(
            f"\x1b[1;36m> item {i:02}\x1b[0m" if i == n % 20 else f"  item {i:02}"
            for i in range(20)
        )
    if scenario == "log":
        return "\n".join(
            f"{k:06} GET /api/v1/items/{k * 7 % 1000:<4} 200 {k * 37 % 997:>3} ms user={k * 13 % 10000:04}"
            for k in range(n, n + 24)
        )
    if scenario == "spinner":
        return SPINNER[n % 10] + " Loading forty-two items..."
    return "\n".join(
        "".join(LETTERS[n % 2][mix(c, r, n) % 26] for c in range(79)) for r in range(24)
    )


def mix(c, r, n):
    """A 32-bit hash of a cell's column, row and frame."""
    m = 0xFFFFFFFF
    x = (c * 0x9E3779B1 + r * 0x85EBCA77 + n * 0xC2B2AE3D) & m
    x ^= x >> 15
    x = (x * 0x2C1B3C6D) & m
    x ^= x >> 12
    return x


def fnv(scenario, frames):
    h = 0xCBF29CE484222325
    for n in range(frames + 1):
        for b in view(scenario, n).encode() + b"\0":
            h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


class Term:
    def __init__(self, onlcr):
        self.rows = [[" "] * W for _ in range(H)]
        self.x = self.y = 0
        self.onlcr = onlcr

    def scroll(self, n):
        for _ in range(n):
            self.rows.pop(0)
            self.rows.append([" "] * W)

    def lf(self):
        if self.y == H - 1:
            self.scroll(1)
        else:
            self.y += 1

    def put(self, ch):
        if self.x < W:
            self.rows[self.y][self.x] = ch
            self.x += 1

    def erase(self, mode):
        if mode == 2:
            self.rows = [[" "] * W for _ in range(H)]
            return
        self.rows[self.y][self.x:] = [" "] * (W - self.x)
        for r in range(self.y + 1, H):
            self.rows[r] = [" "] * W

    def csi(self, prefix, params, final):
        if prefix or final in "mhlpuW":
            return
        ps = [int(p) if p else 0 for p in params.split(";")] if params else [0]
        n = max(ps[0], 1)
        row = self.rows[self.y]
        if final == "A":
            self.y = max(self.y - n, 0)
        elif final == "B":
            self.y = min(self.y + n, H - 1)
        elif final == "C":
            self.x = min(self.x + n, W - 1)
        elif final == "D":
            self.x = max(min(self.x, W - 1) - n, 0)
        elif final == "G":
            self.x = n - 1
        elif final == "d":
            self.y = n - 1
        elif final in "Hf":
            self.y = n - 1
            self.x = max(ps[1], 1) - 1 if len(ps) > 1 else 0
        elif final == "J":
            self.erase(ps[0])
        elif final == "K":
            a, b = {0: (self.x, W), 1: (0, self.x + 1), 2: (0, W)}[ps[0]]
            row[a:b] = [" "] * (b - a)
        elif final == "X":
            row[self.x : self.x + n] = [" "] * min(n, W - self.x)
        elif final == "P":
            del row[self.x : self.x + n]
            row.extend([" "] * (W - len(row)))
        elif final == "@":
            row[self.x : self.x] = [" "] * n
            del row[W:]
        elif final == "S":
            self.scroll(n)
        else:
            raise ValueError(f"unhandled CSI {prefix}{params}{final}")

    def feed(self, text):
        i = 0
        while i < len(text):
            ch = text[i]
            if ch == "\x1b":
                m = CSI.match(text, i)
                if m:
                    self.csi(m.group(1) or m.group(3), m.group(2), m.group(4))
                    i = m.end()
                    continue
                i += 2  # ESC and one character, such as ESC 7
                continue
            if ch == "\r":
                self.x = 0
            elif ch == "\n":
                if self.onlcr:
                    self.x = 0
                self.lf()
            elif ch == "\b":
                self.x = max(min(self.x, W - 1) - 1, 0)
            elif ch == "\t":
                self.x = min((self.x // 8 + 1) * 8, W - 1)
            elif ch >= " ":
                self.put(ch)
            i += 1

    def text(self):
        return ["".join(r).rstrip() for r in self.rows]


def expected(scenario, n):
    lines = re.sub(r"\x1b\[[0-9;]*m", "", view(scenario, n)).split("\n")
    return [l.rstrip() for l in lines] + [""] * (H - len(lines))


def check(out, frames_file):
    name = out.stem
    scenario = name.rsplit("-", 1)[1]
    data = out.read_bytes()
    sizes = [int(s) for s in frames_file.read_text().split()]
    term = Term(onlcr=name.startswith("go-noinput-"))
    offset = 0
    for n, size in enumerate(sizes):
        term.feed(data[offset : offset + size].decode())
        offset += size
        if term.text() != expected(scenario, n):
            got, want = term.text(), expected(scenario, n)
            row = next(i for i in range(H) if got[i] != want[i])
            return f"{name}: frame {n} differs at row {row}: {got[row]!r} != {want[row]!r}"
    return f"{name}: {len(sizes)} frames ok"


def main():
    d = pathlib.Path(sys.argv[1])
    bad = 0
    for out in sorted(d.glob("*.out")):
        result = check(out, out.with_suffix(".frames"))
        bad += "differs" in result
        print(result)
    print("hashes:", ", ".join(f"{s}: {fnv(s, 100):016x}" for s in
                              ["counter", "list", "log", "spinner", "full"]))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
