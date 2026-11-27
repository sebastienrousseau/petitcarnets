//! Fuzzy matching as `sahilm/fuzzy` v0.1.3 does it, which Bubbles'
//! default list filter uses: the pattern's characters in order, scored
//! with bonuses for the first character, camel case, a character after
//! a separator, and runs of adjacent matches, and penalties for the
//! characters before the first match and those left unmatched.

/// The bonuses and penalties, as in Go.
const FIRST_CHAR_MATCH_BONUS: i64 = 10;
const MATCH_FOLLOWING_SEPARATOR_BONUS: i64 = 20;
const CAMEL_CASE_MATCH_BONUS: i64 = 20;
const ADJACENT_MATCH_BONUS: i64 = 5;
const UNMATCHED_LEADING_CHAR_PENALTY: i64 = -5;
const MAX_UNMATCHED_LEADING_CHAR_PENALTY: i64 = -15;

/// A target that matched: its index, the byte offsets of the matched
/// characters, and its score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Match {
    pub(super) index: usize,
    pub(super) matched: Vec<usize>,
    pub(super) score: i64,
}

/// The targets that match `pattern`, in their order: Go's `FindNoSort`.
pub(super) fn find_no_sort(pattern: &str, targets: &[String]) -> Vec<Match> {
    let runes: Vec<char> = pattern.chars().collect();
    if runes.is_empty() {
        return Vec::new();
    }
    targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            let clean = target.split('\0').next().unwrap_or_default();
            let m = score(&runes, clean, index);
            (m.matched.len() == runes.len()).then_some(m)
        })
        .collect()
}

/// The targets that match `pattern`, best first, ties in order: Go's
/// `Find`.
pub(super) fn find(pattern: &str, targets: &[String]) -> Vec<Match> {
    let mut matches = find_no_sort(pattern, targets);
    matches.sort_by_key(|m| std::cmp::Reverse(m.score));
    matches
}

/// The state of matching one target.
struct Scan {
    pattern_index: usize,
    best_score: i64,
    matched_index: Option<usize>,
    adjacent: i64,
    last: char,
    last_index: usize,
}

impl Scan {
    /// One character of the target, `(byte offset, character)`, with the
    /// next character after it (NUL at the end).
    fn step(&mut self, m: &mut Match, pattern: &[char], (j, candidate): (usize, char), next: char) {
        if pattern
            .get(self.pattern_index)
            .is_some_and(|p| equal_fold(candidate, *p))
        {
            consider(self, m, j, candidate);
        }
        let next_p = pattern.get(self.pattern_index + 1).copied().unwrap_or('\0');
        if equal_fold(next_p, next) || next == '\0' {
            commit(self, m);
        }
        self.last_index = j;
        self.last = candidate;
    }
}

/// Matches `pattern` against `target`, Go's inner loop.
fn score(pattern: &[char], target: &str, index: usize) -> Match {
    let mut m = Match {
        index,
        matched: Vec::new(),
        score: 0,
    };
    let mut scan = Scan {
        pattern_index: 0,
        best_score: -1,
        matched_index: None,
        adjacent: 0,
        last: '\0',
        last_index: 0,
    };
    let chars: Vec<(usize, char)> = target.char_indices().collect();
    for (k, &here) in chars.iter().enumerate() {
        let next = chars.get(k + 1).map_or('\0', |c| c.1);
        scan.step(&mut m, pattern, here, next);
    }
    let penalty = signed(m.matched.len()) - signed(target.len());
    m.score += penalty;
    m
}

/// A candidate equal to the pattern's character: its score, kept if
/// it is the best so far.
fn consider(scan: &mut Scan, m: &Match, j: usize, candidate: char) {
    let mut score = 0;
    if j == 0 {
        score += FIRST_CHAR_MATCH_BONUS;
    }
    if scan.last.is_lowercase() && candidate.is_uppercase() {
        score += CAMEL_CASE_MATCH_BONUS;
    }
    if j != 0 && is_separator(scan.last) {
        score += MATCH_FOLLOWING_SEPARATOR_BONUS;
    }
    if let Some(&last_match) = m.matched.last() {
        let bonus = if last_match == scan.last_index {
            scan.adjacent * 2 + ADJACENT_MATCH_BONUS
        } else {
            0
        };
        score += bonus;
        scan.adjacent += bonus;
    }
    if score > scan.best_score {
        scan.best_score = score;
        scan.matched_index = Some(j);
    }
}

/// The best candidate so far becomes the match for the pattern's
/// character, and the next character is looked for.
fn commit(scan: &mut Scan, m: &mut Match) {
    let Some(matched) = scan.matched_index else {
        return;
    };
    if m.matched.is_empty() {
        let penalty = signed(matched) * UNMATCHED_LEADING_CHAR_PENALTY;
        scan.best_score += penalty.max(MAX_UNMATCHED_LEADING_CHAR_PENALTY);
    }
    m.score += scan.best_score;
    m.matched.push(matched);
    scan.best_score = -1;
    scan.pattern_index += 1;
}

/// Whether `a` and `b` are the same letter in any case, as Go's
/// `equalFold` asks with simple case folding.
fn equal_fold(a: char, b: char) -> bool {
    use crate::unicode::{to_lower, to_upper};
    a == b || to_lower(a) == to_lower(b) || to_upper(a) == to_upper(b)
}

/// The characters after which a match scores more.
fn is_separator(c: char) -> bool {
    matches!(c, '/' | '-' | '_' | ' ' | '.' | '\\')
}

/// `n` as a score.
fn signed(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}
