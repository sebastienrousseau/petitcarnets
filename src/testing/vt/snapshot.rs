//! The whole state a test usually checks, as text, for golden files.

use super::Vt;
use std::fmt::Write as _;

impl Vt {
    /// The screen and the state a test usually checks, as text: the
    /// size and which screen is shown, the cursor (place, whether it is
    /// shown, its style), the DEC private modes set, the keyboard modes,
    /// the title if there is one, then a rule and what the screen shows.
    /// Stable from run to run, for [`assert_golden`](crate::testing::assert_golden).
    ///
    /// ```
    /// use petitcarnets::testing::Vt;
    ///
    /// let mut vt = Vt::new(8, 2);
    /// vt.feed(b"\x1b[?2004hhi");
    /// assert_eq!(
    ///     vt.snapshot(),
    ///     "size 8x2, main screen\n\
    ///      cursor 2,0 shown, style 0\n\
    ///      modes 7 25 2004\n\
    ///      keyboard flags 0, modifyOtherKeys 0\n\
    ///      --------\n\
    ///      hi\n"
    /// );
    /// ```
    #[must_use]
    pub fn snapshot(&self) -> String {
        let (x, y) = self.cursor();
        let screen = if self.alt_screen() {
            "alternate"
        } else {
            "main"
        };
        let shown = if self.cursor_visible() {
            "shown"
        } else {
            "hidden"
        };
        let modes: Vec<String> = self.modes().map(|m| m.to_string()).collect();
        let mut out = format!("size {}x{}, {screen} screen\n", self.columns, self.rows);
        let _ = writeln!(out, "cursor {x},{y} {shown}, style {}", self.cursor_style());
        let _ = writeln!(out, "modes {}", modes.join(" "));
        let _ = writeln!(
            out,
            "keyboard flags {}, modifyOtherKeys {}",
            self.keyboard_flags(),
            self.modify_other_keys()
        );
        if !self.title().is_empty() {
            let _ = writeln!(out, "title {:?}", self.title());
        }
        out.push_str(&"-".repeat(usize::from(self.columns)));
        out.push('\n');
        for line in self.text().lines() {
            out.push_str(line);
            out.push('\n');
        }
        out
    }
}
