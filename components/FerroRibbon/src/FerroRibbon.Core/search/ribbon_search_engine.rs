//! Source Core indexing/ranking. Native indexing, UI navigation and invocation
//! live in Ribbon.Search.cs and are separately unported.
use super::search_data;
use crate::collections::RibbonFixedItems;
use crate::collections::RibbonItems;
use crate::{
    RibbonString,
    collections::{RibbonArray, RibbonList, RibbonReadOnlyList},
    model::{RibbonItemsSource, RibbonObject},
    text::{empty, letter_or_digit, whitespace},
};
use std::{cell::Cell, cell::RefCell, cmp::Ordering, rc::Rc};

pub type RibbonSearchStrings = RibbonReadOnlyList<Option<RibbonString>>;
#[derive(Clone)]
pub struct RibbonSearchEntryOptions {
    pub path: Option<RibbonString>,
    pub description: Option<RibbonString>,
    pub keywords: Option<RibbonSearchStrings>,
    pub shortcut: Option<RibbonString>,
    pub target: Option<RibbonObject>,
    pub is_enabled: bool,
}
impl Default for RibbonSearchEntryOptions {
    fn default() -> Self {
        Self {
            path: None,
            description: None,
            keywords: None,
            shortcut: None,
            target: None,
            is_enabled: true,
        }
    }
}
pub type RibbonSearchEntryParts = (
    Option<RibbonString>,
    Option<RibbonString>,
    Option<RibbonString>,
    Option<RibbonString>,
    Option<RibbonSearchStrings>,
    Option<RibbonString>,
    Option<RibbonObject>,
    bool,
);
#[derive(Clone)]
pub struct RibbonSearchEntry {
    id: Option<RibbonString>,
    label: Option<RibbonString>,
    options: RibbonSearchEntryOptions,
}
impl RibbonSearchEntry {
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, RibbonSearchEntryOptions::default())
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        options: RibbonSearchEntryOptions,
    ) -> Rc<Self> {
        Rc::new(Self { id, label, options })
    }
    pub fn id(&self) -> Option<RibbonString> {
        self.id.clone()
    }
    pub fn label(&self) -> Option<RibbonString> {
        self.label.clone()
    }
    pub fn path(&self) -> Option<RibbonString> {
        self.options.path.clone()
    }
    pub fn description(&self) -> Option<RibbonString> {
        self.options.description.clone()
    }
    pub fn keywords(&self) -> Option<RibbonSearchStrings> {
        self.options.keywords.clone()
    }
    pub fn shortcut(&self) -> Option<RibbonString> {
        self.options.shortcut.clone()
    }
    pub fn target(&self) -> Option<RibbonObject> {
        self.options.target.clone()
    }
    pub fn is_enabled(&self) -> bool {
        self.options.is_enabled
    }
    pub fn deconstruct(&self) -> RibbonSearchEntryParts {
        (
            self.id(),
            self.label(),
            self.path(),
            self.description(),
            self.keywords(),
            self.shortcut(),
            self.target(),
            self.is_enabled(),
        )
    }
    /// Record clone: new record identity, shared immutable/reference-valued fields.
    pub fn clone_record(&self) -> Rc<Self> {
        Rc::new(self.clone())
    }
}
impl PartialEq for RibbonSearchEntry {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || (self.id == other.id
                && self.label == other.label
                && self.options.path == other.options.path
                && self.options.description == other.options.description
                && self.options.keywords == other.options.keywords
                && self.options.shortcut == other.options.shortcut
                && self.options.target == other.options.target
                && self.options.is_enabled == other.options.is_enabled)
    }
}
pub struct RibbonSearchResult {
    entry: Option<Rc<RibbonSearchEntry>>,
    score: f64,
}
impl RibbonSearchResult {
    pub fn new(entry: Option<Rc<RibbonSearchEntry>>, score: f64) -> Rc<Self> {
        Rc::new(Self { entry, score })
    }
    pub fn entry(&self) -> Option<Rc<RibbonSearchEntry>> {
        self.entry.clone()
    }
    pub fn score(&self) -> f64 {
        self.score
    }
    pub fn clone_record(&self) -> Rc<Self> {
        Self::new(self.entry(), self.score)
    }
    pub fn deconstruct(&self) -> (Option<Rc<RibbonSearchEntry>>, f64) {
        (self.entry(), self.score)
    }
}
impl PartialEq for RibbonSearchResult {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || (self.entry == other.entry
                && (self.score == other.score || (self.score.is_nan() && other.score.is_nan())))
    }
}
pub type RibbonSearchResults = RibbonArray<Option<Rc<RibbonSearchResult>>>;
pub type RibbonSearchLabelComparer = dyn Fn(&RibbonString, &RibbonString) -> Ordering;
pub type RibbonSearchLabelComparerProvider = dyn Fn() -> Rc<RibbonSearchLabelComparer>;
ferroui_system::ferro_thread_local! {
    static LABEL_COMPARER:RefCell<Option<Rc<RibbonSearchLabelComparerProvider>>> = RefCell::default();
}
pub struct RibbonSearchEngine {
    entries: RibbonList<Option<Rc<RibbonSearchEntry>>>,
    recent: RibbonList<Option<RibbonString>>,
    max_recent: Cell<i32>,
}
impl PartialEq for RibbonSearchEngine {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for RibbonSearchEngine {}
impl RibbonSearchEngine {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            entries: RibbonItems::new(),
            recent: RibbonItems::new(),
            max_recent: Cell::new(8),
        })
    }
    pub fn max_recent(&self) -> i32 {
        self.max_recent.get()
    }
    pub fn set_max_recent(&self, value: i32) {
        self.max_recent.set(value);
    }
    pub fn entries(&self) -> RibbonList<Option<Rc<RibbonSearchEntry>>> {
        self.entries.clone()
    }
    pub fn recent(&self) -> RibbonList<Option<RibbonString>> {
        self.recent.clone()
    }
    /// Composition seam for the source StringComparer.CurrentCultureIgnoreCase. A provider is
    /// captured after scoring, once per nonempty-query Search call. Without one, a required
    /// culture tie fails explicitly; ordinal substitution would produce incorrect results.
    pub fn set_label_comparer_provider(
        provider: Option<Rc<RibbonSearchLabelComparerProvider>>,
    ) -> Option<Rc<RibbonSearchLabelComparerProvider>> {
        LABEL_COMPARER.with(|value| value.replace(provider))
    }
    /// Replace indexed entries from an ordinary Rust iterator. The first entry
    /// for each ID wins, as in the source algorithm. No runtime enumerable is
    /// required by native control consumers.
    pub fn replace_entries(&self, entries: impl IntoIterator<Item = Rc<RibbonSearchEntry>>) {
        self.entries.clear();
        self.index_entries(entries.into_iter().map(Some));
    }
    pub fn set_entries(&self, entries: Option<RibbonItemsSource<Option<Rc<RibbonSearchEntry>>>>) {
        self.entries.clear();
        self.index_entries(entries.expect("NullReference: entries"));
    }
    fn index_entries(&self, entries: impl IntoIterator<Item = Option<Rc<RibbonSearchEntry>>>) {
        for entry in entries {
            if self.entries.iter().all(|e| {
                e.as_ref().expect("NullReference: indexed entry").id
                    != entry.as_ref().expect("NullReference: entry").id
            }) {
                self.entries.add(entry);
            }
        }
    }
    pub fn add(&self, entry: Option<Rc<RibbonSearchEntry>>) {
        let entry = entry.expect("ArgumentNull: entry");
        let index = self
            .entries
            .iter()
            .position(|e| e.as_ref().expect("NullReference: indexed entry").id == entry.id);
        if let Some(index) = index {
            self.entries.set(index, Some(entry));
        } else {
            self.entries.add(Some(entry));
        }
    }
    pub fn mark_used(&self, id: Option<RibbonString>) {
        if id.as_ref().is_none_or(RibbonString::is_empty) {
            return;
        }
        self.recent.remove(&id);
        self.recent.insert(0, id);
        self.trim_recent();
    }
    pub fn set_recent(&self, ids: Option<RibbonItemsSource<Option<RibbonString>>>) {
        let ids = ids.expect("ArgumentNull: ids");
        self.recent.clear();
        for id in ids {
            if id.as_ref().is_some_and(|v| !v.is_empty()) && !self.recent.iter().any(|v| v == id) {
                self.recent.add(id);
            }
        }
        self.trim_recent();
    }
    fn trim_recent(&self) {
        while self.recent.len() > self.max_recent().max(0) as usize {
            self.recent.remove_at(self.recent.len() - 1);
        }
    }
    pub fn get_recent_entries(self: Rc<Self>) -> RibbonItemsSource<Rc<RibbonSearchEntry>> {
        RibbonItemsSource::deferred(move || {
            let owner = self.clone();
            Box::new(self.recent.iter().filter_map(move |id| {
                owner
                    .entries
                    .iter()
                    .find(|entry| entry.as_ref().expect("NullReference: indexed entry").id == id)
                    .flatten()
            }))
        })
    }
    pub fn search_default(self: &Rc<Self>, query: Option<&RibbonString>) -> RibbonSearchResults {
        self.search(query, 12)
    }
    pub fn search(
        self: &Rc<Self>,
        query: Option<&RibbonString>,
        max_results: i32,
    ) -> RibbonSearchResults {
        let normalized = Self::normalize(query);
        let terms = normalized
            .as_utf16()
            .split(|&c| c == 32)
            .filter(|s| !s.is_empty())
            .map(|s| Some(RibbonString::from(s)))
            .collect::<Vec<_>>();
        if terms.is_empty() {
            return RibbonFixedItems::new(
                self.clone()
                    .get_recent_entries()
                    .iter()
                    .enumerate()
                    .take(max_results.max(0) as usize)
                    .map(|(i, e)| {
                        Some(RibbonSearchResult::new(
                            Some(e),
                            (100_i32.wrapping_sub(i as i32)) as f64,
                        ))
                    })
                    .collect(),
            );
        }
        let terms = RibbonFixedItems::new(terms) as RibbonReadOnlyList<_>;
        let mut results = vec![];
        for entry in self.entries.iter() {
            let mut score = Self::score_entry(entry.as_deref(), Some(&terms));
            if score <= 0. {
                continue;
            }
            let entry = entry.expect("NullReference: entry");
            if let Some(index) = self.recent.iter().position(|id| id == entry.id) {
                score += 5. - (index as f64 * 0.5).min(4.);
            }
            if !entry.is_enabled() {
                score *= 0.5;
            }
            results.push(RibbonSearchResult::new(Some(entry), score));
        }
        let provider = LABEL_COMPARER.with(|v| v.borrow().clone());
        let comparer = provider.map(|provider| provider());
        // Source Take(0) skips ordered-key evaluation, but not the scoring above.
        if max_results <= 0 {
            return RibbonFixedItems::new(vec![]);
        }
        let mut keys = results
            .into_iter()
            .map(|r| {
                let label = r
                    .entry
                    .as_ref()
                    .unwrap()
                    .label()
                    .expect("NullReference: Label.Length");
                (r, label)
            })
            .collect::<Vec<_>>();
        keys.sort_by(|(a,al),(b,bl)| {
            b.score.partial_cmp(&a.score).unwrap_or_else(||match (a.score.is_nan(),b.score.is_nan()){(true,false)=>Ordering::Greater,(false,true)=>Ordering::Less,_=>Ordering::Equal})
            .then_with(||al.len().cmp(&bl.len()))
            .then_with(||if al==bl { Ordering::Equal } else { comparer.as_ref().expect("Unavailable: source current-culture search collation; install a faithful comparer provider")(al,bl) })
        });
        RibbonFixedItems::new(
            keys.into_iter()
                .take(max_results as usize)
                .map(|(r, _)| Some(r))
                .collect(),
        )
    }
    pub fn score_entry(
        entry: Option<&RibbonSearchEntry>,
        terms: Option<&RibbonSearchStrings>,
    ) -> f64 {
        let entry = entry.expect("NullReference: entry.Label");
        let label = Self::normalize(entry.label.as_ref());
        let keywords = entry
            .options
            .keywords
            .as_ref()
            .map(|v| {
                v.iter()
                    .map(|k| Self::normalize(k.as_ref()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let path = Self::normalize(entry.options.path.as_ref());
        let description = Self::normalize(entry.options.description.as_ref());
        let terms = terms.expect("NullReference: terms.Count");
        if terms.count() == 0 {
            return 0.;
        }
        let mut total = 0.;
        for term in terms.iter() {
            let label_score = score_text(&label, term.as_ref(), 1.);
            let keyword_score = keywords
                .iter()
                .map(|k| score_text(k, term.as_ref(), 0.8))
                .reduce(f64::max)
                .unwrap_or(0.);
            let best = label_score.max(keyword_score.max(
                score_text(&path, term.as_ref(), 0.45).max(score_text(
                    &description,
                    term.as_ref(),
                    0.35,
                )),
            ));
            if best <= 0. {
                return 0.;
            }
            total += best;
        }
        let mut joined = vec![];
        for (i, term) in terms.iter().enumerate() {
            if i > 0 {
                joined.push(32);
            }
            if let Some(term) = term {
                joined.extend_from_slice(term.as_utf16());
            }
        }
        if label.as_utf16() == joined {
            total += 50.;
        }
        let count = terms.count();
        if total == 0. && count == 0 {
            return f64::NAN;
        }
        total / f64::from(count)
    }
    pub fn normalize(text: Option<&RibbonString>) -> RibbonString {
        let Some(text) = text else { return empty() };
        if text.as_utf16().iter().all(|&c| whitespace(c)) {
            return empty();
        }
        let mut output = vec![];
        for scalar in char::decode_utf16(text.as_utf16().iter().copied()) {
            let scalar = scalar.expect("ArgumentException: invalid UTF16 text") as u32;
            assert!(
                search_data::INVALID.binary_search(&scalar).is_err(),
                "ArgumentException: invalid normalization character"
            );
            let storage;
            let units =
                if let Ok(i) = search_data::PROJECTION.binary_search_by_key(&scalar, |&(c, _)| c) {
                    search_data::PROJECTION[i].1
                } else {
                    storage = basic(scalar);
                    &storage.0[..storage.1]
                };
            for &unit in units {
                if unit != 32 || output.last().is_some_and(|&c| c != 32) {
                    output.push(unit);
                }
            }
        }
        if output.last() == Some(&32) {
            output.pop();
        }
        if output.is_empty() {
            empty()
        } else {
            RibbonString::from_utf16(output)
        }
    }
}
fn basic(scalar: u32) -> ([u16; 2], usize) {
    if scalar > 65535 {
        return ([32, 0], 1);
    }
    let c = scalar as u16;
    let at = search_data::MARKS.partition_point(|&(a, _)| a <= c);
    if at > 0 && c <= search_data::MARKS[at - 1].1 {
        return ([0; 2], 0);
    }
    if !letter_or_digit(c) {
        return ([32, 0], 1);
    }
    let lower = search_data::LOWER
        .binary_search_by_key(&c, |&(c, _)| c)
        .map_or(c, |i| search_data::LOWER[i].1);
    match lower {
        0x142 => ([108, 0], 1),
        0xf8 => ([111, 0], 1),
        0x111 => ([100, 0], 1),
        0x127 => ([104, 0], 1),
        0x131 => ([105, 0], 1),
        0xe6 => ([97, 101], 2),
        0x153 => ([111, 101], 2),
        0xdf => ([115, 115], 2),
        0xfe => ([116, 104], 2),
        _ => ([lower, 0], 1),
    }
}
fn score_text(text: &RibbonString, term: Option<&RibbonString>, weight: f64) -> f64 {
    let text = text.as_utf16();
    if text.is_empty() {
        return 0.;
    }
    let term = term.expect("ArgumentNull: value").as_utf16();
    if text == term {
        return 100. * weight;
    }
    if text.starts_with(term) {
        return 85. * weight;
    }
    let word = std::iter::once(32)
        .chain(term.iter().copied())
        .collect::<Vec<_>>();
    if text.windows(word.len()).any(|w| w == word) {
        return 70. * weight;
    }
    if term.is_empty() || text.windows(term.len()).any(|w| w == term) {
        return 50. * weight;
    }
    let words = text
        .split(|&c| c == 32)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    if term.len() >= 2
        && words.len() >= term.len()
        && words
            .iter()
            .map(|w| w[0])
            .take(term.len())
            .eq(term.iter().copied())
    {
        return 60. * weight;
    }
    if term.len() >= 3 {
        let mut ti = 0;
        let mut gaps = 0;
        let mut last = None;
        for (i, &c) in text.iter().enumerate() {
            if ti >= term.len() {
                break;
            }
            if c == term[ti] {
                if last.is_some_and(|last| i - last > 1) {
                    gaps += 1;
                }
                last = Some(i);
                ti += 1;
            }
        }
        if ti == term.len() && gaps <= 1.max(term.len() / 2) {
            return (30. - gaps as f64 * 5.) * weight;
        }
    }
    0.
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn corpus() -> Value {
        serde_json::from_str(include_str!("../../../tests/fixtures/search.json")).unwrap()
    }
    fn read(v: &Value) -> Option<RibbonString> {
        v.as_array().map(|v| {
            RibbonString::from_utf16(
                v.iter()
                    .map(|v| v.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            )
        })
    }
    #[test]
    fn source_private_text_tiers_and_negative_zero_weights_match_exact_bits() {
        for row in corpus()["scores"]["text"].as_array().unwrap() {
            let text = read(&row["text"]).unwrap();
            let term = read(&row["term"]);
            let actual = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                score_text(&text, term.as_ref(), row["weight"].as_f64().unwrap())
            }));
            assert_eq!(actual.is_err(), !row["error"].is_null(), "{row}");
            if let Ok(score) = actual {
                assert_eq!(
                    format!("{:016X}", score.to_bits()),
                    row["score"].as_str().unwrap(),
                    "{row}"
                );
            }
        }
    }
    #[test]
    fn every_valid_scalar_matches_source_retained_projection_with_independent_category_tables() {
        use std::collections::HashMap;
        let data = corpus();
        let unicode = &data["unicode"];
        let gestures: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/gestures.json")).unwrap();
        let ranges = |value: &Value| {
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    (
                        row[0].as_u64().unwrap() as u32,
                        row[1].as_u64().unwrap() as u32,
                    )
                })
                .collect::<Vec<_>>()
        };
        let letters = ranges(&gestures["letters"]);
        let digits = ranges(&gestures["digits"]);
        let marks = ranges(&unicode["marks"]);
        let lower = unicode["lower"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r[0].as_u64().unwrap() as u32, r[1].as_u64().unwrap() as u16))
            .collect::<HashMap<_, _>>();
        let projected = unicode["projection"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r["scalar"].as_u64().unwrap() as u32,
                    read(&r["units"]).unwrap(),
                )
            })
            .collect::<HashMap<_, _>>();
        let in_ranges = |c: u32, rs: &[(u32, u32)]| {
            let i = rs.partition_point(|&(a, _)| a <= c);
            i > 0 && c <= rs[i - 1].1
        };
        for scalar in 128..=0x10ffff {
            let Some(c) = char::from_u32(scalar) else {
                continue;
            };
            let input = RibbonString::from_utf16(
                std::iter::once(120)
                    .chain(c.encode_utf16(&mut [0; 2]).iter().copied())
                    .chain(std::iter::once(121))
                    .collect::<Vec<_>>(),
            );
            if unicode["errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_u64() == Some(scalar as u64))
            {
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        RibbonSearchEngine::normalize(Some(&input))
                    }))
                    .is_err()
                );
                continue;
            }
            let expected = if let Some(value) = projected.get(&scalar) {
                value.as_utf16().to_vec()
            } else if scalar <= 65535 && in_ranges(scalar, &marks) {
                vec![]
            } else if scalar <= 65535 && (in_ranges(scalar, &letters) || in_ranges(scalar, &digits))
            {
                let lower = lower.get(&scalar).copied().unwrap_or(scalar as u16);
                match lower {
                    0x142 => vec![108],
                    0xf8 => vec![111],
                    0x111 => vec![100],
                    0x127 => vec![104],
                    0x131 => vec![105],
                    0xe6 => vec![97, 101],
                    0x153 => vec![111, 101],
                    0xdf => vec![115, 115],
                    0xfe => vec![116, 104],
                    _ => vec![lower],
                }
            } else {
                vec![32]
            };
            let expected = std::iter::once(120)
                .chain(expected)
                .chain(std::iter::once(121))
                .collect::<Vec<_>>();
            assert_eq!(
                RibbonSearchEngine::normalize(Some(&input)).as_utf16(),
                expected,
                "{scalar:x}"
            );
        }
    }
}
