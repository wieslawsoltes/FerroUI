use super::{SearchOptions, SearchResult};
use crate::document::{DocumentError, TextDocument};
use fancy_regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::OnceLock;

#[derive(Debug)]
pub enum SearchError {
    Pattern(String),
    MatchLimit(String),
    Range,
    Document(DocumentError),
}
impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pattern(e) => write!(f, "Invalid search expression: {e}"),
            Self::MatchLimit(e) => write!(f, "Search expression exceeded its match budget: {e}"),
            Self::Range => f.write_str("Search range lies outside the document"),
            Self::Document(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for SearchError {}
impl From<DocumentError> for SearchError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}

pub trait ISearchStrategy {
    fn find_all(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Vec<SearchResult>, SearchError>;
    fn find_next(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Option<SearchResult>, SearchError> {
        Ok(self.find_all(text, offset, length)?.into_iter().next())
    }
}

#[derive(Clone, Debug)]
pub struct RegexSearchStrategy {
    regex: Regex,
    options: SearchOptions,
    names: Rc<HashMap<String, usize>>,
}
impl RegexSearchStrategy {
    pub(crate) fn new(regex: Regex, options: SearchOptions) -> Self {
        let names = Rc::new(
            regex
                .capture_names()
                .enumerate()
                .filter_map(|(i, n)| n.map(|n| (n.to_owned(), i)))
                .collect(),
        );
        Self {
            regex,
            options,
            names,
        }
    }
    pub fn options(&self) -> SearchOptions {
        self.options
    }
    pub fn pattern(&self) -> &str {
        self.regex.as_str()
    }
    pub fn find_all(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        <Self as ISearchStrategy>::find_all(self, text, offset, length)
    }
    pub fn find_next(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Option<SearchResult>, SearchError> {
        <Self as ISearchStrategy>::find_next(self, text, offset, length)
    }
    pub fn replace_all(
        &self,
        document: &TextDocument,
        offset: usize,
        length: usize,
        replacement: &str,
    ) -> Result<usize, SearchError> {
        let matches = self.find_all(&document.text(), offset, length)?;
        let _update = document.begin_update()?;
        for result in matches.iter().rev() {
            document.replace(
                result.offset,
                result.length,
                &result.replace_with(replacement),
            )?;
        }
        Ok(matches.len())
    }
}
impl ISearchStrategy for RegexSearchStrategy {
    fn find_all(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let mut units = 0;
        let mut positions = Vec::with_capacity(text.chars().count() + 1);
        for (byte, ch) in text.char_indices() {
            positions.push((byte, units));
            units += ch.len_utf16();
        }
        positions.push((text.len(), units));
        if offset > units || length > units - offset {
            return Err(SearchError::Range);
        }
        let end = offset + length;
        let input: Rc<str> = Rc::from(text);
        let mut output = Vec::new();
        for captures in self.regex.captures_iter(text) {
            let captures = captures.map_err(|e| SearchError::MatchLimit(e.to_string()))?;
            let matched = captures.get(0).expect("capture zero always exists");
            let start = positions[positions
                .binary_search_by_key(&matched.start(), |p| p.0)
                .unwrap()]
            .1;
            let finish = positions[positions
                .binary_search_by_key(&matched.end(), |p| p.0)
                .unwrap()]
            .1;
            if start < offset || finish > end {
                continue;
            }
            if self.options.match_whole_words
                && (!word_border(text, matched.start()) || !word_border(text, matched.end()))
            {
                continue;
            }
            output.push(SearchResult {
                offset: start,
                length: finish - start,
                input: input.clone(),
                captures: captures.iter().map(|c| c.map(|m| m.range())).collect(),
                names: self.names.clone(),
            });
        }
        Ok(output)
    }
}

fn word_border(text: &str, offset: usize) -> bool {
    let before = text[..offset].chars().next_back();
    let after = text[offset..].chars().next();
    match (before, after) {
        (None, None) => false,
        (None, Some(c)) | (Some(c), None) => !c.is_whitespace(),
        (Some(before), Some(after)) => {
            let after = character_class(after);
            after != 3 && character_class(before) != after
        }
    }
}
fn character_class(ch: char) -> u8 {
    if ch == '\r' || ch == '\n' {
        return 0;
    }
    if ch.is_whitespace() || ch.is_control() {
        return 1;
    }
    static IDENTIFIER: OnceLock<regex::Regex> = OnceLock::new();
    static MARK: OnceLock<regex::Regex> = OnceLock::new();
    let mut bytes = [0; 4];
    let s = ch.encode_utf8(&mut bytes);
    if IDENTIFIER
        .get_or_init(|| regex::Regex::new(r"^[\p{L}\p{Nd}_]$").unwrap())
        .is_match(s)
    {
        2
    } else if MARK
        .get_or_init(|| regex::Regex::new(r"^\p{M}$").unwrap())
        .is_match(s)
    {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{SearchMode, SearchStrategyFactory};
    #[test]
    fn upstream_whole_word_cases() {
        for (pattern, text, count) in [
            ("All", " FindAllTests ", 0),
            ("AllTests", "name=\"{FindAllTests}\"", 0),
            ("// find", "            // findtest", 0),
            ("// find", "            // find me", 1),
            ("result", "result           // find me", 1),
            ("me", "result           // find me", 1),
            ("Text", ".Text.", 1),
        ] {
            let strategy =
                SearchStrategyFactory::create(pattern, false, true, SearchMode::Normal).unwrap();
            assert_eq!(
                strategy.find_all(text, 0, text.len()).unwrap().len(),
                count,
                "{pattern} in {text}"
            );
        }
    }
    #[test]
    fn offsets_are_utf16_and_range_does_not_change_anchors() {
        let strategy =
            SearchStrategyFactory::create("^foo", false, false, SearchMode::RegEx).unwrap();
        assert!(strategy.find_all("😀foo", 2, 3).unwrap().is_empty());
        let strategy =
            SearchStrategyFactory::create("foo", false, false, SearchMode::Normal).unwrap();
        let results = strategy.find_all("😀foo", 0, 5).unwrap();
        assert_eq!((results[0].offset, results[0].length), (2, 3));
    }
    #[test]
    fn replacement_forms_and_zero_length_matches() {
        let strategy =
            SearchStrategyFactory::create(r"(?P<a>foo)(bar)?", false, false, SearchMode::RegEx)
                .unwrap();
        let results = strategy.find_all("xfooy", 0, 5).unwrap();
        assert_eq!(
            results[0].replace_with("${a}|$2|$$|$&|$`|$'|$_|$10"),
            "foo||$|foo|x|y|xfooy|$10"
        );
        let document = TextDocument::from_text("ab");
        let empty = SearchStrategyFactory::create("", false, false, SearchMode::RegEx).unwrap();
        assert_eq!(empty.replace_all(&document, 0, 2, "|").unwrap(), 3);
        assert_eq!(document.text(), "|a|b|");
        document.undo().unwrap();
        assert_eq!(document.text(), "ab");
    }
}
