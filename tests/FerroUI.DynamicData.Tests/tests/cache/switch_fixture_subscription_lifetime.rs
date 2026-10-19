//! `SwitchFixture.SubscriptionLifetime` (`DynamicData.Tests/Cache/SwitchFixture.SubscriptionLifetime.cs`):
//! the lifetime of the subscriptions of `Switch` when sources are selected, released and failed
//! from inside the operator's own callbacks.
//!
//! Upstream writes the seed of the people to the output of the test (`ITestOutputHelper`), which
//! is left out.

use crate::domain::{fakers, Person};
use crate::utilities::{CacheItemRecordingObserver, CacheObservableExtensions, ObservableExtensions, TestSourceCache};
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, SourceCache, SourceCacheEx, SwitchCacheEx};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::disposables::{Disposable, SingleAssignmentDisposable};
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::{NotificationKind, Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The change sets of a cache of people (`IObservable<IChangeSet<Person, string>>`).
type PeopleChanges = Rc<dyn IObservable<ChangeSet<Rc<Person>, String>>>;

/// `AddSubscriptionPerson`.
fn add_subscription_person(person: &Rc<Person>) -> ChangeSet<Rc<Person>, String> {
    ChangeSet::from(vec![Change::new(ChangeReason::Add, person.name(), person.clone())])
}

/// `CreateSubscriptionPeople`.
fn create_subscription_people(count: usize) -> Vec<Rc<Person>> {
    const SEED: i32 = 0x51A7;
    fakers::person().use_seed(SEED).generate(count)
}

/// `results.RecordedItemsByKey.Values`.
fn recorded_items(results: &CacheItemRecordingObserver<Rc<Person>, String>) -> Vec<Rc<Person>> {
    results.recorded_items_by_key().values().cloned().collect()
}

/// Whether `actual` are the people of `expected`, the same objects in the same order.
fn same_people(actual: &[Rc<Person>], expected: &[&Rc<Person>]) -> bool {
    actual.len() == expected.len() && actual.iter().zip(expected).all(|(actual, expected)| Rc::ptr_eq(actual, expected))
}

/// `results.RecordedChangeSets.SelectMany(..).Select(change => (change.Reason, change.Key,
/// change.Current)).Should().Equal(expected)`.
fn assert_recorded_changes(results: &CacheItemRecordingObserver<Rc<Person>, String>, expected: &[(ChangeReason, &Rc<Person>)]) {
    let actual: Vec<Change<Rc<Person>, String>> = results.recorded_change_sets().iter().flat_map(|changes| changes.changes().to_vec()).collect();
    assert_eq!(actual.len(), expected.len(), "the number of the changes");
    for (change, (reason, person)) in actual.iter().zip(expected) {
        assert_eq!(change.reason(), *reason);
        assert_eq!(*change.key(), person.name());
        assert!(Rc::ptr_eq(change.current(), person), "the change is of {}", person.name());
    }
}

/// The resource that one subscription at a time may own (upstream's `resourceOwner` and
/// `released`).
struct ExclusiveResource {
    owner: RefCell<Option<Rc<Person>>>,
    released: RefCell<Vec<Rc<Person>>>,
}

impl ExclusiveResource {
    fn new() -> Rc<Self> {
        Rc::new(Self { owner: RefCell::new(None), released: RefCell::new(Vec::new()) })
    }

    fn owner(&self) -> Option<Rc<Person>> {
        self.owner.borrow().clone()
    }

    fn released(&self) -> Vec<Rc<Person>> {
        self.released.borrow().clone()
    }

    /// `CreateExclusiveSource(person, teardown)`: a source that fails when the resource is owned,
    /// and otherwise owns it and adds `person`; its disposal runs `teardown` while it still owns
    /// the resource, then releases it.
    fn create_exclusive_source(self: &Rc<Self>, person: &Rc<Person>, teardown: Option<Rc<dyn Fn()>>) -> PeopleChanges {
        let (this, person) = (self.clone(), person.clone());
        Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Person>, String>>>| {
            if this.owner.borrow().is_some() {
                observer.on_error(new_error("The previous subscription still owns the resource."));
                return Disposable::empty();
            }

            *this.owner.borrow_mut() = Some(person.clone());
            observer.on_next(add_subscription_person(&person));
            let (this, person, teardown) = (this.clone(), person.clone(), teardown.clone());
            Disposable::create(move || {
                // Selects the replacement while this teardown still owns the resource.
                if let Some(teardown) = &teardown {
                    teardown();
                }
                this.released.borrow_mut().push(person.clone());
                *this.owner.borrow_mut() = None;
            })
        })
    }
}

