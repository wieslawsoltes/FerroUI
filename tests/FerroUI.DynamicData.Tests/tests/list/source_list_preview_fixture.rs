//! `SourceListPreviewFixture` (`DynamicData.Tests/List/SourceListPreviewFixture.cs`).

use ferroui_dynamic_data::list::{IExtendedList, IObservableList, ListChangeReason, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::assert_throws;
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    source: SourceList<i32>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceList::new() }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

/// The items of the list an edit receives (`IEnumerable<T>` of the list).
fn items_of(list: &dyn IExtendedList<i32>) -> Vec<i32> {
    (0..list.count()).map(|index| list.get(index)).collect()
}

/// `i => i % 2 == 0`.
fn is_even() -> Option<Rc<dyn Fn(&i32) -> bool>> {
    Some(Rc::new(|i: &i32| i % 2 == 0))
}

#[test]
fn changes_are_not_yet_applied_during_preview() {
    let fixture = Fixture::new();
    fixture.source.clear();

    // On preview, make sure the list is empty
    let d = {
        let source = fixture.source.clone();
        fixture.source.preview().subscribe_next(move |_| {
            assert!(source.count() == 0);
            assert!(source.items().is_empty());
        })
    };

    // Trigger a change
    fixture.source.add(1);

    // Cleanup
    d.dispose();
}

#[test]
fn connect_preview_predicate_is_applied() {
    let fixture = Fixture::new();
    fixture.source.clear();

    // Collect preview messages about even numbers only
    let aggregator = fixture.source.preview_with(is_even()).as_aggregator();

    // Trigger changes
    fixture.source.add(1);
    fixture.source.add(2);

    let messages = aggregator.messages();
    assert!(messages.len() == 1);
    assert!(messages[0].count() == 1);
    assert!(*messages[0][0].item().current() == 2);
    assert!(messages[0][0].reason() == ListChangeReason::Add);

    // Cleanup
    aggregator.dispose();
}

#[test]
fn form_new_list_from_changes() {
    let fixture = Fixture::new();
    fixture.source.clear();

    fixture.source.add_range(1..=100);

    // Collect preview messages about even numbers only
    let aggregator = fixture.source.preview_with(is_even()).as_aggregator();

    fixture.source.remove_at(10);
    fixture.source.remove_range(10, 5);
    // Trigger changes
    fixture.source.add(1);
    fixture.source.add(2);

    let messages = aggregator.messages();
    assert!(messages.len() == 1);
    assert!(messages[0].count() == 1);
    assert!(*messages[0][0].item().current() == 2);
    assert!(messages[0][0].reason() == ListChangeReason::Add);

    // Cleanup
    aggregator.dispose();
}

#[test]
fn no_changes_allowed_during_preview() {
    let fixture = Fixture::new();
    // On preview, try adding an arbitrary item
    let d = {
        let source = fixture.source.clone();
        // `InvalidOperationException`: an edit from the preview panics.
        fixture.source.preview().subscribe_next(move |_| assert_throws(|| source.add(1)))
    };

    // Trigger a change
    fixture.source.add(1);

    // Cleanup
    d.dispose();
}

#[test]
fn preview_events_are_correct() {
    let fixture = Fixture::new();
    let preview = fixture.source.preview().as_aggregator();
    let connect = fixture.source.connect().as_aggregator();
    fixture.source.edit(|l| {
        l.add(1);
        fixture.source.edit(|l2| l2.add(2));
        l.remove(&2);
        l.add_range(vec![3, 4, 5]);
        l.move_index(1, 0);
    });

    assert!(preview.messages() == connect.messages());
    assert!(fixture.source.items() == [3, 1, 4, 5]);
}

#[test]
fn recursive_edits_have_postponed_events() {
    let fixture = Fixture::new();
    let preview = fixture.source.preview().as_aggregator();
    let connect = fixture.source.connect().as_aggregator();
    fixture.source.edit(|_l| {
        fixture.source.edit(|l2| l2.add(1));
        assert_eq!(preview.messages().len(), 0);
        assert_eq!(connect.messages().len(), 0);
    });

    assert_eq!(preview.messages().len(), 1);
    assert_eq!(connect.messages().len(), 1);

    assert!(fixture.source.items() == [1]);
}

#[test]
fn recursive_edits_work() {
    let fixture = Fixture::new();
    fixture.source.edit(|l| {
        fixture.source.edit(|l2| l2.add(1));
        assert!(fixture.source.items() == [1]);
        assert!(items_of(l) == [1]);
    });

    assert!(fixture.source.items() == [1]);
}
