//! `SortAndPageWithComparerChangesFixture`, `SortAndPageFixture` and `SortAndPageFixtureBase`
//! (`DynamicData.Tests/Cache/SortAndPageFixture.cs`): the pages of the sorted items of a cache.
//!
//! The tests of the abstract base run for both fixtures (`base_tests!`): with a fixed comparer and
//! with an observable one.

use crate::domain::{Person, Randomizer};
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::cache::testing::ContextChangeSetAggregator;
use ferroui_dynamic_data::cache::{ContextObservableCacheEx, IConnectableCache, ObservableCacheEx, PageContext, PageRequest, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObserver};
use std::cmp::Ordering;
use std::rc::Rc;

type PersonComparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

struct Fixture {
    source: SourceCache<Rc<Person>, String>,
    comparer: PersonComparer,
    page_requests: Rc<BehaviorSubject<PageRequest>>,
    comparer_subject: Option<Rc<BehaviorSubject<PersonComparer>>>,
    aggregator: ContextChangeSetAggregator<Rc<Person>, String, PageContext<Rc<Person>>>,
}

impl Fixture {
    /// `SortAndPageFixture` when `with_comparer_changes` is false,
    /// `SortAndPageWithComparerChangesFixture` when it is true (`SetUpTests`).
    fn new(with_comparer_changes: bool) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let comparer: PersonComparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).into();
        let page_requests = BehaviorSubject::new(PageRequest::new(1, 25));
        let (comparer_subject, aggregator) = if with_comparer_changes {
            let comparer_subject = BehaviorSubject::new(comparer.clone());
            let aggregator = source.connect().sort_and_page_dynamic(comparer_subject.as_observable(), page_requests.as_observable()).as_context_aggregator();
            (Some(comparer_subject), aggregator)
        } else {
            (None, source.connect().sort_and_page(comparer.clone(), page_requests.as_observable()).as_context_aggregator())
        };
        Self { source, comparer, page_requests, comparer_subject, aggregator }
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
        self.page_requests.on_completed();
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

            // For the first batch, it uses the result of the page requests subject (a behaviour subject).
            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_equivalent(&actual_result, &expected_result);

            fixture.page_requests.on_next(PageRequest::new(3, 25));

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().skip(50).take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_equivalent(&actual_result, &expected_result);
        }

        // Not ported: ThrowsForNegativePage - argument validation (the page of a PageRequest is a usize)
        // Not ported: ThrowsForNegativeSizeParameters - argument validation (the size of a PageRequest is a usize)

        #[test]
        fn page_greater_than_number_of_pages_available() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            // The last page is selected.
            fixture.page_requests.on_next(PageRequest::new(10, 25));

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().skip(75).take(25).collect();
            let actual_result = fixture.data_ordered_by(&fixture.comparer);
            assert_equivalent(&actual_result, &expected_result);
        }

        #[test]
        fn overlapping_shift() {
            let fixture = fixture();
            let people = shuffled_people();
            fixture.add_or_update(&people);

            fixture.page_requests.on_next(PageRequest::new(3, 10));

            let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().skip(20).take(10).collect();
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

/// `SortAndPageFixture`: a fixed comparer.
mod sort_and_page_fixture {
    base_tests!(false);
}

/// `SortAndPageWithComparerChangesFixture`: an observable comparer.
mod sort_and_page_with_comparer_changes_fixture {
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

        // For the first batch, it uses the result of the page requests subject (a behaviour subject).
        let expected_result: Vec<Rc<Person>> = ordered_by(&people, &fixture.comparer).into_iter().take(25).collect();
        let actual_result = fixture.data_ordered_by(&fixture.comparer);
        assert_equivalent(&actual_result, &expected_result);

        // Change the comparer.
        fixture.comparer_subject.as_ref().unwrap().on_next(desc_comparer.clone());

        assert!(Rc::ptr_eq(&fixture.aggregator.messages().last().unwrap().context().comparer, &desc_comparer));
        let expected_result: Vec<Rc<Person>> = ordered_by(&people, &desc_comparer).into_iter().take(25).collect();
        let actual_result = fixture.data_ordered_by(&desc_comparer);
        assert_equivalent(&actual_result, &expected_result);
    }

    #[test]
    fn change_comparer_with_only_one_page() {
        let fixture = fixture();
        let desc_comparer = desc_comparer();
        fixture.page_requests.on_next(PageRequest::new(1, 200));

        let people = shuffled_people();
        fixture.add_or_update(&people);

        // For the first batch, it uses the result of the page requests subject (a behaviour subject).
        let expected_result = ordered_by(&people, &fixture.comparer);
        let actual_result = fixture.data_ordered_by(&fixture.comparer);
        assert_equivalent(&actual_result, &expected_result);
        let _change_set_count = fixture.aggregator.messages().len();

        // Change the comparer.
        fixture.comparer_subject.as_ref().unwrap().on_next(desc_comparer.clone());

        assert!(Rc::ptr_eq(&fixture.aggregator.messages().last().unwrap().context().comparer, &desc_comparer));
        let expected_result = ordered_by(&people, &desc_comparer);
        let actual_result = fixture.data_ordered_by(&desc_comparer);
        assert_equivalent(&actual_result, &expected_result);
    }
}
