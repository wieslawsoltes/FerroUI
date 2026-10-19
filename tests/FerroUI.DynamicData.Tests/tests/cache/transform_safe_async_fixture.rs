//! `TransformSafeAsyncFixture` (`DynamicData.Tests/Cache/TransformSafeAsyncFixture.cs` and
//! `TransformSafeAsyncFixture.ArgumentValidation.cs`): the transform of a cache by futures that may
//! fail, whose errors go to the error handler.
//!
//! Upstream's factories return `Task.FromResult(..)`, a task that has ended; here they return a
//! future that ends at its first poll. A test that awaits a delay awaits a task of a worker thread
//! (`Task::run`), and `block_on` waits for it (docs/porting/reactive.md, section 5.8).

use crate::domain::{Person, PersonWithAgeGroup, PersonWithGender};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx, TransformAsyncOptions};
use ferroui_dynamic_data::kernel::Error;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::new_error;
use ferroui_system::threading::tasks::{block_on, run, Task, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

type TransformResult = Pin<Box<dyn Future<Output = Result<Rc<PersonWithGender>, ObservableError>>>>;

/// `TransformFactory` (`Func<Person, Task<PersonWithGender>>`).
type TransformFactory = Rc<dyn Fn(Rc<Person>) -> TransformResult>;

/// The predicates of a forced transform.
type ForceTransform = Rc<dyn IObservable<Rc<dyn Fn(&Rc<Person>, &String) -> bool>>>;

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
    handled_errors: Rc<RefCell<Vec<Error<Rc<Person>, String>>>>,
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

    /// `TransformStub(IObservable<Unit> retransformer)`.
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
        let handled_errors = Rc::new(RefCell::new(Vec::new()));
        let factory = transform_factory.clone();
        // `ErrorHandler`.
        let target = handled_errors.clone();
        let error_handler = move |error: Error<Rc<Person>, String>| target.borrow_mut().push(error);
        let transformed = match force {
            None => source.connect().transform_safe_async(move |p: Rc<Person>| factory(p), error_handler),
            Some(force) => source.connect().transform_safe_async_forced(move |p: Rc<Person>| factory(p), error_handler, force),
        };
        let results = ChangeSetAggregator::new(transformed);
        Self { handled_errors, results, source, transform_factory }
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
fn remove() {
    const KEY: &str = "Adult1";
    let person = Person::new(KEY, 50);

    let stub = TransformStub::new();
    stub.source.add_or_update(person);
    stub.source.remove_key(KEY.to_owned());

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 1, "Should be 80 adds");
    assert_eq!(stub.results.messages()[1].removes(), 1, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
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
fn clear() {
    let stub = TransformStub::new();
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();

    stub.source.add_or_update_items(people);
    stub.source.clear();

    assert_eq!(stub.results.messages().len(), 2, "Should be 2 updates");
    assert_eq!(stub.results.messages()[0].adds(), 100, "Should be 80 addes");
    assert_eq!(stub.results.messages()[1].removes(), 100, "Should be 80 removes");
    assert_eq!(stub.results.data().count(), 0, "Should be nothing cached");
}

#[test]
fn handle_error() {
    let stub = TransformStub::with_factory(|p| {
        if p.age() <= 50 {
            return Ok(PersonWithGender::from_person(p, gender(p)));
        }

        Err(new_error("Broken"))
    });
    let people: Vec<Rc<Person>> = (1..=100).map(|l| Person::new(format!("Name{l}"), l)).collect();
    stub.source.add_or_update_items(people);

    assert!(stub.results.error().is_none());

    let error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let target = error.clone();
    let _subscription = stub.source.connect().subscribe_next_error(|_changes| {}, move |ex| *target.borrow_mut() = Some(ex));

    assert!(error.borrow().is_none());

    assert_eq!(stub.handled_errors.borrow().len(), 50);
    assert_eq!(stub.results.data().count(), 50);
}

/// `TransformOnRefresh(bool transformOnRefresh)`.
fn transform_on_refresh(transform_on_refresh: bool) {
    let error_count = 0;
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
    assert_eq!(error_count, 0);

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
fn transform_safe_async_cancels_token_on_un_subscribe() {
    let source = SourceCache::new(|p: &Rc<Person>| p.name());
    let tcs = TaskCompletionSource::<Rc<Person>>::new();
    let factory_tcs = tcs.clone();
    let sub = source
        .connect()
        .transform_safe_async_with_token(
            move |_c: Rc<Person>, _p: Option<Rc<Person>>, _key: String, cancel: CancellationToken| {
                let tcs = factory_tcs.clone();
                async move {
                    let canceled = tcs.clone();
                    let registration = cancel.register_local(move || canceled.set_canceled());
                    let result = tcs.task().await;
                    registration.dispose();
                    result
                }
            },
            // The cancellation should not reach the handler, which is torn down with the
            // subscription.
            |error: Error<Rc<Person>, String>| panic!("Unexpected error: {:?}", error.exception().map(|e| e.to_string())),
        )
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
    let error_count = Rc::new(Cell::new(0));

    let source = SourceCache::new(|p: &Rc<Person>| p.name());
    let counter = error_count.clone();
    let results = source
        .connect()
        .transform_safe_async_with_key_and_options(
            |p: Rc<Person>, _key: String| async move {
                delay(100).await?;
                let age_group = if p.age() < 18 { "Child" } else { "Adult" };
                Ok::<_, ObservableError>(PersonWithAgeGroup::new(p, age_group))
            },
            move |_error| counter.set(counter.get() + 1),
            TransformAsyncOptions { maximum_concurrency: Some(max_concurrency), ..TransformAsyncOptions::DEFAULT },
        )
        .as_aggregator();

    source.add_or_update_items((1..=TRANSFORM_COUNT as i32).map(|l| Person::new(format!("Person{l}"), l)));

    block_on(results.data().count_changed().where_(|c| *c == TRANSFORM_COUNT).take(1).to_task()).expect("every item is transformed");

    assert_eq!(error_count.get(), 0);

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

// Not ported: OptionsOverload_NullFactory_ThrowsBeforeSubscription (2 cases) - argument validation