/// `Observable.Defer(() => { count++; return Observable.Never(); })`.
fn counted_never(subscriptions: &Rc<Cell<i32>>) -> PeopleChanges {
    let subscriptions = subscriptions.clone();
    Observable::defer(move || {
        subscriptions.set(subscriptions.get() + 1);
        Observable::never()
    })
}

/// Whether a change set adds the person of `name`.
fn adds(changes: &ChangeSet<Rc<Person>, String>, name: &str) -> bool {
    changes.changes().iter().any(|change| change.reason() == ChangeReason::Add && change.key() == name)
}

/// Whether a change set removes the person of `name`.
fn removes(changes: &ChangeSet<Rc<Person>, String>, name: &str) -> bool {
    changes.changes().iter().any(|change| change.reason() == ChangeReason::Remove && change.key() == name)
}

#[test]
fn replacement_releases_previous_subscription_before_acquiring_resource() {
    let people = create_subscription_people(2);
    let resource = ExclusiveResource::new();
    let switchable = Subject::<PeopleChanges>::new();
    let (subscription, results) =
        switchable.as_observable().switch_cache().validate_synchronization().validate_cache_change_sets(|person: &Rc<Person>| person.name()).record_cache_items();

    switchable.on_next(resource.create_exclusive_source(&people[0], None));
    assert!(same_people(&recorded_items(&results), &[&people[0]]));

    switchable.on_next(resource.create_exclusive_source(&people[1], None));

    assert!(results.error().is_none(), "the previous subscription must release the resource before its replacement subscribes");
    assert!(same_people(&recorded_items(&results), &[&people[1]]));
    assert!(resource.owner().is_some_and(|owner| Rc::ptr_eq(&owner, &people[1])));
    assert!(same_people(&resource.released(), &[&people[0]]));

    subscription.dispose();

    assert!(resource.owner().is_none());
    assert!(same_people(&resource.released(), &[&people[0], &people[1]]));
    assert!(!switchable.has_observers());
    assert!(!results.has_completed(), "unsubscription is not completion");

    switchable.dispose();
}

#[test]
fn reentrant_selection_during_synchronous_add_releases_previous_subscription_before_acquiring_resource() {
    let people = create_subscription_people(2);
    let resource = ExclusiveResource::new();
    let switchable = Subject::<PeopleChanges>::new();
    let (selector, replacement, first_name) = (switchable.clone(), resource.clone(), people[0].name());
    let replacement_person = people[1].clone();
    let (subscription, results) = switchable
        .as_observable()
        .switch_cache()
        .validate_synchronization()
        .validate_cache_change_sets(|person: &Rc<Person>| person.name())
        .do_(move |changes: &ChangeSet<Rc<Person>, String>| {
            // Selects the replacement before the first Subscribe has returned its disposable.
            if adds(changes, &first_name) {
                selector.on_next(replacement.create_exclusive_source(&replacement_person, None));
            }
        })
        .record_cache_items();

    switchable.on_next(resource.create_exclusive_source(&people[0], None));

    assert!(results.error().is_none(), "a reentrant replacement must wait for the in-progress subscription to release the resource");
    assert!(same_people(&recorded_items(&results), &[&people[1]]));
    assert!(resource.owner().is_some_and(|owner| Rc::ptr_eq(&owner, &people[1])));
    assert!(same_people(&resource.released(), &[&people[0]]));
    assert_recorded_changes(&results, &[(ChangeReason::Add, &people[0]), (ChangeReason::Remove, &people[0]), (ChangeReason::Add, &people[1])]);

    subscription.dispose();

    assert!(resource.owner().is_none());
    assert!(same_people(&resource.released(), &[&people[0], &people[1]]));

    switchable.dispose();
}

