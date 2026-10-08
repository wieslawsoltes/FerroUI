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
    ///
    /// # Panics
    ///
    /// When `secondary_size` is negative (upstream throws
    /// `ArgumentOutOfRangeException`).
    pub fn with_secondary_size(secondary_size: i32) -> Self {
        Self::with_options(secondary_size, None, None)
    }

    /// Creates a cache with the given secondary capacity, eviction action and
    /// key comparer (`None` compares with `==`).
    ///
    /// # Panics
    ///
    /// When `secondary_size` is negative (upstream throws
    /// `ArgumentOutOfRangeException`).
    pub fn with_options(
        secondary_size: i32,
        eviction_action: Option<EvictionAction<TValue>>,
        comparer: Option<KeyComparer<TKey>>,
    ) -> Self {
        let Ok(secondary_size) = usize::try_from(secondary_size) else {
            panic!("secondary_size is out of range: {secondary_size}");
        };

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
