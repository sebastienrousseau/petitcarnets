package main

import (
	"charm.land/bubbles/v2/help"
	"charm.land/bubbles/v2/paginator"
	"charm.land/bubbles/v2/table"
	"github.com/charmbracelet/x/ansi"
)

func helps() {
	km := table.DefaultKeyMap()
	for _, all := range []bool{false, true} {
		for _, w := range []int{0, 10, 20, 26, 30, 40, 60, 80} {
			h := help.New()
			h.ShowAll = all
			h.SetWidth(w)
			row("help", w, all, esc(ansi.Strip(h.View(km))))
		}
	}
}

func paginators() {
	for _, kind := range []paginator.Type{paginator.Arabic, paginator.Dots} {
		for _, items := range []int{0, 1, 7, 30} {
			p := paginator.New()
			p.Type = kind
			p.PerPage = 3
			p.SetTotalPages(items)
			for range 2 {
				p.NextPage()
			}
			start, end := p.GetSliceBounds(items)
			row("paginator", int(kind), items, p.Page, p.TotalPages, start, end, p.ItemsOnPage(items), esc(p.View()))
		}
	}
}