#[test]
fn reentrant_selection_during_teardown_releases_previous_subscription_before_acquiring_resource() {
    let people = create_subscription_people(3);
    let resource = ExclusiveResource::new();
    let superseded_subscriptions = Rc::new(Cell::new(0));
    let switchable = Subject::<PeopleChanges>::new();
    let superseded = counted_never(&superseded_subscriptions);
    let (subscription, results) =
        switchable.as_observable().switch_cache().validate_synchronization().validate_cache_change_sets(|person: &Rc<Person>| person.name()).record_cache_items();

    let (selector, replacement, replacement_person) = (switchable.clone(), resource.clone(), people[2].clone());
    let teardown: Rc<dyn Fn()> = Rc::new(move || selector.on_next(replacement.create_exclusive_source(&replacement_person, None)));
    switchable.on_next(resource.create_exclusive_source(&people[0], Some(teardown)));
    switchable.on_next(superseded);

    assert_eq!(superseded_subscriptions.get(), 0, "the teardown callback selected a newer source before the replacement could start");
    assert!(results.error().is_none(), "a replacement selected during teardown must wait for that teardown to release the resource");
    assert!(same_people(&recorded_items(&results), &[&people[2]]));
    assert!(resource.owner().is_some_and(|owner| Rc::ptr_eq(&owner, &people[2])));
    assert!(same_people(&resource.released(), &[&people[0]]));
    assert_recorded_changes(&results, &[(ChangeReason::Add, &people[0]), (ChangeReason::Remove, &people[0]), (ChangeReason::Add, &people[2])]);

    subscription.dispose();

    assert!(resource.owner().is_none());
    assert!(same_people(&resource.released(), &[&people[0], &people[2]]));

    switchable.dispose();
}

/// How the superseded source of `ReentrantSelectionDuringSynchronousAddKeepsLatestSubscription`
/// ends before its Subscribe returns (`NotificationKind?`).
#[derive(Clone, Copy)]
enum SupersededTermination {
    None,
    OnCompleted,
    OnError,
}

/// `ReentrantSelectionDuringSynchronousAddKeepsLatestSubscription(NotificationKind?)`.
fn reentrant_selection_during_synchronous_add_keeps_latest_subscription(superseded_termination: SupersededTermination) {
    let people = create_subscription_people(2);
    let superseded_disposals = Rc::new(Cell::new(0));
    let superseded_error = new_error("The superseded source failed before Subscribe returned.");
    let switchable = Subject::<PeopleChanges>::new();
    let latest = Subject::<ChangeSet<Rc<Person>, String>>::new();
    let (first_person, disposals) = (people[0].clone(), superseded_disposals.clone());
    let first: PeopleChanges = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Person>, String>>>| {
        // The downstream callback selects latest before this Subscribe can return its disposable.
        observer.on_next(add_subscription_person(&first_person));
        match superseded_termination {
            SupersededTermination::OnCompleted => observer.on_completed(),
            SupersededTermination::OnError => observer.on_error(superseded_error.clone()),
            SupersededTermination::None => {}
        }

        let disposals = disposals.clone();
        Disposable::create(move || disposals.set(disposals.get() + 1))
    });
    let (selector, latest_changes, first_name): (Rc<Subject<PeopleChanges>>, PeopleChanges, String) = (switchable.clone(), latest.clone(), people[0].name());
    let (subscription, results) = switchable
        .as_observable()
        .switch_cache()
        .validate_synchronization()
        .validate_cache_change_sets(|person: &Rc<Person>| person.name())
        .do_(move |changes: &ChangeSet<Rc<Person>, String>| {
            if adds(changes, &first_name) {
                selector.on_next(latest_changes.clone());
            }
        })
        .record_cache_items();

    switchable.on_next(first);
    let latest_remained_subscribed = latest.has_observers();
    latest.on_next(add_subscription_person(&people[1]));
    switchable.on_completed();
    let completed_before_latest = results.has_completed();
    latest.on_completed();

    assert!(results.error().is_none(), "termination of a superseded source must not terminate the selected source");
    assert!(latest_remained_subscribed, "the late return from the first Subscribe must not dispose the newer subscription");
    assert_eq!(superseded_disposals.get(), 1);
    assert!(!completed_before_latest);
    assert!(results.has_completed());
    assert!(same_people(&recorded_items(&results), &[&people[1]]));
    assert_recorded_changes(&results, &[(ChangeReason::Add, &people[0]), (ChangeReason::Remove, &people[0]), (ChangeReason::Add, &people[1])]);

    subscription.dispose();
    latest.dispose();
    switchable.dispose();
}

