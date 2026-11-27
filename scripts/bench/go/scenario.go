package main

import (
	"fmt"
	"strings"
)

// The same frames as scripts/bench/src/scenario.rs; both programs print
// a hash of every frame's text so they can be checked against each other.

var names = []string{"counter", "list", "log", "spinner", "full"}

var spinner = []string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"}

// Even frames of "full" use the capitals, odd frames the small letters.
var letters = [2]string{"ABCDEFGHIJKLMNOPQRSTUVWXYZ", "abcdefghijklmnopqrstuvwxyz"}

// view is the text of frame n (0 is the first frame) of scenario.
func view(scenario string, n int) string {
	switch scenario {
	case "counter":
		return fmt.Sprintf("Count: %d\n\n+/- or arrows to change, q to quit", n)
	case "list":
		return list(n)
	case "log":
		lines := make([]string, 0, 24)
		for k := n; k < n+24; k++ {
			lines = append(lines, logLine(k))
		}
		return strings.Join(lines, "\n")
	case "spinner":
		return spinner[n%len(spinner)] + " Loading forty-two items..."
	case "full":
		rows := make([]string, 0, 24)
		for r := 0; r < 24; r++ {
			rows = append(rows, fullRow(r, n))
		}
		return strings.Join(rows, "\n")
	}
	panic("unknown scenario " + scenario)
}

func list(n int) string {
	selected := n % 20
	rows := make([]string, 0, 20)
	for i := 0; i < 20; i++ {
		if i == selected {
			rows = append(rows, fmt.Sprintf("\x1b[1;36m> item %02d\x1b[0m", i))
		} else {
			rows = append(rows, fmt.Sprintf("  item %02d", i))
		}
	}
	return strings.Join(rows, "\n")
}

func logLine(k int) string {
	return fmt.Sprintf("%06d GET /api/v1/items/%-4d 200 %3d ms user=%04d",
		k, k*7%1000, k*37%997, k*13%10000)
}

func fullRow(r, n int) string {
	var b strings.Builder
	for c := 0; c < 79; c++ {
		b.WriteByte(letters[n%2][mix(c, r, n)%26])
	}
	return b.String()
}

// mix is a 32-bit hash of a cell's column, row and frame.
func mix(c, r, n int) uint32 {
	x := uint32(c)*0x9e3779b1 + uint32(r)*0x85ebca77 + uint32(n)*0xc2b2ae3d
	x ^= x >> 15
	x *= 0x2c1b3c6d
	x ^= x >> 12
	return x
}

// hash is FNV-1a over frames 0..=frames, each followed by a 0 byte.
func hash(scenario string, frames int) uint64 {
	h := uint64(0xcbf29ce484222325)
	for n := 0; n <= frames; n++ {
		for _, b := range append([]byte(view(scenario, n)), 0) {
			h ^= uint64(b)
			h *= 0x100000001b3
		}
	}
	return h
}
