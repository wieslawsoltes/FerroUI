//! `SourceCacheFixture` (`DynamicData.Tests/Cache/SourceCacheFixture.cs`, with the parts of the
//! partial class in `SourceCacheFixture.UnitTests.cs` and `SourceCacheFixture.IntegrationTests.cs`):
//! the tests of `SourceCache`, one nested module per nested class.

/// `SomeObject(int Id, int Value)`, a record: compared by value.
#[derive(Clone, Debug, PartialEq)]
struct SomeObject {
    id: i32,
    value: i32,
}

impl SomeObject {
    fn new(id: i32, value: i32) -> Self {
        Self { id, value }
    }
}

// Upstream's `TestItem(string Key, string Value)` is used only by the integration tests, which are
// not ported; it is left out.

/// `SourceCacheFixture.UnitTests`.
mod unit_tests {
    use super::SomeObject;
    use crate::domain::{Person, RandomPersonGenerator};
    use crate::utilities::{assert_equivalent, CacheObservableExtensions};
    use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
    use ferroui_reactive::disposables::CompositeDisposable;
    use ferroui_reactive::ObservableExt;
    use ferroui_system::IDisposable;
    use std::cell::{Cell, RefCell};
    use std::panic::{self, AssertUnwindSafe};
    use std::rc::Rc;

    #[test]
    fn can_handle_a_batch_of_updates() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = source.connect().as_aggregator();

        source.edit(|updater| {
            let torequery = Person::new("Adult1", 44);

            updater.add_or_update(Person::new("Adult1", 40));
            updater.add_or_update(Person::new("Adult1", 41));
            updater.add_or_update(Person::new("Adult1", 42));
            updater.add_or_update(Person::new("Adult1", 43));
            updater.refresh_item(&torequery);
            updater.remove_item(&torequery);
            updater.refresh_item(&torequery);
        });

        let messages = results.messages();
        assert_eq!(results.summary().overall.count, 6, "Should be  6 up`dates");
        assert_eq!(messages.len(), 1, "Should be 1 message");
        assert_eq!(messages[0].adds(), 1, "Should be 1 update");
        assert_eq!(messages[0].updates(), 3, "Should be 3 updates");
        assert_eq!(messages[0].removes(), 1, "Should be  1 remove");
        assert_eq!(messages[0].refreshes(), 1, "Should be 1 evaluate");