#[test]
fn reentrant_selection_during_synchronous_add_keeps_latest_subscription_case1() {
    reentrant_selection_during_synchronous_add_keeps_latest_subscription(SupersededTermination::None);
}

#[test]
fn reentrant_selection_during_synchronous_add_keeps_latest_subscription_case2() {
    reentrant_selection_during_synchronous_add_keeps_latest_subscription(SupersededTermination::OnCompleted);
}

#[test]
fn reentrant_selection_during_synchronous_add_keeps_latest_subscription_case3() {
    reentrant_selection_during_synchronous_add_keeps_latest_subscription(SupersededTermination::OnError);
}

#[test]
fn reentrant_selection_during_reset_does_not_subscribe_superseded_replacement() {
    let people = create_subscription_people(3);
    let superseded_subscriptions = Rc::new(Cell::new(0));
    let first = SourceCache::new(|person: &Rc<Person>| person.name());
    let latest = TestSourceCache::new(|person: &Rc<Person>| person.name());
    let switchable = Subject::<PeopleChanges>::new();
    first.add_or_update(people[0].clone());
    latest.add_or_update(people[1].clone());
    let superseded = counted_never(&superseded_subscriptions);
    let (selector, replacement, first_name) = (switchable.clone(), latest.clone(), people[0].name());
    let (subscription, results) = switchable
        .as_observable()
        .switch_cache()
        .validate_synchronization()
        .validate_cache_change_sets(|person: &Rc<Person>| person.name())
        .do_(move |changes: &ChangeSet<Rc<Person>, String>| {
            if removes(changes, &first_name) {
                selector.on_next(replacement.connect());
            }
        })
        .record_cache_items();

    switchable.on_next(first.connect());
    switchable.on_next(superseded);
    latest.add_or_update(people[2].clone());
    switchable.on_completed();
    latest.complete();

    assert_eq!(superseded_subscriptions.get(), 0, "the reset callback selected a newer source before the pending subscription started");
    assert!(results.error().is_none());
    assert!(results.has_completed());
    let mut recorded = recorded_items(&results);
    recorded.sort_by_key(|person| person.name());
    let mut expected: Vec<Rc<Person>> = people[1..].to_vec();
    expected.sort_by_key(|person| person.name());
    assert!(same_people(&recorded, &expected.iter().collect::<Vec<_>>()));
    assert_recorded_changes(
        &results,
        &[(ChangeReason::Add, &people[0]), (ChangeReason::Remove, &people[0]), (ChangeReason::Add, &people[1]), (ChangeReason::Add, &people[2])],
    );

    subscription.dispose();
    switchable.dispose();
    latest.dispose();
    first.dispose();
}

