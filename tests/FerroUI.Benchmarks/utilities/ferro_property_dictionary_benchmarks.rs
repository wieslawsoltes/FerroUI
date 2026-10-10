//! The dictionary of values by property against the store it replaced
//! upstream and against the hash map of the standard library: looking
//! values up, adding them, removing them and enumerating them.
//!
//! Differences from upstream that the language asks for:
//!
//! - The values upstream are null objects; here they are `None` of
//!   [`Object`].
//! - The mock properties upstream are created without being registered. A
//!   property is created here by registering it
//!   (`FerroProperty::register`), on the base object class, in the registry
//!   of the thread of the benchmark.
//! - The lookups hand what they found to `black_box`: upstream discards it,
//!   and a lookup nothing reads would not be measured here.

use crate::harness::Registry;
use ferroui_base::utilities::FerroPropertyDictionary;
use ferroui_base::{FerroObject, FerroProperty};
use std::any::Any;
use std::collections::HashMap;
use std::hint::black_box;
use std::rc::Rc;

/// The value type of the stores: an object reference that may be null.
pub type Object = Option<Rc<dyn Any>>;

/// The identifier no property has: the last entry of the old store.
const MAX_PROPERTY_ID: u32 = u32::MAX;

#[derive(Clone)]
struct Entry<TValue> {
    property_id: u32,
    value: TValue,
}

/// The store of values by property the dictionary replaced upstream; the
/// baseline of these benchmarks.
///
/// Upstream shares one empty array among all stores of a value type. A
/// generic type has no statics here, so the empty array is a member of the
/// store (`empty_entries`), and `entries` is `None` while the store is
/// empty: creating a store allocates nothing, as upstream.
pub struct FerroPropertyValueStoreOld<TValue> {
    // The last item in the list is always the maximum identifier.
    empty_entries: [Entry<TValue>; 1],
    entries: Option<Box<[Entry<TValue>]>>,
}

impl<TValue: Clone + Default> Default for FerroPropertyValueStoreOld<TValue> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TValue: Clone + Default> FerroPropertyValueStoreOld<TValue> {
    pub fn new() -> Self {
        Self { empty_entries: [Entry { property_id: MAX_PROPERTY_ID, value: TValue::default() }], entries: None }
    }

    #[inline]
    fn entries(&self) -> &[Entry<TValue>] {
        match &self.entries {
            Some(entries) => entries,
            None => &self.empty_entries,
        }
    }

    pub fn count(&self) -> usize {
        self.entries().len() - 1
    }

    /// The value at an index (the indexer of the upstream type).
    pub fn get_at(&self, index: usize) -> &TValue {
        &self.entries()[index].value
    }

    fn try_find_entry(&self, property_id: u32) -> (usize, bool) {
        let entries = self.entries();

        if entries.len() <= 12 {
            // For small lists, we use an optimized linear search. Since the last item in the list
            // is always the maximum identifier, we can skip a conditional branch in each iteration.
            // By unrolling the loop, we can skip another unconditional branch in each iteration.

            if entries[0].property_id >= property_id {
                return (0, entries[0].property_id == property_id);
            }
            if entries[1].property_id >= property_id {
                return (1, entries[1].property_id == property_id);
            }
            if entries[2].property_id >= property_id {
                return (2, entries[2].property_id == property_id);
            }
            if entries[3].property_id >= property_id {
                return (3, entries[3].property_id == property_id);
            }
            if entries[4].property_id >= property_id {
                return (4, entries[4].property_id == property_id);
            }
            if entries[5].property_id >= property_id {
                return (5, entries[5].property_id == property_id);
            }
            if entries[6].property_id >= property_id {
                return (6, entries[6].property_id == property_id);
            }
            if entries[7].property_id >= property_id {
                return (7, entries[7].property_id == property_id);
            }
            if entries[8].property_id >= property_id {
                return (8, entries[8].property_id == property_id);
            }
            if entries[9].property_id >= property_id {
                return (9, entries[9].property_id == property_id);
            }
            if entries[10].property_id >= property_id {
                return (10, entries[10].property_id == property_id);
            }
        } else {
            let mut low = 0_usize;
            let mut high = entries.len();
            let mut id;

            while high - low > 3 {
                let pivot = (high + low) / 2;
                id = entries[pivot].property_id;

                if property_id == id {
                    return (pivot, true);
                }

                if property_id <= id {
                    high = pivot;
                } else {
                    low = pivot + 1;
                }
            }

            loop {
                id = entries[low].property_id;

                if id == property_id {
                    return (low, true);
                }

                if id > property_id {
                    break;
                }

                low += 1;

                if low >= high {
                    break;
                }
            }
        }

        (0, false)
    }

