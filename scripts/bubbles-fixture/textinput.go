package main

import (
	"strings"

	"charm.land/bubbles/v2/textinput"
	"github.com/charmbracelet/x/ansi"
)

// inputCase is a text input set up, then sent keys.
type inputCase struct {
	name        string
	width       int
	limit       int
	echo        textinput.EchoMode
	placeholder string
	suggestions []string
	keys        string
}

var inputCases = []inputCase{
	{name: "empty"},
	{name: "placeholder", placeholder: "Pikachu"},
	{name: "placeholder-narrow", width: 4, placeholder: "Pikachu"},
	{name: "placeholder-wide", width: 10, placeholder: "Pikachu"},
	{name: "placeholder-cjk", width: 6, placeholder: "日本語です"},
	{name: "type", keys: "t:hello·world"},
	{name: "move", keys: "t:hello·world left left home right end"},
	{name: "ctrl-moves", keys: "t:abc ctrl+b ctrl+b ctrl+f ctrl+a ctrl+e"},
	{name: "words", keys: "t:one·two··three alt+left alt+left alt+right ctrl+left ctrl+right alt+b alt+f"},
	{name: "backspace", keys: "t:hello backspace backspace left backspace"},
	{name: "delete", keys: "t:hello home delete delete ctrl+d end delete"},
	{name: "delete-word-back", keys: "t:one·two·three ctrl+w alt+backspace"},
	{name: "delete-word-back-lead", keys: "t:·foo ctrl+w"},
	{name: "delete-word-back-mid", keys: "t:one·two·three alt+left ctrl+w"},
	{name: "delete-word-back-spaces", keys: "t:ab··· ctrl+w"},
	{name: "delete-word-fwd", keys: "t:one·two·three home alt+d alt+delete"},
	{name: "delete-word-fwd-mid", keys: "t:one·two·three home right alt+d"},
	{name: "kill", keys: "t:hello·world left left left ctrl+k"},
	{name: "unkill", keys: "t:hello·world left left left ctrl+u"},
	{name: "limit", limit: 5, keys: "t:abcdefgh"},
	{name: "limit-paste", limit: 6, keys: "t:ab p:123456789"},
	{name: "paste", keys: "t:ab left p:x·y¶z"},
	{name: "scroll", width: 5, keys: "t:abcdefghij"},
	{name: "scroll-back", width: 5, keys: "t:abcdefghij home"},
	{name: "scroll-left", width: 5, keys: "t:abcdefghij left left left left left left left"},
	{name: "scroll-mid", width: 5, keys: "t:abcdefghij home right right right right right right right"},
	{name: "scroll-wide", width: 6, keys: "t:日本語の文字です"},
	{name: "scroll-wide-home", width: 6, keys: "t:日本語の文字です home right right"},
	{name: "scroll-wide-left", width: 6, keys: "t:日本語の文字です left left left left left"},
	{name: "fits", width: 12, keys: "t:short"},
	{name: "fits-left", width: 12, keys: "t:short left left"},
	{name: "password", echo: textinput.EchoPassword, keys: "t:secret ctrl+w"},
	{name: "password-words", echo: textinput.EchoPassword, keys: "t:a·b·c alt+left t:X alt+right"},
	{name: "none", echo: textinput.EchoNone, keys: "t:hidden"},
	{name: "suggest", suggestions: []string{"apple", "apricot", "banana"}, keys: "t:ap"},
	{name: "suggest-next", suggestions: []string{"apple", "apricot", "banana"}, keys: "t:ap down"},
	{name: "suggest-prev", suggestions: []string{"apple", "apricot", "banana"}, keys: "t:ap up"},
	{name: "suggest-accept", suggestions: []string{"apple", "apricot", "banana"}, keys: "t:ap down tab"},
	{name: "suggest-case", suggestions: []string{"Apple"}, keys: "t:AP tab"},
	{name: "suggest-none", suggestions: []string{"apple"}, keys: "t:x tab"},
	{name: "suggest-width", width: 6, suggestions: []string{"apricots"}, keys: "t:ap"},
	{name: "suggest-left", width: 10, suggestions: []string{"apricots"}, keys: "t:apr left"},
	{name: "tab-types-nothing", keys: "t:a tab"},
	{name: "space", keys: "t:a space t:b"},
	{name: "accent-left", keys: "t:cafe\u0301s left left"},
	{name: "accent-right", keys: "t:cafe\u0301s home right right right right"},
	{name: "accent-delete", keys: "t:cafe\u0301s left left delete"},
	{name: "accent-backspace", keys: "t:cafe\u0301 backspace"},
	{name: "emoji-backspace", keys: "t:a\U0001F44D\U0001F3FD backspace"},
	{name: "zwj-delete", keys: "t:\U0001F468\u200d\U0001F469\u200d\U0001F467x home delete"},
	{name: "accent-scroll", width: 4, keys: "t:e\u0301e\u0301e\u0301e\u0301e\u0301e\u0301 home right"},
}

func textinputs() {
	for _, c := range inputCases {
		m := textinput.New()
		m.SetWidth(c.width)
		m.CharLimit = c.limit
		m.EchoMode = c.echo
		m.Placeholder = c.placeholder
		if c.suggestions != nil {
			m.ShowSuggestions = true
			m.SetSuggestions(c.suggestions)
		}
		m.Focus()
		for _, msg := range messages(c.keys) {
			m, _ = m.Update(msg)
		}
		row("textinput", c.name, c.width, c.limit, int(c.echo), esc(c.placeholder),
			esc(strings.Join(c.suggestions, ",")), c.keys, esc(m.Value()), m.Position(),
			esc(ansi.Strip(m.View())))
	}
}
