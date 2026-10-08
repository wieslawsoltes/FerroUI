use std::collections::hash_map::{self, Entry};
use std::collections::HashMap;
use std::hash::Hash;

/// The number of entries of the inline array.
const ARRAY_LENGTH: usize = 6;

/// A dictionary that holds one entry in place, up to six entries in a small
/// array and more in a hash map.
///
/// The original restricts keys to reference types and compares them by
/// reference while the dictionary holds its entries in place or in the
/// array; here keys compare with `==` in every state, which is identity for
/// the handle types of this crate.
pub struct InlineDictionary<TKey, TValue> {
    data: Data<TKey, TValue>,
    value: TValue,
}

/// The original's `object? _data`: nothing, the one key, the small array or
/// the dictionary.
enum Data<TKey, TValue> {
    Null,
    Key(TKey),
    Array(Vec<KeyValuePair<TKey, TValue>>),
    Dictionary(HashMap<TKey, TValue>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DataKind {
    Null,
    Key,
    Array,
    Dictionary,
}

struct KeyValuePair<TKey, TValue> {
    key: Option<TKey>,
    value: TValue,
}

impl<TKey, TValue: Default> Default for KeyValuePair<TKey, TValue> {
    fn default() -> Self {
        Self { key: None, value: TValue::default() }
    }
}

impl<TKey, TValue> KeyValuePair<TKey, TValue> {
    fn new(key: TKey, value: TValue) -> Self {
        Self { key: Some(key), value }
    }
}

impl<TKey: Clone + Eq + Hash, TValue: Default> Default for InlineDictionary<TKey, TValue> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TKey: Clone + Eq + Hash, TValue: Default> InlineDictionary<TKey, TValue> {
    pub fn new() -> Self {
        Self { data: Data::Null, value: TValue::default() }
    }

    fn kind(&self) -> DataKind {
        match &self.data {
            Data::Null => DataKind::Null,
            Data::Key(_) => DataKind::Key,
            Data::Array(_) => DataKind::Array,
            Data::Dictionary(_) => DataKind::Dictionary,
        }
    }

    fn new_array() -> Vec<KeyValuePair<TKey, TValue>> {
        (0..ARRAY_LENGTH).map(|_| KeyValuePair::default()).collect()
    }

    /// Moves the entries of the small array into a new dictionary.
    fn upgrade_to_dictionary(arr: Vec<KeyValuePair<TKey, TValue>>) -> HashMap<TKey, TValue> {
        let mut new_dic = HashMap::new();
        for kvp in arr {
            if let Some(key) = kvp.key {
                new_dic.insert(key, kvp.value);
            }
        }
        new_dic
    }

    fn set_core(&mut self, key: TKey, value: TValue, overwrite: bool) {
        match self.kind() {
            DataKind::Null => {
                self.data = Data::Key(key);
                self.value = value;
            }
            DataKind::Array => {
                let Data::Array(arr) = &mut self.data else { unreachable!() };
                let mut free: Option<usize> = None;
                let mut existing: Option<usize> = None;
                for (c, entry) in arr.iter().enumerate() {
                    if entry.key.as_ref() == Some(&key) {
                        existing = Some(c);
                        break;
                    }

                    if entry.key.is_none() && free.is_none() {
                        free = Some(c);
                    }
                }

                if let Some(c) = existing {
                    if overwrite {
                        arr[c] = KeyValuePair::new(key, value);
                        return;
                    }
                    panic!("Key already exists in dictionary");
                }

                if let Some(free) = free {
                    arr[free] = KeyValuePair::new(key, value);
                    return;
                }

                // Upgrade to dictionary
                let mut new_dic = Self::upgrade_to_dictionary(std::mem::take(arr));
                new_dic.insert(key, value);
                self.data = Data::Dictionary(new_dic);
            }
            DataKind::Dictionary => {
                let Data::Dictionary(dic) = &mut self.data else { unreachable!() };
                if overwrite {
                    dic.insert(key, value);
                } else {
                    if dic.contains_key(&key) {
                        panic!("An item with the same key has already been added.");
                    }
                    dic.insert(key, value);
                }
            }
            DataKind::Key => {
                // We have a single element, check if we should update the value.
                let same = matches!(&self.data, Data::Key(data) if *data == key);
                if same && overwrite {
                    self.value = value;

                    return;
                }
                // If we do not replace it, upgrade to array.
                let Data::Key(data) = std::mem::replace(&mut self.data, Data::Null) else { unreachable!() };
                let mut arr = Self::new_array();
                arr[0] = KeyValuePair::new(data, std::mem::take(&mut self.value));
                arr[1] = KeyValuePair::new(key, value);
                self.data = Data::Array(arr);
            }
        }
    }

    /// Adds an entry.
    ///
    /// Panics when the small array or the dictionary has the key already.
    /// As in the original, a dictionary that holds one entry in place does
    /// not test for it.
    pub fn add(&mut self, key: TKey, value: TValue) {
        self.set_core(key, value, false)
    }

    /// Adds an entry or replaces its value.
    pub fn set(&mut self, key: TKey, value: TValue) {
        self.set_core(key, value, true)
    }

    /// The value of a key (the getter of the indexer).
    ///
    /// Panics when the dictionary does not have the key.
    pub fn get(&self, key: &TKey) -> &TValue {
        match self.try_get_value(key) {
            Some(rv) => rv,
            None => throw_key_not_found(),
        }
    }

    pub fn remove(&mut self, key: &TKey) -> bool {
        match self.kind() {
            DataKind::Key => {
                let same = matches!(&self.data, Data::Key(data) if data == key);
                if same {
                    self.data = Data::Null;
                    self.value = TValue::default();
                    return true;
                }

                false
            }
            DataKind::Array => {
                let Data::Array(arr) = &mut self.data else { unreachable!() };
                for entry in arr.iter_mut() {
                    if entry.key.as_ref() == Some(key) {
                        *entry = KeyValuePair::default();
                        return true;
                    }
                }

                false
            }
            DataKind::Dictionary => {
                let Data::Dictionary(dic) = &mut self.data else { unreachable!() };
                dic.remove(key).is_some()
            }
            DataKind::Null => false,
        }
    }

    /// Removes every entry.
    ///
    /// As in the original, the value of an entry that is held in place is
    /// kept and becomes the value of the next key that
    /// [`get_value_ref_or_add_default`](Self::get_value_ref_or_add_default)
    /// adds in place.
    pub fn clear(&mut self) {
        match self.kind() {
            DataKind::Null => {}
            DataKind::Array => {
                let Data::Array(arr) = &mut self.data else { unreachable!() };
                for entry in arr.iter_mut() {
                    *entry = KeyValuePair::default();
                }
            }
            DataKind::Dictionary => {
                let Data::Dictionary(dic) = &mut self.data else { unreachable!() };
                dic.clear();
            }
            DataKind::Key => self.data = Data::Null,
        }
    }

    pub fn has_entries(&self) -> bool {
        !matches!(self.data, Data::Null)
    }

    pub fn try_get_value(&self, key: &TKey) -> Option<&TValue> {
        match &self.data {
            Data::Key(data) => {
                if data == key {
                    Some(&self.value)
                } else {
                    None
                }
            }
            Data::Array(arr) => arr.iter().find(|entry| entry.key.as_ref() == Some(key)).map(|entry| &entry.value),
            Data::Dictionary(dic) => dic.get(key),
            Data::Null => None,
        }
    }

    /// A reference to the value of a key, or `None` (the original's null
    /// reference) when the dictionary does not have the key.
    pub fn get_value_ref_or_null_ref(&mut self, key: &TKey) -> Option<&mut TValue> {
        match self.kind() {
            DataKind::Key => {
                let same = matches!(&self.data, Data::Key(data) if data == key);
                if same {
                    Some(&mut self.value)
                } else {
                    None
                }
            }
            DataKind::Array => {
                let Data::Array(arr) = &mut self.data else { unreachable!() };
                arr.iter_mut().find(|entry| entry.key.as_ref() == Some(key)).map(|entry| &mut entry.value)
            }
            DataKind::Dictionary => {
                let Data::Dictionary(dic) = &mut self.data else { unreachable!() };
                dic.get_mut(key)
            }
            DataKind::Null => None,
        }
    }

    /// A reference to the value of a key, which is added with the default
    /// value when the dictionary does not have it, and whether it existed.
    pub fn get_value_ref_or_add_default(&mut self, key: TKey) -> (&mut TValue, bool) {
        match self.kind() {
            DataKind::Null => {
                self.data = Data::Key(key);
                (&mut self.value, false)
            }
            // Single element
            DataKind::Key => {
                let same = matches!(&self.data, Data::Key(data) if *data == key);
                if same {
                    return (&mut self.value, true);
                }

                // This is a second element, convert to array
                // We have a single element, upgrade to array
                let Data::Key(data) = std::mem::replace(&mut self.data, Data::Null) else { unreachable!() };
                let mut arr = Self::new_array();
                arr[0] = KeyValuePair::new(data, std::mem::take(&mut self.value));
                arr[1] = KeyValuePair::new(key, TValue::default());
                self.data = Data::Array(arr);
                let Data::Array(arr) = &mut self.data else { unreachable!() };
                (&mut arr[1].value, false)
            }
            // Small array
            DataKind::Array => {
                // Try to find the element and look for the first free slot while we are at it
                let mut free: Option<usize> = None;
                let mut existing: Option<usize> = None;
                if let Data::Array(arr) = &self.data {
                    for (c, entry) in arr.iter().enumerate() {
                        if entry.key.as_ref() == Some(&key) {
                            existing = Some(c);
                            break;
                        } else if free.is_none() && entry.key.is_none() {
                            free = Some(c);
                        }
                    }
                }

                if let Some(c) = existing {
                    let Data::Array(arr) = &mut self.data else { unreachable!() };
                    return (&mut arr[c].value, true);
                }

                // There is a free slot, use it
                if let Some(free) = free {
                    let Data::Array(arr) = &mut self.data else { unreachable!() };
                    arr[free] = KeyValuePair::new(key, TValue::default());
                    return (&mut arr[free].value, false);
                }

                // Upgrade to dictionary
                let Data::Array(arr) = std::mem::replace(&mut self.data, Data::Null) else { unreachable!() };
                self.data = Data::Dictionary(Self::upgrade_to_dictionary(arr));
                Self::dictionary_value_ref_or_add_default(&mut self.data, key)
            }
            DataKind::Dictionary => Self::dictionary_value_ref_or_add_default(&mut self.data, key),
        }
    }

    fn dictionary_value_ref_or_add_default(data: &mut Data<TKey, TValue>, key: TKey) -> (&mut TValue, bool) {
        let Data::Dictionary(dic) = data else { unreachable!() };
        match dic.entry(key) {
            Entry::Occupied(entry) => (entry.into_mut(), true),
            Entry::Vacant(entry) => (entry.insert(TValue::default()), false),
        }
    }

    /// Removes an entry and returns its value.
    ///
    /// As in the original, an entry of a dictionary that has grown into a
    /// hash map is removed without its value being returned.
    pub fn try_get_and_remove_value(&mut self, key: &TKey) -> Option<TValue> {
        match self.kind() {
            DataKind::Key => {
                let same = matches!(&self.data, Data::Key(data) if data == key);
                if same {
                    let value = std::mem::take(&mut self.value);
                    self.data = Data::Null;
                    return Some(value);
                }

                None
            }
            DataKind::Array => {
                let Data::Array(arr) = &mut self.data else { unreachable!() };
                for entry in arr.iter_mut() {
                    if entry.key.as_ref() == Some(key) {
                        let removed = std::mem::take(entry);
                        return Some(removed.value);
                    }
                }

                None
            }
            DataKind::Dictionary => {
                let Data::Dictionary(dic) = &mut self.data else { unreachable!() };
                dic.remove(key);

                None
            }
            DataKind::Null => None,
        }
    }

    /// Removes an entry and returns its value.
    ///
    /// Panics when [`try_get_and_remove_value`](Self::try_get_and_remove_value)
    /// returns nothing.
    pub fn get_and_remove(&mut self, key: &TKey) -> TValue {
        match self.try_get_and_remove_value(key) {
            Some(v) => v,
            None => throw_key_not_found(),
        }
    }

    pub fn get_enumerator(&self) -> InlineDictionaryEnumerator<'_, TKey, TValue> {
        InlineDictionaryEnumerator::new(self)
    }
}

impl<'a, TKey: Clone + Eq + Hash, TValue: Default> IntoIterator for &'a InlineDictionary<TKey, TValue> {
    type Item = (&'a TKey, &'a TValue);
    type IntoIter = InlineDictionaryEnumerator<'a, TKey, TValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.get_enumerator()
    }
}

