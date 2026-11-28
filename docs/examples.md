# Bubble Tea's examples in petitcarnets

Every example in Bubble Tea v2.1.1's
[`examples`](https://github.com/charmbracelet/bubbletea/tree/v2.1.1/examples)
directory, and where it stands here. A port keeps Go's name (run it with
`cargo run --example <name>`), its keys and its text; where the Rust
version differs, the example's own header says how and why.

Every example runs in CI: its model in a golden test (`TestProgram` on a
virtual clock, screens compared with `tests/golden/*.golden`), or, for
those that hand the terminal to another process, its binary in the
real-terminal smoke test (`scripts/pty-smoke.sh`). The test
`every_example_runs_in_a_test` in `tests/examples.rs` fails if an
example is added without either.

**Ported: 39 of 64, and one adapted. Not yet: 24.**

## Ported

| Go example | petitcarnets | Tests |
| --- | --- | --- |
| altscreen-toggle | [examples/altscreen-toggle.rs](../examples/altscreen-toggle.rs) | `tests/golden.rs`: `altscreen_toggle_switches_screens` |
| chat | [examples/chat.rs](../examples/chat.rs) | `tests/bubbles.rs`: `chat_sends_messages_above_the_text_area` |
| composable-views | [examples/composable-views.rs](../examples/composable-views.rs) | `tests/bubbles_examples.rs`: `composable_views_moves_the_focus_and_ticks_both` |
| cursor-style | [examples/cursor-style.rs](../examples/cursor-style.rs) | `tests/golden.rs`: `cursor_style_moves_through_the_shapes` |
| debounce | [examples/debounce.rs](../examples/debounce.rs) | `tests/examples.rs`: `debounce_quits_one_second_after_the_last_key` |
| exec | [examples/exec.rs](../examples/exec.rs) | `scripts/pty-smoke.sh`: `exec` |
| focus-blur | [examples/focus-blur.rs](../examples/focus-blur.rs) | `tests/examples.rs`: `focus_blur_follows_focus_reports_and_toggles_them` |
| fullscreen | [examples/fullscreen.rs](../examples/fullscreen.rs) | `tests/examples.rs`: `fullscreen_counts_down_on_the_alternate_screen` |
| help | [examples/help.rs](../examples/help.rs) | `tests/bubbles.rs`: `help_switches_between_short_and_full` |
| http | [examples/http.rs](../examples/http.rs) | `tests/examples.rs`: `http_shows_the_status_of_a_local_server_and_quits` and two more, against a local server |
| keyboard-enhancements | [examples/keyboard-enhancements.rs](../examples/keyboard-enhancements.rs) | `tests/golden.rs`: `keyboard_enhancements_show_presses_and_releases` |
| list-default | [examples/list-default.rs](../examples/list-default.rs) | `tests/bubbles.rs`: `list_default_filters_as_you_type` |
| list-simple | [examples/list-simple.rs](../examples/list-simple.rs) | `tests/bubbles.rs`: `list_simple_pages_and_chooses` |
| mouse | [examples/mouse.rs](../examples/mouse.rs) | `tests/golden.rs`: `mouse_shows_clicks_motion_wheel_and_focus`; `scripts/pty-smoke.sh`: `mouse` |
| package-manager | [examples/package-manager.rs](../examples/package-manager.rs) | `tests/golden.rs`: `package_manager_prints_above_its_progress_line`; `scripts/pty-smoke.sh`: `inline` |
| pager | [examples/pager.rs](../examples/pager.rs) | `tests/bubbles.rs`: `pager_scrolls_with_keys_and_the_wheel` |
| paginator | [examples/paginator.rs](../examples/paginator.rs) | `tests/bubbles.rs`: `paginator_turns_pages` |
| prevent-quit | [examples/prevent-quit.rs](../examples/prevent-quit.rs) | `tests/bubbles_examples.rs`: `prevent_quit_asks_before_dropping_changes` and two more |
| progress-animated | [examples/progress-animated.rs](../examples/progress-animated.rs) | `tests/bubbles.rs`: `progress_animated_springs_to_each_quarter` |
| progress-static | [examples/progress-static.rs](../examples/progress-static.rs) | `tests/bubbles.rs`: `progress_static_fills_a_quarter_a_second` |
| realtime | [examples/realtime.rs](../examples/realtime.rs) | `tests/bubbles_examples.rs`: `realtime_counts_events_from_a_channel` |
| result | [examples/result.rs](../examples/result.rs) | `tests/examples.rs`: `result_returns_the_chosen_tea`, `result_returns_no_choice_on_q` |
| send-msg | [examples/send-msg.rs](../examples/send-msg.rs) | `tests/bubbles_examples.rs`: `send_msg_shows_the_last_five_meals_sent_from_outside` |
| sequence | [examples/sequence.rs](../examples/sequence.rs) | `tests/examples.rs`: `sequence_prints_in_order_then_quits` |
| set-terminal-color | [examples/set-terminal-color.rs](../examples/set-terminal-color.rs) | `tests/bubbles_examples.rs`: `set_terminal_color_sets_the_colours_the_view_asks_for` |
| set-window-title | [examples/set-window-title.rs](../examples/set-window-title.rs) | `tests/golden.rs`: `set_window_title_sets_then_gives_the_title_back` |
| simple | [examples/simple.rs](../examples/simple.rs) | `tests/examples.rs`: `simple_counts_down_and_quits`, `simple_quits_sooner_on_q_with_the_same_bytes_every_run` |
| spinners | [examples/spinners.rs](../examples/spinners.rs) | `tests/bubbles.rs`: `spinners_tick_on_their_own_and_change_with_the_arrows` |
| split-editors | [examples/split-editors.rs](../examples/split-editors.rs) | `tests/bubbles_examples.rs`: `split_editors_adds_moves_between_and_removes_editors` |
| stopwatch | [examples/stopwatch.rs](../examples/stopwatch.rs) | `tests/bubbles.rs`: `stopwatch_counts_up_stops_and_resets` |
| suspend | [examples/suspend.rs](../examples/suspend.rs) | `scripts/pty-smoke.sh`: `ctrl-z`, `sigtstp` |
| table | [examples/table.rs](../examples/table.rs) | `tests/bubbles.rs`: `table_moves_its_selection_and_blurs` |
| textarea | [examples/textarea.rs](../examples/textarea.rs) | `tests/bubbles.rs`: `textarea_wraps_numbers_and_scrolls_its_lines` |
| textinput | [examples/textinput.rs](../examples/textinput.rs) | `tests/bubbles.rs`: `textinput_shows_the_placeholder_then_scrolls_what_is_typed`; `scripts/pty-smoke.sh`: `textinput` |
| textinputs | [examples/textinputs.rs](../examples/textinputs.rs) | `tests/bubbles.rs`: `textinputs_move_focus_hide_the_password_and_change_cursor_modes` |
| timer | [examples/timer.rs](../examples/timer.rs) | `tests/bubbles.rs`: `timer_counts_down_stops_and_runs_out` |
| vanish | [examples/vanish.rs](../examples/vanish.rs) | `tests/examples.rs`: `vanish_leaves_nothing_behind` |
| views | [examples/views.rs](../examples/views.rs) | `tests/examples.rs`: `views_counts_down_while_choosing`, `views_fills_the_bar_with_a_bounce_then_quits`, `views_blends_the_bar_as_go_does` |
| window-size | [examples/window-size.rs](../examples/window-size.rs) | `tests/examples.rs`: `window_size_prints_the_size_on_any_key` |

