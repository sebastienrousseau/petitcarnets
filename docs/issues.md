# Bubble Tea and Bubbles issues addressed

An index of the issues in [Bubble Tea](https://github.com/charmbracelet/bubbletea)
and [Bubbles](https://github.com/charmbracelet/bubbles) that petitcarnets
addresses, from the CHANGELOG and the source. For each: what the issue
is about, what petitcarnets does, the release that did it, and the test
that shows it. Where no test exercises the case directly, the entry says
so.

Some numbers the CHANGELOG cites are pull requests, not issues; they are
marked "(PR)". Tests are named as `file`: `function`. Checks in
`scripts/pty-smoke.sh` run the examples in a real pseudo-terminal in CI
(Linux) and are named by the label they print.

## Bubble Tea

### 0.0.3: the renderer

| Issue | Problem | petitcarnets | Test |
| --- | --- | --- | --- |
| [#1780](https://github.com/charmbracelet/bubbletea/issues/1780) | A one-tick stale frame on resize: a race between `resize()` and the 60 Hz ticker's flush. | Frames are not on a ticker. A window size reaches the renderer before the model, and the next frame is drawn in full, on the same thread. | `src/renderer/tests.rs`: `a_resize_redraws_in_full`; `tests/harness.rs`: `a_resize_reaches_the_model_and_the_screen` |
| [#1857](https://github.com/charmbracelet/bubbletea/issues/1857) | Keystrokes wait for the next frame tick (about 13 ms at 60 fps). | The frame after a key, mouse event or paste is drawn at once, whatever the frame rate. | `tests/runtime.rs`: `a_frame_after_a_key_is_drawn_at_once` |
| [#1873](https://github.com/charmbracelet/bubbletea/issues/1873) | Idle programs keep waking the CPU. | No ticker and no polling: the loop sleeps until an event, the earliest timer or a held back frame; timers are heap entries, not threads. | `scripts/pty-smoke.sh`: `idle` (context switches over one idle second); `src/exec/tests.rs`: `a_thousand_ticks_are_heap_entries_not_threads` |
| [#1736](https://github.com/charmbracelet/bubbletea/issues/1736) | No way to order `tea.Println` with the frame flush; a stale frame row ends up in the scrollback. | `println` is an item in the same queue as messages: the frame for the update that printed is drawn first, then the lines go above it. | `src/program/tests.rs`: `println_goes_above_the_frame_of_the_update_that_printed`; `tests/runtime.rs`: `println_comes_after_the_frame_before_it_and_before_quitting` |
| [#1881](https://github.com/charmbracelet/bubbletea/issues/1881) | A `tea.Println` returned from the update that leaves the alternate screen is written into the alternate screen and lost. | Before printing, the frame for that state is drawn, leaving the alternate screen, so the lines land on the normal screen. | No dedicated test for the same update; `src/program/tests.rs`: `the_alternate_screen_comes_and_goes_mid_run` leaves on one key and prints on the next. |
| [#1909](https://github.com/charmbracelet/bubbletea/issues/1909) | `tea.Raw` output sent before `tea.Quit` is written after the terminal is restored. | Output is applied in program order before the end; the modes are turned off after. | `tests/runtime.rs`: `println_comes_after_the_frame_before_it_and_before_quitting` (a `raw` in a sequence before `quit`), `raw_bytes_are_written_before_the_terminal_is_restored` |
| [#1898](https://github.com/charmbracelet/bubbletea/issues/1898) | The last frame on quit is sometimes kept, sometimes erased, under CPU load. | A quit or interrupt always draws the last frame before the terminal is restored. | `tests/harness.rs`: `quit_ends_with_the_model_and_a_final_frame`; `src/program/tests.rs`: `an_inline_run_leaves_its_whole_last_frame_with_the_cursor_below` |
| [#1838](https://github.com/charmbracelet/bubbletea/issues/1838) | Hard-tab cursor moves break after the window widens: tab stops are set only at start-up. | The writer never moves the cursor with tabs; tabs in a view are expanded to spaces. | `src/renderer/tests.rs`: `tabs_expand_to_every_eight_columns` (no tab byte written) |
| [#1541](https://github.com/charmbracelet/bubbletea/issues/1541) | Tab characters in a view are dropped. | Tabs expand to the next multiple of eight columns, in frames and in `println`. | `src/renderer/tests.rs`: `tabs_expand_to_every_eight_columns`, `println_keeps_styles_and_ends_them` |
| [#1777](https://github.com/charmbracelet/bubbletea/issues/1777) | Traditional Chinese and Hindi do not render. | One width method per program (`WcWidth`, or grapheme widths when the terminal reports mode 2027), used by layout, diff and `println` alike, measured as Go's x/ansi measures; wide characters and clusters are never split. | `src/unicode/width/tests.rs`: `widths_and_tokens_match_go` (the fixture includes CJK and Devanagari); `src/renderer/tests.rs`: `combining_marks_stay_with_their_letter`, `wide_characters_at_the_right_edge_are_never_split` |
| [#1737](https://github.com/charmbracelet/bubbletea/issues/1737) | Hindi rendering broken. | As for #1777. | As for #1777. |
| [#1802](https://github.com/charmbracelet/bubbletea/issues/1802) | `tea.Println` can add an extra line for a wide grapheme in the `WcWidth` fallback. | `println` counts wrapped rows as the terminal wraps them, with the program's width method, wide characters included. | `src/renderer/insert/tests.rs`: `wrapped_rows_count_wide_characters_as_the_terminal_wraps_them`; `src/renderer/tests.rs`: `println_counts_wrapped_rows_with_wide_characters` |
| [#1906](https://github.com/charmbracelet/bubbletea/issues/1906) (PR) | Count wide-character wrap rows in `insertAbove`. | As for #1802. | As for #1802. |
| [#918](https://github.com/charmbracelet/bubbletea/issues/918) | No way to set the terminal's real cursor position. | `View::with_cursor(Cursor::new(x, y))` puts the terminal's cursor where the view says, every frame. | `src/renderer/tests.rs`: `the_cursor_goes_where_the_view_says`, `the_cursor_of_a_tall_inline_frame_moves_up_with_its_rows` |
| [#874](https://github.com/charmbracelet/bubbletea/issues/874) | IME input appears in the wrong position. | As for #918: an input method follows the real cursor. | As for #918. |
| [#1481](https://github.com/charmbracelet/bubbletea/issues/1481) | v2 is much slower than v1. | The frame after input is drawn at once; unchanged rows are skipped by hash and changed rows write only the cells that differ. | `tests/runtime.rs`: `a_frame_after_a_key_is_drawn_at_once`; bytes per frame in `docs/benchmarks.md`. No test of CPU time. |
| [#1724](https://github.com/charmbracelet/bubbletea/issues/1724) | Scroll frames in the cursed renderer are much slower than in v1. | As for #1481; on the alternate screen a moved frame is scrolled by the terminal, and the cheaper of scroll and plain diff is written. | `src/renderer/tests.rs`: `a_scrolled_log_scrolls_the_terminal`, `a_scroll_under_a_fixed_header_uses_a_scroll_region`. No test of CPU time. |
| [#1869](https://github.com/charmbracelet/bubbletea/issues/1869) | Leftovers next to the prompt on exit when the alternate screen is not available. | An inline program that quits leaves every row of its last frame and puts the cursor at the start of a fresh line below it, erasing what is further down; an empty last view leaves nothing. Related: [#1798](https://github.com/charmbracelet/bubbletea/issues/1798) (PR). | `src/program/tests.rs`: `an_inline_run_leaves_its_whole_last_frame_with_the_cursor_below`; `tests/runtime.rs`: `quitting_inline_keeps_the_last_line_and_moves_below_it`; `scripts/pty-smoke.sh`: `inline` |

### 0.0.4: the lifecycle and the colour profile

| Issue | Problem | petitcarnets | Test |
| --- | --- | --- | --- |
| [#616](https://github.com/charmbracelet/bubbletea/issues/616) | Input lost on `ReleaseTerminal` and at shutdown. | The reader waits in `poll`, never in `read`, and pauses through a handshake on its wake pipe, so no byte is read for the wrong program. | `src/reader/tests.rs`: `a_paused_reader_leaves_the_terminal_alone_until_it_resumes`; `scripts/pty-smoke.sh`: `exec` |
| [#1116](https://github.com/charmbracelet/bubbletea/issues/1116) | After an exec, the first key press is ignored. | As for #616; input is read again as soon as the terminal is taken back. | `scripts/pty-smoke.sh`: `exec` (the child reads the keys typed while it runs, the program the rest); `tests/lifecycle.rs`: `an_exec_callback_is_the_next_message_and_modes_come_back` |
| [#1424](https://github.com/charmbracelet/bubbletea/issues/1424) | `tea.ExecProcess` does not restore mouse motion. | Every mode is turned off for the child and on again after, in the first order. | `tests/lifecycle.rs`: `an_exec_callback_is_the_next_message_and_modes_come_back`; `src/program/tests.rs`: `after_an_exec_the_alternate_screen_and_modes_are_back_and_the_main_screen_kept` |
| [#1778](https://github.com/charmbracelet/bubbletea/issues/1778) | The renderer can stay frozen after `tea.Exec`: a race between stopping and restarting its ticker. | No render goroutine or ticker: taking the terminal back marks the whole frame due at once. | `src/program/tests.rs`: `after_an_exec_the_alternate_screen_and_modes_are_back_and_the_main_screen_kept` (the frame is back after the exec) |
| [#497](https://github.com/charmbracelet/bubbletea/issues/497) | A Bubble Tea process cannot be suspended. | Ctrl-Z, `suspend()` and `kill -TSTP` suspend: the terminal is handed over, the process group stopped, and on continue the terminal set up again and `Msg::Resume` sent. `Program::without_suspend` makes Ctrl-Z a key. | `tests/lifecycle.rs`: `suspend_and_resume_from_the_key_the_command_and_signals`, `without_suspend_ctrl_z_is_a_key`; `scripts/pty-smoke.sh`: `ctrl-z`, `sigtstp` |
| [#500](https://github.com/charmbracelet/bubbletea/issues/500) (PR) | Suspend on Ctrl-Z. | As for #497. | As for #497. |
| [#1673](https://github.com/charmbracelet/bubbletea/issues/1673) | Each suspend leaks a signal channel (`signal.Stop` missing). | Signals reach the reader through one pipe for the whole run; a suspend only switches `SIGTSTP` to its default handler and back. | No dedicated test of the leak; `tests/lifecycle.rs`: `suspend_and_resume_from_the_key_the_command_and_signals` suspends several times in one run. |
| [#1808](https://github.com/charmbracelet/bubbletea/issues/1808) | No way to recover after something else damaged the terminal (`ClearScreen` is not enough). Related: [#1809](https://github.com/charmbracelet/bubbletea/issues/1809), [#1846](https://github.com/charmbracelet/bubbletea/issues/1846) (PRs). | `repaint()` turns every mode off and on again (the alternate screen entered again) and draws the whole frame, even unchanged. | `tests/lifecycle.rs`: `repaint_turns_every_mode_off_and_on_and_draws_again`; `src/program/tests.rs`: `repaint_enters_the_alternate_screen_again_with_every_mode_and_the_frame` |
| [#1709](https://github.com/charmbracelet/bubbletea/issues/1709) | `tea.Println` bypasses the colour profile: `NO_COLOR` and low-colour terminals get full colour in the scrollback. | Printed lines go through the same writer and colour profile as frames. | `src/renderer/tests.rs`: `styles_come_down_to_the_colour_profile_in_frames_and_printed_lines`; `scripts/pty-smoke.sh`: `no-color` |
| [#1773](https://github.com/charmbracelet/bubbletea/issues/1773) (PR) | A forced colour profile is overridden by a capability report. | A profile set with `with_color_profile` is never upgraded by an `RGB` or `Tc` reply. | `tests/lifecycle.rs`: `a_forced_profile_is_kept_whatever_the_terminal_says` |
| [#1590](https://github.com/charmbracelet/bubbletea/issues/1590) | A program that quits early prints garbage characters. | Before the end, and before any handover, replies to queries written in the last second are read, until the terminal answers a `CSI c` sentinel or 500 ms pass. | `tests/lifecycle.rs`: `an_unanswered_query_delays_the_end_by_half_a_second`, `an_answered_sentinel_ends_the_wait_at_once`; `scripts/pty-smoke.sh`: `replies` |
| [#1627](https://github.com/charmbracelet/bubbletea/issues/1627) | Escape sequences leak from short-lived programs. | As for #1590. | As for #1590. |
| [#1771](https://github.com/charmbracelet/bubbletea/issues/1771) | The terminal is queried at package init, for every binary that links Bubble Tea, even one that never runs a program. | Nothing is written before a program starts; the start query goes only to a real terminal, gated as Bubble Tea gates it, never blocks start-up, and its replies are drained as for #1590. | `src/program/tests.rs`: `queries_are_gated_as_bubble_tea_gates_them`; `tests/lifecycle.rs`: `a_backend_that_is_not_a_terminal_is_never_waited_for` |

### 0.0.5: testing

| Issue | Problem | petitcarnets | Test |
| --- | --- | --- | --- |
| [#1654](https://github.com/charmbracelet/bubbletea/issues/1654) | Proposal: a testing framework for Bubble Tea applications. | `testing::TestProgram` (teatest): the real loop, renderer and modes on a virtual clock, in an emulated terminal (`Vt`), with scripted replies (`TestTerminal`) and golden files (`assert_golden`). | `tests/harness.rs`, for example `a_tick_fires_only_when_the_clock_reaches_it`, `task_messages_arrive_in_the_order_the_tasks_started`, `an_unanswered_query_holds_the_end_for_half_a_second_of_virtual_time`; golden tests in `tests/golden.rs` and `tests/bubbles.rs` |

### 0.0.7: accessibility, and a print taller than the window

| Issue | Problem | petitcarnets | Test |
| --- | --- | --- | --- |
| [#780](https://github.com/charmbracelet/bubbletea/issues/780) | Screen reader accessibility. | `Program::with_accessible(true)`, or a non-empty `ACCESSIBLE` on a real terminal, prints each changed, non-blank line of a frame once, as plain text, with no cursor movement, alternate screen or synchronized output (`src/renderer/lines.rs`). | `tests/accessible.rs`: `the_counter_prints_each_changed_line_once`, `printed_lines_and_frames_are_plain_text_in_program_order`, `a_real_terminal_with_accessible_set_prints_lines`; `src/renderer/lines/tests.rs`; `scripts/pty-smoke.sh`: `accessible` |
| [#1822](https://github.com/charmbracelet/bubbletea/issues/1822) | In inline mode, one `tea.Println` with more rows than fit above the view removes the view. | Counted as fixed by design in 0.0.3, but it was only in part: the frame came back only after the next update. Since 0.0.7 a print that pushes the frame off the screen draws it again in the next frame, with no update needed. | `tests/harness.rs`: `a_print_taller_than_the_window_leaves_the_frame_on_screen` |

## Bubbles

### 0.0.6: the components

| Issue | Problem | petitcarnets | Test |
| --- | --- | --- | --- |
| [#1097](https://github.com/charmbracelet/bubbles/issues/1097) | The text input's cursor stops inside Thai grapheme clusters; Backspace and Delete then break the word. | The text input moves, deletes and scrolls by grapheme cluster, as Bubbles' own text area does. | `src/bubbles/textinput/tests.rs`: `thai_clusters_and_emoji_placeholders_stay_whole` |
| [#573](https://github.com/charmbracelet/bubbles/issues/573) | Emoji placeholders do not render in the text input and text area. | In the text input, a placeholder's first cluster stays whole under the cursor, and a wide placeholder is cut to the width. | `src/bubbles/textinput/tests.rs`: `thai_clusters_and_emoji_placeholders_stay_whole` |
| [#729](https://github.com/charmbracelet/bubbles/issues/729) | A foreground in the table's cell style breaks the selected style. | The selected style reaches every cell of the selected row, even over a coloured cell style. | `src/bubbles/table/tests.rs`: `the_selected_style_reaches_every_cell_over_a_coloured_cell_style` |

## Cited, but not listed above

- Bubble Tea [#1036](https://github.com/charmbracelet/bubbletea/issues/1036):
  cited in the CHANGELOG (0.0.4) with #1709 for `println` and the colour
  profile, but the issue is about `View()` hanging while Lip Gloss
  queries the background colour. petitcarnets' views never query the
  terminal, but nothing in the code or tests is about this issue.
- Bubble Tea [#1167](https://github.com/charmbracelet/bubbletea/issues/1167):
  cited in the CHANGELOG (0.0.4) with #616 and #1116. It is a
  Windows-only bug (the first key lost in a second program in one
  process); petitcarnets runs on Unix only, so it cannot address or
  test it yet.
