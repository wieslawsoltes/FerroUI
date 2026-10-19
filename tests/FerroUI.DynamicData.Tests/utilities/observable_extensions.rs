//! `ObservableExtensions` (`Utilities/ObservableExtensions.cs`): failing and parallelized
//! sequences, the recording observers, and the validators of change sets and of the
//! synchronization of notifications.
//!
//! The methods are in three traits: [`ObservableExtensions`] for every sequence,
//! [`CacheObservableExtensions`] for the change sets of caches and [`ListObservableExtensions`]
//! for those of lists. Upstream's two `ValidateChangeSets` overloads are
//! [`validate_cache_change_sets`](CacheObservableExtensions::validate_cache_change_sets) and
//! [`validate_list_change_sets`](ListObservableExtensions::validate_list_change_sets).
//!
//! Deviation (DEVIATIONS.md, DynamicData): the recorders whose scheduler upstream defaults to
//! `GlobalConfig.DefaultScheduler` (the task pool) default to the default scheduler of the thread
//! (`SchedulerDefaults::default_scheduler`); the scheduler only stamps the notifications.

use super::{is_equivalent, CacheItemRecordingObserver, ListItemRecordingObserver, RawAnonymousObservable, RawAnonymousObserver, UnsynchronizedNotificationException, ValueRecordingObserver};
use ferroui_dynamic_data::cache::{self, ChangeReason, ObservableCacheEx};
use ferroui_dynamic_data::list::{self, ChangeType, ListChangeReason, ObservableListEx};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::{Notification, Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// The error of a validator of change sets: a change set that breaks a rule of change sets (the
/// failed assertion of upstream).
#[derive(Clone, Debug)]
pub struct ChangeSetValidationError(pub String);

impl fmt::Display for ChangeSetValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ChangeSetValidationError {}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn validation_error(message: String) -> ObservableError {
    Rc::new(ChangeSetValidationError(message))
}

/// `parallel` counts that add up to `count` (`Distribute`): one each when there are fewer values
/// than subscriptions, else equal shares with the remainder in the last.
fn distribute(count: usize, parallel: usize) -> Vec<usize> {
    if count <= parallel {
        return vec![1; count];
    }
    let ratio = count / parallel;
    if count % parallel == 0 {
        return vec![ratio; parallel];
    }
    let mut shares = vec![ratio; parallel - 1];
    shares.push(count - ratio * (parallel - 1));
    shares
}

/// The extensions of every sequence (`ObservableExtensions`).
pub trait ObservableExtensions<T: Clone + 'static> {
    /// The first `count` values, then the end with `error`; the sequence itself without an error
    /// (`ForceFail`).
    fn force_fail(&self, count: usize, error: Option<ObservableError>) -> Rc<dyn IObservable<T>>;

    /// `parallel` subscriptions to the sequence that take `count` values together, merged
    /// (`Parallelize(int count, int parallel)`).
    fn parallelize(&self, count: usize, parallel: usize) -> Rc<dyn IObservable<T>>;

    /// [`parallelize`](Self::parallelize) with work attached to each subscription before the merge
    /// (`Parallelize(int count, int parallel, Func<IObservable<T>, IObservable<U>>)`).
    fn parallelize_with<U: 'static>(&self, count: usize, parallel: usize, attach: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<U>>) -> Rc<dyn IObservable<U>>;

    /// Subscribes a [`ValueRecordingObserver`], stamped by the default scheduler of the thread
    /// (`RecordValues(out observer)`); returns the subscription and the observer.
    fn record_values(&self) -> (Rc<dyn IDisposable>, Rc<ValueRecordingObserver<T>>);

    /// [`record_values`](Self::record_values) stamped by `scheduler`
    /// (`RecordValues(out observer, IScheduler)`).
    fn record_values_on(&self, scheduler: Rc<dyn IScheduler>) -> (Rc<dyn IDisposable>, Rc<ValueRecordingObserver<T>>);

    /// The sequence, with an [`UnsynchronizedNotificationException`] delivered to the observer when
    /// a notification arrives while another one is being delivered, re-entrantly here
    /// (`ValidateSynchronization`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): upstream also delivers an exception that an
    /// observer downstream throws as an error; here a panic downstream propagates.
    fn validate_synchronization(&self) -> Rc<dyn IObservable<T>>;
}