    pub fn try_get_value(&self, property: &FerroProperty) -> Option<&TValue> {
        let (index, found) = self.try_find_entry(property.id());
        if !found {
            return None;
        }

        Some(&self.entries()[index].value)
    }

    pub fn add_value(&mut self, property: &FerroProperty, value: TValue) {
        let old = self.entries();
        let mut entries: Vec<Entry<TValue>> = Vec::with_capacity(old.len() + 1);

        for (i, entry) in old.iter().enumerate() {
            if entry.property_id > property.id() {
                if i > 0 {
                    entries.extend_from_slice(&old[..i]);
                }

                entries.push(Entry { property_id: property.id(), value });
                entries.extend_from_slice(&old[i..]);
                break;
            }
        }

        // Upstream leaves an array of empty entries when no entry has a
        // greater identifier, which the last entry rules out.
        if entries.is_empty() {
            entries.resize(old.len() + 1, Entry { property_id: 0, value: TValue::default() });
        }

        self.entries = Some(entries.into_boxed_slice());
    }

    pub fn set_value(&mut self, property: &FerroProperty, value: TValue) {
        let index = self.try_find_entry(property.id()).0;
        match &mut self.entries {
            Some(entries) => entries[index].value = value,
            None => self.empty_entries[index].value = value,
        }
    }

    pub fn remove(&mut self, property: &FerroProperty) {
        let (index, found) = self.try_find_entry(property.id());

        if found {
            let old = self.entries();
            let new_length = old.len() - 1;

            // Special case - one element left means that value store is empty so we can just reuse our "empty" array.
            if new_length == 1 {
                self.entries = None;

                return;
            }

            let mut entries: Vec<Entry<TValue>> = Vec::with_capacity(new_length);

            for (i, entry) in old.iter().enumerate() {
                if i != index {
                    entries.push(entry.clone());
                }
            }

            self.entries = Some(entries.into_boxed_slice());
        }
    }
}

/// The mock property of upstream: a styled property of an integer whose
/// owner is the base object class.
fn mock_property(name: &str) -> &'static FerroProperty {
    FerroProperty::register::<FerroObject, i32>(name, 0).as_property()
}

/// The pseudo-random numbers of a seeded generator of the upstream runtime
/// (its subtractive generator), so that the properties are shuffled into the
/// order they have upstream.
struct SeededRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl SeededRandom {
    const MBIG: i32 = i32::MAX;
    const MSEED: i32 = 161_803_398;

    fn new(seed: i32) -> Self {
        let mut seed_array = [0_i32; 56];

        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = Self::MSEED - subtraction;
        seed_array[55] = mj;
        let mut mk = 1_i32;

        let mut ii = 0_usize;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }

            seed_array[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += Self::MBIG;
            }

            mj = seed_array[ii];
        }

        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }

                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + n]);
                if seed_array[i] < 0 {
                    seed_array[i] += Self::MBIG;
                }
            }
        }

        Self { seed_array, inext: 0, inextp: 21 }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }

        let mut loc_inextp = self.inextp + 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }

        let mut ret_val = self.seed_array[loc_inext].wrapping_sub(self.seed_array[loc_inextp]);

        if ret_val == Self::MBIG {
            ret_val -= 1;
        }
        if ret_val < 0 {
            ret_val += Self::MBIG;
        }

        self.seed_array[loc_inext] = ret_val;
        self.inext = loc_inext;
        self.inextp = loc_inextp;

        ret_val
    }

    fn sample(&mut self) -> f64 {
        f64::from(self.internal_sample()) * (1.0 / f64::from(Self::MBIG))
    }

    /// A number that is not negative and less than `max_value`.
    fn next(&mut self, max_value: usize) -> usize {
        (self.sample() * max_value as f64) as usize
    }
}

