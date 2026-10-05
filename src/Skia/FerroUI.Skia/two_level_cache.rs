/// Compares cache keys.
type KeyComparer<TKey> = Box<dyn Fn(&TKey, &TKey) -> bool>;

/// Receives the values that leave the cache.
type EvictionAction<TValue> = Box<dyn Fn(&TValue)>;

/// Provides a lightweight two-level cache for storing key-value pairs,
/// supporting fast retrieval and optional eviction handling.
///
/// The cache maintains a primary entry for the most recently added item and a
/// secondary array for additional items, up to the specified capacity. When
/// the cache exceeds its capacity, evicted values are passed to the eviction
/// action. It is optimized for scenarios where a small number of items are
/// frequently accessed.
pub struct TwoLevelCache<TKey, TValue: Clone> {
    secondary_size: usize,
    primary: Option<(TKey, TValue)>,
    secondary: Option<Vec<(TKey, TValue)>>,
    eviction_action: Option<EvictionAction<TValue>>,
    comparer: KeyComparer<TKey>,
}

impl<TKey: PartialEq + Clone + 'static, TValue: Clone> TwoLevelCache<TKey, TValue> {
    /// Creates a cache with the default secondary capacity (3), no eviction
    /// action and the default comparer.
    pub fn new() -> Self {
        Self::with_options(3, None, None)
    }

    /// Creates a cache with the given secondary capacity.
    pub fn with_secondary_size(secondary_size: usize) -> Self {
        Self::with_options(secondary_size, None, None)
    }

    /// Creates a cache with the given secondary capacity, eviction action and
    /// key comparer (`None` compares with `==`).
    pub fn with_options(
        secondary_size: usize,
        eviction_action: Option<EvictionAction<TValue>>,
        comparer: Option<KeyComparer<TKey>>,
    ) -> Self {
        Self {
            secondary_size,
            primary: None,
            secondary: None,
            eviction_action,
            comparer: comparer.unwrap_or_else(|| Box::new(|a, b| a == b)),
        }
    }
}

