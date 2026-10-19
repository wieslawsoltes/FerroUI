//! `TransformManyRefreshFixture` (`DynamicData.Tests/List/TransformManyRefreshFixture.cs`): the
//! friends of people, recursively, as one list, taken again when a person is refreshed.

use crate::domain::PersonWithFriends;
use crate::utilities::{assert_equivalent, SelectManyExtensions};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Rc<PersonWithFriends>>,
    source: SourceList<Rc<PersonWithFriends>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();

        let results = source.connect().auto_refresh_any().transform_many(|p: &Rc<PersonWithFriends>| p.friends().recursive_select(|r| r.friends())).as_aggregator();
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
    let friend1 = PersonWithFriends::new("Friend1", 40);
    let friend2 = PersonWithFriends::new("Friend2", 45);

    let person = PersonWithFriends::new("Person", 50);
    fixture.source.edit(|list| list.add(person.clone()));

    person.set_friends(vec![friend1.clone(), friend2.clone()]);

    assert_eq!(fixture.results.data().count(), 2, "Should be 2 in the cache");
    assert_equivalent(&fixture.results.data().items(), &[friend1, friend2]);
}

// Not ported: AutoRefreshOnOtherProperty - upstream adds a friend to the list the person shares and
// then changes the age; the port's `PersonWithFriends` holds its own list, which only `set_friends`
// changes (and that raises `Friends`).

#[test]
fn auto_refresh_recursive() {
    let fixture = Fixture::new();
    let friend1 = PersonWithFriends::new("Friend1", 30);
    let friend2 = PersonWithFriends::new("Friend2", 35);
    let friend3 = PersonWithFriends::with_friends("Friend3", 40, vec![friend1]);
    let friend4 = PersonWithFriends::with_friends("Friend4", 45, vec![friend2.clone()]);

    let person = PersonWithFriends::with_friends("Person", 50, vec![friend3]);
    fixture.source.edit(|list| list.add(person.clone()));

    person.set_friends(vec![friend4.clone()]);

    assert_eq!(fixture.results.data().count(), 2, "Should be 2 in the cache");
    assert_equivalent(&fixture.results.data().items(), &[friend4, friend2]);
}
