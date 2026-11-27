#!/usr/bin/env python3
"""Write the text width tables under src/unicode/ from the Go sources.

Bubble Tea v2.2.0 measures text with github.com/charmbracelet/x/ansi
v0.12.0, which builds on three packages; the Rust port carries the same
data so it measures and splits text exactly as Go does:

  segment_table.in       grapheme break properties (uax29/v2 graphemes)
  wcwidth_table.in       ansi's wcRuneWidth: Go's unicode.Mn, Me and Cf
                         (from GOROOT), then go-runewidth with
                         EastAsianWidth false and StrictEmojiNeutral true
  displaywidth_table.in  displaywidth's per code point width, with
                         EastAsianWidth false (ambiguous is one cell)
  parser_table.in        ansi's DEC parser transition table, dumped by
                         the width fixture program

Each table but the last is a list of (first code point, value) pairs in
code point order: a value holds from its code point up to the next one.
The uax29 and displaywidth tables are read by walking their tries for
every code point, so nothing about their layout is assumed beyond the
lookup function they ship.

Usage, from the repository root (needs Go, for the module cache):

    (cd scripts/width-fixture && go mod download)
    python3 scripts/gen-width-tables.py
"""

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "scripts" / "width-fixture"
OUT = ROOT / "src" / "unicode"
MAX = 0x110000
# uax29's property bits, in the order Rust's segment.rs numbers them.
SEGMENT_PROPS = ["CR", "Control", "Extend", "ExtendedPictographic", "InCBConsonant",
                 "InCBExtend", "InCBLinker", "L", "LF", "LV", "LVT", "Prepend",
                 "RegionalIndicator", "SpacingMark", "T", "V", "ZWJ"]


def go(*args):
    return subprocess.run(["go", *args], cwd=FIXTURE, capture_output=True, text=True,
                          check=True).stdout.strip()


def module(path):
    """The module's directory in the cache and its version."""
    out = go("list", "-m", "-f", "{{.Dir}} {{.Version}}", path)
    moddir, version = out.split()
    return Path(moddir), f"{path}@{version}"


def go_array(text, name):
    """The entries of a sparse Go array literal `var name = [n]T{i: v, ...}`."""
    start = text.index(f"var {name} = [")
    size = int(re.match(r"\[(\d+)\]", text[text.index("[", start):]).group(1))
    body = text[text.index("{", start) + 1:text.index("\n}", start)]
    array = [0] * size
    for index, value in re.findall(r"(0x[0-9a-f]+):\s*(0x[0-9a-f]+)", body):
        array[int(index, 16)] = int(value, 16)
    return array


def trie(path, prefix):
    """A lookup function over a triegen trie, as the Go `lookup` walks it."""
    text = path.read_text()
    values, index = go_array(text, prefix + "Values"), go_array(text, prefix + "Index")

    def lookup(r):
        b = chr(r).encode()
        if len(b) == 1:
            return values[b[0]]
        i = index[b[0]]
        for c in b[1:-1]:
            i = index[(i << 6) + c]
        return values[(i << 6) + b[-1]]

    return lookup


def segment_table():
    moddir, version = module("github.com/clipperhouse/uax29/v2")
    path = moddir / "graphemes" / "trie.go"
    consts = re.search(r"const \(\n\t_CR property = 1 << iota\n(.*?)\)", path.read_text(), re.S)
    names = ["CR"] + re.findall(r"\t_(\w+)", consts.group(1))
    assert names == SEGMENT_PROPS, names
    lookup = trie(path, "graphemes")
    unicode = re.search(r"Public/([\d.]+)/", path.read_text()).group(1)
    return [lookup(r) if not 0xD800 <= r <= 0xDFFF else 0 for r in range(MAX)], \
        f"{version} (Unicode {unicode})"


def displaywidth_table():
    moddir, version = module("github.com/clipperhouse/displaywidth")
    lookup = trie(moddir / "trie.go", "stringWidth")
    # Property 1 is zero width, 2 wide, 3 East Asian ambiguous (one cell
    # with EastAsianWidth false); anything else is the default, one cell.
    widths = {1: 0, 2: 2}
    return [widths.get(lookup(r), 1) if not 0xD800 <= r <= 0xDFFF else 0
            for r in range(MAX)], version