fn throw_key_not_found() -> ! {
    panic!("The given key was not present in the dictionary.")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnumeratorType {
    Empty,
    Single,
    Array,
    Dictionary,
}

/// The enumerator of an [`InlineDictionary`] (the original's nested
/// `Enumerator`). It is an `Iterator` as well.
pub struct InlineDictionaryEnumerator<'a, TKey, TValue> {
    dictionary: Option<&'a HashMap<TKey, TValue>>,
    inner: Option<hash_map::Iter<'a, TKey, TValue>>,
    inner_current: Option<(&'a TKey, &'a TValue)>,
    arr: Option<&'a [KeyValuePair<TKey, TValue>]>,
    first: Option<(&'a TKey, &'a TValue)>,
    index: i32,
    type_: EnumeratorType,
}

impl<'a, TKey, TValue> InlineDictionaryEnumerator<'a, TKey, TValue> {
    pub fn new(parent: &'a InlineDictionary<TKey, TValue>) -> Self {
        let mut result = Self {
            dictionary: None,
            inner: None,
            inner_current: None,
            arr: None,
            first: None,
            index: -1,
            type_: EnumeratorType::Empty,
        };
        match &parent.data {
            Data::Dictionary(inner) => {
                result.dictionary = Some(inner);
                result.inner = Some(inner.iter());
                result.type_ = EnumeratorType::Dictionary;
            }
            Data::Array(arr) => {
                result.type_ = EnumeratorType::Array;
                result.arr = Some(arr.as_slice());
            }
            Data::Key(key) => {
                result.type_ = EnumeratorType::Single;
                result.first = Some((key, &parent.value));
            }
            Data::Null => result.type_ = EnumeratorType::Empty,
        }
        result
    }

