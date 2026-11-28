//! The width method is shared by the whole process, and every
//! `TestProgram` starts by setting it back to `WcWidth`; this test
//! changes it, so it runs in a binary of its own, where no other test
//! can set it back while it looks (it was flaky among the harness's
//! tests, which run in parallel).

use petitcarnets as tea;
use tea::Input;
use tea::testing::TestProgram;

/// A model that draws a word and nothing else.
struct Word;

impl tea::Model for Word {
    type Msg = ();

    fn update(&mut self, _msg: tea::Msg<()>) -> tea::Cmd<()> {
        tea::Cmd::none()
    }

    fn view(&self) -> tea::View {
        "word".into()
    }
}

#[test]
fn a_terminal_with_grapheme_widths_gets_them() {
    let mut test = TestProgram::new(Word, 20, 3);
    test.input(Input::ModeReport {
        mode: tea::ReportedMode::Dec(2027),
        value: tea::ModeSetting::Set,
    });
    assert!(test.screen().mode(2027));
    assert_eq!(
        tea::style::width_method(),
        tea::style::WidthMethod::GraphemeWidth
    );
    test.quit();
    assert!(!test.final_screen().mode(2027));
    assert_eq!(tea::style::width_method(), tea::style::WidthMethod::WcWidth);
}