#[test]
fn reentrant_selection_during_disposal_does_not_subscribe_superseded_replacement() {
    let people = create_subscription_people(2);
    let first_disposals = Rc::new(Cell::new(0));
    let superseded_subscriptions = Rc::new(Cell::new(0));
    let latest = Subject::<ChangeSet<Rc<Person>, String>>::new();
    let switchable = Subject::<PeopleChanges>::new();
    let (first_person, disposals, selector, latest_changes): (Rc<Person>, Rc<Cell<i32>>, Rc<Subject<PeopleChanges>>, PeopleChanges) =
        (people[0].clone(), first_disposals.clone(), switchable.clone(), latest.clone());
    let first: PeopleChanges = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<Person>, String>>>| {
        observer.on_next(add_subscription_person(&first_person));
        let (disposals, selector, latest_changes) = (disposals.clone(), selector.clone(), latest_changes.clone());
        Disposable::create(move || {
            disposals.set(disposals.get() + 1);
            selector.on_next(latest_changes.clone());
        })
    });
    let superseded = counted_never(&superseded_subscriptions);
    let (subscription, results) =
        switchable.as_observable().switch_cache().validate_synchronization().validate_cache_change_sets(|person: &Rc<Person>| person.name()).record_cache_items();

    switchable.on_next(first);
    switchable.on_next(superseded);
    latest.on_next(add_subscription_person(&people[1]));
    switchable.on_completed();
    latest.on_completed();

    assert_eq!(first_disposals.get(), 1);
    assert_eq!(superseded_subscriptions.get(), 0, "releasing the old subscription selected latest before the replacement could start");
    assert!(results.error().is_none());
    assert!(results.has_completed());
    assert!(same_people(&recorded_items(&results), &[&people[1]]));
    assert_recorded_changes(&results, &[(ChangeReason::Add, &people[0]), (ChangeReason::Remove, &people[0]), (ChangeReason::Add, &people[1])]);

    subscription.dispose();
    switchable.dispose();
    latest.dispose();
}

#[test]
fn disposal_during_reset_does_not_subscribe_replacement() {
    let people = create_subscription_people(2);
    let replacement_subscriptions = Rc::new(Cell::new(0));
    let first = BehaviorSubject::new(add_subscription_person(&people[0]));
    let replacement_changes = Subject::<ChangeSet<Rc<Person>, String>>::new();
    let switchable = Subject::<PeopleChanges>::new();
    let (subscriptions, changes) = (replacement_subscriptions.clone(), replacement_changes.clone());
    let replacement: PeopleChanges = Observable::defer(move || {
        subscriptions.set(subscriptions.get() + 1);
        let changes: PeopleChanges = changes.clone();
        changes
    });
    let results = CacheItemRecordingObserver::<Rc<Person>, String>::new(ImmediateScheduler::instance());
    let subscription = SingleAssignmentDisposable::new();
    let (recorder, error_recorder, completed_recorder, owner) = (results.clone(), results.clone(), results.clone(), subscription.clone());
    let switched = switchable.as_observable().switch_cache().validate_synchronization().validate_cache_change_sets(|person: &Rc<Person>| person.name()).subscribe_all(
        move |changes: ChangeSet<Rc<Person>, String>| {
            // Record the reset before disposing, so the expected observable state is unambiguous.
            let has_removes = changes.removes() != 0;
            recorder.on_next(changes);
            if has_removes {
                owner.dispose();
            }
        },
        move |error| error_recorder.on_error(error),
        move || completed_recorder.on_completed(),
    );
    subscription.set_disposable(Some(switched));

    let first_changes: PeopleChanges = first.clone();
    switchable.on_next(first_changes);
    switchable.on_next(replacement);
    let notifications_at_disposal = results.notifications().len();
    replacement_changes.on_next(add_subscription_person(&people[1]));
    replacement_changes.on_completed();

    assert_eq!(replacement_subscriptions.get(), 0, "disposal in the reset callback must cancel the pending subscription, not just immediately dispose it");
    assert!(!first.has_observers());
    assert!(!replacement_changes.has_observers());
    assert!(!switchable.has_observers());
    assert_eq!(results.recorded_items_by_key().len(), 0);
    assert_eq!(results.notifications().len(), notifications_at_disposal);
    assert!(results.error().is_none());
    assert!(!results.has_completed());

    subscription.dispose();
    switchable.dispose();
    replacement_changes.dispose();
    first.dispose();
}

