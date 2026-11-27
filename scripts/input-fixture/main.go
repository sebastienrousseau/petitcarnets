// Command input-fixture writes tests/fixtures/input.tsv: a corpus of
// terminal input byte sequences and what Bubble Tea's input reader (the
// ultraviolet TerminalReader, at the version bubbletea v2.2.0 pins)
// decodes from each, in a plain text form the Rust tests compare against
// line for line.
//
// Regenerate from the repository root:
//
//	cd scripts/input-fixture && go run . > ../../tests/fixtures/input.tsv
//
// Each input is fed to a TerminalReader exactly as Bubble Tea sets one up
// (NewTerminalReader with zero-value Legacy flags and no terminfo), one
// read per chunk, then end of input, which flushes what is left as if the
// 50 ms escape timeout had passed.
//
// Line format, tab separated: the input as hex chunks separated by
// spaces, then one field per event. Strings inside events are hex, so the
// file has no escaping rules. A code point is U+XXXX; Go's special key
// codes above unicode.MaxRune are K<n>, n counted from KeyExtended; a code
// that is not a valid rune is written U+FFFD, as Rust cannot hold it.
//
// Left out of the corpus, because they depend on Windows or terminfo or
// cannot be represented on the Rust side: win32-input-mode sequences
// (CSI ... _), mouse coordinates outside 0..=65535, and kitty key codes
// whose printability depends on Unicode tables beyond the ones the Rust
// port carries (unassigned code points).
package main

import (
	"bytes"
	"context"
	"encoding/hex"
	"fmt"
	"image/color"
	"io"
	"os"
	"strings"
	"unicode/utf8"

	uv "github.com/charmbracelet/ultraviolet"
	"github.com/charmbracelet/x/ansi"
)

// chunks is one corpus input: what each read returns, in order.
type chunks [][]byte

func one(s string) chunks { return chunks{[]byte(s)} }

func split(parts ...string) chunks {
	var c chunks
	for _, p := range parts {
		c = append(c, []byte(p))
	}
	return c
}

// reader returns one chunk per Read, then io.EOF.
type reader struct{ c chunks }

func (r *reader) Read(p []byte) (int, error) {
	if len(r.c) == 0 {
		return 0, io.EOF
	}
	n := copy(p, r.c[0])
	r.c = r.c[1:]
	return n, nil
}

func decode(in chunks) []uv.Event {
	r := uv.NewTerminalReader(&reader{c: append(chunks(nil), in...)}, "xterm-256color")
	events := make(chan uv.Event, 4096)
	if err := r.StreamEvents(context.Background(), events); err != nil {
		panic(err)
	}
	close(events)
	var out []uv.Event
	for e := range events {
		out = append(out, e)
	}
	return out
}

func hexs(s string) string { return hex.EncodeToString([]byte(s)) }

func code(r rune) string {
	switch {
	case r >= uv.KeyExtended:
		return fmt.Sprintf("K%d", r-uv.KeyExtended)
	case !utf8.ValidRune(r):
		return "U+FFFD"
	default:
		return fmt.Sprintf("U+%04X", r)
	}
}

func optCode(r rune) string {
	if r == 0 {
		return "-"
	}
	return code(r)
}

func key(k uv.Key) string {
	return fmt.Sprintf("code=%s text=%s mod=%d shifted=%s base=%s repeat=%t str=%s ks=%s",
		code(k.Code), hexs(k.Text), int(k.Mod), optCode(k.ShiftedCode), optCode(k.BaseCode),
		k.IsRepeat, hexs(k.String()), hexs(k.Keystroke()))
}

func mouse(kind string, m uv.Mouse, str string) string {
	return fmt.Sprintf("%s x=%d y=%d button=%s mod=%d str=%s",
		kind, m.X, m.Y, m.Button.String(), int(m.Mod), hexs(str))
}

type darkColor interface{ IsDark() bool }

func colour(kind string, c color.Color, e darkColor) string {
	if c == nil {
		return kind + " nil"
	}
	r, g, b, _ := c.RGBA()
	shift := func(v uint32) uint32 {
		if v > 0xff {
			return v >> 8
		}
		return v
	}
	return fmt.Sprintf("%s #%02x%02x%02x dark=%t", kind, shift(r), shift(g), shift(b), e.IsDark())
}

func event(e uv.Event) string {
	switch e := e.(type) {
	case uv.KeyPressEvent:
		return "press " + key(uv.Key(e))
	case uv.KeyReleaseEvent:
		return "release " + key(uv.Key(e))
	case uv.MouseClickEvent:
		return mouse("click", uv.Mouse(e), e.String())
	case uv.MouseReleaseEvent:
		return mouse("mouse-release", uv.Mouse(e), e.String())
	case uv.MouseWheelEvent:
		return mouse("wheel", uv.Mouse(e), e.String())
	case uv.MouseMotionEvent:
		return mouse("motion", uv.Mouse(e), e.String())
	case uv.PasteEvent:
		return "paste " + hexs(e.Content)
	case uv.PasteStartEvent:
		return "paste-start"
	case uv.PasteEndEvent:
		return "paste-end"
	case uv.FocusEvent:
		return "focus"
	case uv.BlurEvent:
		return "blur"
	case uv.CursorPositionEvent:
		return fmt.Sprintf("cursor x=%d y=%d", e.X, e.Y)
	case uv.KeyboardEnhancementsEvent:
		return fmt.Sprintf("keyboard flags=%d", e.Flags)
	case uv.ModeReportEvent:
		_, dec := e.Mode.(ansi.DECMode)
		return fmt.Sprintf("mode dec=%t mode=%d value=%d", dec, e.Mode.Mode(), int(e.Value))
	case uv.TerminalVersionEvent:
		return "version " + hexs(e.Name)
	case uv.ForegroundColorEvent:
		return colour("fg", e.Color, e)
	case uv.BackgroundColorEvent:
		return colour("bg", e.Color, e)
	case uv.CursorColorEvent:
		return colour("cursor-colour", e.Color, e)
	default:
		return "other " + strings.TrimPrefix(fmt.Sprintf("%T", e), "uv.")
	}
}

func line(in chunks) string {
	var parts []string
	for _, c := range in {
		parts = append(parts, hex.EncodeToString(c))
	}
	fields := []string{strings.Join(parts, " ")}
	for _, e := range decode(in) {
		fields = append(fields, event(e))
	}
	return strings.Join(fields, "\t")
}

func main() {
	var out bytes.Buffer
	out.WriteString("# Generated by scripts/input-fixture (go run .); do not edit.\n")
	out.WriteString("# input hex chunks<TAB>events decoded by ultraviolet's TerminalReader\n")
	seen := map[string]bool{}
	for _, in := range corpus() {
		l := line(in)
		input := strings.SplitN(l, "\t", 2)[0]
		if seen[input] {
			continue
		}
		seen[input] = true
		out.WriteString(l)
		out.WriteByte('\n')
	}
	if _, err := os.Stdout.Write(out.Bytes()); err != nil {
		panic(err)
	}
}
