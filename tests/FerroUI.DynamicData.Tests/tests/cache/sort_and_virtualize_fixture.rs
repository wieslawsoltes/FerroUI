//! `SortAndVirtualizeWithComparerChangesFixture`, `SortAndVirtualizeFixture` and
//! `SortAndVirtualizeFixtureBase` (`DynamicData.Tests/Cache/SortAndVirtualizeFixture.cs`): the
//! windows of the sorted items of a cache.
//!
//! The tests of the abstract base run for both fixtures (`base_tests!`): with a fixed comparer and
//! with an observable one.

use crate::domain::{Person, Randomizer};
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::cache::testing::ContextChangeSetAggregator;
use ferroui_dynamic_data::cache::{ContextObservableCacheEx, IConnectableCache, ObservableCacheEx, SourceCache, VirtualContext, VirtualRequest};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObserver};
use std::cmp::Ordering;
use std::rc::Rc;

type PersonComparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

struct Fixture {
    source: SourceCache<Rc<Person>, String>,
    comparer: PersonComparer,
    virtual_requests: Rc<BehaviorSubject<VirtualRequest>>,
    comparer_subject: Option<Rc<BehaviorSubject<PersonComparer>>>,
    aggregator: ContextChangeSetAggregator<Rc<Person>, String, VirtualContext<Rc<Person>>>,
}

impl Fixture {
    /// `SortAndVirtualizeFixture` when `with_comparer_changes` is false,
    /// `SortAndVirtualizeWithComparerChangesFixture` when it is true (`SetUpTests`).
    fn new(with_comparer_changes: bool) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let comparer: PersonComparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).into();
        let virtual_requests = BehaviorSubject::new(VirtualRequest::new(0, 25));
        let (comparer_subject, aggregator) = if with_comparer_changes {
            let comparer_subject = BehaviorSubject::new(comparer.clone());
            let aggregator = source.connect().sort_and_virtualize_dynamic(comparer_subject.as_observable(), virtual_requests.as_observable()).as_context_aggregator();
            (Some(comparer_subject), aggregator)
        } else {
            (None, source.connect().sort_and_virtualize(comparer.clone(), virtual_requests.as_observable()).as_context_aggregator())
        };
        Self { source, comparer, virtual_requests, comparer_subject, aggregator }
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }

    fn add_or_update_one(&self, person: &Rc<Person>) {
        self.source.edit(|updater| updater.add_or_update(person.clone()));
    }

    fn data_ordered_by(&self, comparer: &PersonComparer) -> Vec<Rc<Person>> {
        use ferroui_dynamic_data::cache::IObservableCache;
        ordered_by(&self.aggregator.data().items(), comparer)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.aggregator.dispose();
        self.virtual_requests.on_completed();
    }
}

fn ordered_by(people: &[Rc<Person>], comparer: &PersonComparer) -> Vec<Rc<Person>> {
    let mut ordered = people.to_vec();
    ordered.sort_by(|x, y| comparer(x, y));
    ordered
}

/// `Enumerable.Range(1, 100).Select(i => new Person($"P{i:000}", i)).OrderBy(p => Guid.NewGuid())`.
fn shuffled_people() -> Vec<Rc<Person>> {
    let ordered: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("P{i:03}"), i)).collect();
    Randomizer::new_unseeded().shuffle(&ordered)
}

