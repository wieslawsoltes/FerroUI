//! `SortFixtureWithReorder` and `SortFixture` (`DynamicData.Tests/Cache/SortFixture.cs`): the
//! sort of a cache by a comparer, with the sorted change sets applied to a collection by the
//! sorted adaptor.
//!
//! The two fixtures are the same upstream but for two tests; the shared tests come from
//! `shared_sort_tests!`.

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::cache::{IConnectableCache, SourceCache};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

type PersonComparer = Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering>;

fn add_or_update(source: &SourceCache<Rc<Person>, String>, people: &[Rc<Person>]) {
    source.edit(|updater| updater.add_or_update_items(&mut people.iter().cloned()));
}

fn add_or_update_one(source: &SourceCache<Rc<Person>, String>, person: &Rc<Person>) {
    source.edit(|updater| updater.add_or_update(person.clone()));
}

fn remove_key(source: &SourceCache<Rc<Person>, String>, key: &str) {
    source.edit(|updater| updater.remove_key(&key.to_owned()));
}

fn ordered_by(people: &[Rc<Person>], comparer: &PersonComparer) -> Vec<Rc<Person>> {
    let mut ordered = people.to_vec();
    ordered.sort_by(|x, y| comparer(x, y));
    ordered
}

/// `SortFixture.Data`.
#[allow(dead_code)]
pub(crate) struct Data {
    id: i32,
    #[allow(dead_code)]
    value: String,
}

impl Data {
    fn id(&self) -> i32 {
        self.id
    }
}

/// `SortFixture.TestString`: a string equal to another regardless of case.
#[derive(Clone, Debug)]
pub(crate) struct TestString(String);

impl TestString {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TestString {
    fn from(source: String) -> Self {
        Self(source)
    }
}

impl PartialEq for TestString {
    /// `StringComparer.OrdinalIgnoreCase.Equals`.
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for TestString {}

impl Hash for TestString {
    /// `StringComparer.OrdinalIgnoreCase.GetHashCode`.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_uppercase().hash(state);
    }
}

/// `SortFixture.ViewModel`: equal only to itself, as upstream's class.
#[derive(Debug)]
pub(crate) struct ViewModel {
    name: String,
}

impl ViewModel {
    fn new(name: impl Into<String>) -> Rc<Self> {
        Rc::new(Self { name: name.into() })
    }

