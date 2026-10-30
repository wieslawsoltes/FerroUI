// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Shared source-runtime string operations absent from the current System package.
use crate::workspace_ordinal_data::ORDINAL_UPPER;
fn ordinal_upper(value: char) -> char {
    let scalar = value as u32;
    match ORDINAL_UPPER.binary_search_by_key(&scalar, |&(lower, _)| lower) {
        Ok(index) => char::from_u32(ORDINAL_UPPER[index].1).expect("a captured Unicode scalar"),
        Err(_) => value,
    }
}
/// Compares valid Unicode strings using the captured source OrdinalIgnoreCase mapping.
pub fn ordinal_ignore_case_equal(left: &str, right: &str) -> bool {
    left.chars()
        .map(ordinal_upper)
        .eq(right.chars().map(ordinal_upper))
}
/// Tests a prefix with source ordinal casing, without culture or multi-character expansion.
pub fn ordinal_ignore_case_starts_with(value: &str, prefix: &str) -> bool {
    let mut chars = value.chars();
    prefix.chars().all(|p| {
        chars
            .next()
            .is_some_and(|v| ordinal_upper(v) == ordinal_upper(p))
    })
}
/// Tests substring containment with the original ordinal-ignore-case scalar mapping.
pub fn ordinal_ignore_case_contains(value: &str, needle: &str) -> bool {
    needle.is_empty()
        || value
            .char_indices()
            .any(|(index, _)| ordinal_ignore_case_starts_with(&value[index..], needle))
}
/// Source String.IsNullOrWhiteSpace for Rust's non-null string representation.
pub fn is_white_space(value: &str) -> bool {
    crate::dock_workspace::is_blank(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_comparisons_and_prefixes_match_the_independent_dotnet_corpus() {
        fn decode(hex: &str) -> String {
            String::from_utf8(
                hex.as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect(),
            )
            .unwrap()
        }
        let mut count = 0;
        for line in include_str!("../../../tests/Dock/data/workspace-ordinal.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let mut fields = line.split('\t');
            let left = decode(fields.next().unwrap());
            let right = decode(fields.next().unwrap());
            let equal = fields.next().unwrap() == "1";
            assert_eq!(
                ordinal_ignore_case_equal(&left, &right),
                equal,
                "{left:?} vs {right:?}"
            );
            assert_eq!(
                ordinal_ignore_case_starts_with(&format!("{left}/tail"), &format!("{right}/")),
                equal,
                "prefix {left:?} vs {right:?}"
            );
            count += 1;
        }
        assert_eq!(count, 3005);
        assert!(ordinal_ignore_case_starts_with("anything", ""));
        assert!(!ordinal_ignore_case_starts_with("", "a"));
        assert!(is_white_space("\u{0085}\u{2000}\u{3000}"));
        assert!(!is_white_space("\u{200b}"));
    }
}
