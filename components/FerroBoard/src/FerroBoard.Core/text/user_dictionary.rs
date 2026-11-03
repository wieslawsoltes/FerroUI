//! UserDictionary portion of Text/Suggestions.cs. Kept separate from provider presentation logic.
use super::{NormalizationError, TextCulture, Utf16Text, WordDictionary, ordinal::OrdinalKey};
use crate::{
    events::SharedEvent,
    serialization::{JsonBridgeError, StrictJsonValue, write_json_string},
};
use parking_lot::ReentrantMutex;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Write,
    sync::{
        Arc, Weak,
        atomic::{AtomicI32, Ordering},
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserDictionaryInputError {
    ArgumentNull(&'static str),
}
impl std::fmt::Display for UserDictionaryInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgumentNull(name) => write!(f, "missing user dictionary argument: {name}"),
        }
    }
}
impl std::error::Error for UserDictionaryInputError {}
// Priorities are observable in equal-count eviction, completion, prediction and JSON.
// Reuse retired priorities in reverse retirement order, as the pinned keyboard does.
// This is private learning policy; no generic collection/runtime contract is exposed.
#[derive(Default)]
struct WordCounts {
    usages: HashMap<Utf16Text, (usize, i32)>,
    priority: BTreeMap<usize, Utf16Text>,
    retired: Vec<usize>,
    next_priority: usize,
}
impl WordCounts {
    fn record(&mut self, word: Utf16Text, count: i32) {
        if let Some((_, usage)) = self.usages.get_mut(&word) {
            *usage = count;
            return;
        }
        let priority = self.retired.pop().unwrap_or_else(|| {
            let priority = self.next_priority;
            self.next_priority += 1;
            priority
        });
        self.priority.insert(priority, word.clone());
        self.usages.insert(word, (priority, count));
    }
    fn count(&self, word: &Utf16Text) -> Option<&i32> {
        self.usages.get(word).map(|(_, count)| count)
    }
    fn forget(&mut self, word: &Utf16Text) {
        if let Some((priority, _)) = self.usages.remove(word) {
            self.priority.remove(&priority);
            self.retired.push(priority);
        }
    }
    fn ordered(&self) -> impl Iterator<Item = (&Utf16Text, &i32)> {
        self.priority
            .values()
            .map(|word| (word, &self.usages.get(word).expect("ranked learned word").1))
    }
    fn clear(&mut self) {
        self.usages.clear();
        self.priority.clear();
        self.retired.clear();
        self.next_priority = 0;
    }
}
struct PredictionContext {
    priority: usize,
    words: WordCounts,
}
#[derive(Default)]
struct State {
    words: WordCounts,
    bigrams: HashMap<Utf16Text, PredictionContext>,
    context_priority: BTreeMap<usize, Utf16Text>,
    retired_contexts: Vec<usize>,
    next_context_priority: usize,
    blocked: HashSet<OrdinalKey>,
}
impl State {
    fn replace_context(&mut self, previous: Utf16Text, words: WordCounts) {
        if let Some(context) = self.bigrams.get_mut(&previous) {
            context.words = words;
            return;
        }
        let priority = self.retired_contexts.pop().unwrap_or_else(|| {
            let priority = self.next_context_priority;
            self.next_context_priority += 1;
            priority
        });
        self.context_priority.insert(priority, previous.clone());
        self.bigrams
            .insert(previous, PredictionContext { priority, words });
    }
    fn forget_context(&mut self, previous: &Utf16Text) {
        if let Some(context) = self.bigrams.remove(previous) {
            self.context_priority.remove(&context.priority);
            self.retired_contexts.push(context.priority);
        }
    }
    fn clear_learning(&mut self) {
        self.words.clear();
        self.bigrams.clear();
        self.context_priority.clear();
        self.retired_contexts.clear();
        self.next_context_priority = 0;
    }
}
/// Synchronous shared learned-word store. Words is a new independent snapshot on every access.
struct UserDictionaryData {
    gate: ReentrantMutex<RefCell<State>>,
    changed: SharedEvent<UserDictionary>,
    learn_threshold: AtomicI32,
    max_words: AtomicI32,
}
// Public handles retain an owning registry when needed; registry-internal cache handles do not.
// Identity belongs to the shared data allocation, independent of the handle's retention edges.
pub(crate) trait UserDictionaryOwner: Send + Sync {}
impl<T: Send + Sync> UserDictionaryOwner for T {}
/// A reference handle: cloning shares the same store, settings, subscriptions and identity.
/// Registry-owned stores retain their owner through external handles without a strong cache cycle.
#[derive(Clone)]
pub struct UserDictionary {
    inner: Arc<UserDictionaryData>,
    owner: Option<Arc<dyn UserDictionaryOwner>>,
}
#[derive(Clone)]
pub struct WeakUserDictionary {
    inner: Weak<UserDictionaryData>,
    owner: Option<Weak<dyn UserDictionaryOwner>>,
}
impl PartialEq for UserDictionary {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}
impl Eq for UserDictionary {}
impl std::hash::Hash for UserDictionary {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(Arc::as_ptr(&self.inner), state);
    }
}
impl WeakUserDictionary {
    /// Original store identity, independent of native view and owner-retention handles.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.inner, &other.inner)
    }
    pub fn upgrade(&self) -> Option<UserDictionary> {
        let owner = match &self.owner {
            Some(owner) => Some(owner.upgrade()?),
            None => None,
        };
        Some(UserDictionary {
            inner: self.inner.upgrade()?,
            owner,
        })
    }
}
impl Default for UserDictionary {
    fn default() -> Self {
        Self {
            inner: Arc::new(UserDictionaryData {
                gate: ReentrantMutex::new(RefCell::new(State::default())),
                changed: SharedEvent::default(),
                learn_threshold: AtomicI32::new(2),
                max_words: AtomicI32::new(5000),
            }),
            owner: None,
        }
    }
}
impl UserDictionary {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
    /// Merge retention edges when native views canonicalize handles for this same store.
    #[doc(hidden)]
    pub fn retaining_owner_from(&self, other: &Self) -> Self {
        assert!(
            self.ptr_eq(other),
            "retention edges belong to different dictionaries"
        );
        Self {
            inner: self.inner.clone(),
            owner: self.owner.clone().or_else(|| other.owner.clone()),
        }
    }
    pub fn downgrade(&self) -> WeakUserDictionary {
        WeakUserDictionary {
            inner: Arc::downgrade(&self.inner),
            owner: self.owner.as_ref().map(Arc::downgrade),
        }
    }
    pub(crate) fn retaining(&self, owner: Arc<dyn UserDictionaryOwner>) -> Self {
        Self {
            inner: self.inner.clone(),
            owner: Some(owner),
        }
    }

