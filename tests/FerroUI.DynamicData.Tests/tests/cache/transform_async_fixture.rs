//! `TransformAsyncFixture` (`DynamicData.Tests/Cache/TransformAsyncFixture.cs`): the transform of
//! a cache by futures.
//!
//! Upstream's factories return `Task.FromResult(..)`, a task that has ended; here they return a
//! future that ends at its first poll, so the transforms end inside the edit, as upstream's do. A
//! test that awaits a delay awaits a task of a worker thread (`Task::run`), and `block_on` waits for
//! it (docs/porting/reactive.md, section 5.8).

use crate::domain::{Person, PersonWithAgeGroup, PersonWithGender, Randomizer};
use crate::utilities::{CacheObservableExtensions, ObservableExtensions};
use ferroui_dynamic_data::binding::ObservableCollectionEx;
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx, TransformAsyncOptions};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Notification, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::threading::tasks::{block_on, run, Task, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

type TransformResult = Pin<Box<dyn Future<Output = Result<Rc<PersonWithGender>, ObservableError>>>>;

/// `TransformFactory`: the future of the transform of a person (`Func<Person, Task<PersonWithGender>>`).
type TransformFactory = Rc<dyn Fn(Rc<Person>) -> TransformResult>;

/// The predicates of a forced transform.
type ForceTransform = Rc<dyn IObservable<Rc<dyn Fn(&Rc<Person>, &String) -> bool>>>;

/// The gender of the transform of the tests: `M` for an even age, `F` otherwise.
fn gender(p: &Person) -> &'static str {
    if p.age() % 2 == 0 {
        "M"
    } else {
        "F"
    }
}

/// A factory whose future ends at once with what `factory` gives (`Task.FromResult(factory(p))`).
fn completed_factory(factory: impl Fn(&Rc<Person>) -> Result<Rc<PersonWithGender>, ObservableError> + 'static) -> TransformFactory {
    Rc::new(move |p: Rc<Person>| -> TransformResult {
        let result = factory(&p);
        Box::pin(async move { result })
    })
}

/// The default factory of the stub.
fn default_factory() -> TransformFactory {
    completed_factory(|p| Ok(PersonWithGender::from_person(p, gender(p))))
}

