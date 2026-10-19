//! `ToObservableOptionalFixture` (`DynamicData.Tests/Cache/ToObservableOptionalFixture.cs` and
//! `ToObservableOptionalFixture.InitialValue.cs`): the item of one key of a cache as an option each
//! time it changes. Upstream's `Optional<T>` is `Option<T>`.

use crate::domain::Randomizer;
use crate::utilities::{ObservableExtensions, TestSourceCache};
use ferroui_dynamic_data::cache::testing::ChangeSetAggregator;
use ferroui_dynamic_data::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::{NotificationKind, Observable, ObservableExt};
use ferroui_reactive_testing::new_error;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const KEY1: &str = "Key1";
const KEY2: &str = "Key2";
const VALUE1: &str = "Value1";
const VALUE2: &str = "Value2";
const VALUE1_ALL_CAPS: &str = "VALUE1";

/// `InitialValueSeed`.
const INITIAL_VALUE_SEED: i32 = 0x0F710;

/// `KeyValuePair`, a class without an override of `Equals`: equal only to itself.
#[derive(Debug)]
struct KeyValuePair {
    key: String,
    value: String,
}

impl PartialEq for KeyValuePair {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `Create`.
fn create(key: &str, value: &str) -> Rc<KeyValuePair> {
    Rc::new(KeyValuePair { key: key.to_owned(), value: value.to_owned() })
}

/// `CaseInsensitiveComparer`: the values compared as `StringComparer.OrdinalIgnoreCase` does.
fn case_insensitive_comparer(x: &Rc<KeyValuePair>, y: &Rc<KeyValuePair>) -> bool {
    x.value.to_uppercase() == y.value.to_uppercase()
}

/// `CaseSensitiveComparer`: the values compared as `StringComparer.Ordinal` does.
fn case_sensitive_comparer(x: &Rc<KeyValuePair>, y: &Rc<KeyValuePair>) -> bool {
    x.value == y.value
}

/// `CreateInitialValue`.
fn create_initial_value() -> Rc<KeyValuePair> {
    let randomizer = Randomizer::new(INITIAL_VALUE_SEED);
    let key = randomizer.alpha_numeric(12);
    let value = randomizer.alpha_numeric(16);
    create(&key, &value)
}

type Optionals = Rc<RefCell<Vec<Option<Rc<KeyValuePair>>>>>;

/// `optionals.Add`, as the action of `Do`.
fn add_to(optionals: &Optionals) -> impl Fn(&Option<Rc<KeyValuePair>>) + 'static {
    let optionals = optionals.clone();
    move |optional: &Option<Rc<KeyValuePair>>| optionals.borrow_mut().push(optional.clone())
}

/// The kinds of the recorded notifications (`Notifications.Select(n => n.Value.Kind)`).
fn kinds_of(notifications: &[ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<Option<Rc<KeyValuePair>>>>]) -> Vec<NotificationKind> {
    notifications.iter().map(|notification| notification.value.kind()).collect()
}

/// The fixture (`ToObservableOptionalFixture` with its constructor and `Dispose`).
struct Fixture {
    source: SourceCache<Rc<KeyValuePair>, String>,
    results: ChangeSetAggregator<Rc<KeyValuePair>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|kvp: &Rc<KeyValuePair>| kvp.key.clone());
        let results = source.connect().as_aggregator();
        Self { source, results }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.results.dispose();
        self.source.dispose();
    }
}

// Not ported: NullChecks - argument validation

#[test]
fn adding_to_cache_emits_optional_some() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    let optional_observable = fixture.source.connect().to_observable_optional(KEY1.to_owned()).do_(add_to(&optionals)).subscribe_ignore();

    // when
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.len(), 1);
    assert!(optionals[0].is_some());
    assert_eq!(optionals[0].as_ref().unwrap().value, VALUE1);

    optional_observable.dispose();
}