Adapted rather than ported:

| Go example | petitcarnets | How it differs |
| --- | --- | --- |
| query-term | [examples/query-term.rs](../examples/query-term.rs) | One key per question instead of typed escape sequences (`tests/golden.rs`: `query_term_prints_each_reply`; `scripts/pty-smoke.sh`: `replies`) |

The differences worth knowing, each said in the example's header:

- `simple`, `sequence`: Go's commands sleep on a goroutine; here a
  `tea::tick` waits, so the tests run on the virtual clock. The text and
  timing are the same.
- `http`: plain HTTP/1.1 over `std::net`, as the crate has no
  dependencies and the standard library has no TLS; it checks
  `http://charm.sh/` (Go checks `https://charm.sh/`), or the URL given
  as its argument. Its tests use a local server.
- `prevent-quit`: Go stops the quit message with `tea.WithFilter`, which
  petitcarnets does not have; the model asks before quitting instead,
  with the same screens for the same keys.
- `realtime`: the sender is a thread started by `main`, not a command
  that never returns; a closed channel ends the wait.
- `composable-views`: `Style` has no alignment yet, so the example
  centres the boxes' text itself, as Lip Gloss does.
- `views`: go-colorful's `BlendLuv` and fogleman/ease's `OutBounce` are
  ported in the example; a test checks the bar's colours against Go's.
- `split-editors`: removing the focused editor focuses the new last one;
  Go leaves no editor focused.

## Not yet ported

| Go example | Why not yet |
| --- | --- |
| autocomplete | Fetches the suggestions from the GitHub API over HTTPS; the standard library has no TLS. |
| canvas | Needs Lip Gloss v2's canvas and layers and the `charmtone` palette, which `petitcarnets::style` does not have. |
| capability | Not yet done; nothing missing (`tea::request_capability` and the text input exist). |
| cellbuffer | Needs harmonica's spring as a public API (the progress bar has one inside). |
| clickable | Needs Lip Gloss layers with hit testing (`View.OnMouse`), not ported. |
| colorprofile | Needs colorprofile's colour conversion as a public API. |
| doom-fire | Not yet done; nothing missing. |
| dynamic-textarea | Not yet done; the text area's dynamic height exists. |
| eyes | Not yet done; nothing missing. |
| file-picker | Bubbles' file picker is not ported. |
| glamour | Needs Glamour, a Markdown renderer, out of scope for a crate with no dependencies. |
| isbn-form | Needs the `charmtone` palette. |
| list-fancy | Needs the list's default delegate update and help functions, not ported (see the 0.0.6 CHANGELOG). |
| pipe | Reads piped standard input while keys come from `/dev/tty`; not yet done. |
| print-key | Prints Go's type names (`%T`), which have no Rust equivalent; the `keyboard-enhancements` example shows the same keys. |
| program-status | The Program Status Protocol (OSC 7501) is not ported. |
| progress-bar | Not yet done; `View::with_progress_bar` exists (see `package-manager`). |
| progress-download | Downloads over HTTPS; the standard library has no TLS. |
| space | Not yet done; nothing missing. |
| spinner | The name is taken by an older petitcarnets example, a different program (a spinner over slow work); Go's is not ported under another name yet. |
| splash | Not yet done; nothing missing. |
| table-resize | Needs Lip Gloss's `table` package (not Bubbles' table), not ported. |
| tabs | Needs Lip Gloss borders with custom corners per side, not in `petitcarnets::style`. |
| tui-daemon-combo | Needs `tea.WithoutRenderer`, not ported. |
