use super::small_dictionary::{InlineDictionary, InlineDictionaryEnumerator};
use std::hash::Hash;

/// Counts the references to keys in an [`InlineDictionary`].
pub struct RefCountingSmallDictionary<TKey> {
    counts: InlineDictionary<TKey, i32>,
}

impl<TKey: Clone + Eq + Hash> Default for RefCountingSmallDictionary<TKey> {
    fn default() -> Self {
        Self { counts: InlineDictionary::new() }
    }
}

impl<TKey: Clone + Eq + Hash> RefCountingSmallDictionary<TKey> {
    /// Adds a reference to the key. Returns whether it is the first one.
    pub fn add(&mut self, key: TKey) -> bool {
        let (cnt, exists) = self.counts.get_value_ref_or_add_default(key);
        *cnt += 1;
        !exists
    }

    /// Removes a reference to the key. Returns whether it was the last one.
    ///
    /// Panics when the key has no reference.
    pub fn remove(&mut self, key: &TKey) -> bool {
        let remaining = match self.counts.get_value_ref_or_null_ref(key) {
            Some(cnt) => {
                *cnt -= 1;
                *cnt
            }
            None => panic!("Object reference not set to an instance of an object."),
        };
        if remaining == 0 {
            self.counts.remove(key);
            return true;
        }

        false
    }

    pub fn get_enumerator(&self) -> InlineDictionaryEnumerator<'_, TKey, i32> {
        self.counts.get_enumerator()
    }
}

impl<'a, TKey: Clone + Eq + Hash> IntoIterator for &'a RefCountingSmallDictionary<TKey> {
    type Item = (&'a TKey, &'a i32);
    type IntoIter = InlineDictionaryEnumerator<'a, TKey, i32>;

    fn into_iter(self) -> Self::IntoIter {
        self.get_enumerator()
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn counts_references() {
        let mut target = RefCountingSmallDictionary::<&'static str>::default();

        assert!(target.add("a"));
        assert!(!target.add("a"));
        assert!(target.add("b"));

        let counts: Vec<(&'static str, i32)> = target.get_enumerator().map(|(key, count)| (*key, *count)).collect();
        assert_eq!(vec![("a", 2), ("b", 1)], counts);

        assert!(!target.remove(&"a"));
        assert!(target.remove(&"a"));
        assert!(target.remove(&"b"));
        assert_eq!(0, target.get_enumerator().count());
    }

    #[test]
    #[should_panic(expected = "Object reference not set to an instance of an object.")]
    fn remove_of_a_key_without_reference_throws() {
        let mut target = RefCountingSmallDictionary::<&'static str>::default();
        target.remove(&"a");
    }
}