fn shuffle<T>(array: &mut [T], seed: i32) {
    let mut rng = SeededRandom::new(seed);

    let mut n = array.len();
    while n > 1 {
        let k = rng.next(n);
        n -= 1;
        array.swap(n, k);
    }
}

/// The properties of the benchmarks.
pub struct MockProperties;

impl MockProperties {
    /// 32 mock properties in a shuffled order. They belong to the thread
    /// (properties do), which creates them when it first asks for them.
    pub fn shuffled_properties() -> Rc<[&'static FerroProperty]> {
        thread_local! {
            static SHUFFLED_PROPERTIES: Rc<[&'static FerroProperty]> = {
                let mut shuffled_properties: Vec<&'static FerroProperty> = Vec::with_capacity(32);

                for i in 0..32 {
                    shuffled_properties.push(mock_property(&format!("Property#{i}")));
                }

                shuffle(&mut shuffled_properties, 42);

                shuffled_properties.into()
            };
        }

        SHUFFLED_PROPERTIES.with(Rc::clone)
    }
}

/// The values of the parameter `PropertyCount` of every class of this file.
const PROPERTY_COUNTS: [usize; 5] = [2, 6, 10, 20, 30];

pub struct ValueStoreLookup {
    property_count: usize,
    properties: Rc<[&'static FerroProperty]>,
    store: FerroPropertyDictionary<Object>,
    old_store: FerroPropertyValueStoreOld<Object>,
    dictionary: HashMap<&'static FerroProperty, Object>,
}

impl ValueStoreLookup {
    pub fn new(property_count: usize) -> Self {
        let properties = MockProperties::shuffled_properties();

        let mut store = FerroPropertyDictionary::<Object>::new();
        let mut old_store = FerroPropertyValueStoreOld::<Object>::new();
        let mut dictionary: HashMap<&'static FerroProperty, Object> = HashMap::new();

        for i in 0..property_count {
            store.add(properties[i], None);
            old_store.add_value(properties[i], None);
            dictionary.insert(properties[i], None);
        }

        Self { property_count, properties, store, old_store, dictionary }
    }

    pub fn lookup_properties(&self) {
        for i in 0..self.property_count {
            black_box(self.store.try_get_value(self.properties[i]));
        }
    }

    pub fn lookup_properties_old(&self) {
        for i in 0..self.property_count {
            black_box(self.old_store.try_get_value(self.properties[i]));
        }
    }

    pub fn lookup_properties_dict(&self) {
        for i in 0..self.property_count {
            black_box(self.dictionary.get(self.properties[i]));
        }
    }
}

pub struct ValueStoreAddBenchmarks {
    property_count: usize,
    properties: Rc<[&'static FerroProperty]>,
}

impl ValueStoreAddBenchmarks {
    pub fn new(property_count: usize) -> Self {
        Self { property_count, properties: MockProperties::shuffled_properties() }
    }

    pub fn add(&self) {
        let mut store = FerroPropertyDictionary::<Object>::new();

        for i in 0..self.property_count {
            store.add(self.properties[i], None);
        }
    }

    pub fn add_old(&self) {
        let mut store = FerroPropertyValueStoreOld::<Object>::new();

        for i in 0..self.property_count {
            store.add_value(self.properties[i], None);
        }
    }

    pub fn add_dict(&self) {
        let mut store: HashMap<&'static FerroProperty, Object> = HashMap::new();

        for i in 0..self.property_count {
            store.insert(self.properties[i], None);
        }
    }
}

pub struct ValueStoreAddRemoveBenchmarks {
    property_count: usize,
    properties: Rc<[&'static FerroProperty]>,
}

impl ValueStoreAddRemoveBenchmarks {
    pub fn new(property_count: usize) -> Self {
        Self { property_count, properties: MockProperties::shuffled_properties() }
    }