#[test]
fn adding_other_keys_does_not_emit() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    let optional_observable = fixture.source.connect().to_observable_optional(KEY1.to_owned()).do_(add_to(&optionals)).subscribe_ignore();

    // when
    fixture.source.add_or_update(create(KEY2, VALUE1));

    // then
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.borrow().len(), 0);

    optional_observable.dispose();
}

#[test]
fn existing_value_emits_optional_some() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // when
    let optional_observable = fixture.source.connect().to_observable_optional(KEY1.to_owned()).do_(add_to(&optionals)).subscribe_ignore();

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.len(), 1);
    assert!(optionals[0].is_some());
    assert_eq!(optionals[0].as_ref().unwrap().value, VALUE1);

    optional_observable.dispose();
}

#[test]
fn removing_from_cache_emits_optional_none() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    let optional_observable = fixture.source.connect().to_observable_optional(KEY1.to_owned()).do_(add_to(&optionals)).subscribe_ignore();
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // when
    fixture.source.remove_key(KEY1.to_owned());

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 0);
    assert_eq!(optionals.len(), 2);
    assert!(optionals[1].is_none());

    optional_observable.dispose();
}

#[test]
fn update_cache_emits_optional_some() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    let optional_observable = fixture.source.connect().to_observable_optional(KEY1.to_owned()).do_(add_to(&optionals)).subscribe_ignore();
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // when
    fixture.source.add_or_update(create(KEY1, VALUE2));

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.len(), 2);
    assert!(optionals[1].is_some());
    assert_eq!(optionals[1].as_ref().unwrap().value, VALUE2);

    optional_observable.dispose();
}

#[test]
fn update_uses_equality_comparer() {
    let fixture = Fixture::new();

    // having
    let optionals_cs: Optionals = Rc::default();
    let optionals_non_cs: Optionals = Rc::default();
    let optional_cs_observable =
        fixture.source.connect().to_observable_optional_with_comparer(KEY1.to_owned(), case_sensitive_comparer).do_(add_to(&optionals_cs)).subscribe_ignore();
    let optional_non_cs_observable =
        fixture.source.connect().to_observable_optional_with_comparer(KEY1.to_owned(), case_insensitive_comparer).do_(add_to(&optionals_non_cs)).subscribe_ignore();
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // when
    fixture.source.add_or_update(create(KEY1, VALUE1_ALL_CAPS));

    // then
    let optionals_cs = optionals_cs.borrow();
    let optionals_non_cs = optionals_non_cs.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals_non_cs.len(), 1);
    assert!(optionals_non_cs[0].is_some());
    assert_eq!(optionals_non_cs[0].as_ref().unwrap().value, VALUE1);
    assert_eq!(optionals_cs.len(), 2);
    assert!(optionals_cs[0].is_some());
    assert_eq!(optionals_cs[0].as_ref().unwrap().value, VALUE1);
    assert!(optionals_cs[1].is_some());
    assert_eq!(optionals_cs[1].as_ref().unwrap().value, VALUE1_ALL_CAPS);

    optional_cs_observable.dispose();
    optional_non_cs_observable.dispose();
}

#[test]
fn update_when_reference_equal_does_not_emit() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    let optional_observable = fixture.source.connect().to_observable_optional(KEY1.to_owned()).do_(add_to(&optionals)).subscribe_ignore();
    let kvp = create(KEY1, VALUE1);
    fixture.source.add_or_update(kvp.clone());

    // when
    fixture.source.add_or_update(kvp.clone());
    fixture.source.add_or_update(kvp.clone());
    fixture.source.add_or_update(kvp);

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.len(), 1);
    assert!(optionals[0].is_some());
    assert_eq!(optionals[0].as_ref().unwrap().value, VALUE1);

    optional_observable.dispose();
}

// Not ported: InitialOptionalAvoidsNoneAfterSomeRaceConditions - real threads