impl<T: Clone + 'static> ObservableExtensions<T> for Rc<dyn IObservable<T>> {
    fn force_fail(&self, count: usize, error: Option<ObservableError>) -> Rc<dyn IObservable<T>> {
        match error {
            Some(error) => self.take(count).concat(Observable::throw(error)),
            None => self.clone(),
        }
    }

    fn parallelize(&self, count: usize, parallel: usize) -> Rc<dyn IObservable<T>> {
        let sources: Vec<Rc<dyn IObservable<T>>> = distribute(count, parallel).into_iter().map(|share| self.take(share)).collect();
        Observable::merge_all(sources)
    }

    fn parallelize_with<U: 'static>(&self, count: usize, parallel: usize, attach: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<U>>) -> Rc<dyn IObservable<U>> {
        let sources: Vec<Rc<dyn IObservable<U>>> = distribute(count, parallel).into_iter().map(|share| attach(self.take(share))).collect();
        Observable::merge_all(sources)
    }

    fn record_values(&self) -> (Rc<dyn IDisposable>, Rc<ValueRecordingObserver<T>>) {
        self.record_values_on(SchedulerDefaults::default_scheduler())
    }

    fn record_values_on(&self, scheduler: Rc<dyn IScheduler>) -> (Rc<dyn IDisposable>, Rc<ValueRecordingObserver<T>>) {
        let observer = ValueRecordingObserver::new(scheduler);
        let subscription = self.subscribe(observer.clone());
        (subscription, observer)
    }

    fn validate_synchronization(&self) -> Rc<dyn IObservable<T>> {
        let source = self.clone();
        RawAnonymousObservable::create(move |observer: Rc<dyn IObserver<T>>| {
            let in_flight: Rc<RefCell<Option<Notification<T>>>> = Rc::new(RefCell::new(None));
            let process: Rc<dyn Fn(Notification<T>)> = Rc::new(move |incoming: Notification<T>| {
                let prior = in_flight.replace(Some(incoming.clone()));
                match prior {
                    Some(prior) => observer.on_error(Rc::new(UnsynchronizedNotificationException { incoming_notification: incoming, prior_notification: prior })),
                    None => incoming.accept(&*observer),
                }
                // `finally`: the notification is no longer in flight.
                *in_flight.borrow_mut() = None;
            });
            let (on_next, on_error, on_completed) = (process.clone(), process.clone(), process);
            source.subscribe(RawAnonymousObserver::create(
                move |value: T| on_next(Notification::OnNext(value)),
                move |error: ObservableError| on_error(Notification::OnError(error)),
                move || on_completed(Notification::OnCompleted),
            ))
        })
    }
}

