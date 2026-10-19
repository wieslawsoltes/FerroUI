//! `SourceListFixture` (`DynamicData.Tests/List/SourceListFixture.cs`).

use crate::utilities::{assert_equivalent_strict, ListItemRecordingObserver, ListObservableExtensions};
use ferroui_dynamic_data::list::{ChangeSet, ChangeType, IObservableList, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The message of a panic, as `WithMessage` reads the message of an exception.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<String> {
    payload.downcast_ref::<&str>().map(|message| (*message).to_owned()).or_else(|| payload.downcast_ref::<String>().cloned())
}

// Covers https://github.com/reactivemarbles/DynamicData/issues/1129
#[test]
fn connect_during_edit_does_not_duplicate() {
    let items = SourceList::<i32>::new();

    let subscriptions = CompositeDisposable::new();

    // An initial subscription is required to initiate internal buffering of changes, during the upcoming .Edit().
    // That is, we want there to be changes buffered, internally, when the mid-edit subscription comes in, to
    // ensure that they don't get duplicated. This is the scenario that came in up #1129.
    subscriptions.add(items.connect().subscribe_ignore());

    let mut results: Option<Rc<ListItemRecordingObserver<i32>>> = None;

    items.edit(|inner| {
        inner.add(1);

        let (subscription, recorder) = items.connect().validate_list_change_sets().record_list_items();
        subscriptions.add(subscription);

        assert!(recorder.error().is_none(), "no errors should have occurred");
        assert!(recorder.recorded_change_sets().is_empty(), "no changes should be published in the middle of an edit");

        inner.add(2);

        assert!(recorder.error().is_none(), "no errors should have occurred");
        assert!(recorder.recorded_change_sets().is_empty(), "no changes should be published in the middle of an edit");

        results = Some(recorder);
    });

    let results = results.expect("the edit delegate should have been invoked");
    assert!(results.error().is_none(), "no errors should have occurred");
    assert_eq!(results.recorded_change_sets().len(), 1, "subscribers should only receive a single initial changeset");
    // "all items in the source should have propagated downstream"
    assert_equivalent_strict(&results.recorded_items(), &[1, 2]);

    assert!(!results.has_completed(), "the source has not yet completed");

    subscriptions.dispose();
    items.dispose();
}

#[test]
fn initial_change_is_range() {
    let source = SourceList::<String>::new();
    source.add("A".to_owned());
    let change_sets: Rc<RefCell<Vec<ChangeSet<String>>>> = Rc::new(RefCell::new(Vec::new()));

    {
        let change_sets = change_sets.clone();
        source.connect().subscribe_next(move |changes| change_sets.borrow_mut().push(changes)).dispose();
    }

    let change_sets = change_sets.borrow();
    assert_eq!(change_sets[0][0].change_type(), ChangeType::Range);
    assert_eq!(change_sets[0][0].range().index(), Some(0));
}

#[test]
fn connect_continues_to_work_normally_after_a_failed_edit() {
    let source = SourceList::<i32>::new();

    source.add(1);

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        source.edit(|_| {
            panic!("Test");
        })
    }));
    let payload = outcome.expect_err("the edit should throw");
    assert_eq!(panic_message(&*payload).as_deref(), Some("Test"));

    let (subscription, results) = source.connect().record_list_items();

    assert!(results.error().is_none(), "new subscribers should not receive previous errors");
    assert_eq!(results.recorded_change_sets().len(), 1, "an initial changeset should have been published.");

    subscription.dispose();
    source.dispose();
}
