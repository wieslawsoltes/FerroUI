use crate::media::text_formatting::formatting_buffer_helper::FormattingBufferHelper;
use std::collections::HashMap;
use std::hash::Hash;

/// A simple bi-directional dictionary.
pub struct BidiDictionary<T1, T2> {
    forward: HashMap<T1, T2>,
    reverse: HashMap<T2, T1>,
}

impl<T1: Clone + Eq + Hash, T2: Clone + Eq + Hash> Default for BidiDictionary<T1, T2> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T1: Clone + Eq + Hash, T2: Clone + Eq + Hash> BidiDictionary<T1, T2> {
    pub fn new() -> Self {
        Self { forward: HashMap::new(), reverse: HashMap::new() }
    }

    pub fn clear_then_reset_if_too_large(&mut self) {
        FormattingBufferHelper::clear_then_reset_if_too_large_map(&mut self.forward);
        FormattingBufferHelper::clear_then_reset_if_too_large_map(&mut self.reverse);
    }

    /// Panics when the key or the value has been added already.
    pub fn add(&mut self, key: T1, value: T2) {
        if self.forward.contains_key(&key) {
            throw_duplicate();
        }
        self.forward.insert(key.clone(), value.clone());

        if self.reverse.contains_key(&value) {
            throw_duplicate();
        }
        self.reverse.insert(value, key);
    }

    pub fn try_get_value(&self, key: &T1) -> Option<&T2> {
        self.forward.get(key)
    }

    pub fn try_get_key(&self, value: &T2) -> Option<&T1> {
        self.reverse.get(value)
    }

    pub fn contains_key(&self, key: &T1) -> bool {
        self.forward.contains_key(key)
    }

    pub fn contains_value(&self, value: &T2) -> bool {
        self.reverse.contains_key(value)
    }
}

fn throw_duplicate() -> ! {
    panic!("An item with the same key has already been added.")
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of this type.
    use super::*;

    #[test]
    fn finds_values_by_key_and_keys_by_value() {
        let mut target = BidiDictionary::<i32, char>::new();
        target.add(1, 'a');
        target.add(2, 'b');

        assert_eq!(Some(&'a'), target.try_get_value(&1));
        assert_eq!(Some(&2), target.try_get_key(&'b'));
        assert_eq!(None, target.try_get_value(&3));
        assert_eq!(None, target.try_get_key(&'c'));
        assert!(target.contains_key(&2));
        assert!(target.contains_value(&'a'));

        target.clear_then_reset_if_too_large();
        assert!(!target.contains_key(&1));
        assert!(!target.contains_value(&'a'));
    }

    #[test]
    #[should_panic(expected = "An item with the same key has already been added.")]
    fn add_throws_for_a_value_that_exists() {
        let mut target = BidiDictionary::<i32, char>::new();
        target.add(1, 'a');
        target.add(2, 'a');
    }
}
