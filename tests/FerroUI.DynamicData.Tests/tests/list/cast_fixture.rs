//! `CastFixture` (`DynamicData.Tests/List/CastFixture.cs`).
//!
//! Upstream casts to `decimal`; the port casts to `f64`, the nearest type of Rust.

use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx, SourceListEx};
use ferroui_system::IDisposable;

struct Fixture {
    results: ChangeSetAggregator<f64>,
    source: SourceList<i32>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::<i32>::new();
        let results = source.cast_list(|i: &i32| f64::from(*i)).as_aggregator();
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn can_cast() {
    let fixture = Fixture::new();
    fixture.source.add_range(1..=10);
    assert_eq!(fixture.results.data().count(), 10);

    fixture.source.clear();
    assert_eq!(fixture.results.data().count(), 0);
}
