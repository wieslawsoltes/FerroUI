//! `TransformManyRefreshFixture` (`DynamicData.Tests/Cache/TransformManyRefreshFixture.cs`): the
//! friends of people as one cache, taken again when a person is refreshed.

use crate::domain::PersonWithFriends;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Rc<PersonWithFriends>, String>,
    source: SourceCache<Rc<PersonWithFriends>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<PersonWithFriends>| p.key());

        let results = source.connect().auto_refresh_any().transform_many(|p: &Rc<PersonWithFriends>| p.friends(), |p: &Rc<PersonWithFriends>| p.name()).as_aggregator();
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
fn auto_refresh() {
    let fixture = Fixture::new();
    let person = PersonWithFriends::new("Person", 50);
    fixture.source.edit(|updater| updater.add_or_update(person.clone()));

    person.set_friends(vec![PersonWithFriends::new("Friend1", 40), PersonWithFriends::new("Friend2", 45)]);

    assert_eq!(fixture.results.data().count(), 2, "Should be 2 in the cache");
    assert!(fixture.results.data().lookup(&"Friend1".to_owned()).is_some());
    assert!(fixture.results.data().lookup(&"Friend2".to_owned()).is_some());
}

// Not ported: AutoRefreshOnOtherProperty - upstream adds a friend to the list the person shares and
// then changes the age; the port's `PersonWithFriends` holds its own list, which only `set_friends`
// changes (and that raises `Friends`).
// Not ported: DirectRefresh - upstream adds a friend to the list the person shares and then
// refreshes the person; the port's `PersonWithFriends` holds its own list, which only
// `set_friends` changes (and that raises `Friends`).
