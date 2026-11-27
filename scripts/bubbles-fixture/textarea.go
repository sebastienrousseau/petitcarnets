package main

import (
	"charm.land/bubbles/v2/textarea"
	"github.com/charmbracelet/x/ansi"
)

// areaCase is a text area set up, then sent keys.
type areaCase struct {
	name        string
	width       int
	height      int
	numbers     bool
	limit       int
	maxHeight   int
	placeholder string
	keys        string
}

var areaCases = []areaCase{
	{name: "empty", width: 20, height: 3, numbers: true},
	{name: "placeholder", width: 20, height: 3, numbers: true, placeholder: "Once upon a time..."},
	{name: "placeholder-wrap", width: 16, height: 4, numbers: true, placeholder: "Once upon a time there was a cat"},
	{name: "placeholder-plain", width: 16, height: 3, placeholder: "Tell me a story"},
	{name: "type", width: 20, height: 3, numbers: true, keys: "t:hello·world"},
	{name: "lines", width: 20, height: 4, numbers: true, keys: "t:one enter t:two enter t:three"},
	{name: "no-numbers", width: 20, height: 3, keys: "t:one enter t:two"},
	{name: "soft-wrap", width: 16, height: 4, numbers: true, keys: "t:the·quick·brown·fox·jumps·over"},
	{name: "long-word", width: 14, height: 4, numbers: true, keys: "t:abcdefghijklmnopqrstuvwxyz"},
	{name: "up-down", width: 20, height: 4, numbers: true, keys: "t:one enter t:two·three enter t:x up up end down"},
	{name: "up-wrapped", width: 16, height: 5, numbers: true, keys: "t:the·quick·brown·fox·jumps up up"},
	{name: "backspace-join", width: 20, height: 3, numbers: true, keys: "t:ab enter t:cd home backspace"},
	{name: "delete-join", width: 20, height: 3, numbers: true, keys: "t:ab enter t:cd up end delete"},
	{name: "kill-join", width: 20, height: 3, numbers: true, keys: "t:ab enter t:cd up end ctrl+k"},
	{name: "kill", width: 20, height: 3, numbers: true, keys: "t:hello·world home right right ctrl+k"},
	{name: "unkill", width: 20, height: 3, numbers: true, keys: "t:hello·world left left ctrl+u"},
	{name: "unkill-join", width: 20, height: 3, numbers: true, keys: "t:ab enter t:cd home ctrl+u"},
	{name: "words", width: 30, height: 3, numbers: true, keys: "t:one·two·three alt+left alt+left alt+right"},
	{name: "word-left-across", width: 30, height: 3, numbers: true, keys: "t:one enter t:two home alt+left"},
	{name: "word-right-across", width: 30, height: 3, numbers: true, keys: "t:one enter t:two up end alt+right"},
	{name: "delete-word", width: 30, height: 3, numbers: true, keys: "t:one·two·three ctrl+w alt+left alt+d"},
	{name: "delete-word-lead", width: 30, height: 3, numbers: true, keys: "t:·foo ctrl+w"},
	{name: "delete-word-join", width: 30, height: 3, numbers: true, keys: "t:a enter t:b home ctrl+w"},
	{name: "upper", width: 30, height: 3, numbers: true, keys: "t:hello·world home alt+u"},
	{name: "lower", width: 30, height: 3, numbers: true, keys: "t:HELLO·WORLD home alt+right alt+l"},
	{name: "capitalize", width: 30, height: 3, numbers: true, keys: "t:hello·world home alt+c alt+c"},
	{name: "transpose", width: 30, height: 3, numbers: true, keys: "t:abc left ctrl+t"},
	{name: "transpose-end", width: 30, height: 3, numbers: true, keys: "t:abc ctrl+t"},
	{name: "begin-end", width: 30, height: 3, numbers: true, keys: "t:a enter t:b enter t:c alt+< alt+>"},
	{name: "scroll", width: 20, height: 2, numbers: true, keys: "t:1 enter t:2 enter t:3 enter t:4"},
	{name: "scroll-up", width: 20, height: 2, numbers: true, keys: "t:1 enter t:2 enter t:3 enter t:4 up up up"},
	{name: "page", width: 20, height: 2, numbers: true, keys: "t:1 enter t:2 enter t:3 enter t:4 enter t:5 pgup pgup pgdown"},
	{name: "limit", width: 20, height: 3, numbers: true, limit: 5, keys: "t:abc enter t:defg"},
	{name: "max-height", width: 20, height: 3, numbers: true, maxHeight: 2, keys: "t:a enter t:b enter t:c"},
	{name: "paste", width: 20, height: 4, numbers: true, keys: "t:ab left p:x¶y¶z"},
	{name: "tab-paste", width: 20, height: 3, numbers: true, keys: "p:a⇥b"},
	{name: "select-type", width: 20, height: 3, numbers: true, keys: "t:hello shift+left shift+left t:X"},
	{name: "select-all-delete", width: 20, height: 3, numbers: true, keys: "t:ab enter t:cd ctrl+g backspace"},
	{name: "select-lines", width: 20, height: 3, numbers: true, keys: "t:ab enter t:cd up shift+down delete"},
	{name: "select-move-clears", width: 20, height: 3, numbers: true, keys: "t:abc shift+left right t:Z"},
	{name: "wide", width: 12, height: 3, numbers: true, keys: "t:日本語の文字です"},
	{name: "wide-move", width: 12, height: 3, numbers: true, keys: "t:日本語の文字です up"},
	{name: "accent-left", width: 20, height: 2, keys: "t:cafe\u0301s left left"},
	{name: "accent-backspace", width: 20, height: 2, keys: "t:cafe\u0301 backspace"},
	{name: "emoji-backspace", width: 20, height: 2, keys: "t:a\U0001F44D\U0001F3FD backspace"},
	{name: "zwj-delete", width: 20, height: 2, keys: "t:\U0001F468\u200d\U0001F469\u200d\U0001F467x home delete"},
}

func textareas() {
	for _, c := range areaCases {
		m := textarea.New()
		m.ShowLineNumbers = c.numbers
		if c.maxHeight > 0 {
			m.MaxHeight = c.maxHeight
		}
		m.SetWidth(c.width)
		m.SetHeight(c.height)
		m.CharLimit = c.limit
		m.Placeholder = c.placeholder
		m.Focus()
		for _, msg := range messages(c.keys) {
			m, _ = m.Update(msg)
		}
		row("textarea", c.name, c.width, c.height, c.numbers, c.limit, c.maxHeight,
			esc(c.placeholder), c.keys, esc(m.Value()), m.Line(), m.Column(),
			esc(ansi.Strip(m.View())))
	}
}