    pub fn move_next(&mut self) -> bool {
        match self.type_ {
            EnumeratorType::Single => {
                if self.index != -1 {
                    return false;
                }
                self.index = 0;
                true
            }
            EnumeratorType::Array => {
                let Some(arr) = self.arr else { return false };
                let mut next = self.index + 1;
                while (next as usize) < arr.len() {
                    if arr[next as usize].key.is_some() {
                        self.index = next;
                        return true;
                    }
                    next += 1;
                }
                false
            }
            EnumeratorType::Dictionary => {
                self.inner_current = match &mut self.inner {
                    Some(inner) => inner.next(),
                    None => None,
                };
                self.inner_current.is_some()
            }
            EnumeratorType::Empty => false,
        }
    }

    pub fn reset(&mut self) {
        self.index = -1;
        if self.type_ == EnumeratorType::Dictionary {
            self.inner = self.dictionary.map(|dictionary| dictionary.iter());
            self.inner_current = None;
        }
    }

    /// The current entry.
    ///
    /// Panics when the enumerator is not positioned on an entry.
    pub fn current(&self) -> (&'a TKey, &'a TValue) {
        let current = match self.type_ {
            EnumeratorType::Single => self.first,
            EnumeratorType::Array => match self.arr {
                Some(arr) if self.index >= 0 && (self.index as usize) < arr.len() => {
                    let entry = &arr[self.index as usize];
                    entry.key.as_ref().map(|key| (key, &entry.value))
                }
                _ => None,
            },
            EnumeratorType::Dictionary => self.inner_current,
            EnumeratorType::Empty => None,
        };
        match current {
            Some(current) => current,
            None => panic!("Operation is not valid due to the current state of the object."),
        }
    }

