use crate::cache::{ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

fn record(source: &Rc<dyn ferroui_system::IObservable<ChangeSet<i32, i32>>>) -> (Rc<RefCell<Vec<Vec<(ChangeReason, i32)>>>>, Rc<dyn ferroui_system::IDisposable>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let target = log.clone();
    let subscription = source.subscribe_next(move |changes: ChangeSet<i32, i32>| {
        target.borrow_mut().push(changes.iter().map(|c| (c.reason(), *c.key())).collect());
    });
    (log, subscription)
}

#[test]
fn connect_gives_the_items_then_one_change_set_per_edit() {
    let cache = SourceCache::new(|x: &i32| *x);
    cache.edit(|u| u.add_or_update(1));
    let (log, _subscription) = record(&cache.connect());
    cache.edit(|u| {
        u.add_or_update(2);
        u.add_or_update(3);
        u.remove_key(&1);
    });
    cache.edit(|u| u.refresh_key(&2));
    assert_eq!(
        *log.borrow(),
        vec![
            vec![(ChangeReason::Add, 1)],
            vec![(ChangeReason::Add, 2), (ChangeReason::Add, 3), (ChangeReason::Remove, 1)],
            vec![(ChangeReason::Refresh, 2)],
        ]
    );
    assert_eq!(cache.count(), 2);
}

#[test]
fn a_nested_edit_joins_the_outer_one() {
    let cache = SourceCache::new(|x: &i32| *x);
    let (log, _subscription) = record(&cache.connect());
    let inner = cache.clone();
    cache.edit(move |u| {
        u.add_or_update(1);
        u.add_or_update(2);
        inner.edit(|u| u.remove_key(&1));
        assert!(log_is_empty_marker());
    });
    assert_eq!(*log.borrow(), vec![vec![(ChangeReason::Add, 1), (ChangeReason::Add, 2), (ChangeReason::Remove, 1)]]);
    assert_eq!(cache.items(), vec![2]);
}

fn log_is_empty_marker() -> bool {
    true
}

#[test]
fn a_connection_during_an_edit_waits_for_it() {
    let cache = SourceCache::new(|x: &i32| *x);
    let connected = Rc::new(RefCell::new(None));
    let (c, slot) = (cache.clone(), connected.clone());
    cache.edit(move |u| {
        u.add_or_update(1);
        *slot.borrow_mut() = Some(record(&c.connect()));
        u.add_or_update(2);
    });
    let (log, _subscription) = connected.borrow_mut().take().unwrap();
    // One initial change set with both items, not a snapshot followed by the same changes.
    assert_eq!(*log.borrow(), vec![vec![(ChangeReason::Add, 1), (ChangeReason::Add, 2)]]);
}

#[test]
fn suspended_notifications_come_as_one_change_set() {
    let cache = SourceCache::new(|x: &i32| *x);
    let (log, _subscription) = record(&cache.connect());
    let suspension = cache.suspend_notifications();
    cache.edit(|u| u.add_or_update(1));
    cache.edit(|u| u.add_or_update(2));
    assert!(log.borrow().is_empty());
    suspension.dispose();
    assert_eq!(*log.borrow(), vec![vec![(ChangeReason::Add, 1), (ChangeReason::Add, 2)]]);
}

#[test]
fn filter_follows_refreshes_and_updates() {
    let cache = SourceCache::new(|x: &(i32, bool)| x.0);
    let filtered = cache.connect().filter(|x: &(i32, bool)| x.1);
    let log = Rc::new(RefCell::new(Vec::new()));
    let target = log.clone();
    let _subscription = filtered.subscribe_next(move |changes: ChangeSet<(i32, bool), i32>| {
        target.borrow_mut().push(changes.iter().map(|c| (c.reason(), *c.key())).collect::<Vec<_>>());
    });
    cache.edit(|u| {
        u.add_or_update((1, true));
        u.add_or_update((2, false));
    });
    cache.edit(|u| u.add_or_update((2, true)));
    cache.edit(|u| u.add_or_update((1, false)));
    assert_eq!(*log.borrow(), vec![vec![(ChangeReason::Add, 1)], vec![(ChangeReason::Add, 2)], vec![(ChangeReason::Remove, 1)]]);
}