impl<TKey: PartialEq + Clone + 'static, TValue: Clone> Default for TwoLevelCache<TKey, TValue> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TKey: Clone, TValue: Clone> TwoLevelCache<TKey, TValue> {
    /// Looks up the value stored for `key`.
    pub fn try_get(&self, key: &TKey) -> Option<TValue> {
        if let Some((primary_key, primary_value)) = &self.primary {
            if (self.comparer)(primary_key, key) {
                return Some(primary_value.clone());
            }
        }

        if let Some(secondary) = &self.secondary {
            for (secondary_key, value) in secondary {
                if (self.comparer)(secondary_key, key) {
                    return Some(value.clone());
                }
            }
        }

        None
    }

    /// Returns the value stored for `key`, creating and storing it with
    /// `factory` when the key is not in the cache.
    pub fn get_or_add(&mut self, key: TKey, factory: impl FnOnce(&TKey) -> TValue) -> TValue {
        // Check if the key already exists.
        if let Some(existing) = self.try_get(&key) {
            return existing;
        }

        // The key doesn't exist: create a new value.
        let value = factory(&key);

        // Primary is empty: store in primary.
        if self.primary.is_none() {
            self.primary = Some((key, value.clone()));
            return value;
        }

        // No secondary cache configured: replace primary.
        if self.secondary_size == 0 {
            if let Some((_, evicted)) = self.primary.replace((key, value.clone())) {
                self.evict(&evicted);
            }
            return value;
        }

        // Insert the new entry at the front. This maintains insertion order
        // and evicts the oldest (last) entry when full.
        let secondary_size = self.secondary_size;
        let secondary = self.secondary.get_or_insert_with(|| Vec::with_capacity(secondary_size));
        let evicted = if secondary.len() == secondary_size { secondary.pop() } else { None };
        secondary.insert(0, (key, value.clone()));

        if let Some((_, evicted)) = evicted {
            self.evict(&evicted);
        }

        value
    }

    /// Removes every entry, passing each value to the eviction action.
    pub fn clear_and_dispose(&mut self) {
        if let Some((_, value)) = self.primary.take() {
            self.evict(&value);
        }

        if let Some(secondary) = self.secondary.take() {
            for (_, value) in &secondary {
                self.evict(value);
            }
        }
    }

    fn evict(&self, value: &TValue) {
        if let Some(action) = &self.eviction_action {
            action(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Value = Rc<i32>;

    fn recording_cache(secondary_size: usize) -> (TwoLevelCache<String, Value>, Rc<RefCell<Vec<Value>>>) {
        let evicted = Rc::new(RefCell::new(Vec::new()));
        let sink = evicted.clone();
        let cache = TwoLevelCache::with_options(
            secondary_size,
            Some(Box::new(move |v: &Value| sink.borrow_mut().push(v.clone()))),
            None,
        );
        (cache, evicted)
    }

    fn key(s: &str) -> String {
        s.to_string()
    }

    // Constructor_WithNegativeSecondarySize_ThrowsArgumentOutOfRangeException
    // has no counterpart: the size is unsigned.

    #[test]
    fn constructor_with_zero_secondary_size_does_not_panic() {
        let _cache = TwoLevelCache::<String, Value>::with_secondary_size(0);
    }

    #[test]
    fn try_get_empty_cache_returns_none() {
        let cache = TwoLevelCache::<String, Value>::new();
        assert!(cache.try_get(&key("key")).is_none());
    }

    #[test]
    fn get_or_add_first_item_stores_in_primary() {
        let mut cache = TwoLevelCache::<String, Value>::new();
        let value = Rc::new(1);
        let result = cache.get_or_add(key("key1"), |_| value.clone());
        assert!(Rc::ptr_eq(&value, &result));
        assert!(Rc::ptr_eq(&value, &cache.try_get(&key("key1")).unwrap()));
    }

    #[test]
    fn get_or_add_same_key_returns_existing_value() {
        let mut cache = TwoLevelCache::<String, Value>::new();
        let value1 = Rc::new(1);
        let value2 = Rc::new(2);
        cache.get_or_add(key("key"), |_| value1.clone());
        let result = cache.get_or_add(key("key"), |_| value2.clone());
        assert!(Rc::ptr_eq(&value1, &result));
    }

    #[test]
    fn get_or_add_second_item_stores_in_secondary() {
        let mut cache = TwoLevelCache::<String, Value>::with_secondary_size(3);
        let value1 = Rc::new(1);
        let value2 = Rc::new(2);
        cache.get_or_add(key("key1"), |_| value1.clone());
        cache.get_or_add(key("key2"), |_| value2.clone());
        assert!(Rc::ptr_eq(&value1, &cache.try_get(&key("key1")).unwrap()));
        assert!(Rc::ptr_eq(&value2, &cache.try_get(&key("key2")).unwrap()));
    }

    #[test]
    fn get_or_add_multiple_items_stores_correctly() {
        let mut cache = TwoLevelCache::<String, Value>::with_secondary_size(3);
        let values: Vec<Value> = (0..4).map(Rc::new).collect();
        for (i, value) in values.iter().enumerate() {
            cache.get_or_add(format!("key{i}"), |_| value.clone());
        }
        for (i, value) in values.iter().enumerate() {
            assert!(Rc::ptr_eq(value, &cache.try_get(&format!("key{i}")).unwrap()));
        }
    }

    #[test]
    fn get_or_add_exceeds_capacity_calls_eviction_action() {
        let (mut cache, evicted) = recording_cache(2);
        let values: Vec<Value> = (1..=4).map(Rc::new).collect();
        cache.get_or_add(key("key1"), |_| values[0].clone());
        cache.get_or_add(key("key2"), |_| values[1].clone());
        cache.get_or_add(key("key3"), |_| values[2].clone());
        // No evictions yet.
        assert!(evicted.borrow().is_empty());
        // This should cause eviction.
        cache.get_or_add(key("key4"), |_| values[3].clone());
        assert_eq!(1, evicted.borrow().len());
        assert!(Rc::ptr_eq(&values[1], &evicted.borrow()[0]));
    }

    #[test]
    fn get_or_add_zero_secondary_size_evicts_primary_immediately() {
        let (mut cache, evicted) = recording_cache(0);
        let value1 = Rc::new(1);
        let value2 = Rc::new(2);
        cache.get_or_add(key("key1"), |_| value1.clone());
        cache.get_or_add(key("key2"), |_| value2.clone());
        assert_eq!(1, evicted.borrow().len());
        assert!(Rc::ptr_eq(&value1, &evicted.borrow()[0]));
        // Only the latest value should be retrievable.
        assert!(cache.try_get(&key("key1")).is_none());
        assert!(Rc::ptr_eq(&value2, &cache.try_get(&key("key2")).unwrap()));
    }

    #[test]
    fn get_or_add_duplicate_key_returns_existing_without_calling_factory() {
        let mut cache = TwoLevelCache::<String, Value>::new();
        let value1 = Rc::new(1);
        let mut factory_called = false;
        cache.get_or_add(key("key"), |_| value1.clone());
        let result = cache.get_or_add(key("key"), |_| {
            factory_called = true;
            Rc::new(2)
        });
        assert!(Rc::ptr_eq(&value1, &result));
        assert!(!factory_called);
    }

    #[test]
    fn get_or_add_duplicate_key_in_secondary_returns_existing_without_calling_factory() {
        let mut cache = TwoLevelCache::<String, Value>::with_secondary_size(2);
        let value1 = Rc::new(1);
        let value2 = Rc::new(2);
        let mut factory_called = false;
        cache.get_or_add(key("key1"), |_| value1.clone());
        cache.get_or_add(key("key2"), |_| value2.clone());
        let result = cache.get_or_add(key("key2"), |_| {
            factory_called = true;
            Rc::new(3)
        });
        assert!(Rc::ptr_eq(&value2, &result));
        assert!(!factory_called);
    }

    #[test]
    fn clear_and_dispose_empty_cache_does_not_panic() {
        let mut cache = TwoLevelCache::<String, Value>::new();
        cache.clear_and_dispose();
    }

    #[test]
    fn clear_and_dispose_with_values_calls_eviction_action_for_all() {
        let (mut cache, evicted) = recording_cache(2);
        let values: Vec<Value> = (1..=3).map(Rc::new).collect();
        for (i, value) in values.iter().enumerate() {
            cache.get_or_add(format!("key{}", i + 1), |_| value.clone());
        }
        cache.clear_and_dispose();
        assert_eq!(3, evicted.borrow().len());
        for value in &values {
            assert!(evicted.borrow().iter().any(|e| Rc::ptr_eq(e, value)));
        }
    }

    #[test]
    fn clear_and_dispose_clears_all_entries() {
        let mut cache = TwoLevelCache::<String, Value>::with_secondary_size(2);
        cache.get_or_add(key("key1"), |_| Rc::new(1));
        cache.get_or_add(key("key2"), |_| Rc::new(2));
        cache.clear_and_dispose();
        assert!(cache.try_get(&key("key1")).is_none());
        assert!(cache.try_get(&key("key2")).is_none());
    }

    fn ignore_case() -> Option<KeyComparer<String>> {
        Some(Box::new(|a: &String, b: &String| a.eq_ignore_ascii_case(b)))
    }

    #[test]
    fn get_or_add_with_custom_comparer_uses_comparer() {
        let mut cache = TwoLevelCache::<String, Value>::with_options(3, None, ignore_case());
        let value = Rc::new(1);
        cache.get_or_add(key("KEY"), |_| value.clone());
        assert!(Rc::ptr_eq(&value, &cache.try_get(&key("key")).unwrap()));
    }

    #[test]
    fn try_get_with_custom_comparer_uses_comparer() {
        let mut cache = TwoLevelCache::<String, Value>::with_options(2, None, ignore_case());
        let value1 = Rc::new(1);
        let value2 = Rc::new(2);
        cache.get_or_add(key("PRIMARY"), |_| value1.clone());
        cache.get_or_add(key("SECONDARY"), |_| value2.clone());
        assert!(Rc::ptr_eq(&value1, &cache.try_get(&key("primary")).unwrap()));
        assert!(Rc::ptr_eq(&value2, &cache.try_get(&key("secondary")).unwrap()));
    }

    #[test]
    fn get_or_add_int_keys_works_correctly() {
        let mut cache = TwoLevelCache::<i32, Value>::with_secondary_size(2);
        let values: Vec<Value> = (1..=3).map(Rc::new).collect();
        for (i, value) in values.iter().enumerate() {
            cache.get_or_add(i as i32 + 1, |_| value.clone());
        }
        for (i, value) in values.iter().enumerate() {
            assert!(Rc::ptr_eq(value, &cache.try_get(&(i as i32 + 1)).unwrap()));
        }
    }

    #[test]
    fn get_or_add_rotates_secondary_correctly() {
        let evicted = Rc::new(RefCell::new(Vec::new()));
        let sink = evicted.clone();
        let mut cache = TwoLevelCache::<i32, Value>::with_options(
            2,
            Some(Box::new(move |v: &Value| sink.borrow_mut().push(v.clone()))),
            None,
        );
        let values: Vec<Value> = (0..5).map(Rc::new).collect();
        for (i, value) in values.iter().enumerate() {
            cache.get_or_add(i as i32, |_| value.clone());
        }
        // Primary: 0, secondary: [1, 2]
        // After adding 3: primary: 0, secondary: [3, 2] (evicts 1)
        // After adding 4: primary: 0, secondary: [4, 3] (evicts 2)
        assert_eq!(2, evicted.borrow().len());
        assert!(evicted.borrow().iter().any(|e| Rc::ptr_eq(e, &values[2])));
        assert!(evicted.borrow().iter().any(|e| Rc::ptr_eq(e, &values[1])));
        // These should still be in the cache.
        assert!(cache.try_get(&0).is_some());
        assert!(cache.try_get(&3).is_some());
        assert!(cache.try_get(&4).is_some());
        // These should be evicted.
        assert!(cache.try_get(&1).is_none());
        assert!(cache.try_get(&2).is_none());
    }

    #[test]
    fn factory_function_receives_correct_key() {
        let mut cache = TwoLevelCache::<String, String>::new();
        let mut captured_key = None;
        cache.get_or_add(key("testKey"), |k| {
            captured_key = Some(k.clone());
            "value".to_string()
        });
        assert_eq!(Some(key("testKey")), captured_key);
    }

    #[test]
    fn get_or_add_no_eviction_action_does_not_panic() {
        let mut cache = TwoLevelCache::<String, Value>::with_secondary_size(1);
        cache.get_or_add(key("key1"), |_| Rc::new(1));
        cache.get_or_add(key("key2"), |_| Rc::new(2));
        cache.get_or_add(key("key3"), |_| Rc::new(3)); // Should evict without error.
        cache.clear_and_dispose(); // Should also not panic.
    }
}