        assert_eq!(results.data().count(), 0, "Should be 1 item in` the cache");
    }

    #[test]
    fn connect_continues_to_work_normally_after_a_failed_edit() {
        let source = SourceCache::new(|item: &i32| *item);

        source.add_or_update(1);

        // `Invoking(..).Should().Throw<Exception>().WithMessage("Test")`: the edit panics.
        let failure = panic::catch_unwind(AssertUnwindSafe(|| source.edit(|_| panic!("Test"))));
        let payload = failure.expect_err("the edit should have failed");
        assert_eq!(payload.downcast_ref::<&str>().copied(), Some("Test"));

        let (subscription, results) = source.connect().record_cache_items();

        assert!(results.error().is_none(), "new subscribers should not receive previous errors");
        assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should have been published.");
        subscription.dispose();
    }

    // Covers https://github.com/reactivemarbles/DynamicData/issues/1129
    #[test]
    fn connect_during_edits_does_not_duplicate() {
        let items = SourceCache::new(|item: &i32| *item);

        let subscriptions = CompositeDisposable::new();

        // An initial subscription is required to initiate internal buffering of changes, during the upcoming edit.
        // That is, we want there to be changes buffered, internally, when the mid-edit subscription comes in, to
        // ensure that they don't get duplicated. This is the scenario that came in up #1129.
        subscriptions.add(items.connect().subscribe_ignore());

        let mut results = None;

        items.edit(|inner| {
            inner.add_or_update(1);

            let (subscription, recorder) = items.connect().validate_cache_change_sets(|item: &i32| *item).record_cache_items();
            subscriptions.add(subscription);

            assert!(recorder.error().is_none(), "no errors should have occurred");
            assert!(recorder.recorded_change_sets().is_empty(), "no changes should be published in the middle of an edit");

            inner.add_or_update(2);

            assert!(recorder.error().is_none(), "no errors should have occurred");
            assert!(recorder.recorded_change_sets().is_empty(), "no changes should be published in the middle of an edit");

            // Explicitly doing a nested edit, as that system is closely intertwined with the edit-tracking system that
            // connect uses.
            items.remove(&1);

            assert!(recorder.error().is_none(), "no errors should have occurred");
            assert!(recorder.recorded_change_sets().is_empty(), "no changes should be published in the middle of an edit");

            results = Some(recorder);
        });

        let results = results.expect("the edit delegate should have been invoked");
        assert!(results.error().is_none(), "no errors should have occurred");
        assert_eq!(results.recorded_change_sets().len(), 1, "subscribers should only receive a single initial changeset");
        let recorded_items_by_key = results.recorded_items_by_key();
        assert_eq!(recorded_items_by_key.len(), 1, "all items in the source should have propagated downstream");
        assert_eq!(recorded_items_by_key.get(&2), Some(&2), "all items in the source should have propagated downstream");

        assert!(!results.has_completed(), "the source has not yet completed");

        subscriptions.dispose();
    }

    #[test]
    fn count_changed() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let _results = source.connect().as_aggregator();

        let count = Rc::new(Cell::new(0usize));
        let invoked = Rc::new(Cell::new(0usize));
        let subscription = {
            let (count, invoked) = (count.clone(), invoked.clone());
            source.count_changed().subscribe_next(move |c| {
                count.set(c);
                invoked.set(invoked.get() + 1);
            })
        };

        assert_eq!(invoked.get(), 1);
        assert_eq!(count.get(), 0);

        source.add_or_update_items(RandomPersonGenerator::new().take(100));
        assert_eq!(invoked.get(), 2);
        assert_eq!(count.get(), 100);

        source.clear();
        assert_eq!(invoked.get(), 3);
        assert_eq!(count.get(), 0);

        subscription.dispose();
    }

    #[test]
    fn count_changed_should_always_invoke_upon_subscription() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());

        let result: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let subscription = {
            let result = result.clone();
            source.count_changed().subscribe_next(move |count| result.set(Some(count)))
        };

        let result = result.get();
        assert!(result.is_some());
        assert_eq!(result, Some(0), "Count should be zero");

        subscription.dispose();
    }

    #[test]
    fn count_changed_should_reflect_contents_of_cache_invoke_upon_subscription() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());

        let generator = RandomPersonGenerator::new();
        let result: Rc<Cell<Option<usize>>> = Rc::new(Cell::new(None));
        let subscription = {
            let result = result.clone();
            source.count_changed().subscribe_next(move |count| result.set(Some(count)))
        };

        source.add_or_update_items(generator.take(100));

        let result = result.get();
        assert!(result.is_some());
        assert_eq!(result, Some(100), "Count should be 100");
        subscription.dispose();
    }

    #[test]
    fn empty_changes() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());

        let change: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));

        let subscription = {
            let change = change.clone();
            source.connect_with(None, false).subscribe_next(move |c| *change.borrow_mut() = Some(c))
        };

        let change = change.borrow().clone();
        assert!(change.is_some());
        assert_eq!(change.expect("a change set").count(), 0);
        subscription.dispose();
    }

    #[test]
    fn empty_changes_with_filter() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());

        let change: Rc<RefCell<Option<ChangeSet<Rc<Person>, String>>>> = Rc::new(RefCell::new(None));

        let subscription = {
            let change = change.clone();
            source.connect_with(Some(Rc::new(|p: &Rc<Person>| p.age() == 20)), false).subscribe_next(move |c| *change.borrow_mut() = Some(c))
        };

        let change = change.borrow().clone();
        assert!(change.is_some());
        assert_eq!(change.expect("a change set").count(), 0);
        subscription.dispose();
    }

    #[test]
    fn static_filter_remove() {
        let cache = SourceCache::new(|x: &SomeObject| x.id);

        let above5 = cache.connect_filtered(|x: &SomeObject| x.value > 5).as_observable_cache();
        let below5 = cache.connect_filtered(|x: &SomeObject| x.value <= 5).as_observable_cache();

        cache.add_or_update_items((1..=10).map(|i| SomeObject::new(i, i)));

        assert_equivalent(&above5.items(), &(6..=10).map(|i| SomeObject::new(i, i)).collect::<Vec<_>>());
        assert_equivalent(&below5.items(), &(1..=5).map(|i| SomeObject::new(i, i)).collect::<Vec<_>>());

        // should move from above 5 to below 5
        cache.add_or_update(SomeObject::new(6, -1));

        assert_eq!(above5.count(), 4);
        assert_eq!(below5.count(), 6);

        assert_equivalent(&above5.items(), &(7..=10).map(|i| SomeObject::new(i, i)).collect::<Vec<_>>());
        assert_equivalent(&below5.items(), &(1..=6).map(|i| SomeObject::new(i, if i == 6 { -1 } else { i })).collect::<Vec<_>>());
    }

    #[test]
    fn subscribe_disposes_correctly() {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());

        let called = Rc::new(Cell::new(false));
        let errored = Rc::new(Cell::new(false));
        let completed = Rc::new(Cell::new(false));
        let subscription = {
            let (finally_completed, called, errored, completed) = (completed.clone(), called.clone(), errored.clone(), completed.clone());
            source.connect().finally(move || finally_completed.set(true)).subscribe_all(move |_| called.set(true), move |_| errored.set(true), move || completed.set(true))
        };
        source.add_or_update(Person::new("Adult1", 40));

        subscription.dispose();
        source.dispose();

        assert!(!errored.get());
        assert!(called.get());
        assert!(completed.get());
    }
}

/// `SourceCacheFixture.IntegrationTests`.
mod integration_tests {
    // Not ported: ConnectDuringDeliveryDoesNotDuplicate - real threads
    // Not ported: DirectCrossWriteDoesNotDeadlock - real threads
    // Not ported: MultiCacheFanInDoesNotDeadlock - real threads
}
