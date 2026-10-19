//! Ported from `Splat.Logging.Tests/MemoizingMRUCacheTests.cs` of Splat.
//! Adapted: the values are `Rc<DummyObjectClass1>` (the cache hands out clones, so a reference
//! type is an `Rc`), and `IsSameReferenceAs` is `same`. The keys are `String`; a `null` key does not
//! exist. `StringComparer.OrdinalIgnoreCase` is a comparer closure over `eq_ignore_ascii_case`.
//! `TryGet(key, out value)` is `try_get(&key) -> Option<V>`. A release function that throws
//! panics; `InvalidateAll(true)` returns the panics in the `Err` of `invalidate_all_with(true)`
//! (upstream an `AggregateException`), and `InvalidateAll(false)` lets the first one propagate. The
//! size is a `usize`, so the negative size of upstream does not exist; only the size 0 is checked.
//! The context of `Get(key, context)` is a `&dyn Any`, compared by address.

use crate::tests::common::{panics, same, DummyObjectClass1};
use ferroui_splat::MemoizingMRUCache;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SAMPLE_KEY: &str = "Test1";

const SECOND_SAMPLE_KEY: &str = "Test2";

const DEFAULT_MAX_CACHE_SIZE: usize = 256;

const MAX_CACHE_SIZE: usize = 10;

const EVICTION_CACHE_SIZE: usize = 2;

const TWO_CACHED_ITEMS: usize = 2;

type Cache = MemoizingMRUCache<String, Rc<DummyObjectClass1>>;

fn new_value(_key: &String, _context: Option<&dyn Any>) -> Rc<DummyObjectClass1> {
    Rc::new(DummyObjectClass1)
}

fn get_test_instance() -> Cache {
    MemoizingMRUCache::new(new_value, DEFAULT_MAX_CACHE_SIZE)
}

fn ignore_case(a: &String, b: &String) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// A cache of `MAX_CACHE_SIZE` values whose release function counts the released values.
fn counting_instance(max_size: usize) -> (Cache, Rc<Cell<usize>>) {
    let release_count = Rc::new(Cell::new(0));
    let counter = release_count.clone();
    let instance = MemoizingMRUCache::with_release(new_value, max_size, move |_: Rc<DummyObjectClass1>| {
        counter.set(counter.get() + 1);
    });
    (instance, release_count)
}

/// A cache whose release function panics (`throw new InvalidOperationException("Release error")`).
fn throwing_instance() -> Cache {
    MemoizingMRUCache::with_release(new_value, MAX_CACHE_SIZE, |_: Rc<DummyObjectClass1>| {
        panic!("Release error");
    })
}

#[test]
fn constructor_throws_argument_exception_for_invalid_max_size() {
    assert!(panics(|| {
        let _ = MemoizingMRUCache::<String, Rc<DummyObjectClass1>>::new(new_value, 0);
    }));
}

#[test]
fn cache_evicts_old_items_when_max_size_reached() {
    let (instance, release_count) = counting_instance(EVICTION_CACHE_SIZE);

    let _ = instance.get("key1".to_owned());
    let _ = instance.get("key2".to_owned());
    let _ = instance.get("key3".to_owned()); // evicts key1

    assert_eq!(release_count.get(), 1);
    assert!(instance.try_get(&"key1".to_owned()).is_none());
    assert!(instance.try_get(&"key2".to_owned()).is_some());
    assert!(instance.try_get(&"key3".to_owned()).is_some());
}

#[test]
fn cache_evicts_default_values_when_max_size_reached() {
    let released = Rc::new(RefCell::new(Vec::<i32>::new()));
    let sink = released.clone();
    let instance: MemoizingMRUCache<String, i32> =
        MemoizingMRUCache::with_release(|_: &String, _: Option<&dyn Any>| 0, 1, move |value: i32| {
            sink.borrow_mut().push(value);
        });

    let _ = instance.get("key1".to_owned());
    let _ = instance.get("key2".to_owned());

    assert_eq!(*released.borrow(), vec![0]);
}

#[test]
fn invalidate_all_with_aggregate_exceptions_handles_exceptions() {
    let instance = throwing_instance();

    let _ = instance.get("key1".to_owned());
    let _ = instance.get("key2".to_owned());

    let error = instance.invalidate_all_with(true).expect_err("the release panics are aggregated");
    assert_eq!(error.inner_messages().len(), TWO_CACHED_ITEMS);
}

#[test]
fn invalidate_all_without_aggregate_exceptions_throws_on_first_error() {
    let instance = throwing_instance();

    let _ = instance.get("key1".to_owned());

    assert!(panics(|| {
        let _ = instance.invalidate_all_with(false);
    }));
}

#[test]
fn cached_values_returns_current_cache_contents() {
    let instance = get_test_instance();
    let value1 = instance.get("key1".to_owned());
    let value2 = instance.get("key2".to_owned());

    let cached_values = instance.cached_values();

    assert_eq!(cached_values.len(), TWO_CACHED_ITEMS);
    assert!(cached_values.iter().any(|value| same(value, &value1)));
    assert!(cached_values.iter().any(|value| same(value, &value2)));
}

#[test]
fn constructor_with_custom_comparer_uses_comparer() {
    let instance: Cache = MemoizingMRUCache::with_comparer(new_value, MAX_CACHE_SIZE, ignore_case);

    let value1 = instance.get("KEY".to_owned());
    let value2 = instance.get("key".to_owned());

    assert!(same(&value1, &value2));
}

