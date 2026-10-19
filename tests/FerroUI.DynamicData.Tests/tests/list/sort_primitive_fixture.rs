//! `SortPrimitiveFixture` (`DynamicData.Tests/List/SortPrimitiveFixture.cs`).

use crate::domain::Randomizer;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};

#[test]
fn remove_random_sorts() {
    let comparer = SortExpressionComparer::<i32>::ascending(|i| *i).comparer();
    let source = SourceList::<i32>::new();
    let results = source.connect().sort(comparer.clone()).as_aggregator();

    // Seems an odd test, but it catches exceptions when sorting on primitives.
    let items = Randomizer::new_unseeded().shuffle(&(1..=100).collect::<Vec<i32>>());
    source.edit(|list| list.add_range(items.clone()));

    assert_eq!(results.data().count(), 100);

    let mut expected_result = items.clone();
    expected_result.sort_by(|a, b| comparer(a, b));
    assert_equivalent(&results.data().items(), &expected_result);

    for i in 0..50 {
        source.edit(|list| {
            list.remove(&i);
        });
    }
}