    pub fn changed(&self) -> &SharedEvent<Self> {
        &self.inner.changed
    }
    pub fn learn_threshold(&self) -> i32 {
        self.inner.learn_threshold.load(Ordering::Relaxed)
    }
    pub fn set_learn_threshold(&self, value: i32) {
        self.inner.learn_threshold.store(value, Ordering::Relaxed);
    }
    pub fn max_words(&self) -> i32 {
        self.inner.max_words.load(Ordering::Relaxed)
    }
    pub fn set_max_words(&self, value: i32) {
        self.inner.max_words.store(value, Ordering::Relaxed);
    }
    /// Independent native snapshot; iteration order is unspecified.
    pub fn words(&self) -> HashMap<Utf16Text, i32> {
        self.inner
            .gate
            .lock()
            .borrow()
            .words
            .ordered()
            .map(|(w, c)| (w.clone(), *c))
            .collect()
    }
    /// Preserve ranking/glide encounter order separately from the public unordered snapshot.
    pub(crate) fn words_in_learning_order(&self) -> Vec<(Utf16Text, i32)> {
        self.inner
            .gate
            .lock()
            .borrow()
            .words
            .ordered()
            .map(|(word, count)| (word.clone(), *count))
            .collect()
    }
    pub fn learn(&self, word: &Utf16Text, previous_word: Option<&Utf16Text>) {
        if word.is_empty() || self.is_blocked(word) {
            return;
        }
        {
            let guard = self.inner.gate.lock();
            let mut state = guard.borrow_mut();
            let count = state.words.count(word).map_or(1, |c| c.wrapping_add(1));
            state.words.record(word.clone(), count);
            if state.words.usages.len() as i64 > self.max_words() as i64 {
                let victim = state
                    .words
                    .ordered()
                    .min_by_key(|(_, c)| **c)
                    .expect("Learn just inserted an entry")
                    .0
                    .clone();
                state.words.forget(&victim);
                state.forget_context(&victim);
            }
            if let Some(prev) = previous_word.filter(|p| !p.is_empty()) {
                let prev = TextCulture::to_lower_invariant(prev);
                if !state.bigrams.contains_key(&prev) {
                    state.replace_context(prev.clone(), WordCounts::default());
                }
                let next = &mut state.bigrams.get_mut(&prev).expect("learned context").words;
                let count = next.count(word).map_or(1, |c| c.wrapping_add(1));
                next.record(word.clone(), count);
            }
        }
        self.inner.changed.raise(self);
    }
    pub fn get_count(&self, word: &Utf16Text) -> i32 {
        self.get_count_nullable(Some(word))
            .expect("nonnull dictionary key")
    }
    pub fn get_count_nullable(
        &self,
        word: Option<&Utf16Text>,
    ) -> Result<i32, UserDictionaryInputError> {
        let guard = self.inner.gate.lock();
        let word = word.ok_or(UserDictionaryInputError::ArgumentNull("key"))?;
        Ok(guard.borrow().words.count(word).copied().unwrap_or(0))
    }
    pub fn is_blocked(&self, word: &Utf16Text) -> bool {
        self.is_blocked_nullable(Some(word))
    }
    pub fn is_blocked_nullable(&self, word: Option<&Utf16Text>) -> bool {
        let guard = self.inner.gate.lock();
        let state = guard.borrow();
        word.is_some_and(|word| state.blocked.contains(&OrdinalKey(word.clone())))
    }
    pub fn block(&self, word: &Utf16Text) {
        self.block_nullable(Some(word))
            .expect("nonnull dictionary key");
    }
    pub fn block_nullable(&self, word: Option<&Utf16Text>) -> Result<(), UserDictionaryInputError> {
        {
            let guard = self.inner.gate.lock();
            let word = word.ok_or(UserDictionaryInputError::ArgumentNull("key"))?;
            let mut state = guard.borrow_mut();
            state.words.forget(word);
            state.blocked.insert(OrdinalKey(word.clone()));
            for context in state.bigrams.values_mut() {
                context.words.forget(word);
            }
        }
        self.inner.changed.raise(self);
        Ok(())
    }
    pub fn unblock(&self, word: &Utf16Text) {
        self.unblock_nullable(Some(word));
    }
    pub fn unblock_nullable(&self, word: Option<&Utf16Text>) {
        {
            let guard = self.inner.gate.lock();
            let mut state = guard.borrow_mut();
            if let Some(word) = word {
                state.blocked.remove(&OrdinalKey(word.clone()));
            }
        }
        self.inner.changed.raise(self);
    }
    pub fn complete(
        &self,
        prefix: &Utf16Text,
    ) -> Result<Vec<(Utf16Text, i32)>, NormalizationError> {
        let norm = WordDictionary::normalize(prefix)?;
        let guard = self.inner.gate.lock();
        let state = guard.borrow();
        let mut result = Vec::new();
        for (word, count) in state.words.ordered() {
            if *count >= self.learn_threshold()
                && WordDictionary::normalize(word)?
                    .units()
                    .starts_with(norm.units())
            {
                result.push((word.clone(), *count));
            }
        }
        Ok(result)
    }
    pub fn predict(&self, previous_word: &Utf16Text, max: i32) -> Vec<(Utf16Text, i32)> {
        let guard = self.inner.gate.lock();
        let state = guard.borrow();
        let prev = TextCulture::to_lower_invariant(previous_word);
        let Some(next) = state.bigrams.get(&prev) else {
            return Vec::new();
        };
        let mut result: Vec<_> = next
            .words
            .ordered()
            .filter(|(word, _)| !state.blocked.contains(&OrdinalKey((*word).clone())))
            .map(|(w, c)| (w.clone(), *c))
            .collect();
        result.sort_by_key(|a| std::cmp::Reverse(a.1));
        result.truncate(max.max(0) as usize);
        result
    }
    pub fn clear(&self) {
        {
            let guard = self.inner.gate.lock();
            let mut state = guard.borrow_mut();
            state.clear_learning();
        }
        self.inner.changed.raise(self);
    }
    pub fn export(&self) -> String {
        fn counts(output: &mut String, values: &WordCounts) {
            output.push('{');
            for (i, (word, count)) in values.ordered().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                write_json_string(output, word);
                write!(output, ":{count}").expect("String writes cannot fail");
            }
            output.push('}');
        }
        let mut output = String::from("{\"words\":");
        let guard = self.inner.gate.lock();
        let state = guard.borrow();
        counts(&mut output, &state.words);
        output.push_str(",\"bigrams\":{");
        for (i, previous) in state.context_priority.values().enumerate() {
            let next = &state.bigrams.get(previous).expect("ranked context").words;
            if i > 0 {
                output.push(',');
            }
            write_json_string(&mut output, previous);
            output.push(':');
            counts(&mut output, next);
        }
        output.push_str("},\"blocked\":[");
        let mut blocked: Vec<_> = state.blocked.iter().collect();
        blocked.sort_unstable_by(|left, right| left.0.units().cmp(right.0.units()));
        for (i, key) in blocked.into_iter().enumerate() {
            if i > 0 {
                output.push(',');
            }
            write_json_string(&mut output, &key.0);
        }
        output.push_str("]}");
        output
    }
    pub fn import(&self, json: &str) -> Result<(), JsonBridgeError> {
        let document = StrictJsonValue::parse(json)?;
        {
            let guard = self.inner.gate.lock();
            let mut state = guard.borrow_mut();
            if let Some(words) = document.get("words")? {
                for (name, value) in words.object()? {
                    let word = StrictJsonValue::valid_string(&name.text)?;
                    let count = value.int32()?;
                    state.words.record(word, count);
                }
            }
            if let Some(bigrams) = document.get("bigrams")? {
                for (name, value) in bigrams.object()? {
                    let mut map = WordCounts::default();
                    for (name, value) in value.object()? {
                        let word = StrictJsonValue::valid_string(&name.text)?;
                        let count = value.int32()?;
                        map.record(word, count);
                    }
                    state.replace_context(StrictJsonValue::valid_string(&name.text)?, map);
                }
            }
            if let Some(blocked) = document.get("blocked")? {
                for value in blocked.array()? {
                    state
                        .blocked
                        .insert(OrdinalKey(value.string()?.unwrap_or_default()));
                }
            }
        }
        self.inner.changed.raise(self);
        Ok(())
    }
}

