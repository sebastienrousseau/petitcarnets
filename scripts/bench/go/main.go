// Bytes written per frame by Bubble Tea v2 for the five scenarios of
// scripts/bench, on an 80x24 screen.
//
//	go run . [-frames N] [-tty] [-dump DIR]
//
// Without -tty, input is off (tea.WithInput(nil)), as for a program
// whose output is not a terminal. With -tty, stdin must be a terminal
// (run under `script`): Bubble Tea then sets raw mode and uses the
// terminal's tab and backspace settings, as in real use.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	tea "charm.land/bubbletea/v2"
	"github.com/charmbracelet/colorprofile"
)

const (
	settleFirst = 200 * time.Millisecond // after the first frame shows up
	settle      = 40 * time.Millisecond  // after each later frame: over two 60 fps ticks
	timeout     = 3 * time.Second
)

// capture keeps every byte written and counts Write calls.
type capture struct {
	mu     sync.Mutex
	buf    bytes.Buffer
	writes int
}

func (c *capture) Write(p []byte) (int, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.writes++
	return c.buf.Write(p)
}

func (c *capture) snapshot() (int, int) {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.buf.Len(), c.writes
}

// waitWrites waits until there have been at least n writes.
func (c *capture) waitWrites(n int) bool {
	until := time.Now().Add(timeout)
	for time.Now().Before(until) {
		if _, w := c.snapshot(); w >= n {
			return true
		}
		time.Sleep(time.Millisecond)
	}
	return false
}

type stepMsg struct{}

type model struct {
	scenario string
	n        int
	alt      bool
}

func (m model) Init() tea.Cmd { return nil }

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	if _, ok := msg.(stepMsg); ok {
		m.n++
	}
	return m, nil
}

func (m model) View() tea.View {
	v := tea.NewView(view(m.scenario, m.n))
	v.AltScreen = m.alt
	return v
}

type stats struct {
	first    int
	frames   []int
	writes   []int
	timeouts int
	teardown int
}

func run(scenario string, alt, tty bool, steps int, dump string) stats {
	var c capture
	opts := []tea.ProgramOption{
		tea.WithOutput(&c),
		tea.WithWindowSize(80, 24),
		tea.WithEnvironment([]string{"TERM=xterm-256color"}),
		tea.WithColorProfile(colorprofile.TrueColor),
		tea.WithoutSignalHandler(),
	}
	if tty {
		opts = append(opts, tea.WithInput(os.Stdin))
	} else {
		opts = append(opts, tea.WithInput(nil))
	}
	p := tea.NewProgram(model{scenario: scenario, alt: alt}, opts...)
	done := make(chan tea.Model, 1)
	go func() {
		m, err := p.Run()
		if err != nil {
			fmt.Fprintln(os.Stderr, "run:", err)
		}
		done <- m
	}()
	s := drive(&c, steps, func() { p.Send(stepMsg{}) })
	p.Quit()
	end := (<-done).(model)
	if end.n != steps {
		panic(fmt.Sprintf("%d steps reached the model, want %d", end.n, steps))
	}
	total, _ := c.snapshot()
	s.teardown = total - s.first - sum(s.frames)
	if dump != "" {
		writeDump(dump, scenario, alt, tty, &c, s)
	}
	return s
}

// drive waits for the first frame, then sends steps one at a time,
// waiting for each frame to be written and for two more frame ticks
// before the next, so no two steps share a frame.
func drive(c *capture, steps int, step func()) stats {
	var s stats
	if !c.waitWrites(1) {
		s.timeouts++
	}
	time.Sleep(settleFirst)
	mark, writes := c.snapshot()
	s.first = mark
	for i := 0; i < steps; i++ {
		step()
		if !c.waitWrites(writes + 1) {
			s.timeouts++
		}
		time.Sleep(settle)
		b, w := c.snapshot()
		s.frames = append(s.frames, b-mark)
		s.writes = append(s.writes, w-writes)
		mark, writes = b, w
	}
	return s
}

func writeDump(dir, scenario string, alt, tty bool, c *capture, s stats) {
	name := fmt.Sprintf("go-%s-%s-%s", inputName(tty), screenName(alt), scenario)
	sizes := []string{strconv.Itoa(s.first)}
	for _, f := range s.frames {
		sizes = append(sizes, strconv.Itoa(f))
	}
	c.mu.Lock()
	out := append([]byte(nil), c.buf.Bytes()...)
	c.mu.Unlock()
	must(os.WriteFile(filepath.Join(dir, name+".out"), out, 0o644))
	must(os.WriteFile(filepath.Join(dir, name+".frames"), []byte(strings.Join(sizes, "\n")+"\n"), 0o644))
}

func must(err error) {
	if err != nil {
		panic(err)
	}
}

func sum(xs []int) int {
	t := 0
	for _, x := range xs {
		t += x
	}
	return t
}

func screenName(alt bool) string {
	if alt {
		return "alt"
	}
	return "inline"
}

func inputName(tty bool) string {
	if tty {
		return "tty"
	}
	return "noinput"
}

func row(name, screen, scenario string, s stats) string {
	lo, hi := s.frames[0], s.frames[0]
	one := 0
	for i, f := range s.frames {
		lo, hi = min(lo, f), max(hi, f)
		if s.writes[i] == 1 {
			one++
		}
	}
	mean := float64(sum(s.frames)) / float64(len(s.frames))
	return fmt.Sprintf("| %s | %s | %s | %d | %.1f | %d | %d | %d/%d one write, %d timeouts |",
		name, screen, scenario, s.first, mean, lo, hi, one, len(s.frames), s.timeouts)
}

func main() {
	frames := flag.Int("frames", 100, "frames after the first")
	tty := flag.Bool("tty", false, "read input from stdin, which must be a terminal")
	dump := flag.String("dump", "", "keep each run's bytes in this directory")
	flag.Parse()
	if *dump != "" {
		must(os.MkdirAll(*dump, 0o755))
	}
	name := "bubbletea v2 (" + inputName(*tty) + ")"
	var rows, teardowns []string
	for _, alt := range []bool{false, true} {
		for _, scenario := range names {
			s := run(scenario, alt, *tty, *frames, *dump)
			rows = append(rows, row(name, screenName(alt), scenario, s))
			teardowns = append(teardowns, fmt.Sprintf("%s %s: %d", screenName(alt), scenario, s.teardown))
		}
	}
	fmt.Printf("frames after the first: %d\n\nframe text hashes (FNV-1a, frames 0..=%d):\n", *frames, *frames)
	for _, scenario := range names {
		fmt.Printf("  %s: %016x\n", scenario, hash(scenario, *frames))
	}
	fmt.Println("\n| implementation | screen | scenario | first frame bytes | mean bytes/frame | min | max | frames |")
	fmt.Println("|---|---|---|---:|---:|---:|---:|---|")
	for _, r := range rows {
		fmt.Println(r)
	}
	fmt.Println("\nbytes after the last frame (teardown):", strings.Join(teardowns, ", "))
}
