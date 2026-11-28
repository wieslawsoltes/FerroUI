use super::collation_ignorables;
use crate::document::{DocumentError, SimpleSegment, TextDocument};
use crate::unicode_case::{CaseLocale, is_uppercase_letter};
use crate::utils::pattern_occurrences;
use icu_casemap::CaseMapper;
use icu_normalizer::{ComposingNormalizer, DecomposingNormalizer};
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;

pub trait ICompletionData {
    fn text(&self) -> &str;
    fn description(&self) -> &str;
    fn priority(&self) -> f64 {
        0.0
    }
    fn complete(
        &self,
        document: &TextDocument,
        segment: SimpleSegment,
    ) -> Result<(), DocumentError> {
        document.replace(segment.offset, segment.length, self.text())
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompletionItem {
    pub text: String,
    pub insertion_text: Option<String>,
    pub description: String,
    pub priority: f64,
}
impl CompletionItem {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}
impl ICompletionData for CompletionItem {
    fn text(&self) -> &str {
        &self.text
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn complete(
        &self,
        document: &TextDocument,
        segment: SimpleSegment,
    ) -> Result<(), DocumentError> {
        document.replace(
            segment.offset,
            segment.length,
            self.insertion_text.as_deref().unwrap_or(&self.text),
        )
    }
}

/// Completion filtering and ranking independent of a popup implementation.
/// The list retains item identity across filtering and keyboard navigation.
#[derive(Clone, Debug, Default)]
pub struct CompletionList {
    data: Vec<CompletionItem>,
    visible: Vec<usize>,
    selected: Option<usize>,
    pub is_filtering: bool,
}

/// The text comparisons a host can supply when ranking completion items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionComparison {
    Exact,
    Prefix,
    Substring,
}

impl CompletionList {
    pub fn new(data: Vec<CompletionItem>) -> Self {
        let visible = (0..data.len()).collect();
        let selected = if data.is_empty() { None } else { Some(0) };
        Self {
            data,
            visible,
            selected,
            is_filtering: true,
        }
    }
    pub fn completion_data(&self) -> &[CompletionItem] {
        &self.data
    }
    pub fn current_list(&self) -> impl ExactSizeIterator<Item = &CompletionItem> {
        self.visible.iter().map(|i| &self.data[*i])
    }
    /// Original item indices in display order, for hosts with rich item data.
    pub fn current_indices(&self) -> impl ExactSizeIterator<Item = usize> + '_ {
        self.visible.iter().copied()
    }
    pub fn selected_data_index(&self) -> Option<usize> {
        self.selected.and_then(|index| self.visible.get(index)).copied()
    }
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.selected
            .and_then(|i| self.visible.get(i))
            .map(|i| &self.data[*i])
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }
    pub fn select_index(&mut self, index: usize) {
        self.selected = (!self.visible.is_empty()).then(|| index.min(self.visible.len() - 1));
    }
    pub fn move_selection(&mut self, delta: isize, wrap: bool) {
        if self.visible.is_empty() {
            self.selected = None;
            return;
        }
        let current = self.selected.map_or(-1, |index| index as isize);
        let next = current.saturating_add(delta);
        self.selected = Some(if wrap && self.selected.is_some() {
            next.rem_euclid(self.visible.len() as isize) as usize
        } else {
            next.clamp(0, self.visible.len() as isize - 1) as usize
        });
    }
    pub fn select_item(&mut self, query: &str) {
        self.select_item_for_language(query, "und");
    }

