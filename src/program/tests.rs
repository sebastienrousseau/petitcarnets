//! Whole runs drawn into the test terminal emulator: what the screen
//! shows, and how the terminal is left.

use crate::backend::TestBackend;
use crate::key::Key;
use crate::testing::vt::Vt;
use crate::view::{Cursor, CursorShape, View};
use crate::{Cmd, Model, Msg, Program, println, quit, sequence};

/// Keys choose the next view; `q` prints and quits.
#[derive(Debug, Default)]
struct Scene {
    view: View,
}

impl Model for Scene {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> Cmd<()> {
        let Msg::KeyPress(key) = msg else {
            return Cmd::none();
        };
        let content = self.view.content.clone();
        match key.to_string().as_str() {
            "a" => self.view.alt_screen = !self.view.alt_screen,
            "t" => self.view = View::new(content).with_window_title("my title"),
            "c" => {
                let cursor = Cursor::new(2, 0).with_shape(CursorShape::Bar);
                self.view = View::new(content).with_cursor(cursor);
            }
            "q" => return sequence([println("goodbye"), quit()]),
            "e" => self.view.content.clear(),
            "p" => {
                self.view.content = "after".into();
                return println("printed");
            }
            _ => self.view.content.push_str(&key.to_string()),
        }
        Cmd::none()
    }

    fn view(&self) -> View {
        self.view.clone()
    }
}

/// Runs a scene from `content` with `keys`, then `q`, and plays what it
/// wrote into a 20 by 5 terminal.
fn play(content: &str, keys: &str) -> (Vt, String) {
    let keys = keys.chars().map(Key::char).chain([Key::char('q')]);
    let backend = TestBackend::new(20, 5).keys(keys);
    let screen = backend.screen();
    let scene = Scene {
        view: View::new(content),
    };
    let _ = Program::new(scene).run_with(backend).unwrap();
    let raw = screen.raw();
    let mut vt = Vt::new(20, 5);
    vt.feed(raw.as_bytes());
    assert!(vt.unknown().is_empty(), "{:?}", vt.unknown());
    (vt, raw)
}

#[test]
fn an_inline_run_leaves_its_whole_last_frame_with_the_cursor_below() {
    let (vt, _) = play("typed: ", "xy");
    assert_eq!(
        vt.text(),
        "goodbye\ntyped: xy",
        "no trailing newline needed"
    );
    assert_eq!(vt.cursor(), (0, 2));
}

#[test]
fn an_empty_last_view_leaves_nothing_behind() {
    let (vt, _) = play("working", "e");
    assert_eq!(vt.text(), "goodbye");
    assert_eq!(vt.cursor(), (0, 1), "where the frame started");
}

#[test]
fn a_frame_ending_in_a_newline_leaves_the_cursor_below_it() {
    let (vt, _) = play("line\n", "");
    assert_eq!(vt.text(), "goodbye\nline");
    assert_eq!(vt.cursor(), (0, 3), "below the frame's blank last row");
}

#[test]
fn the_terminal_is_left_as_it_was_found() {
    let (vt, _) = play("x", "atc");
    assert!(!vt.alt_screen());
    assert!(vt.autowrap());
    assert!(vt.cursor_visible());
    assert_eq!(vt.cursor_style(), 0);
    for mode in [2004, 1004, 1002, 1003, 1006, 2027, 2026] {
        assert!(!vt.mode(mode), "mode {mode} left on");
    }
    assert!(!vt.in_sync());
}

#[test]
fn the_alternate_screen_comes_and_goes_mid_run() {
    let (vt, raw) = play("main", "aza");
    assert!(raw.contains("\x1b[?1049h") && raw.contains("\x1b[?1049l"));
    assert!(raw.contains('z'));
    assert_eq!(
        vt.text(),
        "goodbye\nmainz",
        "printed above the inline frame after the alternate screen was left"
    );
}

#[test]
fn println_on_the_alternate_screen_prints_nothing() {
    let (vt, _) = play("main", "a");
    assert!(!vt.text().contains("goodbye"), "{}", vt.text());
}

#[test]
fn the_window_title_is_saved_set_and_given_back() {
    let (vt, raw) = play("x", "t");
    assert!(raw.contains("\x1b[22;2t\x1b]2;my title\x07"), "{raw:?}");
    assert_eq!(vt.title(), "", "popped back to what it was");
    assert!(raw.ends_with("\x1b]2;\x07\x1b[23;2t\x1b[?7h"), "{raw:?}");
}

#[test]
fn the_cursor_shape_is_set_then_reset() {
    let (vt, raw) = play("ab", "c");
    assert!(raw.contains("\x1b[5 q"), "{raw:?}");
    assert!(raw.contains("\x1b[0 q"));
    assert_eq!(vt.cursor_style(), 0);
}

