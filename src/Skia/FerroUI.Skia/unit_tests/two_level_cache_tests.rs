//! Port of upstream's `TwoLevelCacheTests.cs` of the Skia unit tests.
//!
//! Upstream stores `new object()` values and compares them with
//! `Assert.Same`; here they are `Arc<Object>` compared with `Arc::ptr_eq`.
//! `TryGet(key, out value)` is `try_get(&key) -> Option<TValue>`, and a
//! case-insensitive `StringComparer.OrdinalIgnoreCase` is a comparer
//! closure.

use crate::TwoLevelCache;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

struct Object;

fn new_object() -> Arc<Object> {
    Arc::new(Object)
}

type Evicted = Arc<Mutex<Vec<Arc<Object>>>>;

fn eviction_action(evicted_values: &Evicted) -> Option<Box<dyn Fn(&Arc<Object>) + Send>> {
    let evicted_values = evicted_values.clone();
    Some(Box::new(move |v: &Arc<Object>| evicted_values.lock().unwrap().push(v.clone())))
}

fn ordinal_ignore_case() -> Option<Box<dyn Fn(&String, &String) -> bool + Send>> {
    Some(Box::new(|a: &String, b: &String| a.eq_ignore_ascii_case(b)))
}

fn contains(values: &Evicted, value: &Arc<Object>) -> bool {
    values.lock().unwrap().iter().any(|v| Arc::ptr_eq(v, value))
}

#[test]
#[should_panic]
fn constructor_with_negative_secondary_size_throws_argument_out_of_range_exception() {
    TwoLevelCache::<String, Arc<Object>>::with_secondary_size(-1);
}

#[test]
fn constructor_with_zero_secondary_size_does_not_throw() {
    let _cache = TwoLevelCache::<String, Arc<Object>>::with_secondary_size(0);
}

#[test]
fn try_get_empty_cache_returns_false() {
    let cache = TwoLevelCache::<String, Arc<Object>>::new();

    let result = cache.try_get(&"key".to_string());

    assert!(result.is_none());
}

#[test]
fn get_or_add_first_item_stores_in_primary() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::new();
    let value = new_object();

    let result = cache.get_or_add("key1".to_string(), |_| value.clone());

    assert!(Arc::ptr_eq(&value, &result));
    let retrieved = cache.try_get(&"key1".to_string());
    assert!(retrieved.is_some());
    assert!(Arc::ptr_eq(&value, &retrieved.unwrap()));
}

#[test]
fn get_or_add_same_key_returns_existing_value() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::new();
    let value1 = new_object();
    let value2 = new_object();

    cache.get_or_add("key".to_string(), |_| value1.clone());
    let result = cache.get_or_add("key".to_string(), |_| value2.clone());

    assert!(Arc::ptr_eq(&value1, &result));
}

#[test]
fn get_or_add_second_item_stores_in_secondary() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_secondary_size(3);
    let value1 = new_object();
    let value2 = new_object();

    cache.get_or_add("key1".to_string(), |_| value1.clone());
    cache.get_or_add("key2".to_string(), |_| value2.clone());

    let retrieved1 = cache.try_get(&"key1".to_string());
    assert!(retrieved1.is_some());
    assert!(Arc::ptr_eq(&value1, &retrieved1.unwrap()));
    let retrieved2 = cache.try_get(&"key2".to_string());
    assert!(retrieved2.is_some());
    assert!(Arc::ptr_eq(&value2, &retrieved2.unwrap()));
}

#[test]
fn get_or_add_multiple_items_stores_correctly() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_secondary_size(3);
    let mut values = Vec::new();
    for i in 0..4 {
        values.push(new_object());
        cache.get_or_add(format!("key{i}"), |_| values[i].clone());
    }

    // All should be retrievable
    for (i, value) in values.iter().enumerate() {
        let retrieved = cache.try_get(&format!("key{i}"));
        assert!(retrieved.is_some(), "key{i}");
        assert!(Arc::ptr_eq(value, &retrieved.unwrap()), "key{i}");
    }
}