#[test]
fn constructor_with_custom_comparer_and_release_function_works() {
    let release_count = Rc::new(Cell::new(0));
    let counter = release_count.clone();
    let instance: Cache = MemoizingMRUCache::with_release_and_comparer(
        new_value,
        MAX_CACHE_SIZE,
        Some(move |_: Rc<DummyObjectClass1>| counter.set(counter.get() + 1)),
        ignore_case,
    );

    let _ = instance.get("key1".to_owned());
    instance.invalidate(&"key1".to_owned());

    assert_eq!(release_count.get(), 1);
}

#[test]
fn get_with_context_passes_context_to_factory() {
    let received_context = Rc::new(Cell::new(std::ptr::null::<()>()));
    let sink = received_context.clone();
    let instance: Cache = MemoizingMRUCache::new(
        move |_: &String, context: Option<&dyn Any>| {
            if let Some(context) = context {
                sink.set(std::ptr::from_ref(context).cast::<()>());
            }
            Rc::new(DummyObjectClass1)
        },
        MAX_CACHE_SIZE,
    );

    let test_context = String::from("context");

    let _ = instance.get_with_context("key1".to_owned(), Some(&test_context as &dyn Any));

    assert_eq!(received_context.get(), &test_context as *const String as *const ());
}

#[test]
fn invalidate_non_existent_key_does_not_throw() {
    let instance = get_test_instance();
    instance.invalidate(&"nonexistent".to_owned());
}

#[test]
fn invalidate_all_empty_cache_does_not_throw() {
    let instance = get_test_instance();
    instance.invalidate_all();
}

#[test]
fn invalidate_all_with_release_function_releases_every_item() {
    let (instance, release_count) = counting_instance(MAX_CACHE_SIZE);

    let _ = instance.get("key1".to_owned());
    let _ = instance.get("key2".to_owned());

    instance.invalidate_all();

    assert_eq!(release_count.get(), TWO_CACHED_ITEMS);
    assert!(instance.cached_values().is_empty());
}

#[test]
fn invalidate_all_with_aggregate_exceptions_releases_every_item_when_release_succeeds() {
    let (instance, release_count) = counting_instance(MAX_CACHE_SIZE);

    let _ = instance.get("key1".to_owned());
    let _ = instance.get("key2".to_owned());

    assert!(instance.invalidate_all_with(true).is_ok());
    assert_eq!(release_count.get(), TWO_CACHED_ITEMS);
    assert!(instance.cached_values().is_empty());
}

#[test]
fn invalidate_all_null_release_function_works() {
    let instance: Cache = MemoizingMRUCache::new(new_value, MAX_CACHE_SIZE);
    let _ = instance.get("key1".to_owned());

    instance.invalidate_all();
}

#[test]
fn try_get_non_existent_key_returns_false() {
    let instance = get_test_instance();

    let result = instance.try_get(&"nonexistent".to_owned());

    assert!(result.is_none());
}

#[test]
fn returns_value() {
    let instance = get_test_instance();
    let result = instance.get(SAMPLE_KEY.to_owned());
    assert_eq!(Rc::strong_count(&result), 2);
}

#[test]
fn get_returns_same_value() {
    let instance = get_test_instance();
    let result1 = instance.get(SAMPLE_KEY.to_owned());
    let result2 = instance.get(SAMPLE_KEY.to_owned());

    assert!(same(&result1, &result2));
}

#[test]
fn get_returns_different_values() {
    let instance = get_test_instance();
    let result1 = instance.get(SAMPLE_KEY.to_owned());
    let result2 = instance.get(SECOND_SAMPLE_KEY.to_owned());

    assert!(!same(&result1, &result2));
}

#[test]
fn try_get_returns_same_value() {
    let instance = get_test_instance();
    let result1 = instance.get(SAMPLE_KEY.to_owned());
    let result2 = instance.try_get(&SAMPLE_KEY.to_owned()).expect("the value is cached");

    assert!(same(&result1, &result2));
}

#[test]
fn try_get_returns_different_values() {
    let instance = get_test_instance();
    let p1 = instance.get(SAMPLE_KEY.to_owned());
    let p2 = instance.get(SECOND_SAMPLE_KEY.to_owned());

    let result1 = instance.get(SAMPLE_KEY.to_owned());
    let result2 = instance.get(SECOND_SAMPLE_KEY.to_owned());

    assert!(!same(&result1, &result2));
    assert!(same(&result1, &p1));
    assert!(same(&result2, &p2));
}

// Not ported: ThrowsArgumentNullException - argument-null validation (a null key), which Rust's types make impossible
// Not ported: Constructor_ThrowsArgumentNullException_ForNullCalculationFunction - argument-null validation
// Not ported: TryGet_ThrowsArgumentNullException_ForNullKey - argument-null validation
// Not ported: Invalidate_ThrowsArgumentNullException_ForNullKey - argument-null validation
// Not ported: ThreadSafeRetrievalTest - real threads (AsParallel); the cache of the port is single-threaded
// Not ported: ThreadSafeRetrievalTestWithGetAndTryGet - real threads (AsParallel); the cache of the port is single-threaded
// Not ported: GetsResultsFromCacheValuesWhenInvalidateAndGetAreUsed - real threads (AsParallel); the cache of the port is single-threaded
// Not ported: GetsResultsWhenInvalidateAndGetAreUsed - real threads (AsParallel); the cache of the port is single-threaded
// Not ported: GetsResultsWhenInvalidateAllAndGetAreUsed - real threads (AsParallel); the cache of the port is single-threaded
