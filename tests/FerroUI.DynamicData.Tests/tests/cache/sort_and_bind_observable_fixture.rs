//! `SortAndBindObservableFixture` and the fixtures of its targets
//! (`DynamicData.Tests/Cache/SortAndBindObservableFixture.cs`).

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::binding::SortExpressionComparer;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Comparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

/// The items of the bound list, read when a test looks (`IList<Person>`).
type BoundList = Rc<dyn Fn() -> Vec<Rc<Person>>>;

/// `SetUpTests`: the aggregator of the bound sequence and the bound list.
type SetUp = fn(&SourceCache<Rc<Person>, String>, Rc<dyn IObservable<Comparer>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList);

struct Fixture {
    cache: SourceCache<Rc<Person>, String>,
    generator: RandomPersonGenerator,
    results: ChangeSetAggregator<Rc<Person>, String>,
    bound_list: BoundList,
    oldest_comparer: Comparer,
    default_comparer: Comparer,
    comparer_observable: Rc<BehaviorSubject<Comparer>>,
}

impl Fixture {
    fn new(set_up: SetUp) -> Self {
        let cache = SourceCache::new(|p: &Rc<Person>| p.name());
        let oldest_comparer = SortExpressionComparer::<Rc<Person>>::descending(|p| p.age()).then_by_ascending(|p| p.name()).comparer();
        let default_comparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.name()).then_by_ascending(|p| p.age()).comparer();
        let comparer_observable = BehaviorSubject::new(default_comparer.clone());
        // Bind and sort in one hit.
        let (results, bound_list) = set_up(&cache, comparer_observable.as_observable());
        Self { cache, generator: RandomPersonGenerator::new(), results, bound_list, oldest_comparer, default_comparer, comparer_observable }
    }

    fn sorted(items: &[Rc<Person>], comparer: &Comparer) -> Vec<Rc<Person>> {
        let mut items = items.to_vec();
        items.sort_by(|a, b| comparer(a, b));
        items
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.cache.dispose();
        self.results.dispose();
        self.comparer_observable.on_completed();
    }
}

/// `SortAndBindObservableToReadOnlyObservableCollection`.
fn to_read_only_observable_collection(cache: &SourceCache<Rc<Person>, String>, comparer_observable: Rc<dyn IObservable<Comparer>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let (bound, list) = cache.connect().sort_and_bind_read_only_on_observable(comparer_observable);
    (bound.as_aggregator(), Rc::new(move || list.to_vec()))
}

/// `SortAndBindObservableToList`.
fn to_list(cache: &SourceCache<Rc<Person>, String>, comparer_observable: Rc<dyn IObservable<Comparer>>) -> (ChangeSetAggregator<Rc<Person>, String>, BoundList) {
    let list = Rc::new(RefCell::new(Vec::with_capacity(100)));
    let read = list.clone();
    (cache.connect().sort_and_bind_on_observable(&list, comparer_observable).as_aggregator(), Rc::new(move || read.borrow().clone()))
}

fn sort_initial_batch(fixture: &Fixture) {
    let people = fixture.generator.take(100);
    fixture.cache.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));

    let default_order = Fixture::sorted(&people, &fixture.default_comparer);
    assert_eq!((fixture.bound_list)(), default_order);
}

fn change_sort(fixture: &Fixture) {
    let people = fixture.generator.take(100);
    fixture.cache.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));

    // Change to the oldest first sort.
    fixture.comparer_observable.on_next(fixture.oldest_comparer.clone());

    let oldest_first = Fixture::sorted(&people, &fixture.oldest_comparer);
    assert_eq!((fixture.bound_list)(), oldest_first);

    // And back again.
    fixture.comparer_observable.on_next(fixture.default_comparer.clone());

    let default_order = Fixture::sorted(&people, &fixture.default_comparer);
    assert_eq!((fixture.bound_list)(), default_order);
}

macro_rules! sort_and_bind_observable_fixture {
    ($module:ident, $set_up:expr) => {
        mod $module {
            use super::Fixture;

            #[test]
            fn sort_initial_batch() {
                super::sort_initial_batch(&Fixture::new($set_up));
            }

            #[test]
            fn change_sort() {
                super::change_sort(&Fixture::new($set_up));
            }
        }
    };
}

sort_and_bind_observable_fixture!(sort_and_bind_observable_to_read_only_observable_collection, super::to_read_only_observable_collection);
sort_and_bind_observable_fixture!(sort_and_bind_observable_to_list, super::to_list);
