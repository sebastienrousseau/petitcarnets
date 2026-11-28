package main

import (
	"fmt"
	"image/color"
	"strings"

	"charm.land/bubbles/v2/list"
	"charm.land/bubbles/v2/progress"
	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/harmonica"
	"github.com/charmbracelet/x/ansi"
)

// targets are what the filter cases match against.
var targets = []string{
	"Raspberry Pi’s", "Nutella", "Bitter melon", "Nice socks", "Eight hours of sleep",
	"Cats", "Plantasia, the album", "Pour over coffee", "VR", "Noguchi Lamps", "Linux",
	"Business school", "Pottery", "Shampoo", "Table tennis", "Milk crates", "Afternoon tea",
	"Stickers", "20° Weather", "Warm light", "The vernal equinox", "Gaffer’s tape",
	"Terrycloth", "camelCaseName", "snake_case_name", "path/to/file.go", "Ünïcödé",
	"日本語のテキスト", "aaa", "abab", "ba", "",
}

func filters() {
	terms := []string{"a", "e", "ca", "te", "tea", "sn", "cn", "pt", "fg", "lux", "NUT",
		"ü", "日本", "テキ", "aa", "ab", "ba", "xyz", "the", "o o", "°"}
	for _, term := range terms {
		for _, sorted := range []bool{true, false} {
			var ranks []list.Rank
			if sorted {
				ranks = list.DefaultFilter(term, targets)
			} else {
				ranks = list.UnsortedFilter(term, targets)
			}
			parts := make([]string, len(ranks))
			for i, r := range ranks {
				idx := make([]string, len(r.MatchedIndexes))
				for j, m := range r.MatchedIndexes {
					idx[j] = fmt.Sprint(m)
				}
				parts[i] = fmt.Sprintf("%d:%s", r.Index, strings.Join(idx, ","))
			}
			row("filter", sorted, esc(term), strings.Join(parts, ";"))
		}
	}
	row("targets", esc(strings.Join(targets, "\x1f")))
}

// hex writes a colour as #rrggbb, as the terminal gets it.
func hex(c color.Color) string {
	r, g, b, _ := c.RGBA()
	return fmt.Sprintf("#%02x%02x%02x", r>>8, g>>8, b>>8)
}

func blends() {
	stops := [][]string{
		{"#5A56E0", "#EE6FF8"}, {"#FF7CCB", "#FDFF8C"}, {"#000000", "#FFFFFF"},
		{"#FF0000", "#00FF00", "#0000FF"}, {"#123456", "#ABCDEF", "#FEDCBA", "#000000"},
	}
	for _, s := range stops {
		colors := make([]color.Color, len(s))
		for i, h := range s {
			colors[i] = lipgloss.Color(h)
		}
		for _, steps := range []int{0, 1, 2, 3, 5, 10, 33, 80} {
			out := lipgloss.Blend1D(steps, colors...)
			hexes := make([]string, len(out))
			for i, c := range out {
				hexes[i] = hex(c)
			}
			row("blend", steps, strings.Join(s, ","), strings.Join(hexes, ","))
		}
	}
}

func springs() {
	for _, sp := range [][2]float64{{18, 1}, {18, 0.5}, {10, 2}, {0, 1}, {5, 0.1}} {
		s := harmonica.NewSpring(harmonica.FPS(60), sp[0], sp[1])
		pos, vel := 0.0, 0.0
		var steps []string
		for range 12 {
			pos, vel = s.Update(pos, vel, 1)
			steps = append(steps, fmt.Sprintf("%.17g,%.17g", pos, vel))
		}
		row("spring", sp[0], sp[1], strings.Join(steps, ";"))
	}
}

func progresses() {
	type bar struct {
		name string
		m    progress.Model
	}
	bars := []bar{
		{"default", progress.New()},
		{"narrow", progress.New(progress.WithWidth(12))},
		{"blend", progress.New(progress.WithDefaultBlend(), progress.WithWidth(20))},
		{"scaled", progress.New(progress.WithScaled(true), progress.WithColors(lipgloss.Color("#FF7CCB"), lipgloss.Color("#FDFF8C")), progress.WithWidth(30))},
		{"full-block", progress.New(progress.WithFillCharacters('█', '·'), progress.WithWidth(16), progress.WithoutPercentage())},
	}
	for _, b := range bars {
		for _, p := range []float64{-1, 0, 0.004, 0.125, 0.25, 0.5, 0.505, 0.999, 1, 2} {
			row("progress", b.name, p, esc(ansi.Strip(b.m.ViewAs(p))), esc(cellColors(b.m.ViewAs(p))))
		}
	}
}

// cellColors writes the colours of each cell of a rendered bar, as the
// SGR parameters before it, so the gradient can be compared.
func cellColors(s string) string {
	var out []string
	var params string
	p := ansi.NewParser()
	state := byte(0)
	for len(s) > 0 {
		seq, width, n, newState := ansi.DecodeSequence(s, state, p)
		state = newState
		s = s[n:]
		switch {
		case ansi.HasCsiPrefix(seq) && strings.HasSuffix(seq, "m"):
			params = seq[2 : len(seq)-1]
		case width > 0:
			out = append(out, params)
		}
	}
	return strings.Join(out, " ")
}
