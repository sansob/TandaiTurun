//! Find in document.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
}

/// Byte ranges of every non-overlapping match of `needle` in `haystack`.
///
/// Case-insensitive matching compares characters by their lower case, so it
/// works for every script and the ranges always fall on character boundaries.
pub fn find_all(haystack: &str, needle: &str, opts: SearchOptions) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let fold = |c: char| -> char {
        if opts.case_sensitive {
            c
        } else {
            c.to_lowercase().next().unwrap_or(c)
        }
    };
    let pattern: Vec<char> = needle.chars().map(fold).collect();
    let chars: Vec<(usize, char)> = haystack.char_indices().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut i = 0;
    while i + pattern.len() <= chars.len() {
        let hit = pattern.iter().enumerate().all(|(k, p)| fold(chars[i + k].1) == *p);
        if hit {
            let end_idx = i + pattern.len();
            let start = chars[i].0;
            let end = chars.get(end_idx).map_or(haystack.len(), |c| c.0);
            let bounded = !opts.whole_word
                || ((i == 0 || !is_word(chars[i - 1].1)) && chars.get(end_idx).is_none_or(|c| !is_word(c.1)));
            if bounded {
                out.push(start..end);
                i = end_idx;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Replaces every match; returns the new text and how many were replaced.
pub fn replace_all(haystack: &str, needle: &str, replacement: &str, opts: SearchOptions) -> (String, usize) {
    let hits = find_all(haystack, needle, opts);
    let mut out = String::with_capacity(haystack.len());
    let mut last = 0;
    for r in &hits {
        out.push_str(&haystack[last..r.start]);
        out.push_str(replacement);
        last = r.end;
    }
    out.push_str(&haystack[last..]);
    (out, hits.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_insensitive_by_default() {
        assert_eq!(find_all("Rust rust RUST", "rust", SearchOptions::default()), vec![0..4, 5..9, 10..14]);
        let cs = SearchOptions { case_sensitive: true, ..Default::default() };
        assert_eq!(find_all("Rust rust RUST", "rust", cs), vec![5..9]);
    }

    #[test]
    fn unicode_and_whole_words() {
        assert_eq!(find_all("Ägypten ägypten", "ÄGYPTEN", SearchOptions::default()), vec![0..8, 9..17]);
        let ww = SearchOptions { whole_word: true, ..Default::default() };
        assert_eq!(find_all("cat concat cat_x cat.", "cat", ww), vec![0..3, 17..20]);
        assert!(find_all("abc", "", SearchOptions::default()).is_empty());
        assert_eq!(find_all("aaaa", "aa", SearchOptions::default()), vec![0..2, 2..4]);
    }

    #[test]
    fn replace() {
        assert_eq!(replace_all("a-b-a", "a", "x", SearchOptions::default()), ("x-b-x".into(), 2));
    }
}
