package main

import (
	"fmt"
	"strings"

	"charm.land/bubbles/v2/list"
	"charm.land/bubbles/v2/table"
	"charm.land/bubbles/v2/viewport"
	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/x/ansi"
)

// numbered is n lines of content, some wider than others.
func numbered(n int) string {
	lines := make([]string, n)
	for i := range lines {
		lines[i] = fmt.Sprintf("line %d %s", i+1, strings.Repeat("ab字", i%4))
	}
	return strings.Join(lines, "\n")
}

func viewports() {
	cases := []struct {
		name         string
		w, h, lines  int
		soft, gutter bool
		keys         string
	}{
		{"empty", 20, 3, 0, false, false, ""},
		{"top", 20, 4, 10, false, false, ""},
		{"down", 20, 4, 10, false, false, "j j down"},
		{"page", 20, 4, 10, false, false, "f"},
		{"page-end", 20, 4, 10, false, false, "f f f f b"},
		{"half", 20, 4, 10, false, false, "d d u"},
		{"wheel", 20, 4, 10, false, false, "w:down w:down w:up"},
		{"right", 10, 4, 10, false, false, "l"},
		{"right-wide", 10, 4, 10, false, false, "j j j l"},
		{"wheel-right", 10, 4, 10, false, false, "w:shift+down w:right w:left"},
		{"soft", 10, 6, 6, true, false, ""},
		{"soft-down", 10, 4, 6, true, false, "j j j"},
		{"gutter", 20, 4, 10, false, true, "j"},
		{"gutter-soft", 14, 6, 6, true, true, "j"},
		{"short", 20, 6, 3, false, true, ""},
	}
	for _, c := range cases {
		v := viewport.New(viewport.WithWidth(c.w), viewport.WithHeight(c.h))
		v.SoftWrap = c.soft
		if c.gutter {
			v.LeftGutterFunc = func(info viewport.GutterContext) string {
				if info.Soft {
					return "   │ "
				}
				if info.Index >= info.TotalLines {
					return " ~ │ "
				}
				return fmt.Sprintf("%2d │ ", info.Index+1)
			}
		}
		if c.lines > 0 {
			v.SetContent(numbered(c.lines))
		}
		for _, msg := range messages(c.keys) {
			v, _ = v.Update(msg)
		}
		row("viewport", c.name, c.w, c.h, c.lines, c.soft, c.gutter, c.keys, v.YOffset(), v.XOffset(),
			fmt.Sprintf("%.3f", v.ScrollPercent()), fmt.Sprintf("%.3f", v.HorizontalScrollPercent()),
			v.TotalLineCount(), v.VisibleLineCount(), esc(ansi.Strip(v.View())))
	}
}

// cities are the table example's first rows.
var cities = [][]string{
	{"1", "Tokyo", "Japan", "37,274,000"}, {"2", "Delhi", "India", "32,065,760"},
	{"3", "Shanghai", "China", "28,516,904"}, {"4", "Dhaka", "Bangladesh", "22,478,116"},
	{"5", "São Paulo", "Brazil", "22,429,800"}, {"6", "Mexico City", "Mexico", "22,085,140"},
	{"7", "Cairo", "Egypt", "21,750,020"}, {"8", "Beijing", "China", "21,333,332"},
	{"9", "Mumbai", "India", "20,961,472"}, {"10", "Osaka", "Japan", "19,059,856"},
	{"11", "Chongqing", "China", "16,874,740"}, {"12", "Karachi", "Pakistan", "16,839,950"},
	{"13", "Istanbul", "Turkey", "15,636,243"}, {"14", "Kinshasa", "DR Congo", "15,628,085"},
	{"15", "Lagos", "Nigeria", "15,387,639"}, {"16", "Buenos Aires", "Argentina", "15,369,919"},
}

