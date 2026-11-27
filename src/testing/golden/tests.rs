use super::*;

#[test]
fn the_same_text_has_no_difference() {
    assert_eq!(difference("a\nb\n", "a\nb\n"), None);
}

#[test]
fn a_difference_names_the_first_line_that_differs() {
    let report = difference("a\nb\nc\n", "a\nx\nc\n").unwrap();
    assert!(report.starts_with("line 2:\n"), "{report}");
    assert!(report.contains("expected: \"b\""), "{report}");
    assert!(report.contains("actual:   \"x\""), "{report}");
    let shorter = difference("a\nb", "a").unwrap();
    assert!(shorter.contains("actual:   \"<end>\""), "{shorter}");
}

#[test]
fn escaped_output_keeps_one_line_a_newline() {
    assert_eq!(escape(b"a\"b'\tc\n\x1b[m"), "a\"b'\\tc\\n\n\\u{1b}[m");
}

#[test]
fn golden_files_live_under_tests_golden() {
    assert!(golden_path("x").ends_with("tests/golden/x.golden"));
}

/// A fresh directory for one test's files.
fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("petitcarnets-golden-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn an_update_writes_the_file_then_a_check_reads_it_back() {
    let dir = scratch("update");
    let path = dir.join("nested").join("x.golden");
    check(&path, "one\ntwo\n", true);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\ntwo\n");
    check(&path, "one\ntwo\n", false);
    std::fs::write(&path, "one\r\ntwo\r\n").unwrap();
    check(&path, "one\ntwo\n", false);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[should_panic(expected = "differs from the actual output")]
fn a_different_file_fails_the_check() {
    let dir = scratch("differs");
    let path = dir.join("x.golden");
    check(&path, "old\n", true);
    check(&path, "new\n", false);
}

#[test]
#[should_panic(expected = "PETITCARNETS_UPDATE_GOLDEN=1 to write it")]
fn a_missing_file_says_how_to_write_it() {
    check(&scratch("missing").join("x.golden"), "", false);
}

#[test]
#[should_panic(expected = "cannot write")]
fn an_unwritable_file_fails_the_update() {
    let dir = scratch("unwritable");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("file");
    std::fs::write(&file, "").unwrap();
    check(&file.join("x.golden"), "", true);
}
