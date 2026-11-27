package main

import (
	"strings"

	tea "charm.land/bubbletea/v2"
)

// names are the keys a token can name.
var names = map[string]rune{
	"left": tea.KeyLeft, "right": tea.KeyRight, "up": tea.KeyUp, "down": tea.KeyDown,
	"home": tea.KeyHome, "end": tea.KeyEnd, "backspace": tea.KeyBackspace,
	"delete": tea.KeyDelete, "tab": tea.KeyTab, "enter": tea.KeyEnter, "esc": tea.KeyEscape,
	"pgup": tea.KeyPgUp, "pgdown": tea.KeyPgDown, "space": tea.KeySpace,
}

// messages turns a sequence of tokens, space separated, into messages:
// t:text types each character (· is a space), p:text pastes (¶ is a
// line break and ⇥ a tab), w:down turns the mouse wheel (w:shift+up
// with Shift), and a key is written as Go prints it, as ctrl+w.
func messages(tokens string) []tea.Msg {
	var msgs []tea.Msg
	for _, tok := range strings.Fields(tokens) {
		switch {
		case strings.HasPrefix(tok, "t:"):
			for _, r := range strings.ReplaceAll(tok[2:], "·", " ") {
				msgs = append(msgs, tea.KeyPressMsg{Code: r, Text: string(r)})
			}
		case strings.HasPrefix(tok, "w:"):
			msgs = append(msgs, wheel(tok[2:]))
		case strings.HasPrefix(tok, "p:"):
			text := strings.NewReplacer("·", " ", "¶", "\n", "⇥", "\t").Replace(tok[2:])
			msgs = append(msgs, tea.PasteMsg{Content: text})
		default:
			msgs = append(msgs, keyPress(tok))
		}
	}
	return msgs
}

// wheel is a mouse wheel token: up, down, left or right, maybe after
// shift+.
func wheel(tok string) tea.MouseWheelMsg {
	m := tea.Mouse{}
	if rest, ok := strings.CutPrefix(tok, "shift+"); ok {
		m.Mod = tea.ModShift
		tok = rest
	}
	m.Button = map[string]tea.MouseButton{
		"up": tea.MouseWheelUp, "down": tea.MouseWheelDown,
		"left": tea.MouseWheelLeft, "right": tea.MouseWheelRight,
	}[tok]
	return tea.MouseWheelMsg(m)
}

// keyPress is one key token.
func keyPress(tok string) tea.KeyPressMsg {
	parts := strings.Split(tok, "+")
	name := parts[len(parts)-1]
	var mod tea.KeyMod
	for _, p := range parts[:len(parts)-1] {
		switch p {
		case "ctrl":
			mod |= tea.ModCtrl
		case "alt":
			mod |= tea.ModAlt
		case "shift":
			mod |= tea.ModShift
		}
	}
	if code, ok := names[name]; ok {
		k := tea.Key{Code: code, Mod: mod}
		if code == tea.KeySpace && mod == 0 {
			k.Text = " "
		}
		return tea.KeyPressMsg(k)
	}
	r := []rune(name)[0]
	k := tea.Key{Code: r, Mod: mod}
	if mod == 0 {
		k.Text = name
	}
	return tea.KeyPressMsg(k)
}