#[test]
fn get_or_add_exceeds_capacity_calls_eviction_action() {
    let evicted_values: Evicted = Arc::default();
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_options(2, eviction_action(&evicted_values), None);

    let value1 = new_object();
    let value2 = new_object();
    let value3 = new_object();
    let value4 = new_object();

    cache.get_or_add("key1".to_string(), |_| value1.clone());
    cache.get_or_add("key2".to_string(), |_| value2.clone());
    cache.get_or_add("key3".to_string(), |_| value3.clone());

    // No evictions yet
    assert!(evicted_values.lock().unwrap().is_empty());

    // This should cause eviction
    cache.get_or_add("key4".to_string(), |_| value4.clone());

    assert_eq!(1, evicted_values.lock().unwrap().len());
    assert!(Arc::ptr_eq(&value2, &evicted_values.lock().unwrap()[0]));
}

#[test]
fn get_or_add_zero_secondary_size_evicts_primary_immediately() {
    let evicted_values: Evicted = Arc::default();
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_options(0, eviction_action(&evicted_values), None);

    let value1 = new_object();
    let value2 = new_object();

    cache.get_or_add("key1".to_string(), |_| value1.clone());
    cache.get_or_add("key2".to_string(), |_| value2.clone());

    assert_eq!(1, evicted_values.lock().unwrap().len());
    assert!(Arc::ptr_eq(&value1, &evicted_values.lock().unwrap()[0]));

    // Only the latest value should be retrievable
    assert!(cache.try_get(&"key1".to_string()).is_none());
    let retrieved = cache.try_get(&"key2".to_string());
    assert!(retrieved.is_some());
    assert!(Arc::ptr_eq(&value2, &retrieved.unwrap()));
}

#[test]
fn get_or_add_duplicate_key_returns_existing_without_calling_factory() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::new();
    let value1 = new_object();
    let factory_called = Cell::new(false);

    // Add initial value
    cache.get_or_add("key".to_string(), |_| value1.clone());

    // Try to add again - factory should not be called
    let result = cache.get_or_add("key".to_string(), |_| {
        factory_called.set(true);
        new_object()
    });

    // Should return first value without calling factory
    assert!(Arc::ptr_eq(&value1, &result));
    assert!(!factory_called.get());
}

#[test]
fn get_or_add_duplicate_key_in_secondary_returns_existing_without_calling_factory() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_secondary_size(2);
    let value1 = new_object();
    let value2 = new_object();
    let factory_called = Cell::new(false);

    cache.get_or_add("key1".to_string(), |_| value1.clone());
    cache.get_or_add("key2".to_string(), |_| value2.clone());

    // Try to add key2 again - factory should not be called
    let result = cache.get_or_add("key2".to_string(), |_| {
        factory_called.set(true);
        new_object()
    });

    assert!(Arc::ptr_eq(&value2, &result));
    assert!(!factory_called.get());
}

#[test]
fn clear_and_dispose_empty_cache_does_not_throw() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::new();
    cache.clear_and_dispose();
}

#[test]
fn clear_and_dispose_with_values_calls_eviction_action_for_all() {
    let evicted_values: Evicted = Arc::default();
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_options(2, eviction_action(&evicted_values), None);

    let value1 = new_object();
    let value2 = new_object();
    let value3 = new_object();

    cache.get_or_add("key1".to_string(), |_| value1.clone());
    cache.get_or_add("key2".to_string(), |_| value2.clone());
    cache.get_or_add("key3".to_string(), |_| value3.clone());

    cache.clear_and_dispose();

    assert_eq!(3, evicted_values.lock().unwrap().len());
    assert!(contains(&evicted_values, &value1));
    assert!(contains(&evicted_values, &value2));
    assert!(contains(&evicted_values, &value3));
}

#[test]
fn clear_and_dispose_clears_all_entries() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_secondary_size(2);

    cache.get_or_add("key1".to_string(), |_| new_object());
    cache.get_or_add("key2".to_string(), |_| new_object());
    cache.clear_and_dispose();

    assert!(cache.try_get(&"key1".to_string()).is_none());
    assert!(cache.try_get(&"key2".to_string()).is_none());
}