#[test]
fn initial_optional_when_missing_emits_none() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();

    // when
    let optional_observable = fixture.source.connect().to_observable_optional_with_initial(KEY1.to_owned(), true).do_(add_to(&optionals)).subscribe_ignore();

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 0);
    assert_eq!(optionals.len(), 1);
    assert!(optionals[0].is_none());

    optional_observable.dispose();
}

#[test]
fn initial_optional_when_present_emits_some() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // when
    let optional_observable = fixture.source.connect().to_observable_optional_with_initial(KEY1.to_owned(), true).do_(add_to(&optionals)).subscribe_ignore();

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.len(), 1);
    assert!(optionals[0].is_some());
    assert_eq!(optionals[0].as_ref().unwrap().value, VALUE1);

    optional_observable.dispose();
}

#[test]
fn initial_optional_when_added_emits_none_then_some() {
    let fixture = Fixture::new();

    // having
    let optionals: Optionals = Rc::default();
    let optional_observable = fixture.source.connect().to_observable_optional_with_initial(KEY1.to_owned(), true).do_(add_to(&optionals)).subscribe_ignore();

    // when
    fixture.source.add_or_update(create(KEY1, VALUE1));

    // then
    let optionals = optionals.borrow();
    assert_eq!(fixture.results.data().count(), 1);
    assert_eq!(optionals.len(), 2);
    assert!(optionals[0].is_none());
    assert!(optionals[1].is_some());
    assert_eq!(optionals[1].as_ref().unwrap().value, VALUE1);

    optional_observable.dispose();
}

fn observable_completes_if_and_only_if_source_completes(complete_source: bool) {
    let fixture = Fixture::new();

    // having
    let completed = Rc::new(Cell::new(false));
    let mut optional_observable = fixture.source.connect();
    if !complete_source {
        optional_observable = optional_observable.concat(Observable::never::<ChangeSet<Rc<KeyValuePair>, String>>());
    }

    // when
    let on_completed = completed.clone();
    let results = optional_observable.to_observable_optional(KEY1.to_owned()).subscribe_next_completed(|_| {}, move || on_completed.set(true));
    fixture.source.dispose();

    // then
    assert_eq!(completed.get(), complete_source);

    results.dispose();
}

#[test]
fn observable_completes_if_and_only_if_source_completes_case1() {
    observable_completes_if_and_only_if_source_completes(true);
}

#[test]
fn observable_completes_if_and_only_if_source_completes_case2() {
    observable_completes_if_and_only_if_source_completes(false);
}

fn observable_fails_if_and_only_if_source_fails(fail_source: bool) {
    let fixture = Fixture::new();

    // having
    let mut optional_observable = fixture.source.connect();
    let test_exception = new_error("Test");
    let received_error: Rc<RefCell<Option<ObservableError>>> = Rc::default();
    if fail_source {
        optional_observable = optional_observable.concat(Observable::throw::<ChangeSet<Rc<KeyValuePair>, String>>(test_exception.clone()));
    }

    // when
    let on_error = received_error.clone();
    let results = optional_observable.to_observable_optional(KEY1.to_owned()).subscribe_next_error(|_| {}, move |error| *on_error.borrow_mut() = Some(error));
    fixture.source.dispose();

    // then
    let received_error = received_error.borrow();
    if fail_source {
        assert!(received_error.as_ref().is_some_and(|error| Rc::ptr_eq(error, &test_exception)));
    } else {
        assert!(received_error.is_none());
    }

    results.dispose();
}

#[test]
fn observable_fails_if_and_only_if_source_fails_case1() {
    observable_fails_if_and_only_if_source_fails(true);
}

#[test]
fn observable_fails_if_and_only_if_source_fails_case2() {
    observable_fails_if_and_only_if_source_fails(false);
}

/// `ToObservableOptionalFixture.InitialValue`.
mod initial_value {
    use super::*;

    // Not ported: InitialOptionalNeverFollowsConcurrentSomeWithoutRemoval - real threads

