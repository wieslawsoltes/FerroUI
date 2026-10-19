//! `SortObservableFixture` (`DynamicData.Tests/Cache/SortObservableFixtureFixture.cs`): the sort of
//! a cache by the latest comparer of an observable.

use crate::domain::{Person, RandomPersonGenerator, Randomizer};
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{ISortedObservableCollectionAdaptor, ObservableCollectionExtended, SortExpressionComparer, SortedObservableCollectionAdaptor};
use ferroui_dynamic_data::cache::testing::SortedChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SortReason, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObserver};
use std::cmp::Ordering;
use std::rc::Rc;

type PersonComparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

struct Fixture {
    cache: SourceCache<Rc<Person>, String>,
    comparer: PersonComparer,
    comparer_observable: Rc<BehaviorSubject<PersonComparer>>,
    generator: RandomPersonGenerator,
    results: SortedChangeSetAggregator<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let comparer: PersonComparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.name()).then_by_ascending(|p| p.age()).into();
        let comparer_observable = BehaviorSubject::new(comparer.clone());
        let cache = SourceCache::new(|p: &Rc<Person>| p.name());
        let results = SortedChangeSetAggregator::new(cache.connect().sort_dynamic_with(
            comparer_observable.as_observable(),
            ferroui_dynamic_data::cache::SortOptimisations::NONE,
            25,
        ));
        Self { cache, comparer, comparer_observable, generator: RandomPersonGenerator::new(), results }
    }

    fn add_or_update(&self, people: &[Rc<Person>]) {
        self.cache.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.cache.dispose();
        self.results.dispose();
        self.comparer_observable.on_completed();
    }
}

fn ordered_by(people: &[Rc<Person>], comparer: &PersonComparer) -> Vec<Rc<Person>> {
    let mut ordered = people.to_vec();
    ordered.sort_by(|x, y| comparer(x, y));
    ordered
}

fn key_values(people: Vec<Rc<Person>>) -> Vec<(String, Rc<Person>)> {
    people.into_iter().map(|p| (p.name(), p)).collect()
}

#[test]
fn change_sort() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    let desc: PersonComparer = SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).into();

    fixture.comparer_observable.on_next(desc.clone());
    let expected_result = key_values(ordered_by(&people, &desc));
    let actual_result = fixture.results.messages()[0].sorted_items().to_vec();
    let _moves_count = fixture.results.messages()[0].moves();

    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn change_sort_above_threshold() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(30);
    fixture.add_or_update(&people);

    let desc: PersonComparer = SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).into();

    fixture.comparer_observable.on_next(desc.clone());
    let expected_result = key_values(ordered_by(&people, &desc));
    let items = fixture.results.messages().last().unwrap().sorted_items().clone();
    let actual_result = items.to_vec();
    let sort_reason = items.sort_reason();
    assert_equivalent(&actual_result, &expected_result);
    assert_eq!(sort_reason, SortReason::Reset);
}

#[test]
fn change_sort_within_threshold() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(20);
    fixture.add_or_update(&people);

    let desc: PersonComparer = SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).into();

    fixture.comparer_observable.on_next(desc.clone());
    let expected_result = key_values(ordered_by(&people, &desc));
    let items = fixture.results.messages().last().unwrap().sorted_items().clone();
    let actual_result = items.to_vec();
    let sort_reason = items.sort_reason();
    assert_equivalent(&actual_result, &expected_result);
    assert_eq!(sort_reason, SortReason::Reorder);
}

#[test]
fn inline_changes() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(10000);
    fixture.add_or_update(&people);

    // Apply mutable changes to the items.
    let random = Randomizer::new_unseeded();
    let to_change: Vec<Rc<Person>> = random.shuffle(&people).into_iter().take(10).collect();
    for person in &to_change {
        person.set_age(random.next(0, 100));
    }

    fixture.cache.edit(|updater| updater.refresh_items(&mut to_change.iter().cloned()));

    let expected = ordered_by(&people, &fixture.comparer);
    let actual: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
    assert_equivalent(&actual, &expected);

    let list = ObservableCollectionExtended::<Rc<Person>>::new();
    let adaptor = SortedObservableCollectionAdaptor::default();
    for message in fixture.results.messages() {
        adaptor.adapt(&message, &list);
    }

    assert_equivalent(&list.to_vec(), &expected);
}

#[test]
fn reset() {
    let fixture = Fixture::new();
    let ordered: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("P{i}"), i)).collect();
    let people = Randomizer::new_unseeded().shuffle(&ordered);
    fixture.add_or_update(&people);
    fixture.comparer_observable.on_next(SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).into());
    fixture.comparer_observable.on_next(fixture.comparer.clone());

    let expected_result = key_values(ordered_by(&people, &fixture.comparer));
    let actual_result = fixture.results.messages()[2].sorted_items().to_vec();
    assert_equivalent(&actual_result, &expected_result);
}

#[test]
fn sort_initial_batch() {
    let fixture = Fixture::new();
    let people = fixture.generator.take(100);
    fixture.add_or_update(&people);

    assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");

    let expected_result = key_values(ordered_by(&people, &fixture.comparer));
    let actual_result = fixture.results.messages()[0].sorted_items().to_vec();

    assert_equivalent(&actual_result, &expected_result);
}