#[test]
fn outer_error_during_reset_does_not_subscribe_replacement() {
    let person = create_subscription_people(1).remove(0);
    let error = new_error("The outer source failed during reset.");
    let replacement_subscriptions = Rc::new(Cell::new(0));
    let first = BehaviorSubject::new(add_subscription_person(&person));
    let switchable = Subject::<PeopleChanges>::new();
    let replacement = counted_never(&replacement_subscriptions);
    let (selector, outer_error) = (switchable.clone(), error.clone());
    let (subscription, results) = switchable
        .as_observable()
        .switch_cache()
        .validate_synchronization()
        .validate_cache_change_sets(|value: &Rc<Person>| value.name())
        .do_(move |changes: &ChangeSet<Rc<Person>, String>| {
            if changes.removes() != 0 {
                selector.on_error(outer_error.clone());
            }
        })
        .record_cache_items();

    let first_changes: PeopleChanges = first.clone();
    switchable.on_next(first_changes);
    switchable.on_next(replacement);

    assert_eq!(replacement_subscriptions.get(), 0, "a pending subscription must not start after terminal delivery");
    assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
    assert!(!results.has_completed());
    assert_eq!(results.recorded_items_by_key().len(), 0);
    let kinds: Vec<NotificationKind> = results.notifications().iter().map(|notification| notification.value.kind()).collect();
    assert_eq!(kinds, vec![NotificationKind::OnNext, NotificationKind::OnNext, NotificationKind::OnError]);
    assert!(!first.has_observers());
    assert!(!switchable.has_observers());

    subscription.dispose();
    switchable.dispose();
    first.dispose();
}

#[test]
fn outer_completion_during_reset_waits_for_replacement() {
    let people = create_subscription_people(2);
    let first = BehaviorSubject::new(add_subscription_person(&people[0]));
    let replacement = TestSourceCache::new(|person: &Rc<Person>| person.name());
    let switchable = Subject::<PeopleChanges>::new();
    let selector = switchable.clone();
    let (subscription, results) = switchable
        .as_observable()
        .switch_cache()
        .validate_synchronization()
        .validate_cache_change_sets(|person: &Rc<Person>| person.name())
        .do_(move |changes: &ChangeSet<Rc<Person>, String>| {
            if changes.removes() != 0 {
                selector.on_completed();
            }
        })
        .record_cache_items();

    let first_changes: PeopleChanges = first.clone();
    switchable.on_next(first_changes);
    switchable.on_next(replacement.connect());
    assert!(!results.has_completed(), "outer completion must not cancel the already-selected inner source");

    replacement.add_or_update(people[1].clone());
    replacement.complete();

    assert!(results.error().is_none());
    assert!(results.has_completed());
    assert!(same_people(&recorded_items(&results), &[&people[1]]));
    assert!(!first.has_observers());

    subscription.dispose();
    switchable.dispose();
    replacement.dispose();
    first.dispose();
}

// Not ported: SynchronousOuterSubscriptionFailureReleasesActivatedInner - the outer source throws
// from its Subscribe after a selection; an exception thrown by a subscription is a panic here, and
// the operator does not catch panics (docs/porting/reactive.md, section 5.4)

#[test]
fn subscriptions_keep_independent_current_sources() {
    let people = create_subscription_people(2);
    let first = BehaviorSubject::new(add_subscription_person(&people[0]));
    let second = BehaviorSubject::new(add_subscription_person(&people[1]));
    let switchable = Subject::<PeopleChanges>::new();
    let switched = switchable.as_observable().switch_cache();
    let (first_subscription, first_results) = switched.validate_synchronization().validate_cache_change_sets(|person: &Rc<Person>| person.name()).record_cache_items();
    let (second_subscription, second_results) = switched.validate_synchronization().validate_cache_change_sets(|person: &Rc<Person>| person.name()).record_cache_items();

    let first_changes: PeopleChanges = first.clone();
    switchable.on_next(first_changes);
    first_subscription.dispose();
    let second_changes: PeopleChanges = second.clone();
    switchable.on_next(second_changes);
    switchable.on_completed();
    second.on_completed();

    assert!(first_results.error().is_none());
    assert!(!first_results.has_completed());
    assert!(same_people(&recorded_items(&first_results), &[&people[0]]));
    assert!(second_results.error().is_none());
    assert!(second_results.has_completed());
    assert!(same_people(&recorded_items(&second_results), &[&people[1]]));
    assert!(!first.has_observers());
    assert!(!second.has_observers());
    assert!(!switchable.has_observers());

    second_subscription.dispose();
    switchable.dispose();
    second.dispose();
    first.dispose();
}
