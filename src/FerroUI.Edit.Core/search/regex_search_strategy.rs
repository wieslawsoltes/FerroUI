use super::{search_result::CaptureLayout, SearchOptions, SearchResult};
use crate::document::{DocumentEdit, DocumentError, TextDocument};
use crate::regex_syntax::{native_word_boundaries_agree, word_boundary_bytes};
use fancy_regex::{Assertion, Captures, Error, Expr, LookAround, Regex, RegexInput, RuntimeError};
use std::{fmt, ops::ControlFlow};
use std::rc::Rc;
use std::sync::OnceLock;

#[derive(Debug)]
pub enum SearchError {
    Pattern(String),
    Replacement(String),
    MatchLimit(String),
    Range,
    Document(DocumentError),
}
impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pattern(e) => write!(f, "Invalid search expression: {e}"),
            Self::Replacement(e) => write!(f, "Invalid replacement: {e}"),
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
    native_boundaries: Option<Regex>,
    start_boundary: Option<StartBoundary>,
    options: SearchOptions,
    source_pattern: Option<String>,
    layout: Rc<CaptureLayout>,
}
impl RegexSearchStrategy {
    pub(crate) fn new(
        regex: Regex,
        native_boundaries: Option<Regex>,
        start_boundary: Option<StartBoundary>,
        options: SearchOptions,
        source_pattern: Option<String>,
        slots: Vec<(usize, usize)>,
    ) -> Result<Self, SearchError> {
        let backend_names: Vec<_> = regex.capture_names().collect();
        if backend_names.len() != slots.len() {
            return Err(SearchError::Pattern("Unsupported capture layout".into()));
        }
        let names = slots
            .iter()
            .filter_map(|&(number, backend)| {
                backend_names[backend].map(|name| (name.to_owned(), number))
            })
            .collect();
        Ok(Self {
            regex,
            native_boundaries,
            start_boundary,
            options,
            source_pattern,
            layout: Rc::new(CaptureLayout { slots, names }),
        })
    }
    pub fn options(&self) -> SearchOptions {
        self.options
    }
    pub fn pattern(&self) -> &str {
        self.source_pattern
            .as_deref()
            .unwrap_or_else(|| self.regex.as_str())
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
        let version = document.version();
        let edits = self
            .find_all(&document.text(), offset, length)?
            .iter()
            .map(|hit| {
                Ok(DocumentEdit::new(
                    hit.offset,
                    hit.length,
                    hit.try_replace_with(replacement)?,
                ))
            })
            .collect::<Result<Vec<_>, SearchError>>()?;
        if edits.is_empty() {
            return Ok(0);
        }
        let update = document.begin_update()?;
        if document.version() != version {
            return Err(DocumentError::ReentrantChange.into());
        }
        document.apply_edits(&edits)?;
        update.finish()?;
        Ok(edits.len())
    }

