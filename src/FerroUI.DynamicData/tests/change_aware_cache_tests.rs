use crate::cache::{ChangeAwareCache, ChangeReason, ICache, IQuery};

#[test]
fn records_the_changes_in_order_until_captured() {
    let mut cache = ChangeAwareCache::new();
    cache.add_or_update("a", 1);
    cache.add_or_update("b", 2);
    cache.add_or_update("c", 1);
    cache.refresh(&2);
    cache.remove(&2);
    let changes = cache.capture_changes();
    let reasons: Vec<_> = changes.iter().map(|c| (c.reason(), *c.key())).collect();
    assert_eq!(
        reasons,
        vec![(ChangeReason::Add, 1), (ChangeReason::Add, 2), (ChangeReason::Update, 1), (ChangeReason::Refresh, 2), (ChangeReason::Remove, 2)]
    );
    assert_eq!(changes[2].previous(), Some(&"a"));
    assert_eq!((changes.adds(), changes.updates(), changes.removes(), changes.refreshes()), (2, 1, 1, 1));
    assert_eq!(cache.capture_changes().count(), 0);
    assert_eq!(cache.items(), vec!["c"]);
}

#[test]
fn clear_removes_in_the_order_of_the_dictionary() {
    let mut cache = ChangeAwareCache::new();
    for key in 1..=4 {
        cache.add_or_update(key * 10, key);
    }
    cache.remove(&2);
    cache.add_or_update(50, 5);
    cache.capture_changes();
    cache.clear();
    let keys: Vec<_> = cache.capture_changes().iter().map(|c| *c.key()).collect();
    assert_eq!(keys, vec![1, 5, 3, 4]);
}