func tables() {
	keys := []string{"", "j", "j j j j j j j", "j j j j j j j k k k k k", "G", "G g", "f", "f f b", "d d u", "space space", "end k k k home"}
	for _, bordered := range []bool{false, true} {
		for _, k := range keys {
			rows := make([]table.Row, len(cities))
			for i, c := range cities {
				rows[i] = c
			}
			t := table.New(
				table.WithColumns([]table.Column{{Title: "Rank", Width: 4}, {Title: "City", Width: 10}, {Title: "Country", Width: 10}, {Title: "Population", Width: 10}}),
				table.WithRows(rows),
				table.WithFocused(true),
				table.WithHeight(7),
				table.WithWidth(42),
			)
			if bordered {
				s := table.DefaultStyles()
				s.Header = s.Header.BorderStyle(lipgloss.NormalBorder()).BorderBottom(true).Bold(false)
				s.Selected = s.Selected.Bold(false)
				t.SetStyles(s)
			}
			for _, msg := range messages(k) {
				t, _ = t.Update(msg)
			}
			row("table", bordered, k, t.Cursor(), esc(ansi.Strip(t.View())), esc(ansi.Strip(t.HelpView())))
		}
	}
}

// thing is a list item.
type thing struct{ title, desc string }

func (t thing) Title() string       { return t.title }
func (t thing) Description() string { return t.desc }
func (t thing) FilterValue() string { return t.title }

var things = []list.Item{
	thing{"Raspberry Pi’s", "I have ’em all over my house"}, thing{"Nutella", "It's good on toast"},
	thing{"Bitter melon", "It cools you down"}, thing{"Nice socks", "And by that I mean socks without holes"},
	thing{"Eight hours of sleep", "I had this once"}, thing{"Cats", "Usually"},
	thing{"Plantasia, the album", "My plants love it too"}, thing{"Pour over coffee", "It takes forever to make though"},
	thing{"VR", "Virtual reality...what is there to say?"}, thing{"Noguchi Lamps", "Such pleasing organic forms"},
	thing{"Linux", "Pretty much the best OS"}, thing{"Business school", "Just kidding"},
}

func lists() {
	cases := []struct {
		name string
		w, h int
		keys string
	}{
		{"default", 40, 20, ""},
		{"down", 40, 20, "j j"},
		{"next-page", 40, 20, "l"},
		{"end", 40, 20, "G"},
		{"wrap-page", 40, 20, "j j j j j j"},
		{"filter-start", 40, 20, "/"},
		{"filter-typing", 40, 20, "/ t:te"},
		{"filter-accept", 40, 20, "/ t:te enter"},
		{"filter-accept-down", 40, 20, "/ t:te enter j"},
		{"filter-cancel", 40, 20, "/ t:te esc"},
		{"filter-clear", 40, 20, "/ t:te enter esc"},
		{"filter-nothing", 40, 20, "/ t:zzz"},
		{"full-help", 40, 20, "?"},
		{"narrow", 22, 14, ""},
		{"short", 40, 8, "j"},
	}
	for _, c := range cases {
		l := list.New(things, list.NewDefaultDelegate(), c.w, c.h)
		l.Title = "My Fave Things"
		for _, msg := range messages(c.keys) {
			var cmd tea.Cmd
			l, cmd = l.Update(msg)
			// The filter's matches come back as a message.
			if cmd != nil {
				drain(&l, cmd)
			}
		}
		row("list", c.name, c.w, c.h, c.keys, l.Index(), l.FilterState().String(), esc(ansi.Strip(l.View())))
	}
}

// drain runs a command and the commands it gives, feeding the list the
// filter matches, as the runtime would, and leaving out timers.
func drain(l *list.Model, cmd tea.Cmd) {
	msg := cmd()
	switch msg := msg.(type) {
	case tea.BatchMsg:
		for _, c := range msg {
			if c != nil {
				drain(l, c)
			}
		}
	case list.FilterMatchesMsg:
		*l, _ = l.Update(msg)
	}
}
