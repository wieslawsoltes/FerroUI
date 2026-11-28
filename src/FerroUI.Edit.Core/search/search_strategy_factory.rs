use super::{regex_search_strategy::StartBoundary, RegexSearchStrategy, SearchError};
use crate::regex_syntax::{self, NormalizedPattern};
use crate::unicode_case::CaseLocale;
use fancy_regex::RegexBuilder;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchMode {
    #[default]
    Normal,
    Wildcard,
    RegEx,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchOptions {
    pub ignore_case: bool,
    pub match_whole_words: bool,
    pub mode: SearchMode,
}

pub struct SearchStrategyFactory;
impl SearchStrategyFactory {
    /// Compiles a bounded search expression, including lookaround and backreferences.
    /// Expressions outside the supported language are rejected with a diagnostic.
    pub fn create(
        pattern: &str,
        ignore_case: bool,
        match_whole_words: bool,
        mode: SearchMode,
    ) -> Result<RegexSearchStrategy, SearchError> {
        Self::with_options(
            pattern,
            SearchOptions {
                ignore_case,
                match_whole_words,
                mode,
            },
        )
    }
    pub fn with_options(
        pattern: &str,
        options: SearchOptions,
    ) -> Result<RegexSearchStrategy, SearchError> {
        Self::with_options_for_language(pattern, options, "und")
    }

    /// Compiles a search with the supplied language for literal case comparisons.
    /// Regex mode uses the regex backend's Unicode case handling.
    pub fn with_options_for_language(
        pattern: &str,
        options: SearchOptions,
        language: &str,
    ) -> Result<RegexSearchStrategy, SearchError> {
        let expression = match options.mode {
            SearchMode::Normal => regex::escape(pattern),
            SearchMode::Wildcard => literal_expression(pattern, true, None),
            SearchMode::RegEx => pattern.to_owned(),
        };
        let literal_case = (options.ignore_case && options.mode != SearchMode::RegEx)
            .then(|| CaseLocale::from_language_tag(language));
        let executable_literal = literal_case.map(|locale| {
            literal_expression(pattern, options.mode == SearchMode::Wildcard, Some(locale))
        });
        let normalized = if options.mode == SearchMode::RegEx {
            Some(regex_syntax::normalize_search(&expression, true).map_err(SearchError::Pattern)?)
        } else {
            None
        };
        let compile = |expression: &str, normalized: Option<&NormalizedPattern>| {
            RegexBuilder::new(expression)
                .multi_line(true)
                .case_insensitive(options.ignore_case && options.mode == SearchMode::RegEx)
                .seek(normalized.is_some_and(NormalizedPattern::seek_candidates))
                .allow_input_assertion_overrides(
                    normalized.is_some_and(|pattern| pattern.has_contiguous_anchor),
                )
                .backtrack_limit(1_000_000)
                .delegate_size_limit(16 * 1024 * 1024)
                .delegate_dfa_size_limit(8 * 1024 * 1024)
                .build()
        };
        let regex = compile(
            normalized
                .as_ref()
                .map(|pattern| pattern.expression.as_str())
                .or(executable_literal.as_deref())
                .unwrap_or(&expression),
            normalized.as_ref(),
        )
        .map_err(|e| SearchError::Pattern(e.to_string()))?;
        let unlowered = normalized
            .as_ref()
            .filter(|pattern| pattern.has_unicode_word_boundaries)
            .and_then(|_| regex_syntax::normalize_search(&expression, false).ok());
        let native_boundaries = unlowered
            .as_ref()
            .and_then(|pattern| compile(&pattern.expression, Some(pattern)).ok());
        let start_boundary = unlowered
            .as_ref()
            .and_then(|pattern| StartBoundary::required_by(&pattern.expression));
        let (source_pattern, slots) = if let Some(pattern) = normalized {
            (Some(expression), pattern.capture_slots)
        } else {
            (literal_case.is_some().then_some(expression), vec![(0, 0)])
        };
        RegexSearchStrategy::new(
            regex,
            native_boundaries,
            start_boundary,
            options,
            source_pattern,
            slots,
        )
    }
}

fn literal_expression(pattern: &str, wildcard: bool, locale: Option<CaseLocale>) -> String {
    let mut output = String::with_capacity(pattern.len());
    let mut literals = HashMap::new();
    for character in pattern.chars() {
        match (wildcard, character) {
            (true, '*') => output.push_str(".*"),
            (true, '?') => output.push('.'),
            _ => output.push_str(literals.entry(character).or_insert_with(|| {
                // Supplementary literals are two UTF-16 units in the source search
                // and their case does not change the comparison.
                if let Some(locale) = locale.filter(|_| character.len_utf16() == 1) {
                    let mut equivalents = locale.lowercase_equivalents(character);
                    let first = equivalents
                        .next()
                        .expect("case equivalents include the lowercase form");
                    if let Some(second) = equivalents.next() {
                        let mut class = String::from("[");
                        class.extend([first, second]);
                        class.extend(equivalents);
                        class.push(']');
                        return class;
                    }
                }
                regex::escape(character.encode_utf8(&mut [0; 4]))
            })),
        }
    }
    output
}