    /// `ViewModel.Comparer`: by the name, ignoring case (`StringComparer.OrdinalIgnoreCase`).
    fn comparer() -> Rc<dyn Fn(&Rc<ViewModel>, &Rc<ViewModel>) -> Ordering> {
        Rc::new(|x: &Rc<ViewModel>, y: &Rc<ViewModel>| x.name.to_ascii_uppercase().cmp(&y.name.to_ascii_uppercase()))
    }
}

impl PartialEq for ViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The tests that both fixtures have.
macro_rules! shared_sort_tests {
    () => {
        #[test]
        fn append_at_beginning() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            // Create the age 0 to ensure it is inserted first; upstream names it "_Aaron", which sorts
            // first by culture. Names compare ordinally here, where "!" sorts before every letter.
            let insert = Person::new("!Aaron", 0);
            add_or_update_one(&fixture.source, &insert);

            assert_eq!(fixture.results.data().count(), 101, "Should be 101 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            let indexed_item = indexed.get("!Aaron");
            assert!(indexed_item.is_some());
            assert_eq!(indexed_item.unwrap().index(), 0, "Inserted item should have index of zero");
        }

        #[test]
        fn append_at_end() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let insert = Person::new("zzzzz", 1000);
            add_or_update_one(&fixture.source, &insert);

            assert_eq!(fixture.results.data().count(), 101, "Should be 101 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            assert!(indexed.get("zzzzz").is_some());

            // Upstream orders by the comparer object itself, a constant key, which keeps the order.
            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn append_in_middle() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let insert = Person::new("Marvin", 50);
            add_or_update_one(&fixture.source, &insert);

            assert_eq!(fixture.results.data().count(), 101, "Should be 101 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            assert!(indexed.get("Marvin").is_some());

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn batch_update1() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(10);
            add_or_update(&fixture.source, &people);
            let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));

            let to_update = people[3].clone();
            fixture.source.edit(|updater| {
                updater.remove_key(&people[0].key());
                updater.remove_key(&people[1].key());
                updater.add_or_update(Person::new(to_update.name(), to_update.age() - 24));
                updater.remove_item(&people[7]);
            });

            let adaptor = SortedObservableCollectionAdaptor::default();
            adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

            let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
            assert_equivalent(&list.to_vec(), &should_be);
        }

        #[test]
        fn batch_update2() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(10);
            add_or_update(&fixture.source, &people);
            let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));

            let to_update = people[3].clone();
            fixture.source.edit(|updater| {
                updater.add_or_update(Person::new(to_update.name(), to_update.age() - 24));
                updater.add_or_update(Person::from_names_with("Mr", "Z", 50, "M", ""));
            });

            let adaptor = SortedObservableCollectionAdaptor::default();
            adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

            let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
            assert_equivalent(&list.to_vec(), &should_be);
        }

        #[test]
        fn batch_update3() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(10);
            add_or_update(&fixture.source, &people);
            let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));

            let to_update = people[7].clone();
            fixture.source.edit(|updater| {
                updater.add_or_update(Person::new(to_update.name(), to_update.age() - 24));
                updater.add_or_update(Person::from_names_with("Mr", "A", 10, "M", ""));
                updater.add_or_update(Person::from_names_with("Mr", "B", 40, "M", ""));
                updater.add_or_update(Person::from_names_with("Mr", "C", 70, "M", ""));
            });

            let adaptor = SortedObservableCollectionAdaptor::default();
            adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

            let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
            assert_equivalent(&list.to_vec(), &should_be);
        }

        #[test]
        fn batch_update4() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(10);
            add_or_update(&fixture.source, &people);
            let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));

            let to_update = people[3].clone();
            fixture.source.edit(|updater| {
                updater.add_or_update(Person::new(to_update.name(), to_update.age() - 24));
                updater.add_or_update(Person::from_names_with("Mr", "A", 10, "M", ""));
                updater.remove_item(&people[5]);
                updater.add_or_update(Person::from_names_with("Mr", "C", 70, "M", ""));
            });

            let adaptor = SortedObservableCollectionAdaptor::default();
            adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

            let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
            assert_equivalent(&list.to_vec(), &should_be);
        }

        #[test]
        fn batch_update6() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(10);
            add_or_update(&fixture.source, &people);

            let more = fixture.generator.take(10);
            fixture.source.edit(|updater| {
                updater.clear();
                updater.add_or_update_items(&mut more.iter().cloned());
                updater.clear();
            });

            let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));
            let adaptor = SortedObservableCollectionAdaptor::default();
            adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

            let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
            assert_equivalent(&list.to_vec(), &should_be);
        }

        #[test]
        fn does_not_throw1() {
            let cache = SourceCache::<Rc<Data>, i32>::new(|d| d.id());
            let sort_pump = Subject::<()>::new();
            let comparer = SortExpressionComparer::<Rc<Data>>::ascending(|d| d.id()).into();
            let disposable = cache
                .connect()
                .sort_with_resorter(comparer, sort_pump.as_observable(), SortOptimisations::NONE, DEFAULT_SORT_RESET_THRESHOLD)
                .subscribe_ignore();

            disposable.dispose();
        }

        #[test]
        fn does_not_throw2() {
            let cache = SourceCache::<Rc<Data>, i32>::new(|d| d.id());
            let comparer: Rc<dyn Fn(&Rc<Data>, &Rc<Data>) -> Ordering> = SortExpressionComparer::<Rc<Data>>::ascending(|d| d.id()).into();
            let disposable = cache.connect().sort_dynamic(BehaviorSubject::new(comparer).as_observable()).subscribe_ignore();

            disposable.dispose();
        }

        #[test]
        fn inline_update_produces_a_replace() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(10);
            add_or_update(&fixture.source, &people);
            let to_update = people[3].clone();

            add_or_update_one(&fixture.source, &Person::new(to_update.name(), to_update.age() + 1));

            let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));
            let adaptor = SortedObservableCollectionAdaptor::default();
            adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

            let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
            assert_equivalent(&list.to_vec(), &should_be);
        }

        #[test]
        fn remove_first() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let remove = fixture.results.messages()[0].sorted_items()[0].clone();
            remove_key(&fixture.source, &remove.0);

            assert_eq!(fixture.results.data().count(), 99, "Should be 99 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            assert!(indexed.get(&remove.0).is_none());

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn remove_from_end() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let remove = fixture.results.messages()[0].sorted_items().last().unwrap().clone();
            remove_key(&fixture.source, &remove.0);

            assert_eq!(fixture.results.data().count(), 99, "Should be 99 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            assert!(indexed.get(&remove.0).is_none());

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn remove_from_middle() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let remove = fixture.results.messages()[0].sorted_items().iter().skip(50).next().unwrap().clone();
            remove_key(&fixture.source, &remove.0);

            assert_eq!(fixture.results.data().count(), 99, "Should be 99 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            assert!(indexed.get(&remove.0).is_none());

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn sort_after_filter() {
            let source = SourceCache::<Rc<Person>, String>::new(|p| p.key());
            let accept_all: Rc<dyn Fn(&Rc<Person>) -> bool> = Rc::new(|_: &Rc<Person>| true);
            let filter_subject = BehaviorSubject::new(accept_all);

            let _agg = SortedChangeSetAggregator::new(
                source
                    .connect()
                    .filter_dynamic(filter_subject.as_observable())
                    .group(|x: &Rc<Person>| TestString::from(x.key()))
                    .transform(|x| ViewModel::new(String::from(x.key().as_str())))
                    .sort(ViewModel::comparer()),
            );

            source.edit(|x| {
                x.add_or_update(Person::with_gender("A", 1, "F"));
                x.add_or_update(Person::with_gender("a", 1, "M"));
                x.add_or_update(Person::with_gender("B", 1, "F"));
                x.add_or_update(Person::with_gender("b", 1, "M"));
            });

            let only_a: Rc<dyn Fn(&Rc<Person>) -> bool> = Rc::new(|p: &Rc<Person>| p.name().eq_ignore_ascii_case("a"));
            filter_subject.on_next(only_a);
        }

        #[test]
        fn sort_after_filter_list() {
            let source = SourceList::<Rc<Person>>::new();
            let accept_all: Rc<dyn Fn(&Rc<Person>) -> bool> = Rc::new(|_: &Rc<Person>| true);
            let filter_subject = BehaviorSubject::new(accept_all);

            let _agg = source
                .connect()
                .filter_dynamic(filter_subject.as_observable())
                .transform(|x: &Rc<Person>| ViewModel::new(x.name()))
                .sort(ViewModel::comparer())
                .as_aggregator();

            source.edit(|x| {
                x.add(Person::with_gender("A", 1, "F"));
                x.add(Person::with_gender("a", 1, "M"));
                x.add(Person::with_gender("B", 1, "F"));
                x.add(Person::with_gender("b", 1, "M"));
            });

            let only_a: Rc<dyn Fn(&Rc<Person>) -> bool> = Rc::new(|p: &Rc<Person>| p.name().eq_ignore_ascii_case("a"));
            filter_subject.on_next(only_a);
        }

        #[test]
        fn sort_initial_batch() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");

            let expected_result: Vec<(String, Rc<Person>)> = ordered_by(&people, &fixture.comparer).into_iter().map(|p| (p.name(), p)).collect();
            let actual_result = fixture.results.messages()[0].sorted_items().to_vec();

            assert_equivalent(&actual_result, &expected_result);
        }

        #[test]
        fn update_first() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let to_update = fixture.results.messages()[0].sorted_items()[0].1.clone();
            let update = Person::new(to_update.name(), to_update.age() + 5);
            add_or_update_one(&fixture.source, &update);

            assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            let indexed_item = indexed.get(&update.key());
            assert!(indexed_item.is_some());
            assert!(Rc::ptr_eq(&update, indexed_item.unwrap().value()));

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn update_last() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let to_update = fixture.results.messages()[0].sorted_items().last().unwrap().1.clone();
            let update = Person::new(to_update.name(), to_update.age() + 5);
            add_or_update_one(&fixture.source, &update);

            assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            let indexed_item = indexed.get(&update.key());
            assert!(indexed_item.is_some());
            assert!(Rc::ptr_eq(&update, indexed_item.unwrap().value()));

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }

        #[test]
        fn update_middle() {
            let fixture = Fixture::new();
            let people = fixture.generator.take(100);
            add_or_update(&fixture.source, &people);

            let to_update = fixture.results.messages()[0].sorted_items().iter().skip(50).next().unwrap().1.clone();
            let update = Person::new(to_update.name(), to_update.age() + 5);
            add_or_update_one(&fixture.source, &update);

            assert_eq!(fixture.results.data().count(), 100, "Should be 100 people in the cache");
            let indexed = fixture.results.messages()[1].sorted_items().indexed();
            let indexed_item = indexed.get(&update.key());
            assert!(indexed_item.is_some());
            assert!(Rc::ptr_eq(&update, indexed_item.unwrap().value()));

            let list = fixture.results.messages()[1].sorted_items().to_vec();
            let sorted_result = list.clone();
            assert_equivalent(&list, &sorted_result);
        }
    };
}