    pub fn dispose(&mut self) {}
}

impl<'a, TKey, TValue> Iterator for InlineDictionaryEnumerator<'a, TKey, TValue> {
    type Item = (&'a TKey, &'a TValue);

    fn next(&mut self) -> Option<Self::Item> {
        if self.move_next() {
            Some(self.current())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(dic: &InlineDictionary<&'static str, i32>) -> Vec<(&'static str, i32)> {
        dic.get_enumerator().map(|(key, value)| (*key, *value)).collect()
    }

    #[test]
    fn set_twice_with_single_item_works() {
        let mut dic = InlineDictionary::<&'static str, i32>::new();
        dic.set("foo", 1);
        assert_eq!(1, *dic.get(&"foo"));

        dic.set("foo", 2);
        assert_eq!(2, *dic.get(&"foo"));
    }

    #[test]
    fn enumeration_after_add_with_internal_array_works() {
        let mut dic = InlineDictionary::<&'static str, i32>::new();
        dic.add("foo", 1);
        dic.add("bar", 2);
        dic.add("baz", 3);

        assert_eq!(vec![("foo", 1), ("bar", 2), ("baz", 3)], entries(&dic));
    }

    #[test]
    fn enumeration_after_remove_with_internal_array_works() {
        let mut dic = InlineDictionary::<&'static str, i32>::new();
        dic.add("foo", 1);
        dic.add("bar", 2);
        dic.add("baz", 3);

        assert_eq!(vec![("foo", 1), ("bar", 2), ("baz", 3)], entries(&dic));

        dic.remove(&"bar");

        assert_eq!(vec![("foo", 1), ("baz", 3)], entries(&dic));
    }

    // The tests below are not from upstream.

    #[test]
    fn grows_into_a_dictionary_and_keeps_its_entries() {
        let names = ["a", "b", "c", "d", "e", "f", "g", "h"];
        let mut dic = InlineDictionary::<&'static str, i32>::new();
        assert!(!dic.has_entries());
        for (index, name) in names.iter().enumerate() {
            dic.add(*name, index as i32);
        }
        assert!(dic.has_entries());

        for (index, name) in names.iter().enumerate() {
            assert_eq!(Some(&(index as i32)), dic.try_get_value(name));
        }
        assert_eq!(None, dic.try_get_value(&"z"));

        let mut all = entries(&dic);
        all.sort();
        assert_eq!(names.len(), all.len());
        assert_eq!(("a", 0), all[0]);

        assert!(dic.remove(&"a"));
        assert!(!dic.remove(&"a"));
        dic.clear();
        assert_eq!(None, dic.try_get_value(&"b"));
    }

    #[test]
    fn value_references_add_and_find_entries_in_every_state() {
        let mut dic = InlineDictionary::<i32, i32>::new();
        for key in 0..10 {
            assert!(dic.get_value_ref_or_null_ref(&key).is_none());
            let (value, exists) = dic.get_value_ref_or_add_default(key);
            assert!(!exists);
            assert_eq!(0, *value);
            *value = key * 10;

            let (value, exists) = dic.get_value_ref_or_add_default(key);
            assert!(exists);
            assert_eq!(key * 10, *value);
        }

        for key in 0..10 {
            assert_eq!(Some(&mut (key * 10)), dic.get_value_ref_or_null_ref(&key));
        }
    }

    #[test]
    fn get_and_remove_returns_the_value_of_an_entry_held_in_place_or_in_the_array() {
        let mut dic = InlineDictionary::<i32, i32>::new();
        dic.add(1, 10);
        assert_eq!(10, dic.get_and_remove(&1));
        assert!(!dic.has_entries());

        dic.add(1, 10);
        dic.add(2, 20);
        assert_eq!(Some(20), dic.try_get_and_remove_value(&2));
        assert_eq!(None, dic.try_get_and_remove_value(&2));
        assert_eq!(Some(&10), dic.try_get_value(&1));
    }

    #[test]
    #[should_panic(expected = "Key already exists in dictionary")]
    fn add_throws_for_a_key_of_the_array() {
        let mut dic = InlineDictionary::<i32, i32>::new();
        dic.add(1, 10);
        dic.add(2, 20);
        dic.add(2, 21);
    }

    #[test]
    #[should_panic(expected = "The given key was not present in the dictionary.")]
    fn get_throws_for_a_missing_key() {
        let dic = InlineDictionary::<i32, i32>::new();
        dic.get(&1);
    }
}
