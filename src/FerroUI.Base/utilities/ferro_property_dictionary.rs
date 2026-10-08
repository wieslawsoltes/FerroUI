use crate::FerroProperty;

/// Stores values with [`FerroProperty`] as key.
///
/// This struct is very similar to a sorted list, with the following
/// differences:
/// - The key is the id of the property, allowing very fast lookups.
/// - The entries are stored in one array, which is the only heap allocation.
/// - It is a plain value: the owner mutates it in place.
pub struct FerroPropertyDictionary<TValue> {
    entries: Vec<Entry<TValue>>,
}

struct Entry<TValue> {
    id: u32,
    value: TValue,
}

impl<TValue> Entry<TValue> {
    fn new(property: &FerroProperty, value: TValue) -> Self {
        Self { id: property.id(), value }
    }
}

const DEFAULT_INITIAL_CAPACITY: usize = 4;

impl<TValue> Default for FerroPropertyDictionary<TValue> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TValue> FerroPropertyDictionary<TValue> {
    /// Initializes a new instance.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Initializes a new instance with the specified initial capacity.
    pub fn new_with_capacity(capacity: usize) -> Self {
        Self { entries: Vec::with_capacity(capacity) }
    }

    /// Gets the number of items in the collection.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Gets the value associated with the specified property (the getter of
    /// the indexer by property).
    ///
    /// Panics when the property is not found.
    pub fn get(&self, property: &FerroProperty) -> &TValue {
        match self.find_entry(property.id()) {
            Ok(index) => &self.entries[index].value,
            Err(_) => throw_not_found(),
        }
    }

    /// Sets the value associated with the specified property (the setter of
    /// the indexer by property). An existing value is replaced.
    pub fn set(&mut self, property: &FerroProperty, value: TValue) {
        match self.find_entry(property.id()) {
            Ok(index) => self.entries[index] = Entry::new(property, value),
            Err(index) => self.insert_entry(Entry::new(property, value), index),
        }
    }

    /// Gets the value at the specified index (the indexer by index).
    ///
    /// Panics when the index is not less than [`count`](Self::count).
    pub fn get_at(&self, index: usize) -> &TValue {
        if index >= self.entries.len() {
            throw_out_of_range();
        }
        &self.entries[index].value
    }

    /// Adds the specified property and value to the dictionary.
    ///
    /// Panics when the dictionary has the property already.
    pub fn add(&mut self, property: &FerroProperty, value: TValue) {
        match self.find_entry(property.id()) {
            Ok(_) => throw_duplicate(),
            Err(index) => self.insert_entry(Entry::new(property, value), index),
        }
    }

    /// Removes all property and values from the dictionary.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Determines whether the dictionary contains the specified property.
    pub fn contains_key(&self, property: &FerroProperty) -> bool {
        self.find_entry(property.id()).is_ok()
    }

    /// Gets the value at the specified index.
    ///
    /// Panics when the index is not less than [`count`](Self::count).
    pub fn get_value(&self, index: usize) -> &TValue {
        if index >= self.entries.len() {
            throw_out_of_range();
        }
        &self.entries[index].value
    }

    /// Removes the value with the specified property from the dictionary.
    ///
    /// Returns true if the property was found and removed; otherwise false.
    pub fn remove(&mut self, property: &FerroProperty) -> bool {
        match self.find_entry(property.id()) {
            Ok(index) => {
                self.remove_at(index);
                true
            }
            Err(_) => false,
        }
    }

    /// Removes the value with the specified property from the dictionary
    /// and returns it, or `None` when the property was not found.
    pub fn remove_value(&mut self, property: &FerroProperty) -> Option<TValue> {
        match self.find_entry(property.id()) {
            Ok(index) => Some(self.entries.remove(index).value),
            Err(_) => None,
        }
    }

    /// Removes the element at the specified index from the dictionary.
    ///
    /// Panics when the index is not less than [`count`](Self::count).
    pub fn remove_at(&mut self, index: usize) {
        if index >= self.entries.len() {
            throw_out_of_range();
        }

        self.entries.remove(index);
    }

    /// Attempts to add the specified property and value to the dictionary.
    ///
    /// Returns false when the dictionary has the property already.
    pub fn try_add(&mut self, property: &FerroProperty, value: TValue) -> bool {
        match self.find_entry(property.id()) {
            Ok(_) => false,
            Err(index) => {
                self.insert_entry(Entry::new(property, value), index);
                true
            }
        }
    }

