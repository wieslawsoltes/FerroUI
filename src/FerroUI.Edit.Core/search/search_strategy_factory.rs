use super::{RegexSearchStrategy, SearchError};
use fancy_regex::RegexBuilder;

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
        let expression = match options.mode {
            SearchMode::Normal => regex::escape(pattern),
            SearchMode::Wildcard => pattern
                .chars()
                .map(|c| match c {
                    '*' => ".*".into(),
                    '?' => ".".into(),
                    _ => regex::escape(&c.to_string()),
                })
                .collect(),
            SearchMode::RegEx => pattern.to_owned(),
        };
        let regex = RegexBuilder::new(&expression)
            .multi_line(true)
            .case_insensitive(options.ignore_case)
            .backtrack_limit(1_000_000)
            .delegate_size_limit(16 * 1024 * 1024)
            .delegate_dfa_size_limit(8 * 1024 * 1024)
            .build()
            .map_err(|e| SearchError::Pattern(e.to_string()))?;
        Ok(RegexSearchStrategy::new(regex, options))
    }
}