/// The extensions of the sequences of change sets of caches (`ObservableExtensions`).
pub trait CacheObservableExtensions<T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
    /// Subscribes a [`CacheItemRecordingObserver`], stamped by the default scheduler of the thread
    /// (`RecordCacheItems(out observer)`); returns the subscription and the observer.
    fn record_cache_items(&self) -> (Rc<dyn IDisposable>, Rc<CacheItemRecordingObserver<T, K>>);

    /// [`record_cache_items`](Self::record_cache_items) stamped by `scheduler`
    /// (`RecordCacheItems(out observer, IScheduler)`).
    fn record_cache_items_on(&self, scheduler: Rc<dyn IScheduler>) -> (Rc<dyn IDisposable>, Rc<CacheItemRecordingObserver<T, K>>);

    /// The change sets, checked against the rules of change sets of caches: the key is the item's,
    /// the indexes are given always or never, and each change agrees with the items so far. A
    /// change set that breaks a rule ends the sequence with a [`ChangeSetValidationError`]
    /// (`ValidateChangeSets(Func<TObject, TKey> keySelector)`).
    fn validate_cache_change_sets(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<cache::ChangeSet<T, K>>>
    where
        T: PartialEq;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: cache::IChangeSet<T, K>> CacheObservableExtensions<T, K> for Rc<dyn IObservable<C>> {
    fn record_cache_items(&self) -> (Rc<dyn IDisposable>, Rc<CacheItemRecordingObserver<T, K>>) {
        self.record_cache_items_on(SchedulerDefaults::default_scheduler())
    }

    fn record_cache_items_on(&self, scheduler: Rc<dyn IScheduler>) -> (Rc<dyn IDisposable>, Rc<CacheItemRecordingObserver<T, K>>) {
        let observer = CacheItemRecordingObserver::new(scheduler);
        let subscription = <Self as ObservableCacheEx<T, K>>::change_sets(self).subscribe(observer.clone());
        (subscription, observer)
    }

    fn validate_cache_change_sets(&self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<cache::ChangeSet<T, K>>>
    where
        T: PartialEq,
    {
        let source = <Self as ObservableCacheEx<T, K>>::change_sets(self);
        let key_selector: Rc<dyn Fn(&T) -> K> = Rc::new(key_selector);
        RawAnonymousObservable::create(move |observer: Rc<dyn IObserver<cache::ChangeSet<T, K>>>| {
            let state = Rc::new(RefCell::new(CacheValidationState::<T, K> { items_by_key: HashMap::new(), sorted_keys: Vec::new(), is_sorted: None }));
            let key_selector = key_selector.clone();
            let (on_error, on_completed) = (observer.clone(), observer.clone());
            source.subscribe(RawAnonymousObserver::create(
                move |changes: cache::ChangeSet<T, K>| {
                    let result = state.borrow_mut().validate(&changes, &*key_selector);
                    match result {
                        Ok(()) => observer.on_next(changes),
                        Err(message) => observer.on_error(validation_error(message)),
                    }
                },
                move |error: ObservableError| on_error.on_error(error),
                move || on_completed.on_completed(),
            ))
        })
    }
}

struct CacheValidationState<T, K> {
    items_by_key: HashMap<K, T>,
    sorted_keys: Vec<K>,
    is_sorted: Option<bool>,
}

impl<T: Clone + PartialEq, K: Eq + Hash + Clone> CacheValidationState<T, K> {
    fn validate(&mut self, changes: &cache::ChangeSet<T, K>, key_selector: &dyn Fn(&T) -> K) -> Result<(), String> {
        for change in changes {
            let key = change.key();
            check(*key == key_selector(change.current()), "the specified key should match the specified item's key")?;
            match self.is_sorted {
                // The first change decides whether every later change has indexes.
                None => self.is_sorted = Some(change.current_index().is_some()),
                Some(true) => check(change.current_index().is_some(), "indexes should be specified for a stream that specified them initially")?,
                Some(false) => check(change.current_index().is_none(), "indexes should be omitted for a stream that omitted them initially")?,
            }
            let count = self.sorted_keys.len();
            match change.reason() {
                ChangeReason::Add => {
                    check(!self.items_by_key.contains_key(key), "the key to be added should not already exist in the collection")?;
                    check(change.previous().is_none(), "only Update changes should specify a previous item")?;
                    check(change.previous_index().is_none(), "only Moved or Update changes should specify a previous index")?;
                    if let Some(index) = change.current_index() {
                        check(index <= count, "the index to be added should be a valid index of the collection, or the next available index of the collection")?;
                        self.sorted_keys.insert(index, key.clone());
                    }
                    self.items_by_key.insert(key.clone(), change.current().clone());
                }
                ChangeReason::Moved => {
                    check(self.items_by_key.contains_key(key), "the key to be moved should exist in the collection")?;
                    check(change.previous().is_none(), "only Update changes should specify a previous item")?;
                    let previous_index = change.previous_index().filter(|index| *index < count).ok_or("the source index should be a valid index of the collection")?;
                    check(self.items_by_key.get(key) == Some(change.current()), "the item to be moved should match the corresponding item in the collection")?;
                    let current_index = change.current_index().filter(|index| *index < count).ok_or("the target index should be a valid index of the collection")?;
                    self.sorted_keys.remove(previous_index);
                    self.sorted_keys.insert(current_index, key.clone());
                }
                ChangeReason::Refresh => {
                    check(self.items_by_key.contains_key(key), "the key to be refreshed should exist in the collection")?;
                    check(change.previous().is_none(), "only Update changes should specify a previous item")?;
                    check(change.previous_index().is_none(), "only Moved or Update changes should specify a previous index")?;
                    check(self.items_by_key.get(key) == Some(change.current()), "the item to be refreshed should match the corresponding item in the collection")?;
                    if let Some(index) = change.current_index() {
                        check(index < count, "the index to be refreshed should be a valid index of the collection")?;
                        check(self.sorted_keys[index] == *key, "the key to be refreshed should match the corresponding key in the collection")?;
                    }
                }
                ChangeReason::Remove => {
                    check(self.items_by_key.contains_key(key), "the key to be removed should exist in the collection")?;
                    check(change.previous().is_none(), "only Update changes should specify a previous item")?;
                    check(change.previous_index().is_none(), "only Moved or Update changes should specify a previous index")?;
                    check(self.items_by_key.get(key) == Some(change.current()), "the item to be removed should match the corresponding item in the collection")?;
                    if let Some(index) = change.current_index() {
                        check(index < count, "the index to be removed should be a valid index of the collection")?;
                        check(self.sorted_keys[index] == *key, "the key to be removed should match the corresponding key in the collection")?;
                        self.sorted_keys.remove(index);
                    }
                    self.items_by_key.remove(key);
                }
                ChangeReason::Update => {
                    check(self.items_by_key.contains_key(key), "the key to be updated should exist in the collection")?;
                    let previous = change.previous().ok_or("an Update change should specify a previous item")?;
                    check(self.items_by_key.get(key) == Some(previous), "the item to be updated should match the corresponding item in the collection")?;
                    match change.current_index() {
                        None => check(change.previous_index().is_none(), "a previous index should only be specified if a current index is specified")?,
                        Some(current_index) => {
                            let previous_index = change.previous_index().filter(|index| *index < count).ok_or("the source index should be a valid index of the collection")?;
                            check(self.sorted_keys[previous_index] == *key, "the key to be updated should match the corresponding key in the collection")?;
                            check(current_index < count, "the target index should be a valid index of the collection")?;
                            self.sorted_keys.remove(previous_index);
                            self.sorted_keys.insert(current_index, key.clone());
                        }
                    }
                    self.items_by_key.insert(key.clone(), change.current().clone());
                }
            }
        }
        Ok(())
    }
}

/// The extensions of the sequences of change sets of lists (`ObservableExtensions`).
pub trait ListObservableExtensions<T: Clone + PartialEq + 'static> {
    /// Subscribes a [`ListItemRecordingObserver`], stamped by the default scheduler of the thread
    /// (`RecordListItems(out observer)`); returns the subscription and the observer.
    fn record_list_items(&self) -> (Rc<dyn IDisposable>, Rc<ListItemRecordingObserver<T>>);

    /// [`record_list_items`](Self::record_list_items) stamped by `scheduler`
    /// (`RecordListItems(out observer, IScheduler)`).
    fn record_list_items_on(&self, scheduler: Rc<dyn IScheduler>) -> (Rc<dyn IDisposable>, Rc<ListItemRecordingObserver<T>>);

    /// The change sets, checked against the rules of change sets of lists: each change agrees with
    /// its kind and with the items so far. A change set that breaks a rule delivers a
    /// [`ChangeSetValidationError`] and then, as upstream, the change set itself
    /// (`ValidateChangeSets()`).
    fn validate_list_change_sets(&self) -> Rc<dyn IObservable<list::ChangeSet<T>>>;
}

impl<T: Clone + PartialEq + 'static, C: list::IChangeSet<T>> ListObservableExtensions<T> for Rc<dyn IObservable<C>> {
    fn record_list_items(&self) -> (Rc<dyn IDisposable>, Rc<ListItemRecordingObserver<T>>) {
        self.record_list_items_on(SchedulerDefaults::default_scheduler())
    }

    fn record_list_items_on(&self, scheduler: Rc<dyn IScheduler>) -> (Rc<dyn IDisposable>, Rc<ListItemRecordingObserver<T>>) {
        let observer = ListItemRecordingObserver::new(scheduler);
        let subscription = <Self as ObservableListEx<T>>::change_sets(self).subscribe(observer.clone());
        (subscription, observer)
    }

    fn validate_list_change_sets(&self) -> Rc<dyn IObservable<list::ChangeSet<T>>> {
        let source = <Self as ObservableListEx<T>>::change_sets(self);
        RawAnonymousObservable::create(move |observer: Rc<dyn IObserver<list::ChangeSet<T>>>| {
            let sorted_items: Rc<RefCell<Vec<T>>> = Rc::new(RefCell::new(Vec::new()));
            let (on_error, on_completed) = (observer.clone(), observer.clone());
            source.subscribe(RawAnonymousObserver::create(
                move |changes: list::ChangeSet<T>| {
                    let result = validate_list_changes(&mut sorted_items.borrow_mut(), &changes);
                    if let Err(message) = result {
                        observer.on_error(validation_error(message));
                    }
                    observer.on_next(changes);
                },
                move |error: ObservableError| on_error.on_error(error),
                move || on_completed.on_completed(),
            ))
        })
    }
}