    /// Gets the value associated with the specified property, or `None`
    /// when the dictionary does not contain it.
    #[inline]
    pub fn try_get_value(&self, property: &FerroProperty) -> Option<&TValue> {
        match self.find_entry(property.id()) {
            Ok(index) => Some(&self.entries[index].value),
            Err(_) => None,
        }
    }

    /// The index of the entry of a property id, or, as the error, the index
    /// at which an entry for it is to be inserted (the original's
    /// complement of that index).
    #[inline]
    fn find_entry(&self, property_id: u32) -> Result<usize, usize> {
        let mut lo: isize = 0;
        let mut hi: isize = self.entries.len() as isize - 1;

        while lo <= hi {
            // hi and lo are never negative here
            let i = ((hi as usize) + (lo as usize)) >> 1;

            let entry_id = self.entries[i].id;
            if entry_id == property_id {
                return Ok(i);
            }

            if entry_id < property_id {
                lo = i as isize + 1;
            } else {
                hi = i as isize - 1;
            }
        }

        Err(lo as usize)
    }

    /// Inserts an entry, growing the array as the original does: to the
    /// default capacity first, from there to twice of it and then by half.
    fn insert_entry(&mut self, entry: Entry<TValue>, entry_index: usize) {
        let entry_count = self.entries.len();
        if entry_count > 0 {
            if entry_count == self.entries.capacity() {
                let new_size = if entry_count == DEFAULT_INITIAL_CAPACITY {
                    DEFAULT_INITIAL_CAPACITY * 2
                } else {
                    (entry_count as f64 * 1.5) as usize
                };

                // A capacity the original cannot grow from (one entry in an
                // array of one) grows by one entry here.
                self.entries.reserve_exact(new_size.max(entry_count + 1) - entry_count);
            }
        } else if self.entries.capacity() == 0 {
            self.entries.reserve_exact(DEFAULT_INITIAL_CAPACITY);
        }

        self.entries.insert(entry_index, entry);
    }
}

fn throw_out_of_range() -> ! {
    panic!("Index was outside the bounds of the array.")
}

fn throw_duplicate() -> ! {
    panic!("An item with the same key has already been added.")
}

