//! Environment variables as Go reads them.

use super::*;

#[test]
fn the_last_of_a_name_wins_and_flags_read_as_go_parses_bools() {
    let env: Environment = [("A", "1"), ("A", "true"), ("B", "yes"), ("C", "T")]
        .into_iter()
        .collect();
    assert_eq!(env.get("A"), Some("true"));
    assert_eq!(env.var("missing"), "");
    assert!(env.flag("A") && env.flag("C"));
    assert!(!env.flag("B"), "Go's ParseBool does not take yes");
    assert!(!env.flag("missing"));
}

#[test]
fn the_current_environment_is_the_process_one() {
    let path = std::env::var("PATH").ok();
    assert_eq!(Environment::current().get("PATH"), path.as_deref());
}