fn validate_list_changes<T: Clone + PartialEq>(sorted_items: &mut Vec<T>, changes: &list::ChangeSet<T>) -> Result<(), String> {
    for change in changes {
        match change.change_type() {
            ChangeType::Item => {
                let item = change.try_item().ok_or("single-item changes should specify item info")?;
                check(item.reason() == change.reason(), "the reason of the item should be the reason of the change")?;
                check(change.range().count() == 0, "single-item changes should not specify range info")?;
            }
            ChangeType::Range => check(change.try_item().is_none(), "range changes should not specify single-item info")?,
        }
        let count = sorted_items.len();
        match change.reason() {
            ListChangeReason::Add => {
                let item = change.item();
                check(item.previous_index().is_none(), "only Moved changes should specify a previous index")?;
                check(item.previous().is_none(), "only Update changes should specify a previous item")?;
                match item.current_index() {
                    None => sorted_items.push(item.current().clone()),
                    Some(index) => {
                        check(index <= count, "the insertion index should be omitted, a valid index of the collection, or the next available index of the collection")?;
                        sorted_items.insert(index, item.current().clone());
                    }
                }
            }
            ListChangeReason::AddRange => {
                let range = change.range();
                match range.index() {
                    None => sorted_items.extend(range.iter().cloned()),
                    Some(index) => {
                        check(index <= count, "the insertion index should be omitted, a valid index of the collection, or the next available index of the collection")?;
                        let tail = sorted_items.split_off(index);
                        sorted_items.extend(range.iter().cloned());
                        sorted_items.extend(tail);
                    }
                }
            }
            ListChangeReason::Clear => {
                let range = change.range();
                check(range.index().is_none(), "a Clear change applies to an entire collection, it does not have a specific index")?;
                check(is_equivalent(range.items(), sorted_items), "items in the range should match the corresponding items in the collection")?;
                sorted_items.clear();
            }
            ListChangeReason::Moved => {
                check(count != 0, "an item cannot be moved within an empty collection")?;
                let item = change.item();
                let previous_index = item.previous_index().filter(|index| *index < count).ok_or("the source index should be a valid index of the collection")?;
                check(item.previous().is_none(), "only Update changes should specify a previous item")?;
                let current_index = item.current_index().filter(|index| *index < count).ok_or("the target index should be a valid index of the collection")?;
                check(*item.current() == sorted_items[previous_index], "the item to be moved should match the corresponding item in the collection")?;
                sorted_items.remove(previous_index);
                sorted_items.insert(current_index, item.current().clone());
            }
            ListChangeReason::Refresh => {
                check(count != 0, "an item cannot be refreshed within an empty collection")?;
                let item = change.item();
                check(item.previous_index().is_none(), "only Moved changes should specify a previous index")?;
                let current_index = item.current_index().filter(|index| *index < count).ok_or("the target index should be a valid index of the collection")?;
                check(*item.current() == sorted_items[current_index], "the item to be refreshed should match the corresponding item in the collection")?;
            }
            ListChangeReason::Remove => {
                check(count != 0, "an item cannot be removed from an empty collection")?;
                let item = change.item();
                check(item.previous_index().is_none(), "only Moved changes should specify a previous index")?;
                check(item.previous().is_none(), "only Update changes should specify a previous item")?;
                let current_index = item.current_index().filter(|index| *index < count).ok_or("the index to be removed should be a valid index of the collection")?;
                check(*item.current() == sorted_items[current_index], "the item to be removed should match the corresponding item in the collection")?;
                sorted_items.remove(current_index);
            }
            ListChangeReason::RemoveRange => {
                let range = change.range();
                match range.index() {
                    None => check(range.count() == 0, "the removal index was omitted")?,
                    Some(index) => {
                        check(index < count, "the removal index should be omitted, or a valid index of the collection")?;
                        check(range.count() >= 1 && range.count() <= count - index, "the range to be removed should contain more items than exist in the collection, at the given removal index")?;
                        check(*range.items() == sorted_items[index..index + range.count()], "items to be removed should match the corresponding items in the collection")?;
                        sorted_items.drain(index..index + range.count());
                    }
                }
            }
            ListChangeReason::Replace => {
                check(count != 0, "an item cannot be replaced within an empty collection")?;
                let item = change.item();
                check(item.previous_index().is_some_and(|index| index < count), "the index of replacement should be a valid index of the collection")?;
                let current_index = item.current_index().filter(|index| *index < count).ok_or("the index to be replaced should be a valid index of the collection")?;
                let previous = item.previous().ok_or("a Replace change should specify a previous item")?;
                check(*previous == sorted_items[current_index], "the replaced item should match the corresponding item in the collection")?;
                sorted_items[current_index] = item.current().clone();
            }
        }
    }
    Ok(())
}