    /// Ranks suggestions with canonical Unicode matching, root collation
    /// ignorables and case folding for the caller's BCP-47 language.
    pub fn select_item_for_language(&mut self, query: &str, language: &str) {
        let turkic = matches!(CaseLocale::from_language_tag(language), CaseLocale::Turkic);
        let normalized_query = normalize_completion_text(query);
        let folded_query = fold_completion_text(&normalized_query, turkic);
        let ascii_query = normalized_query.is_ascii() && (!turkic || !normalized_query.contains('I'));
        let nfd = DecomposingNormalizer::new_nfd();
        let decomposed_query = nfd.normalize(&normalized_query);
        let decomposed_folded_query = nfd.normalize(&folded_query);
        self.select_item_ranked(query, |text, short_query, substring| {
            let normalized = normalize_completion_text(text);
            if ascii_query && text.is_ascii() && (!turkic || !normalized.contains('I')) {
                return match_quality(text, query, short_query, substring, |comparison, ignore_case| {
                    let text = normalized.as_ref();
                    let query = normalized_query.as_ref();
                    match (comparison, ignore_case) {
                        (CompletionComparison::Exact, true) => text.eq_ignore_ascii_case(query),
                        (CompletionComparison::Exact, false) => text == query,
                        (CompletionComparison::Prefix, true) => text.get(..query.len())
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(query)),
                        (CompletionComparison::Prefix, false) => text.starts_with(query),
                        (CompletionComparison::Substring, true) => text.to_ascii_lowercase().contains(folded_query.as_ref()),
                        (CompletionComparison::Substring, false) => text.contains(query),
                    }
                });
            }
            let folded = fold_completion_text(&normalized, turkic);
            match_quality(text, query, short_query, substring, |comparison, ignore_case| {
                let decomposed = if ignore_case {
                    &decomposed_folded_query
                } else {
                    &decomposed_query
                };
                let case_fold = ignore_case.then_some(turkic);
                match comparison {
                    CompletionComparison::Exact => folded == folded_query,
                    CompletionComparison::Prefix => query.is_empty() || collation_prefix(text, decomposed, case_fold),
                    CompletionComparison::Substring => grapheme_substring(text, decomposed, case_fold),
                }
            })
        });
    }

    /// Uses the host's collation for text equality, prefix and substring tests.
    /// Exact ordinal matches and camel-case abbreviations keep their native ranks.
    pub fn select_item_with_comparison(
        &mut self,
        query: &str,
        matches: impl Fn(&str, CompletionComparison, bool) -> bool,
    ) {
        self.select_item_ranked(query, |text, short_query, substring| {
            match_quality(text, query, short_query, substring, |comparison, ignore_case| {
                matches(text, comparison, ignore_case)
            })
        });
    }

    fn select_item_ranked(&mut self, query: &str, rank: impl Fn(&str, bool, bool) -> u8) {
        let previous = self.selected_data_index();
        self.visible.clear();
        if !self.is_filtering && query.is_empty() {
            self.visible.extend(0..self.data.len());
            self.selected = previous;
            return;
        }
        let short_query = query.encode_utf16().take(3).count() <= 2;
        let mut best = None;
        for (index, item) in self.data.iter().enumerate() {
            let quality = rank(&item.text, short_query, self.is_filtering);
            if quality != 0 {
                let rank = (quality, previous == Some(index), item.priority);
                if best.is_none_or(|(_, best_rank)| rank > best_rank) {
                    best = Some((self.visible.len(), rank));
                }
            }
            if quality != 0 || !self.is_filtering {
                self.visible.push(index);
            }
        }
        self.selected = best.map(|(index, _)| index);
    }
}
fn match_quality(
    text: &str,
    query: &str,
    short_query: bool,
    substring: bool,
    matches: impl Fn(CompletionComparison, bool) -> bool,
) -> u8 {
    use CompletionComparison::{Exact, Prefix, Substring};
    if text == query {
        return 8;
    }
    if matches(Exact, true) {
        return 7;
    }
    if matches(Prefix, false) {
        return 6;
    }
    if matches(Prefix, true) {
        return 5;
    }
    if short_query && camel_case_match(text, query) {
        return 4;
    }
    if substring && matches(Substring, false) {
        return 3;
    }
    if substring && matches(Substring, true) {
        return 2;
    }
    u8::from(!short_query && camel_case_match(text, query))
}

fn normalize_completion_text(text: &str) -> Cow<'_, str> {
    // Printable ASCII and ASCII whitespace have no zero-weight characters.
    if text.bytes().all(|byte| matches!(byte, b'\t'..=b'\r' | b' '..=b'~')) {
        return Cow::Borrowed(text);
    }
    // A composed character can contain a zero-weight component. Compose the
    // remaining scalars again so substring matching respects accents.
    let scalars = DecomposingNormalizer::new_nfd()
        .normalize_iter(text.chars())
        .filter(|&ch| !collation_ignorables::contains(ch));
    Cow::Owned(ComposingNormalizer::new_nfc().normalize_iter(scalars).collect())
}