fn throw_not_found() -> ! {
    panic!("The given key was not present in the dictionary.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FerroObject, StyledProperty, StyledPropertyMetadata};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    thread_local! {
        static TEST_PROPERTIES: Vec<&'static FerroProperty> = create_test_properties();
    }

    fn create_test_properties() -> Vec<&'static FerroProperty> {
        let mut result: Vec<&'static FerroProperty> = Vec::with_capacity(100);

        for i in 0..100 {
            let property: &'static StyledProperty<String> = StyledProperty::create(
                &format!("Test{i}"),
                FerroObject::TYPE,
                FerroObject::TYPE,
                StyledPropertyMetadata::new(Some(String::new())),
                false,
                None,
                None,
                false,
            );
            result.push(property.as_property());
        }

        shuffle(&mut result, 42);
        result
    }

    fn test_property(index: usize) -> &'static FerroProperty {
        TEST_PROPERTIES.with(|properties| properties[index])
    }

    const COUNTS: [usize; 6] = [0, 1, 10, 13, 50, 72];

    fn value(index: usize) -> String {
        format!("Value{index}")
    }

    fn create_target(items: usize) -> FerroPropertyDictionary<String> {
        let mut result = FerroPropertyDictionary::<String>::new();

        for i in 0..items {
            result.add(test_property(i), value(i));
        }

        result
    }

    /// The original shuffles with the random numbers of its runtime; the
    /// order here comes from a linear congruential generator.
    fn shuffle<T>(array: &mut [T], seed: u64) {
        let mut state = seed;
        let mut n = array.len();
        while n > 1 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let k = ((state >> 33) as usize) % n;
            n -= 1;
            array.swap(n, k);
        }
    }

    fn throws<R>(f: impl FnOnce() -> R) -> bool {
        catch_unwind(AssertUnwindSafe(f)).is_err()
    }

    #[test]
    fn property_indexer_finds_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let target = create_target(count);
            let index = count / 2;
            let property = test_property(index);
            let result = target.get(property);

            assert_eq!(&value(index), result);
        }
    }

    #[test]
    fn property_indexer_throws_if_value_not_found() {
        for count in COUNTS {
            let target = create_target(count);
            let index = count;
            let property = test_property(index);

            assert!(throws(|| target.get(property).clone()));
        }
    }

    #[test]
    fn property_indexer_adds_new_value() {
        for count in COUNTS {
            let mut target = create_target(count);
            let index = count;
            let property = test_property(index);

            target.set(property, String::from("new"));

            assert_eq!("new", target.get(property));
        }
    }

    #[test]
    fn property_indexer_sets_existing_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let mut target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert_eq!(&value(index), target.get(property));

            target.set(property, String::from("new"));

            assert_eq!("new", target.get(property));
        }
    }

    #[test]
    fn int_indexer_finds_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let target = create_target(count);
            let index = count / 2;
            let result = target.get_at(index);

            assert!(result.starts_with("Value"));
        }
    }

    #[test]
    fn int_indexer_throws_if_index_out_of_range() {
        for count in COUNTS {
            let target = create_target(count);
            let index = count;

            assert!(throws(|| target.get_at(index).clone()));
        }
    }

    #[test]
    fn add_adds_new_value() {
        for count in COUNTS {
            let mut target = create_target(count);
            let index = count;
            let property = test_property(index);

            target.add(property, String::from("new"));

            assert_eq!("new", target.get(property));
        }
    }

    #[test]
    fn add_throws_if_key_exists() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let mut target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert!(throws(|| target.add(property, String::from("new"))));
        }
    }

    #[test]
    fn contains_key_returns_true_if_value_exists() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert!(target.contains_key(property));
        }
    }

    #[test]
    fn contains_key_returns_false_if_value_does_not_exist() {
        for count in COUNTS {
            let target = create_target(count);
            let index = count;
            let property = test_property(index);

            assert!(!target.contains_key(property));
        }
    }

    #[test]
    fn get_value_finds_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let target = create_target(count);
            let index = count / 2;

            let value = target.get_value(index);

            assert!(value.starts_with("Value"));
        }
    }

    #[test]
    fn get_value_throws_if_index_out_of_range() {
        for count in COUNTS {
            let target = create_target(count);
            let index = count;

            assert!(throws(|| target.get_value(index).clone()));
        }
    }

    #[test]
    fn remove_removes_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let mut target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert!(target.remove(property));
            assert!(!target.contains_key(property));
        }
    }

    #[test]
    fn remove_returns_false_if_value_not_present() {
        for count in COUNTS {
            let mut target = create_target(count);
            let index = count;
            let property = test_property(index);

            assert!(!target.remove(property));
        }
    }

    #[test]
    fn remove_returns_existing_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let mut target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert_eq!(Some(value(index)), target.remove_value(property));
        }
    }

    #[test]
    fn try_add_adds_new_value() {
        for count in COUNTS {
            let mut target = create_target(count);
            let index = count;
            let property = test_property(index);

            assert!(target.try_add(property, String::from("new")));

            assert_eq!("new", target.get(property));
        }
    }

    #[test]
    fn try_add_returns_false_if_key_exists() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let mut target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert!(!target.try_add(property, String::from("new")));
        }
    }

    #[test]
    fn try_get_value_finds_value() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let target = create_target(count);
            let index = count / 2;
            let property = test_property(index);

            assert_eq!(Some(&value(index)), target.try_get_value(property));
        }
    }

    #[test]
    fn try_get_value_returns_false_if_key_does_not_exist() {
        for count in COUNTS {
            if count == 0 {
                continue;
            }

            let target = create_target(count);
            let index = count;
            let property = test_property(index);

            assert_eq!(None, target.try_get_value(property));
        }
    }

    // Not from upstream.
    #[test]
    fn entries_are_ordered_by_property_id_and_removed_by_index() {
        let mut target = FerroPropertyDictionary::<u32>::new_with_capacity(1);
        for index in 0..20 {
            let property = test_property(index);
            target.add(property, property.id());
        }
        assert_eq!(20, target.count());
        for index in 1..target.count() {
            assert!(target.get_at(index - 1) < target.get_at(index));
        }

        let first = *target.get_at(0);
        target.remove_at(0);
        assert_eq!(19, target.count());
        assert!(*target.get_at(0) > first);

        target.clear();
        assert_eq!(0, target.count());
    }
}