    pub fn add_and_remove_value(&self) {
        let mut store = FerroPropertyDictionary::<Object>::new();

        for i in 0..self.property_count {
            store.add(self.properties[i], None);
        }

        for i in (0..self.property_count).rev() {
            store.remove(self.properties[i]);
        }
    }

    pub fn add_and_remove_value_old(&self) {
        let mut store = FerroPropertyValueStoreOld::<Object>::new();

        for i in 0..self.property_count {
            store.add_value(self.properties[i], None);
        }

        for i in (0..self.property_count).rev() {
            store.remove(self.properties[i]);
        }
    }

    pub fn add_and_remove_value_dict(&self) {
        let mut store: HashMap<&'static FerroProperty, Object> = HashMap::new();

        for i in 0..self.property_count {
            store.insert(self.properties[i], None);
        }

        for i in (0..self.property_count).rev() {
            store.remove(self.properties[i]);
        }
    }
}

pub struct ValueStoreAddRemoveInterleavedBenchmarks {
    property_count: usize,
    properties: Rc<[&'static FerroProperty]>,
}

impl ValueStoreAddRemoveInterleavedBenchmarks {
    pub fn new(property_count: usize) -> Self {
        Self { property_count, properties: MockProperties::shuffled_properties() }
    }

    pub fn add_and_remove_value_interleaved(&self) {
        let mut store = FerroPropertyDictionary::<Object>::new();

        for i in 0..self.property_count {
            store.add(self.properties[i], None);
            store.remove(self.properties[i]);
        }
    }

    pub fn add_and_remove_value_interleaved_old(&self) {
        let mut store = FerroPropertyValueStoreOld::<Object>::new();

        for i in 0..self.property_count {
            store.add_value(self.properties[i], None);
            store.remove(self.properties[i]);
        }
    }

    pub fn add_and_remove_value_interleaved_dict(&self) {
        let mut store: HashMap<&'static FerroProperty, Object> = HashMap::new();

        for i in 0..self.property_count {
            store.insert(self.properties[i], None);
            store.remove(self.properties[i]);
        }
    }
}

pub struct ValueStoreEnumeration {
    store: FerroPropertyDictionary<Object>,
    old_store: FerroPropertyValueStoreOld<Object>,
    dictionary: HashMap<&'static FerroProperty, Object>,
}

impl ValueStoreEnumeration {
    pub fn new(property_count: usize) -> Self {
        let properties = MockProperties::shuffled_properties();

        let mut store = FerroPropertyDictionary::<Object>::new();
        let mut old_store = FerroPropertyValueStoreOld::<Object>::new();
        let mut dictionary: HashMap<&'static FerroProperty, Object> = HashMap::new();

        for i in 0..property_count {
            store.add(properties[i], None);
            old_store.add_value(properties[i], None);
            dictionary.insert(properties[i], None);
        }

        Self { store, old_store, dictionary }
    }

    pub fn enumerate(&self) -> i32 {
        let mut result = 0;

        for i in 0..self.store.count() {
            result += if self.store.get_at(i).is_none() { 1 } else { 0 };
        }

        result
    }

    pub fn enumerate_old(&self) -> i32 {
        let mut result = 0;

        // The bound is the count of the new store, as upstream.
        for i in 0..self.store.count() {
            result += if self.old_store.get_at(i).is_none() { 1 } else { 0 };
        }

        result
    }

    pub fn enumerate_dict(&self) -> i32 {
        let mut result = 0;

        for i in &self.dictionary {
            result += if i.1.is_none() { 1 } else { 0 };
        }

        result
    }

