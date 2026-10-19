//! Tests of `collections::generic::Dictionary`: the order of enumeration of .NET.

use crate::collections::generic::Dictionary;

fn keys(dictionary: &Dictionary<i32, &str>) -> Vec<i32> {
    dictionary.keys().copied().collect()
}

#[test]
fn enumerates_in_the_order_of_addition() {
    let mut d = Dictionary::new();
    for key in [5, 1, 9, 3] {
        d.add(key, "v");
    }
    assert_eq!(keys(&d), vec![5, 1, 9, 3]);
}

#[test]
fn an_update_keeps_the_place() {
    let mut d = Dictionary::new();
    d.add(1, "a");
    d.add(2, "b");
    assert_eq!(d.insert(1, "c"), Some("a"));
    assert_eq!(d.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(), vec![(1, "c"), (2, "b")]);
}

#[test]
fn an_addition_after_removals_takes_the_place_removed_last() {
    let mut d = Dictionary::new();
    for key in 1..=5 {
        d.add(key, "v");
    }
    assert_eq!(d.remove(&2), Some("v"));
    assert_eq!(d.remove(&4), Some("v"));
    d.add(10, "v");
    d.add(11, "v");
    d.add(12, "v");
    // .NET: 10 takes the slot of 4 (removed last), 11 the slot of 2, 12 goes to the end.
    assert_eq!(keys(&d), vec![1, 11, 3, 10, 5, 12]);
}

#[test]
fn clear_starts_over() {
    let mut d = Dictionary::new();
    d.add(1, "v");
    d.add(2, "v");
    d.remove(&1);
    d.clear();
    d.add(7, "v");
    d.add(8, "v");
    assert_eq!(keys(&d), vec![7, 8]);
    assert!(d.contains_key(&7) && !d.contains_key(&1));
}

#[test]
fn add_of_an_existing_key_panics() {
    let mut d = Dictionary::new();
    d.add(1, "v");
    assert!(!d.try_add(1, "w"));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || d.add(1, "w"))).is_err());
}