#[test]
fn println_goes_above_the_frame_of_the_update_that_printed() {
    let (vt, raw) = play("before\n", "p");
    let after = raw.find("after").expect("the new frame");
    let printed = raw.find("printed").expect("the line");
    assert!(
        after < printed,
        "the frame for the update is drawn first: {raw:?}"
    );
    assert_eq!(vt.row_text(0), "printed");
    assert_eq!(vt.row_text(1), "goodbye");
}

/// On the alternate screen with the mouse on; keys damage the terminal
/// (`d`), repair it (`r` repaints, `c` clears the screen), run a
/// command (`x`), and mark the output (`m`).
#[derive(Debug, Default)]
struct Damaged(u32);

impl Model for Damaged {
    type Msg = ();

    fn update(&mut self, msg: Msg<()>) -> Cmd<()> {
        let Msg::KeyPress(key) = msg else {
            return Cmd::none();
        };
        self.0 += 1;
        match key.to_string().as_str() {
            "d" => crate::raw("\x1b[?1049l\x1b[?1002l\x1b[?2004lJUNK"),
            "r" => crate::repaint(),
            "c" => crate::clear_screen(),
            "x" => crate::exec_command(Noop, |_| ()),
            "m" => crate::raw("<MARK>"),
            "q" => quit(),
            _ => Cmd::none(),
        }
    }

    fn view(&self) -> View {
        View::new(format!("frame {}", self.0))
            .with_alt_screen(true)
            .with_mouse_mode(crate::MouseMode::CellMotion)
    }
}

/// A command that does nothing.
struct Noop;

impl crate::ExecCommand for Noop {
    fn run(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Runs [`Damaged`] with `keys`, then `m` and `q`; the terminal as it
/// was at the mark, and all that was written.
fn damage(keys: &str) -> (Vt, String) {
    let keys = keys.chars().chain("mq".chars()).map(Key::char);
    let backend = TestBackend::new(20, 5).keys(keys);
    let screen = backend.screen();
    let _ = Program::new(Damaged::default()).run_with(backend).unwrap();
    let raw = screen.raw();
    let mark = raw.find("<MARK>").expect("marked");
    let mut vt = Vt::new(20, 5);
    vt.feed(&raw.as_bytes()[..mark]);
    (vt, raw)
}

#[test]
fn damage_leaves_the_terminal_broken_without_a_repair() {
    let (vt, _) = damage("d");
    assert!(!vt.alt_screen());
    assert!(!vt.mode(1002) && !vt.mode(2004));
}

#[test]
fn repaint_enters_the_alternate_screen_again_with_every_mode_and_the_frame() {
    let (vt, _) = damage("dr");
    assert!(vt.alt_screen());
    assert!(vt.mode(1002) && vt.mode(1006) && vt.mode(2004));
    assert!(!vt.autowrap());
    assert_eq!(vt.text(), "frame 2");
}

#[test]
fn clear_screen_writes_the_modes_again() {
    let (vt, _) = damage("dc");
    assert!(vt.mode(1002) && vt.mode(2004));
    assert_eq!(vt.row_text(0), "frame 2");
}

#[test]
fn after_an_exec_the_alternate_screen_and_modes_are_back_and_the_main_screen_kept() {
    let (vt, raw) = damage("x");
    assert!(vt.alt_screen());
    assert!(vt.mode(1002) && vt.mode(2004));
    assert_eq!(vt.text(), "frame 1");
    let mut end = Vt::new(20, 5);
    end.feed(raw.as_bytes());
    assert!(!end.alt_screen() && !end.mode(1002) && end.autowrap());
    assert!(end.unknown().is_empty(), "{:?}", end.unknown());
}

#[test]
fn queries_are_gated_as_bubble_tea_gates_them() {
    let env = |pairs: &[(&str, &str)]| -> crate::Environment { pairs.iter().copied().collect() };
    assert!(super::should_query(&env(&[])));
    assert!(super::should_query(&env(&[("TERM_PROGRAM", "WezTerm")])));
    assert!(!super::should_query(&env(&[(
        "TERM_PROGRAM",
        "Apple_Terminal"
    )])));
    assert!(!super::should_query(&env(&[("SSH_TTY", "/dev/pts/1")])));
    assert!(super::should_query(&env(&[
        ("SSH_TTY", "x"),
        ("WT_SESSION", "1")
    ])));
    assert!(super::should_query(&env(&[
        ("SSH_TTY", "x"),
        ("TERM", "xterm-kitty")
    ])));
}