    fn scan_matches(
        &self,
        text: &str,
        offset: usize,
        length: usize,
        mut visit: impl FnMut(SearchResult) -> ControlFlow<SearchResult>,
    ) -> Result<Option<SearchResult>, SearchError> {
        let ascii = text.is_ascii();
        let units = if ascii {
            text.len()
        } else {
            text.encode_utf16().count()
        };
        if offset > units || length > units - offset {
            return Err(SearchError::Range);
        }
        let end = offset + length;
        let mut last_byte = 0;
        let mut last_offset = 0;
        let mut offset_at = |byte| {
            if ascii {
                return byte;
            }
            // A lookbehind with \K can move the reported start backwards.
            if byte < last_byte {
                last_offset -= text[byte..last_byte].encode_utf16().count();
            } else {
                last_offset += text[last_byte..byte].encode_utf16().count();
            }
            last_byte = byte;
            last_offset
        };
        let mut input = None;
        let native = self
            .native_boundaries
            .as_ref()
            .filter(|_| native_word_boundaries_agree(text));
        let regex = native.unwrap_or(&self.regex);
        let mut candidates = None;
        let mut remaining = RegexInput::new(text);
        loop {
            let captures = if let Some(candidates) = &mut candidates {
                captures_at_candidates(regex, &remaining, candidates)
            } else {
                match (regex.captures_input(remaining.clone()), self.start_boundary) {
                    (
                        Err(Error::RuntimeError(RuntimeError::BacktrackLimitExceeded)),
                        Some(boundary),
                    ) if native.is_none() => {
                        // The ordinary engine can seek on a selective literal. Retry only
                        // when that scan runs out of budget and a required boundary gives
                        // us independent candidate starts on the same input.
                        candidates = Some(boundary.candidates(text));
                        continue;
                    }
                    (captures, _) => captures,
                }
            };
            let Some(captures) = captures.map_err(|e| SearchError::MatchLimit(e.to_string()))?
            else {
                break;
            };
            let matched = captures.get(0).expect("capture zero always exists");
            let empty = matched.start() == matched.end();
            let advance = if empty {
                text[matched.end()..]
                    .chars()
                    .next()
                    .map_or(1, char::len_utf8)
            } else {
                0
            };
            // A zero-length hit advances the next scan past that position.
            // The skipped position is still the last match end for \G.
            remaining = RegexInput::new(text)
                .from_pos(matched.end() + advance)
                .continue_from_previous_match_end(!empty);
            let start = offset_at(matched.start());
            let finish = offset_at(matched.end());
            if start < offset || finish > end {
                continue;
            }
            if self.options.match_whole_words
                && (!word_border(text, matched.start()) || !word_border(text, matched.end()))
            {
                continue;
            }
            let hit = SearchResult {
                offset: start,
                length: finish - start,
                input: input.get_or_insert_with(|| Rc::<str>::from(text)).clone(),
                captures: self
                    .layout
                    .slots
                    .iter()
                    .map(|&(_, backend)| captures.get(backend).map(|found| found.range()))
                    .collect(),
                layout: self.layout.clone(),
            };
            if let ControlFlow::Break(hit) = visit(hit) {
                return Ok(Some(hit));
            }
        }
        Ok(None)
    }
}

impl ISearchStrategy for RegexSearchStrategy {
    fn find_all(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let mut hits = Vec::new();
        self.scan_matches(text, offset, length, |hit| {
            hits.push(hit);
            ControlFlow::Continue(())
        })?;
        Ok(hits)
    }

