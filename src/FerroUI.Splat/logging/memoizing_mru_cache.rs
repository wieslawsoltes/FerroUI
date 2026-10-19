//! `MemoizingMRUCache<TParam, TVal>`.

use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};

type Calculation<K, V> = Box<dyn Fn(&K, Option<&dyn Any>) -> V>;
type Release<V> = Box<dyn Fn(V)>;
type Comparer<K> = Box<dyn Fn(&K, &K) -> bool>;

struct CacheState<K, V> {
    /// The keys, the most recently used first.
    mru_list: VecDeque<K>,
    /// The values, in the order they were added.
    entries: Vec<(K, V)>,
}

/// The panics of the release function that [`MemoizingMRUCache::invalidate_all_with`] collected
/// (`AggregateException`).
#[derive(Debug)]
pub struct AggregateReleaseError {
    messages: Vec<String>,
}

impl AggregateReleaseError {
    /// The messages of the panics, in the order of the released values (`InnerExceptions`).
    pub fn inner_messages(&self) -> &[String] {
        &self.messages
    }
}

impl fmt::Display for AggregateReleaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Exceptions thrown during MRU Cache InvalidateAll item release.")
    }
}

impl Error for AggregateReleaseError {}

/// A cache of the values of a function, which keeps the most recently used ones
/// (`MemoizingMRUCache<TParam, TVal>`): `get` returns the cached value of a key, or computes it
/// and evicts the least recently used value when the cache is full. Evicted and invalidated values
/// are handed to the release function.
///
/// Keys compare with `PartialEq` or with a comparer (`IEqualityComparer<TParam>`).
// Deviation (DEVIATIONS.md, Splat): single-threaded, without the lock of upstream; the lookups are
// linear in the size of the cache.
pub struct MemoizingMRUCache<K, V> {
    calculation: Calculation<K, V>,
    release: Option<Release<V>>,
    max_cache_size: usize,
    comparer: Comparer<K>,
    state: RefCell<CacheState<K, V>>,
}

impl<K: Clone + PartialEq + 'static, V: Clone + 'static> MemoizingMRUCache<K, V> {
    /// A cache of `calculation_func` with at most `max_size` values
    /// (`MemoizingMRUCache(Func<TParam, object?, TVal>, int)`).
    ///
    /// Panics when `max_size` is 0 (`ArgumentOutOfRangeException`).
    pub fn new(calculation_func: impl Fn(&K, Option<&dyn Any>) -> V + 'static, max_size: usize) -> Self {
        Self::with_release_and_comparer(calculation_func, max_size, None::<fn(V)>, |a: &K, b: &K| a == b)
    }

    /// A cache that hands evicted and invalidated values to `on_release`
    /// (`MemoizingMRUCache(Func<TParam, object?, TVal>, int, Action<TVal>)`).
    pub fn with_release(
        calculation_func: impl Fn(&K, Option<&dyn Any>) -> V + 'static,
        max_size: usize,
        on_release: impl Fn(V) + 'static,
    ) -> Self {
        Self::with_release_and_comparer(calculation_func, max_size, Some(on_release), |a: &K, b: &K| a == b)
    }
}

impl<K: Clone + 'static, V: Clone + 'static> MemoizingMRUCache<K, V> {
    /// A cache whose keys compare with `param_comparer`
    /// (`MemoizingMRUCache(Func<TParam, object?, TVal>, int, IEqualityComparer<TParam>)`).
    pub fn with_comparer(
        calculation_func: impl Fn(&K, Option<&dyn Any>) -> V + 'static,
        max_size: usize,
        param_comparer: impl Fn(&K, &K) -> bool + 'static,
    ) -> Self {
        Self::with_release_and_comparer(calculation_func, max_size, None::<fn(V)>, param_comparer)
    }

    /// A cache with a release function and a comparer of keys
    /// (`MemoizingMRUCache(Func<TParam, object?, TVal>, int, Action<TVal>?,
    /// IEqualityComparer<TParam>)`).
    ///
    /// Panics when `max_size` is 0 (`ArgumentOutOfRangeException`).
    pub fn with_release_and_comparer(
        calculation_func: impl Fn(&K, Option<&dyn Any>) -> V + 'static,
        max_size: usize,
        on_release: Option<impl Fn(V) + 'static>,
        param_comparer: impl Fn(&K, &K) -> bool + 'static,
    ) -> Self {
        assert!(max_size > 0, "maxSize must be greater than 0.");
        Self {
            calculation: Box::new(calculation_func),
            release: on_release.map(|release| Box::new(release) as Release<V>),
            max_cache_size: max_size,
            comparer: Box::new(param_comparer),
            state: RefCell::new(CacheState { mru_list: VecDeque::new(), entries: Vec::new() }),
        }
    }

