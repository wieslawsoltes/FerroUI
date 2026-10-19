//! `IncludeUpdateFixture` (`DynamicData.Tests/Cache/IncludeUpdateFixture.cs`): the updates that a
//! function of the current and the previous item lets through.

use crate::domain::Person;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The fixture (`IncludeUpdateFixture` with its constructor and `Dispose`).
struct Fixture {
    results: ChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = ChangeSetAggregator::new(source.connect().include_update_when(|current: &Rc<Person>, previous: &Rc<Person>| current != previous));
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
fn ignore_function_will_ignore_subsequent_updates_of_an_item() {
    let fixture = Fixture::new();

    let person = Person::new("Person", 10);
    fixture.source.edit(|updater| updater.add_or_update(person.clone()));
    fixture.source.edit(|updater| updater.add_or_update(person.clone()));
    fixture.source.edit(|updater| updater.add_or_update(person.clone()));

    assert_eq!(fixture.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(fixture.results.data().count(), 1, "Should be 1 item in the cache");
}
