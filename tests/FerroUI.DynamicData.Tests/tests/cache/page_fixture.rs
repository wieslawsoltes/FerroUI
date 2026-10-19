//! `PageFixture` (`DynamicData.Tests/Cache/PageFixture.cs`): the pages of a sorted cache.

use crate::domain::{Person, RandomPersonGenerator};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::cache::testing::PagedChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, PageRequest, SortOptimisations, SortedObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObserver};
use std::cmp::Ordering;
use std::rc::Rc;

type PersonComparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

struct Fixture {
    aggregators: PagedChangeSetAggregator<Rc<Person>, String>,
    comparer: PersonComparer,
    generator: RandomPersonGenerator,
    pager: Rc<BehaviorSubject<PageRequest>>,
    sort: Rc<BehaviorSubject<PersonComparer>>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let comparer: PersonComparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.name()).then_by_ascending(|p| p.age()).into();
        let sort = BehaviorSubject::new(comparer.clone());
        let pager = BehaviorSubject::new(PageRequest::new(1, 25));

        let aggregators = source.connect().sort_dynamic_with(sort.as_observable(), SortOptimisations::NONE, 200).page(pager.as_observable()).as_sorted_aggregator();
        Self { aggregators, comparer, generator: RandomPersonGenerator::new(), pager, sort, source }
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.aggregators.dispose();
    }
}

fn ordered_by(people: &[Rc<Person>], comparer: &PersonComparer) -> Vec<Rc<Person>> {
    let mut ordered = people.to_vec();
    ordered.sort_by(|x, y| comparer(x, y));
    ordered
}

fn key_values(people: impl IntoIterator<Item = Rc<Person>>) -> Vec<(String, Rc<Person>)> {
    people.into_iter().map(|p| (p.name(), p)).collect()
}

#[test]
fn change_page() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);
    fixture.pager.on_next(PageRequest::new(2, 25));

    let expected_result = key_values(ordered_by(&people, &fixture.comparer).into_iter().skip(25).take(25));
    let actual_result = fixture.aggregators.messages()[1].sorted_items().to_vec();

    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn change_page_size() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);
    fixture.pager.on_next(PageRequest::new(1, 50));

    assert_eq!(fixture.aggregators.messages()[1].response().page(), 1, "Should be page 1");

    let expected_result = key_values(ordered_by(&people, &fixture.comparer).into_iter().take(50));
    let actual_result = fixture.aggregators.messages()[1].sorted_items().to_vec();

    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn page_greater_than_number_of_pages_available() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);
    fixture.pager.on_next(PageRequest::new(10, 25));

    assert_eq!(fixture.aggregators.messages()[1].response().page(), 4, "Page should move to the last page");

    let expected_result = key_values(ordered_by(&people, &fixture.comparer).into_iter().skip(75).take(25));
    let actual_result = fixture.aggregators.messages()[1].sorted_items().to_vec();

    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn page_initial_batch() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    assert_eq!(fixture.aggregators.data().count(), 25, "Should be 25 people in the cache");
    assert_eq!(fixture.aggregators.messages()[0].response().page_size(), 25, "Page size should be 25");
    assert_eq!(fixture.aggregators.messages()[0].response().page(), 1, "Should be page 1");
    assert_eq!(fixture.aggregators.messages()[0].response().pages(), 4, "Should be page 4 pages");

    let expected_result = key_values(ordered_by(&people, &fixture.comparer).into_iter().take(25));
    let actual_result = fixture.aggregators.messages()[0].sorted_items().to_vec();

    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn reorder_below_threshold() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(50);
    fixture.add_or_update(&people);

    let changed: PersonComparer = SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).into();
    fixture.sort.on_next(changed.clone());

    let expected_result = key_values(ordered_by(&people, &changed).into_iter().take(25));
    let actual_result = fixture.aggregators.messages().last().unwrap().sorted_items().to_vec();
    assert_equivalent(&actual_result, &expected_result);
}

// Not ported: ThrowsForNegativePage - argument validation (the page of a PageRequest is a usize)
// Not ported: ThrowsForNegativeSizeParameters - argument validation (the size of a PageRequest is a usize)