macro_rules! base_tests {
    ($with_comparer_changes:expr) => {
        use super::*;
        use crate::utilities::assert_equivalent;
        use ferroui_dynamic_data::cache::ChangeReason;

        fn fixture() -> Fixture {
            Fixture::new($with_comparer_changes)
        }

        #[test]
        fn initial_batches() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            // For the first batch, it uses the result of the virtual requests subject (a behaviour subject).
            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_equivalent(&actual_result, &expected_result);

            fixture.virtual_requests.on_next(VirtualRequest::new(25, 50));

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().skip(25).take(50).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_equivalent(&actual_result, &expected_result);
        }

        #[test]
        fn overlapping_shift() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            fixture.virtual_requests.on_next(VirtualRequest::new(10, 30));

            // For the first batch, it uses the result of the virtual requests subject (a behaviour subject).
            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().skip(10).take(30).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_equivalent(&actual_result, &expected_result);
        }

        #[test]
        fn add_first_in_range() {
            let fixture = fixture();
            let mut people = shuffled_people();
            fixture.add_or_update(&people);

            // Insert right at the beginning.
            let person = Person::new("_FirstPerson", 1);
            fixture.add_or_update_one(&person);

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 2);

            let first_change = &changes[0];
            assert_eq!(first_change.reason(), ChangeReason::Remove);
            assert_eq!(*first_change.current(), Person::new("P025", 25));

            let second_change = &changes[1];
            assert_eq!(second_change.reason(), ChangeReason::Add);
            assert_eq!(*second_change.current(), person);

            // Check the resulting collection.
            people.push(person);

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn add_outside_of_range() {
            let fixture = fixture();
            let mut people = shuffled_people();
            fixture.add_or_update(&people);

            // Insert right at the end.
            let person = Person::new("X_Last", 100);
            fixture.add_or_update_one(&person);

            // Only the initial message is received.
            assert_eq!(fixture.aggregator.messages().len(), 1);

            people.push(person);
            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn update_move_out_of_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            // Change an item so that it moves from in range to out of range.
            let person = Person::new("P012", 50);
            fixture.add_or_update_one(&person);

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 2);

            let first_change = &changes[0];
            assert_eq!(first_change.reason(), ChangeReason::Remove);
            assert_eq!(*first_change.current(), Person::new("P012", 50));

            let second_change = &changes[1];
            assert_eq!(second_change.reason(), ChangeReason::Add);
            assert_eq!(*second_change.current(), Person::new("P026", 26));

            // Check the resulting collection.
            let mut people = ordered_by(&people, &fixture.comparer);
            people[11] = person;

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn update_stay_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            // Update an item, but keep it within the range.
            let person = Person::new("P012", -1);
            fixture.add_or_update_one(&person);

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 1);

            let first_change = &changes[0];
            assert_eq!(first_change.reason(), ChangeReason::Update);
            assert_eq!(*first_change.current(), Person::new("P012", -1));
            assert_eq!(*first_change.previous().unwrap(), Person::new("P012", 12));

            // Check the resulting collection.
            let mut people = ordered_by(&people, &fixture.comparer);
            people[11] = person;

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn update_out_of_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            let person = Person::new("P050", 100);
            fixture.add_or_update_one(&person);

            // Only the initial message is received.
            assert_eq!(fixture.aggregator.messages().len(), 1);

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn remove_range() {
            let fixture = fixture();
            let mut people = shuffled_people();
            fixture.add_or_update(&people);

            // Remove an element from the active range.
            let person = Person::new("P012", 12);
            fixture.source.edit(|updater| updater.remove_item(&person));

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 2);

            let first_change = &changes[0];
            assert_eq!(first_change.reason(), ChangeReason::Remove);
            assert_eq!(*first_change.current(), person);

            let second_change = &changes[1];
            assert_eq!(second_change.reason(), ChangeReason::Add);
            assert_eq!(*second_change.current(), Person::new("P026", 26));

            // Check the resulting collection.
            if let Some(index) = people.iter().position(|p| *p == person) {
                people.remove(index);
            }

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn remove_out_of_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            let person = Person::new("P050", 50);
            fixture.source.edit(|updater| updater.remove_item(&person));

            // Only the initial message is received.
            assert_eq!(fixture.aggregator.messages().len(), 1);

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn refresh_in_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            let person = people.iter().find(|p| p.name() == "P012").unwrap().clone();
            fixture.source.edit(|updater| updater.refresh_item(&person));

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 1);

            assert_eq!(changes[0].reason(), ChangeReason::Refresh);
        }

        #[test]
        fn refresh_with_inline_change_in_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            let person = people.iter().find(|p| p.name() == "P012").unwrap().clone();

            // The item moves within the range, so it is propagated as a refresh.
            person.set_age(5);
            fixture.source.edit(|updater| updater.refresh_item(&person));

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 1);

            assert_eq!(changes[0].reason(), ChangeReason::Refresh);

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }

        #[test]
        fn refresh_with_inline_change_outside_range() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            let person = people.iter().find(|p| p.name() == "P012").unwrap().clone();

            // The item moves outside the range, which makes a remove and an index shift.
            person.set_age(50);
            fixture.source.edit(|updater| updater.refresh_item(&person));

            assert_eq!(fixture.aggregator.messages().len(), 2);

            let changes = fixture.aggregator.messages()[1].clone();
            assert_eq!(changes.count(), 2);

            let first_change = &changes[0];
            assert_eq!(first_change.reason(), ChangeReason::Remove);
            assert_eq!(*first_change.current(), Person::new("P012", 50));

            let second_change = &changes[1];
            assert_eq!(second_change.reason(), ChangeReason::Add);
            assert_eq!(*second_change.current(), Person::new("P026", 26));

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_eq!(actual_result, expected_result);
        }
    };
}

/// `SortAndVirtualizeFixture`: a fixed comparer.
mod sort_and_virtualize_fixture {
    base_tests!(false);
}

/// `SortAndVirtualizeWithComparerChangesFixture`: an observable comparer.
mod sort_and_virtualize_with_comparer_changes_fixture {
    base_tests!(true);

    fn desc_comparer() -> PersonComparer {
        SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).into()
    }

    #[test]
    fn change_comparer() {
        let fixture = fixture();
        let desc_comparer = desc_comparer();
        let people = shuffled_people();
        fixture.add_or_update(&people);

        // For the first batch, it uses the result of the virtual requests subject (a behaviour subject).
        let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
        let actual_result = fixture.data_ordered_by(&fixture.comparer);
        assert_equivalent(&actual_result, &expected_result);

        // Change the comparer.
        fixture.comparer_subject.as_ref().unwrap().on_next(desc_comparer.clone());

        let expected_result: Vec<Rc<Person>> = ordered_by(&people, &desc_comparer).into_iter().take(25).collect();
        let actual_result = fixture.data_ordered_by(&fixture.comparer);
        assert_equivalent(&actual_result, &expected_result);
    }

    #[test]
    fn change_comparer_data_set_smaller_than_virtual_size() {
        let fixture = fixture();
        let desc_comparer = desc_comparer();
        let ordered: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("P{i:02}"), i)).collect();
        let people = Randomizer::new_unseeded().shuffle(&ordered);
        fixture.add_or_update(&people);

        // For the first batch, it uses the result of the virtual requests subject (a behaviour subject).
        let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(10).collect();
        let actual_result = fixture.data_ordered_by(&fixture.comparer);
        assert_equivalent(&actual_result, &expected_result);

        assert!(fixture.aggregator.messages()[0].iter().all(|c| c.reason() == ChangeReason::Add));

        // Change the comparer.
        fixture.comparer_subject.as_ref().unwrap().on_next(desc_comparer.clone());

        let expected_result: Vec<Rc<Person>> = ordered_by(&people, &desc_comparer).into_iter().take(10).collect();
        let actual_result = fixture.data_ordered_by(&fixture.comparer);
        assert_equivalent(&actual_result, &expected_result);

        assert!(fixture.aggregator.messages()[1].iter().all(|c| c.reason() == ChangeReason::Refresh));
    }
}
