//! `EmptyToChangeSetIssue` (`DynamicData.Tests/Issues/EmptyToChangeSetIssue.cs`): an empty
//! observable collection still gives a change set.

use ferroui_dynamic_data::binding::ObservableCollectionEx;
use ferroui_dynamic_data::list::ObservableListEx;
use ferroui_system::collections::object_model::ObservableCollection;

#[test]
fn empty_collection_to_change_set_behaviour() {
    let collection = ObservableCollection::<()>::new();

    let results = collection.to_observable_change_set().as_aggregator();
    assert!(!results.messages().is_empty(), "An empty collection should still have an update, even if empty.");
}
