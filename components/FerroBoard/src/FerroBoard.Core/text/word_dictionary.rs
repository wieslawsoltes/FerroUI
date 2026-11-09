//! Port of Text/WordDictionary.cs. Trie edges and edit-distance columns use UTF-16 units.
use super::{NormalizationError, TextCulture, TextUtils, Utf16Text, ordinal_ignore_case_equal};
use crate::{
    resources::EmbeddedResources,
    serialization::{JsonBridgeError, StrictJsonValue},
};
use parking_lot::ReentrantMutex;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
};

#[derive(Debug)]
pub enum DictionaryError {
    ArgumentNull(&'static str),
    Normalization(NormalizationError),
    Json(JsonBridgeError),
    EmptyTextParts,
    CompletionIndex,
    CollectionModified,
    Resource(std::io::Error),
    UnsupportedJsonUtf16,
}
impl std::fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgumentNull(name) => write!(f, "missing dictionary argument: {name}"),
            Self::Normalization(error) => write!(f, "dictionary normalization failed: {error}"),
            Self::Json(error) => write!(f, "invalid dictionary JSON: {error}"),
            Self::EmptyTextParts => f.write_str("dictionary text entry has no parts"),
            Self::CompletionIndex => f.write_str("dictionary completion index is out of range"),
            Self::CollectionModified => f.write_str("dictionary changed during iteration"),
            Self::Resource(error) => write!(f, "dictionary resource could not be loaded: {error}"),
            Self::UnsupportedJsonUtf16 => f.write_str("dictionary JSON cannot be encoded as UTF-8"),
        }
    }
}
impl std::error::Error for DictionaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Normalization(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Resource(error) => Some(error),
            _ => None,
        }
    }
}
pub type DictionaryResult<T> = Result<T, DictionaryError>;
impl From<NormalizationError> for DictionaryError {
    fn from(value: NormalizationError) -> Self {
        Self::Normalization(value)
    }
}
#[derive(Clone, Debug)]
pub struct FuzzyMatch {
    pub word: Utf16Text,
    pub frequency: i32,
    pub distance: f64,
}
impl PartialEq for FuzzyMatch {
    fn eq(&self, other: &Self) -> bool {
        self.word == other.word
            && self.frequency == other.frequency
            && (self.distance == other.distance
                || self.distance.is_nan() && other.distance.is_nan())
    }
}
impl Eq for FuzzyMatch {}
struct WordEntry {
    word: Utf16Text,
    frequency: AtomicI32,
}
impl WordEntry {
    fn frequency(&self) -> i32 {
        self.frequency.load(Ordering::Relaxed)
    }
    fn pair(&self) -> (Utf16Text, i32) {
        (self.word.clone(), self.frequency())
    }
}
struct Node {
    unit: u16,
    children: Vec<usize>,
    entries: Vec<Arc<WordEntry>>,
    max_frequency: i32,
}
impl Node {
    fn new(unit: u16) -> Self {
        Self {
            unit,
            children: Vec::new(),
            entries: Vec::new(),
            max_frequency: 0,
        }
    }
}
struct State {
    nodes: Vec<Node>,
    words: HashMap<Utf16Text, Arc<WordEntry>>,
    order: Vec<Arc<WordEntry>>,
}
impl State {
    fn child(&self, node: usize, unit: u16) -> Option<usize> {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .find(|&i| self.nodes[i].unit == unit)
    }
    fn find(&self, key: &Utf16Text) -> Option<usize> {
        let mut node = 0;
        for &c in key.units() {
            node = self.child(node, c)?;
        }
        Some(node)
    }
}
/// Shared, synchronous dictionary. Callbacks execute under a reentrant lock, like the source Monitor.
/// Interior borrows are never retained while invoking extension callbacks.
pub struct WordDictionary {
    language_tag: Option<Utf16Text>,
    gate: ReentrantMutex<RefCell<State>>,
    max_frequency: AtomicI32,
}
impl Default for WordDictionary {
    fn default() -> Self {
        Self::new("en".into())
    }
}
impl WordDictionary {
    pub const TRANSPOSITION_COST: f64 = 0.5;
    pub fn new(language_tag: Utf16Text) -> Self {
        Self::new_nullable(Some(language_tag))
    }
    pub fn new_nullable(language_tag: Option<Utf16Text>) -> Self {
        Self {
            language_tag,
            gate: ReentrantMutex::new(RefCell::new(State {
                nodes: vec![Node::new(0)],
                words: HashMap::new(),
                order: Vec::new(),
            })),
            max_frequency: AtomicI32::new(1),
        }
    }
    pub fn language_tag(&self) -> Option<Utf16Text> {
        self.language_tag.clone()
    }
    pub fn count(&self) -> usize {
        self.gate.lock().borrow().words.len()
    }
    pub fn max_frequency(&self) -> i32 {
        self.max_frequency.load(Ordering::Relaxed)
    }
    pub fn words(&self) -> Vec<Utf16Text> {
        self.gate
            .lock()
            .borrow()
            .order
            .iter()
            .map(|w| w.word.clone())
            .collect()
    }
    pub fn normalize(word: &Utf16Text) -> Result<Utf16Text, NormalizationError> {
        // ASCII has no canonical decompositions or combining marks. Keep one owned result
        // instead of allocating intermediate Unicode normalization buffers for common words.
        if word.units().iter().all(|unit| *unit <= 0x7f) {
            return Ok(Utf16Text::from_units(
                word.units()
                    .iter()
                    .map(|unit| u16::from((*unit as u8).to_ascii_lowercase()))
                    .collect::<Vec<_>>(),
            ));
        }
        TextUtils::remove_diacritics(&TextCulture::to_lower_invariant(word))
    }
    pub fn from_words(
        words: impl IntoIterator<Item = (Utf16Text, i32)>,
        language_tag: Utf16Text,
    ) -> DictionaryResult<Self> {
        let dict = Self::new(language_tag);
        for (word, frequency) in words {
            dict.add(&word, frequency)?;
        }
        Ok(dict)
    }
    /// Strict JsonDocument semantics, including duplicates and delayed validation of string escapes.
    pub fn from_json(json: &str, language_tag: Utf16Text) -> DictionaryResult<Self> {
        Self::from_json_nullable(json, Some(language_tag))
    }
    pub fn from_json_nullable(
        json: &str,
        language_tag: Option<Utf16Text>,
    ) -> DictionaryResult<Self> {
        let value = StrictJsonValue::parse(json).map_err(DictionaryError::Json)?;
        let properties = value.object().map_err(DictionaryError::Json)?;
        let dict = Self::new_nullable(language_tag);
        for (word, value) in properties {
            if matches!(value, StrictJsonValue::Number(_)) {
                let word =
                    StrictJsonValue::valid_string(&word.text).map_err(DictionaryError::Json)?;
                let frequency = value.int32().map_err(DictionaryError::Json)?;
                dict.add(&word, frequency)?;
            }
        }
        Ok(dict)
    }
    pub fn from_text(text: &Utf16Text, language_tag: Utf16Text) -> DictionaryResult<Self> {
        Self::from_text_nullable(text, Some(language_tag))
    }
    pub fn from_text_nullable(
        text: &Utf16Text,
        language_tag: Option<Utf16Text>,
    ) -> DictionaryResult<Self> {
        let dict = Self::new_nullable(language_tag);
        let mut rank = 0i32;
        for raw in text.units().split(|&c| c == 10) {
            let line = trim(raw);
            if line.is_empty() || line[0] == 35 {
                continue;
            }
            let parts: Vec<_> = line
                .split(|&c| matches!(c, 32 | 9 | 44))
                .filter(|s| !s.is_empty())
                .collect();
            let frequency = parts
                .get(1)
                .and_then(|p| parse_integer(p))
                .unwrap_or_else(|| 1.max(255 - rank / 200));
            let word = parts.first().ok_or(DictionaryError::EmptyTextParts)?;
            dict.add(&Utf16Text::from_units(word.to_vec()), frequency)?;
            rank = rank.wrapping_add(1);
        }
        Ok(dict)
    }
    pub fn load_bundled_english() -> DictionaryResult<Self> {
        let json = EmbeddedResources::read_text("dict/en.json")
            .map_err(DictionaryError::Resource)?
            .unwrap_or_else(|| "{}".into());
        Self::from_json(
            &json
                .to_utf8()
                .map_err(|_| DictionaryError::UnsupportedJsonUtf16)?,
            "en".into(),
        )
    }
    pub fn add(&self, word: &Utf16Text, frequency: i32) -> DictionaryResult<()> {
        let word = Utf16Text::from_units(trim(word.units()).to_vec());
        if word.is_empty() {
            return Ok(());
        }
        let key = Self::normalize(&word)?;
        let guard = self.gate.lock();
        let mut state = guard.borrow_mut();
        let mut node = 0;
        state.nodes[node].max_frequency = state.nodes[node].max_frequency.max(frequency);
        for &unit in key.units() {
            node = if let Some(child) = state.child(node, unit) {
                child
            } else {
                let child = state.nodes.len();
                state.nodes.push(Node::new(unit));
                state.nodes[node].children.push(child);
                child
            };
            state.nodes[node].max_frequency = state.nodes[node].max_frequency.max(frequency);
        }
        if let Some(entry) = state.words.get(&word) {
            entry.frequency.fetch_max(frequency, Ordering::Relaxed);
        } else {
            let entry = Arc::new(WordEntry {
                word: word.clone(),
                frequency: AtomicI32::new(frequency),
            });
            state.words.insert(word, entry.clone());
            state.order.push(entry.clone());
            state.nodes[node].entries.push(entry);
        }
        self.max_frequency.fetch_max(frequency, Ordering::Relaxed);
        Ok(())
    }
    pub fn contains(&self, word: &Utf16Text) -> DictionaryResult<bool> {
        Ok(self.get_frequency(word)? > 0)
    }
    pub fn contains_nullable(&self, word: Option<&Utf16Text>) -> DictionaryResult<bool> {
        Ok(self.get_frequency_nullable(word)? > 0)
    }
    pub fn get_frequency(&self, word: &Utf16Text) -> DictionaryResult<i32> {
        self.get_frequency_nullable(Some(word))
    }
    pub fn get_frequency_nullable(&self, word: Option<&Utf16Text>) -> DictionaryResult<i32> {
        let guard = self.gate.lock();
        let word = word.ok_or(DictionaryError::ArgumentNull("key"))?;
        let state = guard.borrow();
        if let Some(entry) = state.words.get(word) {
            return Ok(1.max(entry.frequency()));
        }
        let key = Self::normalize(word)?;
        Ok(state
            .find(&key)
            .and_then(|node| {
                state.nodes[node]
                    .entries
                    .iter()
                    .find(|e| ordinal_ignore_case_equal(&e.word, word))
            })
            .map_or(0, |e| 1.max(e.frequency())))
    }
    pub fn get_variants(&self, word: &Utf16Text) -> DictionaryResult<Vec<(Utf16Text, i32)>> {
        let guard = self.gate.lock();
        let key = Self::normalize(word)?;
        let state = guard.borrow();
        Ok(state.find(&key).map_or_else(Vec::new, |i| {
            state.nodes[i].entries.iter().map(|e| e.pair()).collect()
        }))
    }
    pub fn complete(
        &self,
        prefix: &Utf16Text,
        max: i32,
    ) -> DictionaryResult<Vec<(Utf16Text, i32)>> {
        let key = Self::normalize(prefix)?;
        let mut best = Vec::new();
        {
            let guard = self.gate.lock();
            let state = guard.borrow();
            let Some(node) = state.find(&key) else {
                return Ok(Vec::new());
            };
            collect(&state, node, &mut best, max, 0)?;
        }
        Ok(best.iter().map(|e| e.pair()).collect())
    }
    pub fn find_similar(
        &self,
        word: &Utf16Text,
        max_distance: f64,
        max_results: i32,
        substitution_cost: Option<&dyn Fn(u16, u16) -> f64>,
    ) -> DictionaryResult<Vec<FuzzyMatch>> {
        let key = Self::normalize(word)?;
        if key.is_empty() {
            return Ok(Vec::new());
        }
        let previous_row: Vec<_> = (0..=key.len()).map(|i| i as f64).collect();
        let mut search = Search {
            key: key.units(),
            max_distance,
            substitution_cost,
            results: Vec::new(),
            spare_rows: Vec::new(),
        };
        {
            let guard = self.gate.lock();
            search.children(&guard, 0, 0, &previous_row, None, 1)?;
        }
        search.results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or_else(|| a.distance.is_nan().cmp(&b.distance.is_nan()).reverse())
                .then_with(|| b.frequency.cmp(&a.frequency))
        });
        search.results.truncate(max_results.max(0) as usize);
        Ok(search.results)
    }
    // Internal source entry point, currently unused by other upstream classes.
    #[allow(
        dead_code,
        reason = "Upstream internal entry point has no callers at the pinned revision"
    )]
    pub(crate) fn rebuild_max_frequencies(&self) {
        fn rebuild(state: &mut State, node: usize) -> i32 {
            let mut max = state.nodes[node]
                .entries
                .iter()
                .map(|e| e.frequency())
                .max()
                .unwrap_or(0);
            for i in 0..state.nodes[node].children.len() {
                max = max.max(rebuild(state, state.nodes[node].children[i]));
            }
            state.nodes[node].max_frequency = state.nodes[node].max_frequency.max(max);
            max
        }
        rebuild(&mut self.gate.lock().borrow_mut(), 0);
    }
}
fn collect(
    state: &State,
    node: usize,
    best: &mut Vec<Arc<WordEntry>>,
    max: i32,
    depth: usize,
) -> DictionaryResult<()> {
    for entry in &state.nodes[node].entries {
        let mut i = best.len();
        while i > 0 && best[i - 1].frequency() < entry.frequency() {
            i -= 1;
        }
        if (i as i64) >= max as i64 {
            continue;
        }
        best.insert(i, entry.clone());
        if best.len() as i64 > max as i64 {
            best.pop();
        }
    }
    if depth > 40 {
        return Ok(());
    }
    for &child in &state.nodes[node].children {
        if best.len() as i64 >= max as i64
            && state.nodes[child].max_frequency
                <= best
                    .last()
                    .ok_or(DictionaryError::CompletionIndex)?
                    .frequency()
        {
            continue;
        }
        collect(state, child, best, max, depth + 1)?;
    }
    Ok(())
}
struct Search<'a> {
    key: &'a [u16],
    max_distance: f64,
    substitution_cost: Option<&'a dyn Fn(u16, u16) -> f64>,
    results: Vec<FuzzyMatch>,
    // Scratch rows belong to this invocation. Ancestor rows stay live until recursion returns;
    // completed branches lend their storage to siblings without caching callback results.
    spare_rows: Vec<Vec<f64>>,
}
impl Search<'_> {
    fn children(
        &mut self,
        gate: &RefCell<State>,
        node: usize,
        previous_char: u16,
        previous_row: &[f64],
        pre_previous_row: Option<&[f64]>,
        depth: i32,
    ) -> DictionaryResult<()> {
        // Child lists only grow. Their lengths are therefore an exact version token.
        let version = gate.borrow().nodes[node].children.len();
        let mut index = 0;
        loop {
            let child = {
                let state = gate.borrow();
                let children = &state.nodes[node].children;
                if children.len() != version {
                    return Err(DictionaryError::CollectionModified);
                }
                children.get(index).copied()
            };
            let Some(child) = child else { break };
            index += 1;
            self.node(
                gate,
                child,
                previous_char,
                previous_row,
                pre_previous_row,
                depth,
            )?;
        }
        Ok(())
    }
    fn node(
        &mut self,
        gate: &RefCell<State>,
        node: usize,
        previous_char: u16,
        previous_row: &[f64],
        pre_previous_row: Option<&[f64]>,
        depth: i32,
    ) -> DictionaryResult<()> {
        let c = gate.borrow().nodes[node].unit;
        let mut current_row = self
            .spare_rows
            .pop()
            .unwrap_or_else(|| vec![0.0; self.key.len() + 1]);
        current_row[0] = previous_row[0] + 1.0;
        let mut row_min = current_row[0];
        for i in 1..current_row.len() {
            let insert_cost = current_row[i - 1] + 1.0;
            let delete_cost = previous_row[i] + 1.0;
            let replace = if self.key[i - 1] == c {
                0.0
            } else {
                self.substitution_cost
                    .map_or(1.0, |f| f(self.key[i - 1], c))
            };
            let mut value = dotnet_min(
                dotnet_min(insert_cost, delete_cost),
                previous_row[i - 1] + replace,
            );
            if let Some(row) = pre_previous_row
                && i > 1
                && self.key[i - 1] == previous_char
                && self.key[i - 2] == c
            {
                value = dotnet_min(value, row[i - 2] + WordDictionary::TRANSPOSITION_COST);
            }
            current_row[i] = value;
            row_min = dotnet_min(row_min, value);
        }
        if current_row[self.key.len()] <= self.max_distance {
            let state = gate.borrow();
            for entry in &state.nodes[node].entries {
                self.results.push(FuzzyMatch {
                    word: entry.word.clone(),
                    frequency: entry.frequency(),
                    distance: current_row[self.key.len()],
                });
            }
        }
        // .NET 10 floating-to-int conversion saturates; unchecked integer addition wraps.
        let limit = (self.key.len() as i32)
            .wrapping_add(self.max_distance.ceil() as i32)
            .wrapping_add(1);
        if row_min <= self.max_distance && depth < limit {
            self.children(gate, node, c, &current_row, Some(previous_row), depth + 1)?;
        }
        self.spare_rows.push(current_row);
        Ok(())
    }
}
fn dotnet_min(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() || a < b { a } else { b }
    } else if a.is_sign_negative() {
        a
    } else {
        b
    }
}
fn trim(mut text: &[u16]) -> &[u16] {
    while text.first().is_some_and(|&c| TextUtils::is_white_space(c)) {
        text = &text[1..];
    }
    while text.last().is_some_and(|&c| TextUtils::is_white_space(c)) {
        text = &text[..text.len() - 1];
    }
    text
}
fn parse_integer(mut text: &[u16]) -> Option<i32> {
    // NumberStyles.Integer uses ASCII whitespace, accepts a terminal NUL run, and no group separators.
    while text.first().is_some_and(|c| matches!(c, 9..=13 | 32)) {
        text = &text[1..];
    }
    while text.last() == Some(&0) {
        text = &text[..text.len() - 1];
    }
    while text.last().is_some_and(|c| matches!(c, 9..=13 | 32)) {
        text = &text[..text.len() - 1];
    }
    if text.iter().any(|&c| c > 127) {
        return None;
    }
    let bytes: Vec<_> = text.iter().map(|&c| c as u8).collect();
    std::str::from_utf8(&bytes).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_internal_rebuild_keeps_historically_higher_node_frequencies() {
        let dict = WordDictionary::from_words(
            [("a".into(), -10), ("ab".into(), 5), ("b".into(), -20)],
            "en".into(),
        )
        .unwrap();
        {
            let guard = dict.gate.lock();
            let state = guard.borrow();
            state.words[&Utf16Text::from("ab")]
                .frequency
                .store(2, Ordering::Relaxed);
        }
        dict.rebuild_max_frequencies();
        let guard = dict.gate.lock();
        let state = guard.borrow();
        assert_eq!(state.nodes[0].max_frequency, 5);
        assert_eq!(
            state.nodes[state.find(&"a".into()).unwrap()].max_frequency,
            5
        );
        assert_eq!(
            state.nodes[state.find(&"b".into()).unwrap()].max_frequency,
            0
        );
        assert_eq!(dict.max_frequency(), 5);
    }
}

impl PartialEq for WordDictionary {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod nullable_lock_tests {
    use super::*;
    #[test]
    fn null_lookup_waits_for_the_original_gate_before_failing() {
        use std::{sync::mpsc, thread, time::Duration};
        for contains in [false, true] {
            let dictionary = Arc::new(WordDictionary::default());
            let guard = dictionary.gate.lock();
            let original = dictionary.clone();
            let (ready_tx, ready_rx) = mpsc::channel();
            let (done_tx, done_rx) = mpsc::channel();
            let worker = thread::spawn(move || {
                ready_tx.send(()).unwrap();
                let error = if contains {
                    original.contains_nullable(None).unwrap_err()
                } else {
                    original.get_frequency_nullable(None).unwrap_err()
                };
                done_tx
                    .send(matches!(error, DictionaryError::ArgumentNull("key")))
                    .unwrap();
            });
            ready_rx.recv().unwrap();
            let before = done_rx.recv_timeout(Duration::from_millis(20));
            drop(guard);
            assert!(done_rx.recv_timeout(Duration::from_secs(5)).unwrap());
            worker.join().unwrap();
            assert!(matches!(before, Err(mpsc::RecvTimeoutError::Timeout)));
        }
    }
}
