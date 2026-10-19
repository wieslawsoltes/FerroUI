//! `ObservableChangeSetFixture` (`DynamicData.Tests/Cache/ObservableChangeSetFixture.cs`): the
//! sequences of change sets of caches made by `ObservableChangeSet.Create`, synchronous and
//! asynchronous, with their errors and their release.

use crate::domain::Person;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, IObservableCache, ObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_dynamic_data::ObservableChangeSet;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::new_error;
use ferroui_system::threading::tasks::Task;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

// Not ported: AsyncSubscriptionCanReceiveMultipleResults - its [Fact] is commented out upstream
// ("Disabled due to test failing when run with a test runner"); it loops on Task.Delay until
// cancelled (real time).

/// One hundred people, `Name.1` to `Name.100`, aged by their number.
fn people() -> Vec<Rc<Person>> {
    (1..=100).map(|i| Person::new(format!("Name.{i}"), i)).collect()
}

/// `CreateTask`: the people, as a completed task (`Task.FromResult`).
fn create_task() -> Task<Vec<Rc<Person>>> {
    Task::from_result(people())
}

#[test]
fn handles_async_error() {
    let error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    // `Loader`: a task that fails.
    fn loader() -> Task<Vec<Rc<Person>>> {
        Task::from_exception(new_error("Broken"))
    }

    let observable = ObservableChangeSet::create_cache_async_with_action(
        |cache: SourceCache<Rc<Person>, String>| async move {
            let people = loader().await?;
            cache.add_or_update_items(people);
            Ok::<_, ObservableError>(|| {})
        },
        |p: &Rc<Person>| p.name(),
    );

    let derived = observable.as_observable_cache();
    {
        let recorded = error.clone();
        let subscription = derived.connect().subscribe_next_error(|_| {}, move |ex| *recorded.borrow_mut() = Some(ex));
        assert!(error.borrow().is_some());
        subscription.dispose();
    }
    derived.dispose();
}

#[test]
fn handles_error() {
    let error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));

    // `Loader`: a function that fails.
    fn loader() -> Result<Vec<Rc<Person>>, ObservableError> {
        Err(new_error("Broken"))
    }

    let observable = ObservableChangeSet::try_create_cache_with_action(
        |cache: SourceCache<Rc<Person>, String>| {
            let people = loader()?;
            cache.add_or_update_items(people);
            Ok(|| {})
        },
        |p: &Rc<Person>| p.name(),
    );

    let derived = observable.as_observable_cache();
    {
        let recorded = error.clone();
        let subscription = derived.connect().subscribe_next_error(|_| {}, move |ex| *recorded.borrow_mut() = Some(ex));
        assert!(error.borrow().is_some());
        subscription.dispose();
    }
    derived.dispose();
}

#[test]
fn loads_and_dispose_from_observable_cache() {
    let is_disposed = Rc::new(Cell::new(false));

    let observable = {
        let is_disposed = is_disposed.clone();
        ObservableChangeSet::create_cache_with_action(
            move |_cache: SourceCache<Rc<Person>, String>| {
                let is_disposed = is_disposed.clone();
                move || is_disposed.set(true)
            },
            |p: &Rc<Person>| p.name(),
        )
    };

    observable.as_observable_cache().dispose();
    assert!(is_disposed.get());
}

#[test]
fn loads_and_dispose_using_action() {
    let is_disposed = Rc::new(Cell::new(false));
    let observable = {
        let is_disposed = is_disposed.clone();
        ObservableChangeSet::create_cache_with_action(
            move |cache: SourceCache<Rc<Person>, String>| {
                cache.add_or_update_items(people());
                let is_disposed = is_disposed.clone();
                move || is_disposed.set(true)
            },
            |p: &Rc<Person>| p.name(),
        )
    };
    subscribe_and_assert(observable, false, Some(&|result: &ObservableCache<Rc<Person>, String>| assert_eq!(result.count(), 100)));

    assert!(is_disposed.get());
}

#[test]
fn loads_and_dispose_using_action_async() {
    let is_disposed = Rc::new(Cell::new(false));
    let observable = {
        let is_disposed = is_disposed.clone();
        ObservableChangeSet::create_cache_async_with_action(
            move |cache: SourceCache<Rc<Person>, String>| {
                let is_disposed = is_disposed.clone();
                async move {
                    let people = create_task().await?;
                    cache.add_or_update_items(people);
                    Ok::<_, ObservableError>(move || is_disposed.set(true))
                }
            },
            |p: &Rc<Person>| p.name(),
        )
    };
    subscribe_and_assert(observable, false, Some(&|result: &ObservableCache<Rc<Person>, String>| assert_eq!(result.count(), 100)));

    assert!(is_disposed.get());
}

#[test]
fn loads_and_dispose_using_disposable() {
    let is_disposed = Rc::new(Cell::new(false));
    let observable = {
        let is_disposed = is_disposed.clone();
        ObservableChangeSet::create_cache(
            move |cache: SourceCache<Rc<Person>, String>| {
                cache.add_or_update_items(people());
                let is_disposed = is_disposed.clone();
                Disposable::create(move || is_disposed.set(true))
            },
            |p: &Rc<Person>| p.name(),
        )
    };
    subscribe_and_assert(observable, false, Some(&|result: &ObservableCache<Rc<Person>, String>| assert_eq!(result.count(), 100)));

    assert!(is_disposed.get());
}

#[test]
fn loads_and_dispose_using_disposable_async() {
    let is_disposed = Rc::new(Cell::new(false));
    let observable = {
        let is_disposed = is_disposed.clone();
        ObservableChangeSet::create_cache_async_with_disposable(
            move |cache: SourceCache<Rc<Person>, String>| {
                let is_disposed = is_disposed.clone();
                async move {
                    let people = create_task().await?;
                    cache.add_or_update_items(people);
                    Ok::<Rc<dyn IDisposable>, ObservableError>(Disposable::create(move || is_disposed.set(true)))
                }
            },
            |p: &Rc<Person>| p.name(),
        )
    };
    subscribe_and_assert(observable, false, Some(&|result: &ObservableCache<Rc<Person>, String>| assert_eq!(result.count(), 100)));

    assert!(is_disposed.get());
}

/// `SubscribeAndAssert`: the sequence as a cache, with its error checked and its content checked
/// by `check_content_action`; disposing the cache ends the sequence.
fn subscribe_and_assert<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    observable_changeset: Rc<dyn IObservable<ChangeSet<T, K>>>,
    expects_error: bool,
    check_content_action: Option<&dyn Fn(&ObservableCache<T, K>)>,
) {
    let error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let complete = Rc::new(Cell::new(false));
    let changes: Rc<RefCell<Option<ChangeSet<T, K>>>> = Rc::new(RefCell::new(None));

    {
        let on_finally = complete.clone();
        let cache = observable_changeset.finally(move || on_finally.set(true)).as_observable_cache();
        let subscription = {
            let (changes, error) = (changes.clone(), error.clone());
            cache.connect().subscribe_next_error(move |result| *changes.borrow_mut() = Some(result), move |ex| *error.borrow_mut() = Some(ex))
        };

        if !expects_error {
            assert!(error.borrow().is_none());
        } else {
            assert!(error.borrow().is_some());
        }

        if let Some(check_content_action) = check_content_action {
            check_content_action(&cache);
        }

        subscription.dispose();
        cache.dispose();
    }

    assert!(complete.get());
}