    fn initial_optional_preserves_synchronous_empty_completion(initial_optional_when_missing: bool) {
        let (subscription, results) = Observable::empty::<ChangeSet<Rc<KeyValuePair>, String>>()
            .to_observable_optional_with_initial(KEY1.to_owned(), initial_optional_when_missing)
            .validate_synchronization()
            .record_values();

        let expected: Vec<Option<Rc<KeyValuePair>>> = if initial_optional_when_missing { vec![None] } else { Vec::new() };
        assert_eq!(results.recorded_values(), expected);
        assert!(results.error().is_none());
        assert!(results.has_completed());
        let expected_kinds = if initial_optional_when_missing { vec![NotificationKind::OnNext, NotificationKind::OnCompleted] } else { vec![NotificationKind::OnCompleted] };
        assert_eq!(kinds_of(&results.notifications()), expected_kinds);

        subscription.dispose();
    }

    #[test]
    fn initial_optional_preserves_synchronous_empty_completion_case1() {
        initial_optional_preserves_synchronous_empty_completion(false);
    }

    #[test]
    fn initial_optional_preserves_synchronous_empty_completion_case2() {
        initial_optional_preserves_synchronous_empty_completion(true);
    }

    #[test]
    fn initial_optional_does_not_emit_none_after_synchronous_error() {
        let error = new_error("The source failed during subscription.");
        let (subscription, results) = Observable::throw::<ChangeSet<Rc<KeyValuePair>, String>>(error.clone())
            .to_observable_optional_with_initial(KEY1.to_owned(), true)
            .validate_synchronization()
            .record_values();

        assert!(results.recorded_values().is_empty());
        assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
        assert!(!results.has_completed());
        assert_eq!(kinds_of(&results.notifications()), vec![NotificationKind::OnError]);

        subscription.dispose();
    }

