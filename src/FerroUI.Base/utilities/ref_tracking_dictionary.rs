use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

/// Maintains a set of objects with reference counts
pub struct RefTrackingDictionary<TKey> {
    base: HashMap<TKey, i32>,
}

impl<TKey: Eq + Hash> Default for RefTrackingDictionary<TKey> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TKey: Eq + Hash> RefTrackingDictionary<TKey> {
    pub fn new() -> Self {
        Self { base: HashMap::new() }
    }

    /// Increase reference count for a key by 1.
    ///
    /// Returns true if key was added to the dictionary, false otherwise.
    pub fn add_ref(&mut self, key: TKey) -> bool {
        let count = self.base.entry(key).or_insert(0);
        *count += 1;
        *count == 1
    }

    /// Decrease reference count for a key by 1.
    ///
    /// Returns true if key was removed to the dictionary, false otherwise.
    /// A key without a reference panics in a debug build and returns false
    /// otherwise.
    pub fn release_ref(&mut self, key: &TKey) -> bool {
        let remaining = match self.base.get_mut(key) {
            Some(count) => {
                *count -= 1;
                *count
            }
            None => {
                if cfg!(debug_assertions) {
                    panic!("Attempting to release a non-referenced object");
                }
                return false;
            }
        };
        if remaining == 0 {
            self.base.remove(key);
            return true;
        }

        false
    }
}

impl<TKey> Deref for RefTrackingDictionary<TKey> {
    type Target = HashMap<TKey, i32>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<TKey> DerefMut for RefTrackingDictionary<TKey> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn counts_references() {
        let mut target = RefTrackingDictionary::<&'static str>::new();

        assert!(target.add_ref("a"));
        assert!(!target.add_ref("a"));
        assert!(target.add_ref("b"));
        assert_eq!(Some(&2), target.get("a"));
        assert_eq!(2, target.len());

        assert!(!target.release_ref(&"a"));
        assert!(target.release_ref(&"a"));
        assert!(!target.contains_key("a"));
        assert!(target.release_ref(&"b"));
        assert!(target.is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Attempting to release a non-referenced object")]
    fn release_of_a_key_without_reference_throws() {
        let mut target = RefTrackingDictionary::<&'static str>::new();
        target.release_ref(&"a");
    }
}