#[cfg(test)]
mod nullable_lock_tests {
    use super::*;
    #[test]
    fn null_key_calls_keep_the_original_gate_and_notification_order() {
        use std::{sync::mpsc, thread, time::Duration};
        for operation in 0..4 {
            let dictionary = UserDictionary::default();
            let events = Arc::new(AtomicI32::new(0));
            let observer = events.clone();
            dictionary.changed().add(Arc::new(move |_| {
                observer.fetch_add(1, Ordering::SeqCst);
            }));
            let guard = dictionary.inner.gate.lock();
            let original = dictionary.clone();
            let (ready_tx, ready_rx) = mpsc::channel();
            let (done_tx, done_rx) = mpsc::channel();
            let worker = thread::spawn(move || {
                ready_tx.send(()).unwrap();
                let result = match operation {
                    0 => original.get_count_nullable(None).map(|_| ()),
                    1 => original.block_nullable(None),
                    2 => {
                        assert!(!original.is_blocked_nullable(None));
                        Ok(())
                    }
                    3 => {
                        original.unblock_nullable(None);
                        Ok(())
                    }
                    _ => unreachable!(),
                };
                done_tx.send(result).unwrap();
            });
            ready_rx.recv().unwrap();
            let before = done_rx.recv_timeout(Duration::from_millis(20));
            let events_before = events.load(Ordering::SeqCst);
            drop(guard);
            let result = done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            worker.join().unwrap();
            assert!(matches!(before, Err(mpsc::RecvTimeoutError::Timeout)));
            assert_eq!(events_before, 0);
            assert_eq!(
                result,
                if operation < 2 {
                    Err(UserDictionaryInputError::ArgumentNull("key"))
                } else {
                    Ok(())
                }
            );
            assert_eq!(events.load(Ordering::SeqCst), i32::from(operation == 3));
        }
    }
}