#[test]
fn get_or_add_with_custom_comparer_uses_comparer() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_options(3, None, ordinal_ignore_case());

    let value = new_object();
    cache.get_or_add("KEY".to_string(), |_| value.clone());

    let retrieved = cache.try_get(&"key".to_string());
    assert!(retrieved.is_some());
    assert!(Arc::ptr_eq(&value, &retrieved.unwrap()));
}

#[test]
fn try_get_with_custom_comparer_uses_comparer() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_options(2, None, ordinal_ignore_case());

    let value1 = new_object();
    let value2 = new_object();

    cache.get_or_add("PRIMARY".to_string(), |_| value1.clone());
    cache.get_or_add("SECONDARY".to_string(), |_| value2.clone());

    let retrieved1 = cache.try_get(&"primary".to_string());
    assert!(retrieved1.is_some());
    assert!(Arc::ptr_eq(&value1, &retrieved1.unwrap()));
    let retrieved2 = cache.try_get(&"secondary".to_string());
    assert!(retrieved2.is_some());
    assert!(Arc::ptr_eq(&value2, &retrieved2.unwrap()));
}

#[test]
fn get_or_add_int_keys_works_correctly() {
    let mut cache = TwoLevelCache::<i32, Arc<Object>>::with_secondary_size(2);

    let value1 = new_object();
    let value2 = new_object();
    let value3 = new_object();

    cache.get_or_add(1, |_| value1.clone());
    cache.get_or_add(2, |_| value2.clone());
    cache.get_or_add(3, |_| value3.clone());

    let retrieved1 = cache.try_get(&1);
    assert!(retrieved1.is_some());
    assert!(Arc::ptr_eq(&value1, &retrieved1.unwrap()));
    let retrieved2 = cache.try_get(&2);
    assert!(retrieved2.is_some());
    assert!(Arc::ptr_eq(&value2, &retrieved2.unwrap()));
    let retrieved3 = cache.try_get(&3);
    assert!(retrieved3.is_some());
    assert!(Arc::ptr_eq(&value3, &retrieved3.unwrap()));
}

#[test]
fn get_or_add_rotates_secondary_correctly() {
    let evicted_values: Evicted = Arc::default();
    let mut cache = TwoLevelCache::<i32, Arc<Object>>::with_options(2, eviction_action(&evicted_values), None);

    let mut values = Vec::new();
    for i in 0..5 {
        values.push(new_object());
        cache.get_or_add(i as i32, |_| values[i].clone());
    }

    // Primary: 0, Secondary: [1, 2]
    // After adding 3: Primary: 0, Secondary: [3, 1] (evicts 2)
    // After adding 4: Primary: 0, Secondary: [4, 3] (evicts 1)

    assert_eq!(2, evicted_values.lock().unwrap().len());
    assert!(contains(&evicted_values, &values[2]));
    assert!(contains(&evicted_values, &values[1]));

    // These should still be in cache
    assert!(cache.try_get(&0).is_some());
    assert!(cache.try_get(&3).is_some());
    assert!(cache.try_get(&4).is_some());

    // These should be evicted
    assert!(cache.try_get(&1).is_none());
    assert!(cache.try_get(&2).is_none());
}

#[test]
fn factory_function_receives_correct_key() {
    let mut cache = TwoLevelCache::<String, String>::new();
    let mut captured_key: Option<String> = None;

    cache.get_or_add("testKey".to_string(), |key| {
        captured_key = Some(key.clone());
        "value".to_string()
    });

    assert_eq!(Some("testKey".to_string()), captured_key);
}

#[test]
fn get_or_add_null_eviction_action_does_not_throw() {
    let mut cache = TwoLevelCache::<String, Arc<Object>>::with_options(1, None, None);

    cache.get_or_add("key1".to_string(), |_| new_object());
    cache.get_or_add("key2".to_string(), |_| new_object());
    cache.get_or_add("key3".to_string(), |_| new_object()); // Should evict without error

    cache.clear_and_dispose(); // Should also not throw
}