/// The setup of both fixtures: people sorted by age, then by name.
struct Fixture {
    comparer: PersonComparer,
    generator: RandomPersonGenerator,
    results: ferroui_dynamic_data::cache::testing::SortedChangeSetAggregator<Rc<Person>, String>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        use ferroui_dynamic_data::binding::SortExpressionComparer;
        use ferroui_dynamic_data::cache::ObservableCacheEx;
        let comparer: PersonComparer = SortExpressionComparer::<Rc<Person>>::ascending(|p| p.age()).then_by_ascending(|p| p.name()).into();
        let source = SourceCache::new(|p: &Rc<Person>| p.key());
        let results = ferroui_dynamic_data::cache::testing::SortedChangeSetAggregator::new(source.connect().sort(comparer.clone()));
        Self { comparer, generator: RandomPersonGenerator::new(), results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        use ferroui_system::IDisposable;
        self.source.dispose();
        self.results.dispose();
    }
}

/// `SortFixtureWithReorder`.
mod sort_fixture_with_reorder {
    use super::super::key_value_collection_ex::KeyValueCollectionEx;
    use super::*;
    use crate::utilities::assert_equivalent;
    use ferroui_dynamic_data::binding::{ISortedObservableCollectionAdaptor, ObservableCollectionExtended, SortExpressionComparer, SortedObservableCollectionAdaptor};
    use ferroui_dynamic_data::cache::testing::SortedChangeSetAggregator;
    use ferroui_dynamic_data::cache::{IObservableCache, ObservableCacheEx, SortOptimisations, DEFAULT_SORT_RESET_THRESHOLD};
    use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
    use ferroui_reactive::subjects::{BehaviorSubject, Subject};
    use ferroui_reactive::ObservableExt;
    use ferroui_system::IObserver;