    fn position(&self, state: &CacheState<K, V>, key: &K) -> Option<usize> {
        state.entries.iter().position(|(entry_key, _)| (self.comparer)(entry_key, key))
    }

    /// Makes `key` the most recently used (`Refresh`).
    fn refresh(&self, state: &mut CacheState<K, V>, key: &K) {
        if state.entries.len() <= 1 {
            return;
        }
        if let Some(index) = state.mru_list.iter().position(|mru_key| (self.comparer)(mru_key, key)) {
            if let Some(mru_key) = state.mru_list.remove(index) {
                state.mru_list.push_front(mru_key);
            }
        }
    }

    fn lookup(&self, key: &K) -> Option<V> {
        let mut state = self.state.borrow_mut();
        let index = self.position(&state, key)?;
        let value = state.entries[index].1.clone();
        self.refresh(&mut state, key);
        Some(value)
    }

    /// The value of `key`, computed on a miss (`Get(TParam)`).
    pub fn get(&self, key: K) -> V {
        self.get_with_context(key, None)
    }

    /// The value of `key`, computed with `context` on a miss (`Get(TParam, object?)`).
    pub fn get_with_context(&self, key: K, context: Option<&dyn Any>) -> V {
        if let Some(value) = self.lookup(&key) {
            return value;
        }

        // A miss: computed without a borrow of the cache, which the calculation may use.
        let computed = (self.calculation)(&key, context);

        let evicted = {
            let mut state = self.state.borrow_mut();
            // The calculation may have filled the key.
            if let Some(index) = self.position(&state, &key) {
                let value = state.entries[index].1.clone();
                self.refresh(&mut state, &key);
                return value;
            }
            state.mru_list.push_front(key.clone());
            state.entries.push((key, computed.clone()));

            let mut evicted = Vec::new();
            while state.entries.len() > self.max_cache_size {
                let Some(oldest) = state.mru_list.pop_back() else {
                    break;
                };
                if let Some(index) = self.position(&state, &oldest) {
                    evicted.push(state.entries.remove(index).1);
                }
            }
            evicted
        };

        // The release function runs outside of the borrow.
        if let Some(release) = &self.release {
            for value in evicted {
                release(value);
            }
        }
        computed
    }

    /// The cached value of `key`, without computing it (`TryGet`).
    pub fn try_get(&self, key: &K) -> Option<V> {
        self.lookup(key)
    }

    /// Removes `key` and releases its value (`Invalidate`).
    pub fn invalidate(&self, key: &K) {
        let removed = {
            let mut state = self.state.borrow_mut();
            let Some(index) = self.position(&state, key) else {
                return;
            };
            let (_, value) = state.entries.remove(index);
            if let Some(mru_index) = state.mru_list.iter().position(|mru_key| (self.comparer)(mru_key, key)) {
                state.mru_list.remove(mru_index);
            }
            value
        };
        if let Some(release) = &self.release {
            release(removed);
        }
    }

    /// Removes every value and releases them; a panic of the release function stops the release
    /// (`InvalidateAll()`).
    pub fn invalidate_all(&self) {
        let _ = self.invalidate_all_with(false);
    }

    /// Removes every value and releases them (`InvalidateAll(bool)`). With
    /// `aggregate_release_exceptions`, every value is released even when the release function
    /// panics, and the panics are returned together; otherwise the first panic propagates.
    // Deviation (DEVIATIONS.md, Splat): the `AggregateException` is the `Err` of the result.
    pub fn invalidate_all_with(&self, aggregate_release_exceptions: bool) -> Result<(), AggregateReleaseError> {
        let old_entries = {
            let mut state = self.state.borrow_mut();
            if state.entries.is_empty() {
                return Ok(());
            }
            state.mru_list.clear();
            std::mem::take(&mut state.entries)
        };

        let Some(release) = &self.release else {
            return Ok(());
        };

        if !aggregate_release_exceptions {
            for (_, value) in old_entries {
                release(value);
            }
            return Ok(());
        }

        let mut messages = Vec::new();
        for (_, value) in old_entries {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| release(value))) {
                messages.push(panic_message(payload.as_ref()));
            }
        }
        if messages.is_empty() {
            Ok(())
        } else {
            Err(AggregateReleaseError { messages })
        }
    }

    /// The cached values (`CachedValues`).
    pub fn cached_values(&self) -> Vec<V> {
        self.state.borrow().entries.iter().map(|(_, value)| value.clone()).collect()
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic".to_owned()
    }
}
