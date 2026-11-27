use super::*;

#[test]
fn clusters_keep_marks_and_wide_characters_whole() {
    assert_eq!(
        clusters("e\u{301}字a"),
        [("e\u{301}", 1), ("字", 2), ("a", 1)]
    );
    assert_eq!(clusters(""), []);
}

#[test]
fn grapheme_at_finds_the_whole_cluster() {
    let line: Vec<char> = "ae\u{301}b".chars().collect();
    assert_eq!(grapheme_at(&line, 0), (0, 1));
    assert_eq!(grapheme_at(&line, 1), (1, 3));
    assert_eq!(grapheme_at(&line, 2), (1, 3));
    assert_eq!(grapheme_at(&line, 3), (3, 4));
    assert_eq!(grapheme_at(&line, 4), (4, 4));
}

#[test]
fn sanitize_drops_controls_and_replaces_breaks() {
    let dirty: Vec<char> = "a\tb\r\nc\u{7}\u{FFFD}".chars().collect();
    assert_eq!(
        sanitize(&dirty, " ", "  ").iter().collect::<String>(),
        "a  b  c"
    );
    assert!(is_space('\u{a0}') && !is_space('x'));
}