    shared_sort_tests!();

    #[test]
    fn batch_update_where_update_moves_the_index_down() {
        let fixture = Fixture::new();
        let people = fixture.generator.take(10);
        add_or_update(&fixture.source, &people);

        let to_update = people[3].clone();
        let more = fixture.generator.take(2);
        fixture.source.edit(|updater| {
            updater.remove_key(&people[0].key());
            updater.remove_key(&people[1].key());

            updater.add_or_update(Person::new(to_update.name(), to_update.age() + 50));

            updater.add_or_update_items(&mut more.iter().cloned());

            updater.remove_item(&people[7]);
        });

        let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));
        let adaptor = SortedObservableCollectionAdaptor::default();
        adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

        let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
        assert_equivalent(&list.to_vec(), &should_be);
    }
}

/// `SortFixture`.
mod sort_fixture {
    use super::super::key_value_collection_ex::KeyValueCollectionEx;
    use super::*;
    use crate::utilities::assert_equivalent;
    use ferroui_dynamic_data::binding::{ISortedObservableCollectionAdaptor, ObservableCollectionExtended, SortExpressionComparer, SortedObservableCollectionAdaptor};
    use ferroui_dynamic_data::cache::testing::SortedChangeSetAggregator;
    use ferroui_dynamic_data::cache::{IObservableCache, ObservableCacheEx, SortOptimisations, DEFAULT_SORT_RESET_THRESHOLD};
    use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
    use ferroui_reactive::subjects::{BehaviorSubject, Subject};
    use ferroui_reactive::ObservableExt;
    use ferroui_system::IObserver;

    shared_sort_tests!();

    #[test]
    fn batch_update_shifting_indicies() {
        let fixture = Fixture::new();
        let test_data = vec![
            Person::new("A", 3),
            Person::new("B", 5),
            Person::new("C", 7),
            Person::new("D", 8),
            Person::new("E", 10),
            Person::new("F", 12),
            Person::new("G", 14),
        ];
        add_or_update(&fixture.source, &test_data);
        let list = ObservableCollectionExtended::from_items(ordered_by(&test_data, &fixture.comparer));

        let to_update1 = test_data[0].clone();
        let to_update2 = test_data[3].clone();
        fixture.source.edit(|updater| {
            updater.add_or_update(Person::new(to_update1.name(), 6));
            updater.add_or_update(Person::new(to_update2.name(), 2));
        });

        let adaptor = SortedObservableCollectionAdaptor::default();
        adaptor.adapt(fixture.results.messages().last().unwrap(), &list);
        let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
        assert_equivalent(&list.to_vec(), &should_be);
    }

    #[test]
    fn batch_update_where_update_moves_the_index_down() {
        let fixture = Fixture::new();
        let people = fixture.generator.take(10);
        add_or_update(&fixture.source, &people);

        let to_update = people[3].clone();
        fixture.source.edit(|updater| {
            updater.remove_key(&people[0].key());
            updater.remove_key(&people[1].key());

            updater.add_or_update(Person::new(to_update.name(), to_update.age() + 50));

            updater.remove_item(&people[7]);
        });

        let list = ObservableCollectionExtended::from_items(ordered_by(&people, &fixture.comparer));
        let adaptor = SortedObservableCollectionAdaptor::default();
        adaptor.adapt(fixture.results.messages().last().unwrap(), &list);

        let should_be: Vec<Rc<Person>> = fixture.results.messages().last().unwrap().sorted_items().values().cloned().collect();
        assert_equivalent(&list.to_vec(), &should_be);
    }
}