fn collation_prefix(text: &str, query: &str, case_fold: Option<bool>) -> bool {
    let composed = ComposingNormalizer::new_nfc().normalize(text);
    let text = case_fold.map_or_else(|| Cow::Borrowed(composed.as_ref()), |turkic| fold_completion_text(&composed, turkic));
    let nfd = DecomposingNormalizer::new_nfd();
    let mut scalars = nfd.normalize_iter(text.chars());
    for expected in query.chars() {
        if scalars.by_ref().find(|&ch| !collation_ignorables::contains(ch)) != Some(expected) {
            return false;
        }
    }
    // An accent can carry a secondary weight without starting another base.
    // A zero-weight character at the cut still permits a prefix.
    scalars.next().is_none_or(|ch| !collation_ignorables::has_secondary_without_primary(ch))
}

fn grapheme_substring(text: &str, query: &str, case_fold: Option<bool>) -> bool {
    if query.is_empty() {
        return true;
    }
    let nfc = ComposingNormalizer::new_nfc();
    let nfd = DecomposingNormalizer::new_nfd();
    let mut key = String::with_capacity(text.len());
    let mut boundaries = vec![0];
    for grapheme in text.graphemes(true) {
        let composed = nfc.normalize(grapheme);
        let folded = case_fold.map_or_else(|| Cow::Borrowed(composed.as_ref()), |turkic| fold_completion_text(&composed, turkic));
        if grapheme == "\r\n" {
            boundaries.push(key.len() + 1);
        }
        key.extend(nfd.normalize_iter(folded.chars()).filter(|&ch| !collation_ignorables::contains(ch)));
        boundaries.push(key.len());
    }
    query.len() <= key.len() && pattern_occurrences(key.bytes(), query.as_bytes()).any(|start| {
        boundaries.binary_search(&start).is_ok() && boundaries.binary_search(&(start + query.len())).is_ok()
    })
}

fn fold_completion_text(text: &str, turkic: bool) -> Cow<'_, str> {
    if text.is_ascii() && (!turkic || !text.contains('I')) {
        return if text.bytes().any(|byte| byte.is_ascii_uppercase()) {
            Cow::Owned(text.to_ascii_lowercase())
        } else {
            Cow::Borrowed(text)
        };
    }
    let mapper = CaseMapper::new();
    let fold = |character| {
        if turkic { mapper.simple_fold_turkic(character) } else { mapper.simple_fold(character) }
    };
    let Some((start, _)) = text.char_indices().find(|&(_, character)| fold(character) != character) else {
        return Cow::Borrowed(text);
    };
    let mut folded = String::with_capacity(text.len());
    folded.push_str(&text[..start]);
    folded.extend(text[start..].chars().map(fold));
    Cow::Owned(folded)
}