    fn find_next(
        &self,
        text: &str,
        offset: usize,
        length: usize,
    ) -> Result<Option<SearchResult>, SearchError> {
        self.scan_matches(text, offset, length, ControlFlow::Break)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartBoundary {
    Word,
    NonWord,
}

impl StartBoundary {
    pub(crate) fn required_by(pattern: &str) -> Option<Self> {
        let expr = Expr::parse_tree(pattern).ok()?.expr;
        // These constructs can depend on or change the engine's search start
        // or skip subsequent candidates. Keep their unanchored execution.
        if expr.has_descendant(|part| {
            matches!(
                part,
                Expr::ContinueFromPreviousMatchEnd
                    | Expr::KeepOut
                    | Expr::BacktrackingControlVerb(_)
                    | Expr::Absent(_)
                    | Expr::SubroutineCall(_)
            )
        }) {
            return None;
        }
        Self::at_start(&expr)
    }

    fn at_start(expr: &Expr) -> Option<Self> {
        match expr {
            Expr::Assertion(Assertion::WordBoundary) => Some(Self::Word),
            Expr::Assertion(Assertion::NotWordBoundary) => Some(Self::NonWord),
            Expr::Group(child) => Self::at_start(child),
            Expr::AtomicGroup(child) | Expr::LookAround(child, LookAround::LookAhead) => {
                Self::at_start(child)
            }
            Expr::Repeat { child, lo, .. } if *lo > 0 => Self::at_start(child),
            Expr::Concat(parts) => {
                for part in parts {
                    if let Some(boundary) = Self::at_start(part) {
                        return Some(boundary);
                    }
                    if !is_zero_width(part) {
                        break;
                    }
                }
                None
            }
            Expr::Alt(parts) => {
                let (first, others) = parts.split_first()?;
                let boundary = Self::at_start(first)?;
                others
                    .iter()
                    .all(|part| Self::at_start(part) == Some(boundary))
                    .then_some(boundary)
            }
            _ => None,
        }
    }

    fn candidates(self, text: &str) -> Box<dyn Iterator<Item = usize> + '_> {
        let boundaries = word_boundary_bytes(text);
        match self {
            Self::Word => Box::new(boundaries),
            Self::NonWord => {
                let mut boundaries = boundaries.peekable();
                let positions = text.char_indices().map(|(byte, _)| byte);
                Box::new(
                    positions
                        .chain(std::iter::once(text.len()))
                        .filter(move |byte| boundaries.next_if_eq(byte).is_none()),
                )
            }
        }
    }
}

fn is_zero_width(expr: &Expr) -> bool {
    match expr {
        Expr::Empty | Expr::Assertion(_) | Expr::LookAround(..) => true,
        Expr::Group(child) => is_zero_width(child),
        Expr::AtomicGroup(child) | Expr::Repeat { child, .. } => is_zero_width(child),
        Expr::Concat(parts) | Expr::Alt(parts) => parts.iter().all(is_zero_width),
        _ => false,
    }
}

fn captures_at_candidates<'t>(
    regex: &Regex,
    input: &RegexInput<'t, str>,
    candidates: &mut Box<dyn Iterator<Item = usize> + 't>,
) -> fancy_regex::Result<Option<Captures<'t, str>>> {
    for byte in candidates.filter(|&byte| byte >= input.start()) {
        if let Some(captures) = regex.captures_input(input.clone().from_pos(byte).anchored(true))? {
            return Ok(Some(captures));
        }
    }
    Ok(None)
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
    fn find_next_stops_before_an_expensive_suffix() {
        let strategy = SearchStrategyFactory::create(
            r"(?<hit>needle)|(a|b|ab)*(?>c)",
            false,
            true,
            SearchMode::RegEx,
        )
        .unwrap();
        let text = format!("😀 aneedle needle! {}!", "ab".repeat(64));
        let length = text.encode_utf16().count();
        assert!(matches!(
            strategy.find_all(&text, 0, length),
            Err(SearchError::MatchLimit(_))
        ));
        let search: &dyn ISearchStrategy = &strategy;
        let hit = search.find_next(&text, 3, length - 3).unwrap().unwrap();
        assert_eq!((hit.offset, hit.length), (11, 6));
        assert_eq!(hit.named_capture("hit"), Some("needle"));
        assert_eq!(hit.try_replace_with("[${hit}]").unwrap(), "[needle]");
    }

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
    fn range_filter_preserves_utf16_offsets_after_skipped_and_empty_matches() {
        let strategy = SearchStrategyFactory::create(
            r"(?<hit>é|🧪)|(?=!)",
            false,
            false,
            SearchMode::RegEx,
        )
        .unwrap();
        let text = "😀é/🧪! é";
        let hits = strategy.find_all(text, 3, 5).unwrap();
        assert_eq!(
            hits.iter()
                .map(|hit| (hit.offset, hit.length, hit.named_capture("hit")))
                .collect::<Vec<_>>(),
            [(4, 2, Some("🧪")), (6, 0, None)]
        );
        let hit = strategy.find_next(text, 3, 5).unwrap().unwrap();
        assert_eq!((hit.offset, hit.length), (4, 2));
        assert_eq!(hit.try_replace_with("$`<$0>$'").unwrap(), "😀é/<🧪>! é");
    }

    #[test]
    fn lookbehind_can_report_a_start_before_the_previous_match_end() {
        let strategy =
            SearchStrategyFactory::create(r"!|(?<=\K🧪!)é", false, false, SearchMode::RegEx)
                .unwrap();
        let hits = strategy.find_all("😀🧪!é", 0, 6).unwrap();
        assert_eq!(
            hits.iter()
                .map(|hit| (hit.offset, hit.length, hit.matched_text()))
                .collect::<Vec<_>>(),
            [(4, 1, "!"), (2, 4, "🧪!é")]
        );
    }

    #[test]
    fn literal_and_wildcard_search_match_lowercase_equivalents() {
        for (mode, language, pattern, text, expected) in [
            (SearchMode::Normal, "und", "s", "s S ſ", &["s", "S"][..]),
            (SearchMode::Normal, "und", "σ", "σ Σ ς", &["σ", "Σ"][..]),
            (SearchMode::Normal, "und", "I", "I i İ ı", &["I", "i"][..]),
            (SearchMode::Normal, "und", "k", "k K K", &["k", "K", "K"][..]),
            (SearchMode::Normal, "und", "𐐀", "𐐀 𐐨", &["𐐀"][..]),
            (SearchMode::Wildcard, "und", "s?", "Sa\nsa\nſa", &["Sa", "sa"][..]),
            (SearchMode::Wildcard, "tr-TR", "i?", "iX IY ıX İY", &["iX", "İY"][..]),
        ] {
            let search = if language == "und" {
                SearchStrategyFactory::create(pattern, true, false, mode)
            } else {
                SearchStrategyFactory::with_options_for_language(
                    pattern,
                    SearchOptions {
                        ignore_case: true,
                        match_whole_words: false,
                        mode,
                    },
                    language,
                )
            }
            .unwrap();
            let matches = search.find_all(text, 0, text.encode_utf16().count()).unwrap();
            assert_eq!(
                matches.iter().map(SearchResult::matched_text).collect::<Vec<_>>(),
                expected,
                "{mode:?} {pattern:?} in {text:?}"
            );
        }
    }
    #[test]
    fn replacement_forms_and_zero_length_matches() {
        let strategy =
            SearchStrategyFactory::create(r"(?P<a>foo)(bar)?", false, false, SearchMode::RegEx)
                .unwrap();
        let results = strategy.find_all("xfooy", 0, 5).unwrap();
        assert_eq!(
            results[0].replace_with("${a}|$1|$2|$$|$&|$`|$'|$_|$10"),
            "foo||foo|$|foo|x|y|xfooy|$10"
        );
        let document = TextDocument::from_text("ab");
        let empty = SearchStrategyFactory::create("", false, false, SearchMode::RegEx).unwrap();
        assert_eq!(empty.replace_all(&document, 0, 2, "|").unwrap(), 3);
        assert_eq!(document.text(), "|a|b|");
        document.undo().unwrap();
        assert_eq!(document.text(), "ab");
    }

    #[test]
    fn end_anchors_use_the_final_line_feed_and_keep_the_search_context() {
        let cases: &[(&str, &str, usize, &[(usize, usize)])] = &[
            (r"\Z", "x\n\n", 3, &[(2, 0), (3, 0)]),
            (r"\z", "x\n\n", 3, &[(3, 0)]),
            (r"x\Z", "x\n\n", 3, &[]),
            (r"x\Z", "x\n", 2, &[(0, 1)]),
            (r"x\Z", "x\r\n", 3, &[]),
            (r"x\r\Z", "x\r\n", 3, &[(0, 2)]),
            (r"\bx\Z\n\z", "x\n", 2, &[(0, 2)]),
            (r"\bx\Z\n\z", "x\n\n", 3, &[]),
            (r"(?<!x\Z)\n", "x\n\n", 3, &[(1, 1), (2, 1)]),
            (r"(?-m:^x$)", "x\n", 2, &[(0, 1)]),
            (r"(?-m:^x$)", "x\n\n", 3, &[]),
            (r"(?-m:$)", "x\r\n", 3, &[(2, 0), (3, 0)]),
            (r"(?-m:$)(?m:|^)", "x\nx\n", 4, &[(3, 0), (4, 0)]),
            (r"(?-m:x$)|^y$", "x\ny\nx\n", 6, &[(2, 1), (4, 1)]),
            (r"(?-m:$)", "x\n", 1, &[(1, 0)]),
        ];
        for &(pattern, text, length, expected) in cases {
            let strategy = SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                .unwrap_or_else(|error| panic!("{pattern}: {error}"));
            let found: Vec<_> = strategy
                .find_all(text, 0, length)
                .unwrap_or_else(|error| panic!("{pattern}: {error}"))
                .into_iter()
                .map(|matched| (matched.offset, matched.length))
                .collect();
            assert_eq!(found, expected, "{pattern} in {text:?}");
        }
        for invalid in [r"[\z]", r"[\Z]"] {
            assert!(
                SearchStrategyFactory::create(invalid, false, false, SearchMode::RegEx).is_err()
            );
        }
    }

    #[test]
    fn final_line_anchor_can_seek_to_the_end_of_a_large_document() {
        for (prefix, copies) in [("a", 1_050_000), ("identifier value\n", 62_000)] {
            let mut text = prefix.repeat(copies);
            text.push_str("last\n");
            let end = text.len();
            for pattern in [r"\Z", r"(?-m:$)"] {
                let strategy =
                    SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                        .unwrap();
                let found = strategy.find_all(&text, 0, end).unwrap();
                assert_eq!(found.len(), 2, "{pattern}");
                assert_eq!((found[0].offset, found[0].length), (end - 1, 0));
                assert_eq!((found[1].offset, found[1].length), (end, 0));
            }
        }
    }

    #[test]
    fn empty_matches_after_consuming_matches_keep_reference_iteration_order() {
        let cases: &[(&str, &str, usize, &[(usize, usize)])] = &[
            (
                r"\G[a-z]?|\d+",
                "a12b",
                0,
                &[(0, 1), (1, 0), (2, 1), (3, 1), (4, 0)],
            ),
            (r"\G(?=a)|b", "aba", 0, &[(0, 0), (1, 1), (2, 0)]),
            (r"\G(?=.)", "abc", 0, &[(0, 0)]),
            (r"(?=b)|\G\w+", "-ba", 0, &[(1, 0)]),
            (r"\G\d+[,]?", "12,34", 3, &[(3, 2)]),
            (r"a|$", "a", 0, &[(0, 1), (1, 0)]),
            (r"(?=.)|\z", "😀a", 0, &[(0, 0), (2, 0), (3, 0)]),
        ];
        for &(pattern, text, offset, expected) in cases {
            let strategy = SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                .unwrap_or_else(|error| panic!("{pattern}: {error}"));
            let found: Vec<_> = strategy
                .find_all(text, offset, text.encode_utf16().count() - offset)
                .unwrap()
                .into_iter()
                .map(|matched| (matched.offset, matched.length))
                .collect();
            assert_eq!(found, expected, "{pattern} in {text:?}");
        }
        let document = TextDocument::from_text("a");
        let strategy =
            SearchStrategyFactory::create(r"a|$", false, false, SearchMode::RegEx).unwrap();
        assert_eq!(strategy.replace_all(&document, 0, 1, "<$&>").unwrap(), 2);
        assert_eq!(document.text(), "<a><>");
        document.undo().unwrap();
        assert_eq!(document.text(), "a");
    }

    #[test]
    fn unicode_word_digit_and_boundary_searches_match_the_reference() {
        let words =
            "a\u{093e} b\u{0301} c\u{200d} d\u{20dd} e\u{2163} ٥ a\u{203f}b \u{10400} \u{1d7d9}";
        let boundaries = "a\u{200c}\u{200d}b a\u{093e}😀 a\u{20dd} b\u{2163} b\u{0301}";
        let bmp_boundaries = "a\u{200c}\u{200d}b a\u{093e} a\u{20dd} b\u{2163} b\u{0301}";
        let cases: &[(&str, &str, &[(usize, usize)])] = &[
            (
                r"\w+",
                words,
                &[(0, 1), (3, 2), (6, 1), (9, 1), (12, 1), (15, 1), (17, 3)],
            ),
            (
                r"[\w-[\d]]+",
                words,
                &[(0, 1), (3, 2), (6, 1), (9, 1), (12, 1), (17, 3)],
            ),
            (r"\d+", words, &[(15, 1)]),
            (
                r"[\W]+",
                "a\u{093e}b\u{0301}c\u{200d}d\u{20dd}e\u{2163}_",
                &[(1, 1), (5, 1), (7, 1), (9, 1)],
            ),
            (r"[^\D]+", "a٥b9c\u{1d7d9}d", &[(1, 1), (3, 1)]),
            (
                r"\b",
                boundaries,
                &[
                    (0, 0),
                    (4, 0),
                    (5, 0),
                    (6, 0),
                    (10, 0),
                    (11, 0),
                    (13, 0),
                    (14, 0),
                    (16, 0),
                    (18, 0),
                ],
            ),
            (
                r"\B",
                bmp_boundaries,
                &[(1, 0), (2, 0), (3, 0), (7, 0), (10, 0), (13, 0), (15, 0)],
            ),
            (
                r"\b\w+\b",
                words,
                &[(0, 1), (3, 2), (9, 1), (12, 1), (15, 1), (17, 3)],
            ),
            (r"(?<=\bfoo)bar", "foobar afoobar", &[(3, 3)]),
            (r"(?<=\bfoo)bar", "\u{093e}foobar afoobar", &[(4, 3)]),
            (r"[\b]", "a\u{0008}b", &[(1, 1)]),
            (r"\B", "", &[(0, 0)]),
            (
                "(?x)\\b  # start\n {start} (foo) \\b{end}",
                "xfoo foo",
                &[(5, 3)],
            ),
        ];
        for &(pattern, text, expected) in cases {
            let strategy = SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                .unwrap_or_else(|error| panic!("{pattern}: {error}"));
            let matches: Vec<_> = strategy
                .find_all(text, 0, text.encode_utf16().count())
                .unwrap()
                .into_iter()
                .map(|matched| (matched.offset, matched.length))
                .collect();
            assert_eq!(matches, expected, "{pattern}");
        }

        let captures = SearchStrategyFactory::create(
            r"\b(?<word>foo)\s+\k<word>\b",
            false,
            false,
            SearchMode::RegEx,
        )
        .unwrap();
        for (text, offset) in [("foo foo", 0), ("\u{093e}foo foo", 1)] {
            let matched = captures.find_next(text, 0, offset + 7).unwrap().unwrap();
            assert_eq!((matched.offset, matched.length), (offset, 7));
            assert_eq!(matched.replace_with("${word}"), "foo");
        }
    }

    #[test]
    fn whole_word_regex_searches_large_documents_within_the_match_budget() {
        let strategy =
            SearchStrategyFactory::create(r"\bremains_missing\b", false, false, SearchMode::RegEx)
                .unwrap();
        for line in ["identifier value\n", "标识符 मूल्य\n"] {
            let mut text = line.repeat(60_000);
            let offset = text.encode_utf16().count();
            assert!(strategy.find_all(&text, 0, offset).unwrap().is_empty());

            let word = "remains_missing";
            text.push_str(word);
            let results = strategy.find_all(&text, 0, offset + word.len()).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].offset, offset);
            assert_eq!(results[0].length, word.len());
        }

        let boundary =
            SearchStrategyFactory::create(r"\b", false, false, SearchMode::RegEx).unwrap();
        for separator in [" ", "。", "\u{093e}"] {
            let offset = 1_000_000;
            let text = separator.repeat(offset) + "word";
            let results = boundary.find_all(&text, 0, offset + 4).unwrap();
            assert_eq!(results.len(), 2);
            assert_eq!((results[0].offset, results[0].length), (offset, 0));
            assert_eq!((results[1].offset, results[1].length), (offset + 4, 0));
        }
    }

    #[test]
    fn boundary_candidate_retry_preserves_context_and_match_progression() {
        let mut text = "\u{093e}".repeat(1_000_000);
        let word_start = text.chars().count() + 2;
        text.push_str("😀foo");
        let search = SearchStrategyFactory::create(
            r"(?<=😀)(?=\b)(?<word>foo)|\b\z",
            false,
            false,
            SearchMode::RegEx,
        )
        .unwrap();
        let matches = search.find_all(&text, word_start - 1, 4).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!((matches[0].offset, matches[0].length), (word_start, 3));
        assert_eq!(matches[0].replace_with("[${word}]"), "[foo]");
        assert_eq!((matches[1].offset, matches[1].length), (word_start + 3, 0));
        assert_eq!(matches[1].replace_with("[${word}]"), "[]");

        let mut text = "a\u{093e}".repeat(600_000);
        let end = text.chars().count();
        text.push_str("😀");
        let search = SearchStrategyFactory::create(r"\B", false, false, SearchMode::RegEx).unwrap();
        let matches = search.find_all(&text, 0, end + 2).unwrap();
        let offsets: Vec<_> = matches.iter().map(|hit| (hit.offset, hit.length)).collect();
        assert_eq!(offsets, [(end, 0), (end + 2, 0)]);
    }

    #[test]
    fn boundary_retry_requires_a_boundary_on_every_match_start() {
        for pattern in [
            r"x?\b",
            r"(?:\b)?",
            r"\bfoo|bar",
            r"\b|\B",
            r"\G\b",
            r"\bfoo\K",
        ] {
            assert_eq!(StartBoundary::required_by(pattern), None, "{pattern}");
        }
        for pattern in [r"\b", r"(?=\b)(foo)", r"(\b)foo|(?<=x)\b", r"(?:\b){1,2}"] {
            assert_eq!(
                StartBoundary::required_by(pattern),
                Some(StartBoundary::Word),
                "{pattern}"
            );
        }
    }

    #[test]
    fn source_capture_numbers_apply_to_matching_and_replacement() {
        for (pattern, text, replacement, expected) in [
            (
                r"(?<word>a)(b)\1\2",
                "abba",
                "$1/$2/${word}/$+",
                vec!["b/a/a/a"],
            ),
            (
                r"(?n)(a)(?'word'b)(?-n:(c))\1",
                "abcc",
                "$1/$2/${word}",
                vec!["c/b/b"],
            ),
            (
                r"(?<named>a)?(b)(?(2)c|d)",
                "abc bd",
                "[$1][$2][$+]",
                vec!["[b][a][a]", "[b][][]"],
            ),
            (
                r"(?<x>a)(?<7>b)(c)\7",
                "abcb",
                "[$1][$2][$7][$+][${x}]",
                vec!["[c][a][b][b][a]"],
            ),
            (
                "(a)(b)?",
                "a",
                "[$+] $2::$1::$00::$02::$10",
                vec!["[] ::a::a::::$10"],
            ),
            ("a", "a", "[$+]", vec!["[a]"]),
            (
                "(a)",
                "a",
                "${+1} ${2$1} ${not $1",
                vec!["${+1} ${2a} ${not a"],
            ),
        ] {
            let strategy = SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                .unwrap_or_else(|error| panic!("{pattern}: {error}"));
            assert_eq!(strategy.pattern(), pattern);
            let actual: Vec<_> = strategy
                .find_all(text, 0, text.len())
                .unwrap()
                .iter()
                .map(|hit| hit.try_replace_with(replacement).unwrap())
                .collect();
            assert_eq!(actual, expected, "{pattern}");
        }
        let sparse =
            SearchStrategyFactory::create(r"(?<2147483647>a)", false, false, SearchMode::RegEx)
                .unwrap();
        let hit = sparse.find_next("a", 0, 1).unwrap().unwrap();
        assert_eq!(hit.capture(1), None);
        assert_eq!(hit.capture(2_147_483_647), Some("a"));
        assert_eq!(hit.try_replace_with("${2147483647}").unwrap(), "a");
    }

    #[test]
    fn legacy_classes_and_numeric_escapes_work_through_find_and_replace() {
        let pattern = r"(?<10>[a-z-[aeiou]])\040\10";
        let source = "😀 b b e e c d";
        let units = source.encode_utf16().count();
        let search =
            SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx).unwrap();
        let matches = search.find_all(source, 0, units).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!((matches[0].offset, matches[0].length), (3, 3));
        assert_eq!(matches[0].capture(10), Some("b"));
        assert_eq!(search.pattern(), pattern);

        let document = TextDocument::from_text(source);
        assert_eq!(search.replace_all(&document, 0, units, "[$10]").unwrap(), 1);
        assert_eq!(document.text(), "😀 [b] e e c d");
        document.undo().unwrap();
        assert_eq!(document.text(), source);
    }

    #[test]
    fn supplementary_escape_pairs_keep_source_ranges_and_case() {
        let source = "x😀😀y🧪";
        for (pattern, expected) in [
            (r"\uD83D\uDE00", &[(1, 2), (3, 2)][..]),
            (r"\uD83D(?#ignored)\uDE00", &[(1, 2), (3, 2)][..]),
            ("(?x)\\uD83D # ignored\n \\uDE00", &[(1, 2), (3, 2)][..]),
            (r"(?:\uD83D\uDE00)+", &[(1, 4)][..]),
            (r"(?<face>\uD83D\uDE00)\k<face>", &[(1, 4)][..]),
            (r"\uD83e\uddea", &[(6, 2)][..]),
        ] {
            let search = SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                .unwrap_or_else(|error| panic!("{pattern}: {error}"));
            let hits = search.find_all(source, 0, 8).unwrap();
            let ranges: Vec<_> = hits.iter().map(|hit| (hit.offset, hit.length)).collect();
            assert_eq!(ranges, expected, "{pattern}");
            assert_eq!(search.pattern(), pattern);
        }
        let uppercase =
            SearchStrategyFactory::create(r"\uD801\uDC00", true, false, SearchMode::RegEx).unwrap();
        let hits = uppercase.find_all("𐐀𐐨", 0, 4).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].offset, hits[0].length), (0, 2));

        for pattern in [r"\uD83D\uDE00+", r"\uD83D\uDE00(?# ignored){1,2}"] {
            let error = SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx)
                .unwrap_err();
            assert!(error.to_string().contains("targets its low surrogate"));
        }
        let literal = r"\uD83D\uDE00";
        let search = SearchStrategyFactory::create(literal, false, false, SearchMode::Normal).unwrap();
        let hit = search.find_next(literal, 0, literal.len()).unwrap().unwrap();
        assert_eq!(hit.matched_text(), literal);
    }

    #[test]
    fn named_unicode_blocks_match_ranges_instead_of_scripts_in_find_and_replace() {
        let pattern = r"(?<block>\p{IsGreek}+)";
        let source = "😀 A \u{037e}Ω ἀ Z";
        let units = source.encode_utf16().count();
        let search =
            SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx).unwrap();
        let hits = search.find_all(source, 0, units).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].offset, hits[0].length), (5, 2));
        assert_eq!(hits[0].named_capture("block"), Some("\u{037e}Ω"));
        assert_eq!(search.pattern(), pattern);

        let document = TextDocument::from_text(source);
        assert_eq!(search.replace_all(&document, 0, units, "[${block}]").unwrap(), 1);
        assert_eq!(document.text(), "😀 A [\u{037e}Ω] ἀ Z");
        document.undo().unwrap();
        assert_eq!(document.text(), source);
    }

    #[test]
    fn extended_whitespace_keeps_non_adjacent_count_text_literal_in_find_and_replace() {
        let pattern = "(?x)(?<target> a{1, 3} )\u{c}\\k<target>";
        let source = "😀 a{1,3}a{1,3} aaa";
        let units = source.encode_utf16().count();
        let search =
            SearchStrategyFactory::create(pattern, false, false, SearchMode::RegEx).unwrap();
        let hits = search.find_all(source, 0, units).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].offset, hits[0].length), (3, 12));
        assert_eq!(hits[0].named_capture("target"), Some("a{1,3}"));

        let document = TextDocument::from_text(source);
        assert_eq!(
            search.replace_all(&document, 0, units, "[${target}]").unwrap(),
            1
        );
        assert_eq!(document.text(), "😀 [a{1,3}] aaa");
        document.undo().unwrap();
        assert_eq!(document.text(), source);
    }

    #[test]
    fn invalid_replacement_and_reentrant_updates_leave_the_match_batch_unapplied() {
        let document = TextDocument::from_text("a a");
        let strategy =
            SearchStrategyFactory::create("(a)", false, false, SearchMode::RegEx).unwrap();
        for invalid in ["$2147483648", "${2147483648}", "${2147483648abc}"] {
            assert!(matches!(
                strategy.replace_all(&document, 0, 3, invalid),
                Err(SearchError::Replacement(_))
            ));
            assert_eq!(document.text(), "a a");
            assert!(!document.can_undo());
        }
        let changed = document.clone();
        let _callback = document.update_started(move || changed.insert(0, "prefix ").unwrap());
        assert!(matches!(
            strategy.replace_all(&document, 0, 3, "b"),
            Err(SearchError::Document(DocumentError::ReentrantChange))
        ));
        assert_eq!(document.text(), "prefix a a");
    }
}