/// A task that ends after `milliseconds` on a worker thread (`Task.Delay`).
fn delay(milliseconds: u64) -> Task<()> {
    run(move || {
        std::thread::sleep(Duration::from_millis(milliseconds));
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
}

/// `TransformStub`.
struct TransformStub {
    results: ChangeSetAggregator<Rc<PersonWithGender>, String>,
    source: SourceCache<Rc<Person>, String>,
    transform_factory: TransformFactory,
}

impl TransformStub {
    /// `TransformStub()`.
    fn new() -> Self {
        Self::create(default_factory(), None)
    }

    /// `TransformStub(Func<Person, PersonWithGender> factory)`.
    fn with_factory(factory: impl Fn(&Rc<Person>) -> Result<Rc<PersonWithGender>, ObservableError> + 'static) -> Self {
        Self::create(completed_factory(factory), None)
    }

    /// `TransformStub(IObservable<Unit> retransformer)`: every item is transformed again.
    fn with_retransform_all(retransformer: Rc<dyn IObservable<()>>) -> Self {
        let force: ForceTransform = retransformer.select(|_| {
            let transformer: Rc<dyn Fn(&Rc<Person>, &String) -> bool> = Rc::new(|_: &Rc<Person>, _: &String| true);
            transformer
        });
        Self::create(default_factory(), Some(force))
    }

    /// `TransformStub(IObservable<Func<Person, bool>> retransformer)`.
    fn with_retransform_selected(retransformer: Rc<dyn IObservable<Rc<dyn Fn(&Rc<Person>) -> bool>>>) -> Self {
        let force: ForceTransform = retransformer.select(|selector: Rc<dyn Fn(&Rc<Person>) -> bool>| {
            let transformed: Rc<dyn Fn(&Rc<Person>, &String) -> bool> = Rc::new(move |p: &Rc<Person>, _: &String| selector(p));
            transformed
        });
        Self::create(default_factory(), Some(force))
    }

    fn create(transform_factory: TransformFactory, force: Option<ForceTransform>) -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let factory = transform_factory.clone();
        let transformed = match force {
            None => source.connect().transform_async(move |p: Rc<Person>| factory(p)),
            Some(force) => source.connect().transform_async_forced(move |p: Rc<Person>| factory(p), force),
        };
        let results = ChangeSetAggregator::new(transformed);
        Self { results, source, transform_factory }
    }

    /// `await TransformFactory(person)`: the future ends at once.
    fn transform(&self, person: &Rc<Person>) -> Rc<PersonWithGender> {
        block_on((self.transform_factory)(person.clone())).expect("the transform succeeds")
    }
}

impl Drop for TransformStub {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn add() {
    let stub = TransformStub::new();
    let person = Person::new("Adult1", 50);
    stub.source.add_or_update(person.clone());

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.data().count(), 1, "Should be 1 item in the cache");

    let first_person = stub.transform(&person);

    assert_eq!(stub.results.data().items()[0], first_person, "Should be same person");
}

#[test]
fn batch_of_unique_updates() {
    let people: Vec<Rc<Person>> = (1..=100).map(|i| Person::new(format!("Name{i}"), i)).collect();
    let stub = TransformStub::new();
    stub.source.add_or_update_items(people.iter().cloned());

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should return 100 adds");

    let mut transformed: Vec<Rc<PersonWithGender>> = people.iter().map(|p| stub.transform(p)).collect();
    transformed.sort_by_key(|p| p.age());
    let mut items = stub.results.data().items();
    items.sort_by_key(|p| p.age());
    // Upstream compares the results with themselves; the transformed people are what it means.
    assert_eq!(items, transformed, "Incorrect transform result");
}

#[test]
fn clear() {
    let stub = TransformStub::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();

    stub.source.add_or_update_items(people);
    stub.source.clear();

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should be 80 adds");
    assert_eq!(stub.results.messages()[1].removes(), 100, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn handle_error() {
    let stub = TransformStub::with_factory(|_| Err(new_error("Broken")));
    stub.source.add_or_update(Person::new("Name1", 1));

    assert!(stub.results.error().is_some());
}

#[test]
fn remove() {
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    let stub = TransformStub::new();
    stub.source.add_or_update(person);
    stub.source.remove_key(KEY.to_owned());

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should be 80 addes");
    assert_eq!(stub.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn remove_flows_to_the_end() {
    const COUNT: usize = 100;

    let cache = SourceCache::new(|p: &Rc<Person>| p.name());
    let people: Vec<Rc<Person>> = (1..=COUNT as i32).map(|l| Person::new(format!("Name{l}"), l)).collect();

    let randomizer = Randomizer::shared();
    let (bound, collection) = cache
        .connect()
        .transform_async(move |person: Rc<Person>| {
            let wait = delay(randomizer.next(1, 12) as u64);
            async move {
                wait.await?;
                Ok::<_, ObservableError>(person)
            }
        })
        .bind_read_only();
    let _subscription = bound.subscribe_ignore();

    for p in &people {
        cache.add_or_update(p.clone());
        cache.remove_key(p.name());
    }

    // Add one event as an initial empty change set is sent.
    let changes = block_on(collection.to_observable_change_set().take(COUNT * 2 + 1).to_list().to_task()).expect("the changes arrive");

    assert_eq!(changes.len(), 201);
    assert_eq!(collection.count(), 0);
}

#[test]
fn re_transform_all() {
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("Name{i}"), i)).collect();
    let force_transform = Subject::<()>::new();

    let stub = TransformStub::with_retransform_all(force_transform.as_observable());
    stub.source.add_or_update_items(people);
    force_transform.on_next(());

    assert_eq!(stub.results.messages().len(), 2);
    assert_eq!(stub.results.messages()[1].updates(), 10);

    for i in 1..=10 {
        let original = stub.results.messages()[0].changes()[i - 1].current().clone();
        let updated = stub.results.messages()[1].changes()[i - 1].current().clone();

        assert_eq!(updated, original);
        assert!(!Rc::ptr_eq(&original, &updated));
    }
}

#[test]
fn re_transform_selected() {
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new(format!("Name{i}"), i)).collect();
    let force_transform = Subject::<Rc<dyn Fn(&Rc<Person>) -> bool>>::new();

    let stub = TransformStub::with_retransform_selected(force_transform.as_observable());
    stub.source.add_or_update_items(people);
    force_transform.on_next(Rc::new(|person: &Rc<Person>| person.age() <= 5));

    assert_eq!(stub.results.messages().len(), 2);
    assert_eq!(stub.results.messages()[1].updates(), 5);

    for i in 1..=5 {
        let original = stub.results.messages()[0].changes()[i - 1].current().clone();
        let updated = stub.results.messages()[1].changes()[i - 1].current().clone();
        assert_eq!(updated, original);
        assert!(!Rc::ptr_eq(&original, &updated));
    }
}

#[test]
fn same_key_changes() {
    let stub = TransformStub::new();
    let people: Vec<Rc<Person>> = (1..=10).map(|i| Person::new("Name", i)).collect();

    stub.source.add_or_update_items(people.iter().cloned());

    assert_eq!(stub.results.messages().len(), 1, "Should be 1 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should return 1 adds");
    assert_eq!(stub.results.messages()[0].updates(), 9, "Should return 9 adds");
    assert_eq!(stub.results.data().count(), 1, "Should result in 1 record");

    let last_transformed = stub.transform(people.last().expect("there are people"));
    let only_item_in_cache = stub.results.data().items()[0].clone();

    assert_eq!(only_item_in_cache, last_transformed, "Incorrect transform result");
}

#[test]
fn update() {
    const KEY: &str = "Adult1";
    let newperson = Person::new(KEY, 50);
    let updated = Person::new(KEY, 51);

    let stub = TransformStub::new();
    stub.source.add_or_update(newperson);
    stub.source.add_or_update(updated);

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should be 1 adds");
    assert_eq!(stub.results.messages()[1].updates(), 1, "Should be 1 update");
}

/// `TransformOnRefresh(bool transformOnRefresh)`.
fn transform_on_refresh(transform_on_refresh: bool) {
    let source = SourceCache::new(|p: &Rc<Person>| p.name());
    let results = source
        .connect()
        .auto_refresh_any()
        .transform_async_with_key_and_options(
            |p: Rc<Person>, _key: String| async move {
                let age_group = if p.age() < 18 { "Child" } else { "Adult" };
                Ok::<_, ObservableError>(PersonWithAgeGroup::new(p, age_group))
            },
            TransformAsyncOptions { transform_on_refresh, ..TransformAsyncOptions::DEFAULT },
        )
        .as_aggregator();

    let person = Person::new("SomeOne", 16);
    source.add_or_update(person.clone());

    assert_eq!(results.data().count(), 1);
    assert_eq!(results.data().lookup(&"SomeOne".to_owned()).expect("the person is there").age_group, "Child");

    person.set_age(21);

    assert_eq!(results.data().count(), 1);
    assert_eq!(results.data().lookup(&"SomeOne".to_owned()).expect("the person is there").age_group, if transform_on_refresh { "Adult" } else { "Child" });

    results.dispose();
    source.dispose();
}

#[test]
fn transform_on_refresh_case1() {
    transform_on_refresh(true);
}

#[test]
fn transform_on_refresh_case2() {
    transform_on_refresh(false);
}

#[test]
fn transform_async_cancels_token_on_un_subscribe() {
    let source = SourceCache::new(|p: &Rc<Person>| p.name());
    let tcs = TaskCompletionSource::<Rc<Person>>::new();
    let factory_tcs = tcs.clone();
    let sub = source
        .connect()
        .transform_async_with_token(move |_c: Rc<Person>, _p: Option<Rc<Person>>, _key: String, cancel: CancellationToken| {
            let tcs = factory_tcs.clone();
            async move {
                let canceled = tcs.clone();
                let registration = cancel.register_local(move || canceled.set_canceled());
                let result = tcs.task().await;
                registration.dispose();
                result
            }
        })
        .subscribe_ignore();

    source.add_or_update(Person::unknown());

    sub.dispose();
    assert!(tcs.task().is_canceled());

    source.dispose();
}

/// `WithMaxConcurrency(int maxConcurrency)`: the transforms await a delay of 100 milliseconds on
/// worker threads.
fn with_max_concurrency(max_concurrency: usize) {
    const TRANSFORM_COUNT: usize = 100;

    let source = SourceCache::new(|p: &Rc<Person>| p.name());
    let results = source
        .connect()
        .transform_async_with_key_and_options(
            |p: Rc<Person>, _key: String| async move {
                delay(100).await?;
                let age_group = if p.age() < 18 { "Child" } else { "Adult" };
                Ok::<_, ObservableError>(PersonWithAgeGroup::new(p, age_group))
            },
            TransformAsyncOptions { maximum_concurrency: Some(max_concurrency), ..TransformAsyncOptions::DEFAULT },
        )
        .as_aggregator();

    source.add_or_update_items((1..=TRANSFORM_COUNT as i32).map(|l| Person::new(format!("Person{l}"), l)));

    block_on(results.data().count_changed().where_(|c| *c == TRANSFORM_COUNT).take(1).to_task()).expect("every item is transformed");

    results.dispose();
    source.dispose();
}

#[test]
fn with_max_concurrency_case1() {
    with_max_concurrency(10);
}

#[test]
fn with_max_concurrency_case2() {
    with_max_concurrency(100);
}

// The forced-transform chain applies its cache updates when its transforms end. Upstream holds one
// transform outstanding on each chain and releases them together from two threads; on one thread
// they are released one after the other, and the deliveries are checked for overlap and for the
// integrity of the change sets. Upstream also holds each notification for 100 milliseconds
// (`Thread.Sleep`) to make an overlap observable, which one thread does not need.
#[test]
fn forced_transform_completing_alongside_source_update_are_delivered_serially() {
    let source = SourceCache::new(|p: &Rc<Person>| p.name());
    let force = Subject::<Rc<dyn Fn(&Rc<Person>, &String) -> bool>>::new();
    let registered = Rc::new(Cell::new(0usize));

    // One release handle per transform invocation, so each can be completed on command.
    let outstanding: Rc<RefCell<HashMap<String, VecDeque<TaskCompletionSource<()>>>>> = Rc::new(RefCell::new(HashMap::new()));

    let (factory_outstanding, factory_registered) = (outstanding.clone(), registered.clone());
    let published = source
        .connect()
        .transform_async_forced(
            move |person: Rc<Person>| {
                let release = TaskCompletionSource::<()>::new();
                factory_outstanding.borrow_mut().entry(person.name()).or_default().push_back(release.clone());
                factory_registered.set(factory_registered.get() + 1);
                async move {
                    release.task().await?;
                    Ok::<_, ObservableError>(PersonWithGender::from_person(&person, gender(&person)))
                }
            },
            force.as_observable(),
        )
        .validate_synchronization()
        .validate_cache_change_sets(|person_with_gender: &Rc<PersonWithGender>| person_with_gender.name())
        .publish();

    let shared: Rc<dyn IObservable<ChangeSet<Rc<PersonWithGender>, String>>> = published.clone();
    let terminal = shared.materialize().to_task();
    let results = shared.as_aggregator();
    let connection = published.connect();

    let release = |name: &str| {
        let release = outstanding.borrow_mut().get_mut(name).and_then(VecDeque::pop_front);
        release.unwrap_or_else(|| panic!("a transform for {name} should be outstanding")).set_result(());
    };

    // Seed a single item, so that the forced pass has something to re-transform.
    source.add_or_update(Person::new("Seed", 2));
    assert_eq!(registered.get(), 1, "the seed transform should have started");
    release("Seed");
    assert_eq!(results.data().count(), 1);

    // Leave one transform outstanding on each chain: the forced pass re-transforms the seed, and
    // the source update introduces a second item through the other chain.
    force.on_next(Rc::new(|_: &Rc<Person>, _: &String| true));
    assert_eq!(registered.get(), 2, "the forced transform should have started");

    source.add_or_update(Person::new("Added", 4));
    assert_eq!(registered.get(), 3, "the source transform should have started");

    // Release both.
    release("Seed");
    release("Added");

    // Both merge inputs have to complete before the merged sequence does.
    force.on_completed();
    source.dispose();

    assert!(terminal.is_completed(), "the sequence should have ended");
    let last_notification = terminal.result().expect("the sequence has a last notification");

    match last_notification {
        Notification::OnCompleted => {}
        Notification::OnError(error) => panic!("a forced transform and a source update must never be delivered concurrently: {error}"),
        Notification::OnNext(_) => panic!("the sequence should end by completing, not by faulting"),
    }

    connection.dispose();
    results.dispose();
}

// Not ported: ForcedTransformsUnderConcurrentLoad_AreDeliveredSerially - real threads
