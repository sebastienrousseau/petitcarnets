// Command bubbles-fixture writes what Bubbles v2 (charm.land/bubbles/v2)
// does, for the Rust port in src/bubbles to be checked against: every
// default key binding with its help, the spinners, help and paginator
// views, text input and text area editing sequences (keys in, value,
// cursor and view out), list filter ranks, colour blends and the
// progress bar's spring.
//
// Regenerate from the repository root:
//
//	cd scripts/bubbles-fixture
//	go run . > ../../tests/fixtures/bubbles.tsv
//
// Each line is tab separated, its first field the kind. Text fields
// are escaped: a backslash, a tab and a line break are written \\, \t
// and \n. Views are written without their escape sequences.
package main

import (
	"fmt"
	"os"
	"reflect"
	"strings"

	"charm.land/bubbles/v2/key"
	"charm.land/bubbles/v2/list"
	"charm.land/bubbles/v2/paginator"
	"charm.land/bubbles/v2/spinner"
	"charm.land/bubbles/v2/table"
	"charm.land/bubbles/v2/textarea"
	"charm.land/bubbles/v2/textinput"
	"charm.land/bubbles/v2/viewport"
)

var out = os.Stdout

// esc escapes a text field.
func esc(s string) string {
	s = strings.ReplaceAll(s, "\\", "\\\\")
	s = strings.ReplaceAll(s, "\t", "\\t")
	return strings.ReplaceAll(s, "\n", "\\n")
}

// row writes one line.
func row(fields ...any) {
	parts := make([]string, len(fields))
	for i, f := range fields {
		parts[i] = fmt.Sprint(f)
	}
	fmt.Fprintln(out, strings.Join(parts, "\t"))
}

// keymap writes every binding of a KeyMap struct.
func keymap(component string, km any) {
	v := reflect.ValueOf(km)
	t := v.Type()
	for i := range t.NumField() {
		b, ok := v.Field(i).Interface().(key.Binding)
		if !ok {
			continue
		}
		enabled := 0
		if b.Enabled() {
			enabled = 1
		}
		row("keymap", component, t.Field(i).Name, strings.Join(b.Keys(), " "),
			esc(b.Help().Key), esc(b.Help().Desc), enabled)
	}
}

func keymaps() {
	keymap("textinput", textinput.DefaultKeyMap())
	keymap("textarea", textarea.DefaultKeyMap())
	keymap("viewport", viewport.DefaultKeyMap())
	keymap("list", list.DefaultKeyMap())
	keymap("table", table.DefaultKeyMap())
	keymap("paginator", paginator.DefaultKeyMap())
}

func spinners() {
	all := []struct {
		name string
		s    spinner.Spinner
	}{
		{"LINE", spinner.Line}, {"DOT", spinner.Dot}, {"MINI_DOT", spinner.MiniDot},
		{"JUMP", spinner.Jump}, {"PULSE", spinner.Pulse}, {"POINTS", spinner.Points},
		{"GLOBE", spinner.Globe}, {"MOON", spinner.Moon}, {"MONKEY", spinner.Monkey},
		{"METER", spinner.Meter}, {"HAMBURGER", spinner.Hamburger}, {"ELLIPSIS", spinner.Ellipsis},
	}
	for _, s := range all {
		row("spinner", s.name, s.s.FPS.Nanoseconds(), esc(strings.Join(s.s.Frames, "\x1f")))
	}
}

func main() {
	keymaps()
	spinners()
	helps()
	paginators()
	textinputs()
	textareas()
	filters()
	blends()
	springs()
	progresses()
	viewports()
	tables()
	lists()
}