    pub fn enumerate_dict_values(&self) -> i32 {
        let mut result = 0;

        for i in self.dictionary.values() {
            result += if i.is_none() { 1 } else { 0 };
        }

        result
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("utilities", "ValueStoreLookup");
    for count in PROPERTY_COUNTS {
        let parameters = format!("PropertyCount={count}");
        class.benchmark("lookup_properties", parameters.clone(), move || ValueStoreLookup::new(count), |b| {
            b.lookup_properties()
        });
        class
            .benchmark("lookup_properties_old", parameters.clone(), move || ValueStoreLookup::new(count), |b| {
                b.lookup_properties_old()
            })
            .baseline();
        class.benchmark("lookup_properties_dict", parameters, move || ValueStoreLookup::new(count), |b| {
            b.lookup_properties_dict()
        });
    }

    let mut class = registry.class("utilities", "ValueStoreAddBenchmarks");
    for count in PROPERTY_COUNTS {
        let parameters = format!("PropertyCount={count}");
        class.benchmark("add", parameters.clone(), move || ValueStoreAddBenchmarks::new(count), |b| b.add());
        class
            .benchmark("add_old", parameters.clone(), move || ValueStoreAddBenchmarks::new(count), |b| b.add_old())
            .baseline();
        class.benchmark("add_dict", parameters, move || ValueStoreAddBenchmarks::new(count), |b| b.add_dict());
    }

    let mut class = registry.class("utilities", "ValueStoreAddRemoveBenchmarks");
    for count in PROPERTY_COUNTS {
        let parameters = format!("PropertyCount={count}");
        class.benchmark(
            "add_and_remove_value",
            parameters.clone(),
            move || ValueStoreAddRemoveBenchmarks::new(count),
            |b| b.add_and_remove_value(),
        );
        class
            .benchmark(
                "add_and_remove_value_old",
                parameters.clone(),
                move || ValueStoreAddRemoveBenchmarks::new(count),
                |b| b.add_and_remove_value_old(),
            )
            .baseline();
        class.benchmark(
            "add_and_remove_value_dict",
            parameters,
            move || ValueStoreAddRemoveBenchmarks::new(count),
            |b| b.add_and_remove_value_dict(),
        );
    }

    let mut class = registry.class("utilities", "ValueStoreAddRemoveInterleavedBenchmarks");
    for count in PROPERTY_COUNTS {
        let parameters = format!("PropertyCount={count}");
        class.benchmark(
            "add_and_remove_value_interleaved",
            parameters.clone(),
            move || ValueStoreAddRemoveInterleavedBenchmarks::new(count),
            |b| b.add_and_remove_value_interleaved(),
        );
        class
            .benchmark(
                "add_and_remove_value_interleaved_old",
                parameters.clone(),
                move || ValueStoreAddRemoveInterleavedBenchmarks::new(count),
                |b| b.add_and_remove_value_interleaved_old(),
            )
            .baseline();
        class.benchmark(
            "add_and_remove_value_interleaved_dict",
            parameters,
            move || ValueStoreAddRemoveInterleavedBenchmarks::new(count),
            |b| b.add_and_remove_value_interleaved_dict(),
        );
    }

    let mut class = registry.class("utilities", "ValueStoreEnumeration");
    for count in PROPERTY_COUNTS {
        let parameters = format!("PropertyCount={count}");
        class.benchmark("enumerate", parameters.clone(), move || ValueStoreEnumeration::new(count), |b| b.enumerate());
        class
            .benchmark("enumerate_old", parameters.clone(), move || ValueStoreEnumeration::new(count), |b| {
                b.enumerate_old()
            })
            .baseline();
        class.benchmark("enumerate_dict", parameters.clone(), move || ValueStoreEnumeration::new(count), |b| {
            b.enumerate_dict()
        });
        class.benchmark("enumerate_dict_values", parameters, move || ValueStoreEnumeration::new(count), |b| {
            b.enumerate_dict_values()
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn value_store_lookup() {
        crate::harness::smoke_class(super::register, "ValueStoreLookup");
    }

    #[test]
    fn value_store_add_benchmarks() {
        crate::harness::smoke_class(super::register, "ValueStoreAddBenchmarks");
    }

    #[test]
    fn value_store_add_remove_benchmarks() {
        crate::harness::smoke_class(super::register, "ValueStoreAddRemoveBenchmarks");
    }

    #[test]
    fn value_store_add_remove_interleaved_benchmarks() {
        crate::harness::smoke_class(super::register, "ValueStoreAddRemoveInterleavedBenchmarks");
    }

    #[test]
    fn value_store_enumeration() {
        crate::harness::smoke_class(super::register, "ValueStoreEnumeration");
    }
}
