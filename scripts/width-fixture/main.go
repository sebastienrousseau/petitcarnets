// Command width-fixture writes tests/fixtures/width.tsv: a corpus of
// strings and how github.com/charmbracelet/x/ansi v0.12.0 (the version
// bubbletea v2.2.0 pins) measures and splits each one, in a plain text
// form the Rust tests compare against line for line.
//
// Regenerate from the repository root:
//
//	cd scripts/width-fixture && go run . > ../../tests/fixtures/width.tsv
//
// With the argument parser-table it prints ansi's parser transition table
// instead, as hex bytes, for scripts/gen-width-tables.py.
//
// Line format, five tab separated fields:
//
//  1. the string, hex encoded (so tabs and newlines need no escaping);
//  2. ansi.StringWidthWc of it;
//  3. ansi.StringWidth of it;
//  4. the pieces the cell buffer reads it in with ansi.WcWidth;
//  5. the same with ansi.GraphemeWidth.
//
// A piece list is comma separated len:width pairs, len in bytes and width
// in cells (0 for escape sequences and controls), empty for an empty
// string. The pieces are those ultraviolet's printString (styled.go)
// steps through: ansi.DecodeSequenceWc or ansi.DecodeSequence from the
// normal state, then, for a single printable ASCII byte followed by a byte
// of 0xC0 or more, the whole grapheme cluster from
// ansi.FirstGraphemeCluster when it is longer than one byte.
//
// Every string is valid UTF-8, since Rust strings must be. A string whose
// pieces would split a character is left out: that happens only when a
// UTF-8 continuation byte 0x9C (the 8-bit string terminator) ends an OSC,
// DCS, SOS, PM or APC string partway through a character. The count left
// out is printed on standard error.
package main

import (
	"bufio"
	"encoding/hex"
	"fmt"
	"os"
	"strings"
	"unicode/utf8"

	"github.com/charmbracelet/x/ansi"
	"github.com/charmbracelet/x/ansi/parser"
)

// pieces is printString's loop, recording each piece's length and width.
func pieces(str string, method ansi.Method) (string, bool) {
	decoder := ansi.DecodeSequenceWc[string]
	if method == ansi.GraphemeWidth {
		decoder = ansi.DecodeSequence[string]
	}
	p := ansi.GetParser()
	defer ansi.PutParser(p)
	var out []string
	for len(str) > 0 {
		seq, width, n, _ := decoder(str, 0, p)
		if n == 1 && seq[0] > 0x1f && seq[0] < 0x7f && len(str) > 1 && str[1] >= 0xc0 {
			if cluster, cw := ansi.FirstGraphemeCluster(str, method); len(cluster) > 1 {
				width, n = cw, len(cluster)
			}
		}
		if n < len(str) && !utf8.RuneStart(str[n]) {
			return "", false
		}
		out = append(out, fmt.Sprintf("%d:%d", n, width))
		str = str[n:]
	}
	return strings.Join(out, ","), true
}

func parserTable() {
	var out []string
	for i := range int(parser.Utf8State+1) << parser.IndexStateShift {
		out = append(out, fmt.Sprintf("%02x", parser.Table[i]))
	}
	fmt.Println(strings.Join(out, " "))
}

func main() {
	if len(os.Args) > 1 && os.Args[1] == "parser-table" {
		parserTable()
		return
	}
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	seen := map[string]bool{}
	skipped := 0
	for _, s := range corpus() {
		if seen[s] {
			continue
		}
		seen[s] = true
		if !utf8.ValidString(s) {
			panic(fmt.Sprintf("invalid UTF-8 in corpus: %q", s))
		}
		wc, ok1 := pieces(s, ansi.WcWidth)
		gr, ok2 := pieces(s, ansi.GraphemeWidth)
		if !ok1 || !ok2 {
			skipped++
			continue
		}
		fmt.Fprintf(w, "%s\t%d\t%d\t%s\t%s\n", hex.EncodeToString([]byte(s)),
			ansi.StringWidthWc(s), ansi.StringWidth(s), wc, gr)
	}
	fmt.Fprintf(os.Stderr, "%d strings written, %d left out\n", len(seen)-skipped, skipped)
}