    fn initial_optional_preserves_synchronous_values_and_completion(remove_before_completion: bool) {
        let value = create_initial_value();
        let subscriptions = Rc::new(Cell::new(0));
        let disposals = Rc::new(Cell::new(0));
        let source = {
            let (value, subscriptions, disposals) = (value.clone(), subscriptions.clone(), disposals.clone());
            Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<Rc<KeyValuePair>, String>>>| {
                subscriptions.set(subscriptions.get() + 1);
                observer.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, value.key.clone(), value.clone())]));
                if remove_before_completion {
                    observer.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Remove, value.key.clone(), value.clone())]));
                }
                observer.on_completed();
                let disposals = disposals.clone();
                Disposable::create(move || disposals.set(disposals.get() + 1))
            })
        };
        let (subscription, results) = source.to_observable_optional_with_initial(value.key.clone(), true).validate_synchronization().record_values();

        let expected: Vec<Option<Rc<KeyValuePair>>> = if remove_before_completion { vec![Some(value.clone()), None] } else { vec![Some(value.clone())] };
        assert_eq!(results.recorded_values(), expected, "initialization must neither prepend nor append None to synchronous source values");
        assert!(results.error().is_none());
        assert!(results.has_completed());
        let expected_kinds = if remove_before_completion {
            vec![NotificationKind::OnNext, NotificationKind::OnNext, NotificationKind::OnCompleted]
        } else {
            vec![NotificationKind::OnNext, NotificationKind::OnCompleted]
        };
        assert_eq!(kinds_of(&results.notifications()), expected_kinds);
        assert_eq!(subscriptions.get(), 1);
        assert_eq!(disposals.get(), 1);

        subscription.dispose();
    }

    #[test]
    fn initial_optional_preserves_synchronous_values_and_completion_case1() {
        initial_optional_preserves_synchronous_values_and_completion(false);
    }

    #[test]
    fn initial_optional_preserves_synchronous_values_and_completion_case2() {
        initial_optional_preserves_synchronous_values_and_completion(true);
    }

    #[test]
    fn initial_optional_state_is_per_subscription() {
        let value = create_initial_value();
        let source = TestSourceCache::new(|item: &Rc<KeyValuePair>| item.key.clone());
        let optional = source.connect().to_observable_optional_with_initial(value.key.clone(), true);
        source.add_or_update(value.clone());
        let (first_subscription, first_results) = optional.validate_synchronization().record_values();

        source.remove_key(value.key.clone());
        let (second_subscription, second_results) = optional.validate_synchronization().record_values();
        assert_eq!(second_results.recorded_values(), vec![None::<Rc<KeyValuePair>>], "the new subscriber starts while the key is absent");

        source.add_or_update(value.clone());
        first_subscription.dispose();
        source.remove_key(value.key.clone());
        source.complete();

        assert_eq!(first_results.recorded_values(), vec![Some(value.clone()), None, Some(value.clone())]);
        assert!(!first_results.has_completed());
        assert!(first_results.error().is_none());
        assert_eq!(second_results.recorded_values(), vec![None, Some(value.clone()), None]);
        assert!(second_results.has_completed());
        assert!(second_results.error().is_none());

        second_subscription.dispose();
        first_subscription.dispose();
        source.dispose();
    }

    #[test]
    fn initial_optional_disposal_during_initial_none_releases_source() {
        let subscriptions = Rc::new(Cell::new(0));
        let disposals = Rc::new(Cell::new(0));
        let source = {
            let (subscriptions, disposals) = (subscriptions.clone(), disposals.clone());
            Observable::create(move |_: Rc<dyn IObserver<ChangeSet<Rc<KeyValuePair>, String>>>| {
                subscriptions.set(subscriptions.get() + 1);
                let disposals = disposals.clone();
                Disposable::create(move || disposals.set(disposals.get() + 1))
            })
        };
        let (subscription, results) = source.to_observable_optional_with_initial(KEY1.to_owned(), true).take(1).validate_synchronization().record_values();

        assert_eq!(results.recorded_values(), vec![None::<Rc<KeyValuePair>>]);
        assert!(results.error().is_none());
        assert!(results.has_completed());
        assert_eq!(kinds_of(&results.notifications()), vec![NotificationKind::OnNext, NotificationKind::OnCompleted]);
        assert_eq!(subscriptions.get(), 1);
        assert_eq!(disposals.get(), 1, "Take(1) must unsubscribe even though initialization ran before Subscribe returned");

        subscription.dispose();
    }

    fn initial_optional_propagates_asynchronous_termination(fail_source: bool) {
        let value = create_initial_value();
        let error = new_error("The source failed after subscription.");
        let source = Subject::<ChangeSet<Rc<KeyValuePair>, String>>::new();
        let (subscription, results) = source.as_observable().to_observable_optional_with_initial(value.key.clone(), true).validate_synchronization().record_values();

        assert_eq!(results.recorded_values(), vec![None::<Rc<KeyValuePair>>]);
        assert!(!results.has_completed());
        source.on_next(ChangeSet::from(vec![Change::new(ChangeReason::Add, value.key.clone(), value.clone())]));
        if fail_source {
            source.on_error(error.clone());
        } else {
            source.on_completed();
        }

        assert_eq!(results.recorded_values(), vec![None, Some(value.clone())]);
        if fail_source {
            assert!(results.error().is_some_and(|actual| Rc::ptr_eq(&actual, &error)));
        } else {
            assert!(results.error().is_none());
        }
        assert_eq!(results.has_completed(), !fail_source);
        let terminal = if fail_source { NotificationKind::OnError } else { NotificationKind::OnCompleted };
        assert_eq!(kinds_of(&results.notifications()), vec![NotificationKind::OnNext, NotificationKind::OnNext, terminal]);
        assert!(!source.has_observers());

        subscription.dispose();
        source.dispose();
    }

    #[test]
    fn initial_optional_propagates_asynchronous_termination_case1() {
        initial_optional_propagates_asynchronous_termination(false);
    }

    #[test]
    fn initial_optional_propagates_asynchronous_termination_case2() {
        initial_optional_propagates_asynchronous_termination(true);
    }
}