fn camel_case_match(text: &str, query: &str) -> bool {
    let mut characters = text.chars();
    let first = characters.next();
    let capitals = characters.filter(|&character| u16::try_from(u32::from(character)).is_ok_and(is_uppercase_letter));
    let mut initials = first.into_iter().chain(capitals);
    query.chars().all(|letter| {
        // An abbreviation consumes one UTF-16 unit per initial.
        letter.len_utf16() == 1
            && initials.next().is_some_and(|initial| {
                CaseLocale::Invariant.upper(initial) == CaseLocale::Invariant.upper(letter)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranking_and_camelcase_filtering() {
        let mut list = CompletionList::new(
            ["XmlReader", "XR", "Other", "xmlReader"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        list.select_item("XR");
        assert_eq!(
            list.current_list()
                .map(|i| i.text.as_str())
                .collect::<Vec<_>>(),
            vec!["XmlReader", "XR", "xmlReader"]
        );
        assert_eq!(list.selected_item().unwrap().text, "XR");
        list.select_index(0);
        list.move_selection(-1, true);
        assert_eq!(list.selected_item().unwrap().text, "xmlReader");
        list.select_item("missing");
        assert_eq!(list.current_list().len(), 0);
        assert!(list.selected_item().is_none());
    }

    #[test]
    fn filtering_preserves_provider_order_and_prefers_an_equally_good_selection() {
        let mut list = CompletionList::new(vec![
            CompletionItem::new("Range"),
            CompletionItem { priority: 9.0, ..CompletionItem::new("Rate") },
            CompletionItem { priority: 9.0, ..CompletionItem::new("Ratify") },
        ]);
        list.clear_selection();
        list.select_item("R");
        assert_eq!(list.selected_index(), Some(1));
        assert!(list.current_indices().eq([0, 1, 2]));
        list.select_index(0);
        list.select_item("Ra");
        assert_eq!(list.selected_item().unwrap().text, "Range");
        list.select_item("Rat");
        assert_eq!(list.selected_item().unwrap().text, "Rate");
    }

    #[test]
    fn unfiltered_completion_clears_an_unmatched_selection_and_can_still_navigate() {
        let mut list = CompletionList::new(
            ["Read", "Write", "AlphaBetaCodeData"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        list.is_filtering = false;
        list.select_item("Wr");
        assert_eq!(list.selected_index(), Some(1));
        assert!(list.current_indices().eq([0, 1, 2]));
        list.select_item("missing");
        assert!(list.selected_item().is_none());
        assert!(list.current_indices().eq([0, 1, 2]));
        list.move_selection(1, true);
        assert_eq!(list.selected_index(), Some(0));
        list.select_item("");
        assert_eq!(list.selected_index(), Some(0));
        list.select_item("abc");
        assert_eq!(list.selected_index(), Some(2));
    }

    #[test]
    fn exact_and_substring_matches_beat_weaker_prefix_and_camel_case_matches() {
        let mut list = CompletionList::new(
            ["foo_bar", "FOO", "prefOO", "AlphaBetaCodeData", "xabc", "XMLReaderRoot"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        list.select_item("foo");
        assert_eq!(list.selected_item().unwrap().text, "FOO");
        list.select_item("abc");
        assert_eq!(list.selected_item().unwrap().text, "xabc");
        list.select_item("AB");
        assert_eq!(list.selected_item().unwrap().text, "AlphaBetaCodeData");
        list.select_item("XRR");
        assert!(list.selected_item().is_none());
    }

    #[test]
    fn camel_case_uses_bmp_uppercase_letters_as_word_initials() {
        let mut list = CompletionList::new(
            ["a𐐀Beta", "aⅠBeta", "aⒶBeta", "aBeta", "𐐀elongBeta", "🙂SomeText", "aΩBeta", "x𐐀lateBeta"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        for (query, expected) in [
            ("aB", &["a𐐀Beta", "aⅠBeta", "aⒶBeta", "aBeta"][..]),
            ("𐐀B", &["a𐐀Beta"]),
            ("🙂T", &[]),
            ("𐐨elong", &["𐐀elongBeta"]),
            ("aΩb", &["aΩBeta"]),
            ("xB", &["x𐐀lateBeta"]),
            ("x𐐨B", &[]),
            ("x𐐨", &["x𐐀lateBeta"]),
        ] {
            list.select_item(query);
            assert!(list.current_list().map(|item| item.text.as_str()).eq(expected.iter().copied()), "{query}");
        }
    }

    #[test]
    fn portable_comparisons_fold_case_in_the_requested_language() {
        let mut list = CompletionList::new(
            ["Iota", "İota", "ıota", "xIota", "xİota", "ΣFox", "σFox", "ςFox", "xΣFox", "Straße", "STRASSE"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        for (language, query, expected) in [
            ("en-US", "IOT", &["Iota", "xIota"][..]),
            ("tr-TR", "IOT", &["Iota", "ıota", "xIota"]),
            ("tr-TR", "iot", &["İota", "xİota"]),
            ("az-Latn-AZ", "xıot", &["xIota"]),
            ("az-Latn-AZ", "xiot", &["xİota"]),
            ("el-GR", "σf", &["ΣFox", "σFox", "ςFox", "xΣFox"]),
            ("en-US", "straße", &["Straße"]),
            ("en-US", "strasse", &["STRASSE"]),
        ] {
            list.select_item_for_language(query, language);
            assert!(list.current_list().map(|item| item.text.as_str()).eq(expected.iter().copied()), "{language}: {query}");
        }
    }

    #[test]
    fn canonical_comparisons_keep_verbatim_matches_first() {
        let mut list = CompletionList::new(
            ["Café", "Cafe\u{301}", "xCafé", "xCafe\u{301}"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        for (query, selected) in [("caFÉ", "Café"), ("Cafe\u{301}", "Cafe\u{301}"), ("aFe\u{301}", "Cafe\u{301}")] {
            list.select_item_for_language(query, "en-US");
            assert!(list.current_indices().eq(0..4), "{query}");
            assert_eq!(list.selected_item().unwrap().text, selected, "{query}");
        }
    }

    #[test]
    fn completion_skips_only_root_collation_ignorables_in_comparison_keys() {
        let mut list = CompletionList::new(
            [
                "xA\u{ad}BC", "xA\0BC", "xA\u{200d}BC", "xA\u{fe0f}BC", "ABC",
                "xA\tBC", "xA BC", "xA\u{115f}BC", "xÁBC", "A\u{34f}\u{30a}BC",
                "👩\u{200d}🚀Launch", "\u{1d15e}Play",
            ]
            .into_iter()
            .map(CompletionItem::new)
            .collect(),
        );
        list.select_item("ABC");
        assert!(list.current_indices().eq([0, 1, 2, 3, 4, 9]));
        assert_eq!(list.selected_item().unwrap().text, "ABC");

        list.select_item("xA\0BC");
        assert!(list.current_indices().eq(0..4));
        assert_eq!(list.selected_item().unwrap().text, "xA\0BC");

        list.select_item("\u{c5}BC");
        assert!(list.current_indices().eq([9]));
        assert_eq!(list.selected_item().unwrap().text, "A\u{34f}\u{30a}BC");
        list.select_item("👩🚀");
        assert_eq!(list.selected_item().unwrap().text, "👩\u{200d}🚀Launch");
        list.select_item("\u{1d157}Play");
        assert_eq!(list.selected_item().unwrap().text, "\u{1d15e}Play");
    }

    #[test]
    fn portable_prefix_and_substring_checks_respect_distinct_unicode_boundaries() {
        for (source, query, prefix, substring) in [
            ("a\u{338}bc", "a", false, false),
            ("xa\u{338}bc", "a", false, false),
            ("xa\u{338}bc", "\u{338}", false, false),
            ("xa\u{338}Abc", "A", false, true),
            ("xab\u{301}cd", "ab", false, false),
            ("🇺🇸Flag", "🇺", true, false),
            ("x🇺🇸Flag", "🇺", false, false),
            ("x🇺🇺🇺Flag", "🇺", false, true),
            ("x🇺\u{ad}🇸Flag", "🇺", false, true),
            ("👩🏻Dev", "👩", true, false),
            ("x👩🏻Dev", "👩", false, false),
            ("👩\u{200d}🚀Launch", "👩", true, false),
            ("x👩\u{200d}🚀Launch", "🚀", false, false),
            ("x👩\u{200d}🚀Launch", "👩🚀", false, true),
            ("क्षValue", "क", true, false),
            ("xक्षValue", "क", false, false),
            ("A\0\u{30a}", "A", true, true),
            ("xA\0\u{30a}y", "Å", false, true),
            ("xa\u{301}\u{ad}\u{323}b", "a\u{323}\u{301}", false, false),
            ("é\r\nb", "\r", false, true),
            ("abcd", "\u{ad}", true, true),
            ("\u{338}abc", "\u{ad}", false, true),
            ("\u{338}abc", "\u{338}", true, true),
        ] {
            let normalized = normalize_completion_text(query);
            let nfd = DecomposingNormalizer::new_nfd();
            let pattern = nfd.normalize(&normalized);
            assert_eq!(collation_prefix(source, &pattern, None), prefix, "prefix of {source:?}, {query:?}");
            assert_eq!(grapheme_substring(source, &pattern, None), substring, "substring of {source:?}, {query:?}");
        }
    }

    #[test]
    fn completion_keeps_partial_emoji_prefixes_and_filters_interior_matches() {
        let mut list = CompletionList::new(
            ["👩🏻Dev", "x👩🏻Dev", "x👩\u{200d}🚀Launch", "x👩Dev"]
                .into_iter()
                .map(CompletionItem::new)
                .collect(),
        );
        list.select_item("👩");
        assert!(list.current_indices().eq([0, 3]));
        assert_eq!(list.selected_item().unwrap().text, "👩🏻Dev");
    }
}