def go_ranges(text, name):
    """Code points of a Go `table{{first, last}, ...}` variable."""
    body = text[text.index(f"var {name} = table{{"):]
    body = body[:body.index("\n}")]
    for first, last in re.findall(r"\{(0x[0-9A-Fa-f]+), (0x[0-9A-Fa-f]+)\}", body):
        yield from range(int(first, 16), int(last, 16) + 1)


def stdlib_zero_width():
    """Go's unicode.Mn, Me and Cf, from GOROOT/src/unicode/tables.go."""
    root = Path(go("env", "GOROOT"))
    text = (root / "src" / "unicode" / "tables.go").read_text()
    points = set()
    for name in ("Mn", "Me", "Cf"):
        body = text[text.index(f"var _{name} = &RangeTable{{"):]
        body = body[:body.index("\n}")]
        for lo, hi, stride in re.findall(r"\{(0x[0-9a-f]+), (0x[0-9a-f]+), (\d+)\}", body):
            points.update(range(int(lo, 16), int(hi, 16) + 1, int(stride)))
    unicode = re.search(r'const Version = "([\d.]+)"', text).group(1)
    return points, f"{go('env', 'GOVERSION')} unicode (Unicode {unicode})"


def wcwidth_table():
    moddir, version = module("github.com/mattn/go-runewidth")
    text = (moddir / "runewidth_table.go").read_text() + (moddir / "runewidth.go").read_text()
    zero = set(go_ranges(text, "combining")) | set(go_ranges(text, "nonprint"))
    double = set(go_ranges(text, "doublewidth"))
    marks, stdlib = stdlib_zero_width()
    widths = [1] * MAX
    for r in range(MAX):
        if r < 0x20 or 0x7F <= r < 0xA0 or r in marks or 0xD800 <= r <= 0xDFFF:
            widths[r] = 0
        elif r < 0x300:
            widths[r] = 0 if r == 0xAD else 1
        elif r in zero:
            widths[r] = 0
        elif r in double:
            widths[r] = 2
    return widths, f"{version} and {stdlib}"


def hexa(n):
    """A hex literal, its digits in groups of four from the right."""
    digits = f"{n:X}"
    groups = []
    while digits:
        groups.insert(0, digits[-4:])
        digits = digits[:-4]
    return "0x" + "_".join(groups)


def runs(values):
    """(first code point, value) where the value changes. Surrogates
    cannot be held in a Rust `char`, so they take their neighbour's value
    rather than add runs."""
    values = values[:0xD800] + [values[0xD7FF]] * 0x800 + values[0xE000:]
    return [(r, v) for r, v in enumerate(values) if r == 0 or values[r - 1] != v]


def write(name, what, source, entries, per_line):
    lines = ["    " + " ".join(e + "," for e in entries[i:i + per_line])
             for i in range(0, len(entries), per_line)]
    # A data file, pulled in with `include!`: an array expression, so it
    # needs no formatting exemption and is not counted as code by the
    # complexity gate.
    (OUT / name).write_text(
        f"// {what}, generated by scripts/gen-width-tables.py\n"
        f"// from {source}; do not edit. {len(entries)} entries.\n"
        "[\n" + "\n".join(lines) + "\n]\n"
    )
    print(f"{len(entries)} entries written to src/unicode/{name}")


def write_runs(name, what, table):
    values, source = table
    entries = [f"({hexa(r)}, {hexa(v)})" for r, v in runs(values)]
    write(name, what, source, entries, 4)


def main():
    write_runs("segment_table.in", "Grapheme break property bits by code point",
               segment_table())
    write_runs("wcwidth_table.in", "Cells per code point for ansi.WcWidth",
               wcwidth_table())
    write_runs("displaywidth_table.in", "Cells per grapheme cluster lead for ansi.GraphemeWidth",
               displaywidth_table())
    table = [int(b, 16) for b in go("run", ".", "parser-table").split()]
    _, ansi = module("github.com/charmbracelet/x/ansi")
    write("parser_table.in", "ANSI parser transitions (action << 4 | state) by state << 8 | byte",
          f"{ansi} parser.Table", [f"0x{b:02X}" for b in table], 16)


if __name__ == "__main__":
    main()
