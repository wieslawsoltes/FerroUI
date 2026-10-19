//! `TransformMany<TDestination, TDestinationKey, TSource, TSourceKey>`
//! (`Cache/Internal/TransformMany.cs`).

use super::transform::transform;
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, IObservableCache};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

/// The items an item of the source flattens into (`Func<TSource, IEnumerable<TDestination>>`).
pub(crate) type ManySelector<S, D> = Rc<dyn Fn(&S) -> Vec<D>>;

/// The changes of the items of an item of the source after its first value
/// (`Func<TSource, IObservable<IChangeSet<TDestination, TDestinationKey>>>`).
pub(crate) type ChildChanges<S, D, DK> = Rc<dyn Fn(&S) -> Rc<dyn IObservable<ChangeSet<D, DK>>>>;

/// The items of an item of the source and their keys, and the later changes of them
/// (`ManyContainer`). The items are read when they are needed (upstream's lazy enumerable), so
/// that the removal of an item of the source removes the items it has at that moment.
struct ManyContainer<D, DK> {
    destination: Rc<dyn Fn() -> Vec<(D, DK)>>,
    changes: Rc<dyn IObservable<ChangeSet<D, DK>>>,
    /// The subscription to `changes` (`SubscribeMany` of `MergeMany`).
    subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
}

impl<D, DK> Clone for ManyContainer<D, DK> {
    fn clone(&self) -> Self {
        Self { destination: self.destination.clone(), changes: self.changes.clone(), subscription: self.subscription.clone() }
    }
}

impl<D: 'static, DK: 'static> ManyContainer<D, DK> {
    fn new(destination: impl Fn() -> Vec<(D, DK)> + 'static, changes: Option<Rc<dyn IObservable<ChangeSet<D, DK>>>>) -> Self {
        Self { destination: Rc::new(destination), changes: changes.unwrap_or_else(Observable::empty), subscription: Rc::new(RefCell::new(None)) }
    }
}

/// The items of each item of the source, as one cache keyed by `key_selector`
/// (`TransformMany(source, manySelector, keySelector).Run` without child changes: `Create`).
///
/// The items of an item are taken when it is added, updated or refreshed (the refresh transforms
/// again). An update of an item of the source removes the items it no longer has, adds the new
/// ones and updates those of the same key, unless the item is the same reference.
pub(crate) fn transform_many<D: Clone + Deref + 'static, DK: Eq + Hash + Clone + 'static, S: Clone + 'static, SK: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, SK>>>,
    many_selector: ManySelector<S, D>,
    key_selector: Rc<dyn Fn(&D) -> DK>,
) -> Rc<dyn IObservable<ChangeSet<D, DK>>> {
    let factory = move |item: &S, _previous: Option<&S>, _key: &SK| -> Result<ManyContainer<D, DK>, ObservableError> {
        let destination: Vec<(D, DK)> = many_selector(item).into_iter().map(|many| {
            let key = key_selector(&many);
            (many, key)
        }).collect();
        Ok(ManyContainer::new(move || destination.clone(), None))
    };
    transform(source, factory, None, true).select(|changes: ChangeSet<ManyContainer<D, DK>, SK>| destination_changes(&changes))
}

/// [`transform_many`] for items whose items change later (`CreateWithChangeSet`): the
/// constructors of upstream for `ObservableCollection`, `ReadOnlyObservableCollection` and
/// `IObservableCache` children.
///
/// The changes of the items of the source and those of the children are applied to one cache,
/// whose changes are emitted. A child subscription lives as long as its item of the source; the
/// sequence ends when the source and every child sequence have ended, and fails with an error of
/// the source (the errors of the children are ignored, as `MergeMany` does).
pub(crate) fn transform_many_with_child_changes<D: Clone + Deref + 'static, DK: Eq + Hash + Clone + 'static, S: Clone + 'static, SK: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, SK>>>,
    many_selector: ManySelector<S, D>,
    key_selector: Rc<dyn Fn(&D) -> DK>,
    child_changes: ChildChanges<S, D, DK>,
) -> Rc<dyn IObservable<ChangeSet<D, DK>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, DK>>>| {
        let result = Rc::new(RefCell::new(ChangeAwareCache::<D, DK>::new()));
        let (many_selector, key_selector, child_changes) = (many_selector.clone(), key_selector.clone(), child_changes.clone());
        let factory = move |item: &S, _previous: Option<&S>, _key: &SK| -> Result<ManyContainer<D, DK>, ObservableError> {
            // Only skip the initial value, of the first additions where there are items.
            let changes = child_changes(item).skip(1);
            let (many_selector, key_selector, item) = (many_selector.clone(), key_selector.clone(), item.clone());
            let destination = move || -> Vec<(D, DK)> {
                many_selector(&item).into_iter().map(|many| {
                    let key = key_selector(&many);
                    (many, key)
                }).collect()
            };
            Ok(ManyContainer::new(destination, Some(changes)))
        };
        let transformed = transform(source.clone(), factory, None, false);

        // Upstream publishes the transform and merges `initial` (the changes of the items of the
        // source) with `subsequent` (`MergeMany` of the changes of the children), serialized by
        // a delivery queue. One observer does both here, in the order of upstream's
        // subscriptions: the changes of a change set of the source first, then the subscriptions
        // to the children of the items it adds.
        let children = CompositeDisposable::new();
        // One for the source, one for each child subscription (`MergeMany`'s counter).
        let counter = Rc::new(Cell::new(1usize));
        let terminated = Rc::new(Cell::new(false));
        let complete: Rc<dyn Fn()> = {
            let (counter, terminated, observer) = (counter.clone(), terminated.clone(), observer.clone());
            Rc::new(move || {
                counter.set(counter.get() - 1);
                if counter.get() == 0 && !terminated.replace(true) {
                    observer.on_completed();
                }
            })
        };

        let (next_result, next_observer, next_children, next_counter, next_complete) = (result.clone(), observer.clone(), children.clone(), counter.clone(), complete.clone());
        let (error_observer, error_terminated) = (observer.clone(), terminated.clone());
        let source_complete = complete.clone();
        let subscription = transformed.subscribe_all(
            move |changes: ChangeSet<ManyContainer<D, DK>, SK>| {
                let initial = destination_changes(&changes);
                let captured = {
                    let mut result = next_result.borrow_mut();
                    result.clone_changes(&initial);
                    result.capture_changes()
                };
                next_observer.on_next(captured);
                for change in &changes {
                    match change.reason() {
                        ChangeReason::Add => subscribe_child(change.current(), &next_children, &next_counter, &next_complete, &next_result, &next_observer),
                        ChangeReason::Update => {
                            if let Some(previous) = change.previous() {
                                dispose_child(previous, &next_children);
                            }
                            subscribe_child(change.current(), &next_children, &next_counter, &next_complete, &next_result, &next_observer);
                        }
                        ChangeReason::Remove => dispose_child(change.current(), &next_children),
                        ChangeReason::Refresh | ChangeReason::Moved => {}
                    }
                }
            },
            move |error| {
                if !error_terminated.replace(true) {
                    error_observer.on_error(error);
                }
            },
            move || source_complete(),
        );
        // Terminated first, so that the child subscriptions that end with the disposal do not
        // complete the sequence (upstream disposes its delivery queue first).
        let stop_terminated = terminated.clone();
        let stop = Disposable::create(move || stop_terminated.set(true));
        let children: Rc<dyn IDisposable> = children;
        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([stop, subscription, children]);
        disposables
    })
}

/// Subscribes to the changes of the children of `container` (`SubscribeMany` of `MergeMany`):
/// each change set is applied to the result, whose changes are emitted.
fn subscribe_child<D: Clone + 'static, DK: Eq + Hash + Clone + 'static>(
    container: &ManyContainer<D, DK>,
    children: &Rc<CompositeDisposable>,
    counter: &Rc<Cell<usize>>,
    complete: &Rc<dyn Fn()>,
    result: &Rc<RefCell<ChangeAwareCache<D, DK>>>,
    observer: &Rc<dyn IObserver<ChangeSet<D, DK>>>,
) {
    counter.set(counter.get() + 1);
    let finally_complete = complete.clone();
    let (result, observer) = (result.clone(), observer.clone());
    let subscription = container.changes.finally(move || finally_complete()).subscribe_all(
        move |changes: ChangeSet<D, DK>| {
            let captured = {
                let mut result = result.borrow_mut();
                result.clone_changes(&changes);
                result.capture_changes()
            };
            observer.on_next(captured);
        },
        |_| {},
        || {},
    );
    *container.subscription.borrow_mut() = Some(subscription.clone());
    children.add(subscription);
}

/// Ends the subscription to the children of `container` (`DisposeMany` of `SubscribeMany`).
fn dispose_child<D, DK>(container: &ManyContainer<D, DK>, children: &Rc<CompositeDisposable>) {
    if let Some(subscription) = container.subscription.borrow_mut().take() {
        children.remove(&subscription);
    }
}

/// The changes of the items of the items of the source (`DestinationEnumerator`).
fn destination_changes<D: Clone + Deref, DK: Eq + Hash + Clone, SK>(changes: &ChangeSet<ManyContainer<D, DK>, SK>) -> ChangeSet<D, DK> {
    let mut result = Vec::new();
    for change in changes {
        match change.reason() {
            ChangeReason::Add | ChangeReason::Remove | ChangeReason::Refresh => {
                for (item, key) in (change.current().destination)() {
                    result.push(Change::new(change.reason(), key, item));
                }
            }
            ChangeReason::Update => {
                let previous_items = (change.previous().expect("an update has a previous item").destination)();
                let current_items = (change.current().destination)();

                let removes = except_by_key(&previous_items, &current_items);
                let adds = except_by_key(&current_items, &previous_items);
                let updates = intersect_by_key(&current_items, &previous_items);

                for (item, key) in removes {
                    result.push(Change::new(ChangeReason::Remove, key, item));
                }
                for (item, key) in adds {
                    result.push(Change::new(ChangeReason::Add, key, item));
                }
                for (_, key) in updates {
                    let current = current_items.iter().find(|(_, candidate)| *candidate == key).expect("the key is in the current items");
                    let previous = previous_items.iter().find(|(_, candidate)| *candidate == key).expect("the key is in the previous items");
                    // Do not update if the items are the same reference.
                    // Deviation (DEVIATIONS.md, DynamicData): the reference of an item is the object
                    // its pointer refers to (`TDestination: Deref`, an `Rc<..>` for the classes of
                    // upstream), as `ignore_same_reference_update` does.
                    if !std::ptr::eq(current.0.deref(), previous.0.deref()) {
                        result.push(Change::update(key, current.0.clone(), previous.0.clone()));
                    }
                }
            }
            ChangeReason::Moved => {}
        }
    }
    ChangeSet::from(result)
}

/// `first.Except(second, KeyComparer)`: the items of `first`, without repeated keys, whose keys
/// `second` does not have.
fn except_by_key<D: Clone, DK: PartialEq + Clone>(first: &[(D, DK)], second: &[(D, DK)]) -> Vec<(D, DK)> {
    let mut seen: Vec<&DK> = second.iter().map(|(_, key)| key).collect();
    let mut result = Vec::new();
    for entry in first {
        if !seen.contains(&&entry.1) {
            seen.push(&entry.1);
            result.push(entry.clone());
        }
    }
    result
}

/// `first.Intersect(second, KeyComparer)`: the items of `first`, without repeated keys, whose keys
/// `second` has.
fn intersect_by_key<D: Clone, DK: PartialEq + Clone>(first: &[(D, DK)], second: &[(D, DK)]) -> Vec<(D, DK)> {
    let mut remaining: Vec<&DK> = second.iter().map(|(_, key)| key).collect();
    let mut result = Vec::new();
    for entry in first {
        if let Some(position) = remaining.iter().position(|key| **key == entry.1) {
            remaining.remove(position);
            result.push(entry.clone());
        }
    }
    result
}

/// The child changes of the items of a cache of an item of the source (the constructor of
/// `TransformMany` for `IObservableCache`): the changes of the cache after the first; an empty
/// cache gives an empty first change set, which is skipped.
pub(crate) fn observable_cache_child_changes<S: 'static, D: Clone + 'static, DK: Eq + Hash + Clone + 'static, C: IObservableCache<D, DK> + 'static>(
    many_selector: Rc<dyn Fn(&S) -> C>,
) -> ChildChanges<S, D, DK>
where
    S: Clone,
{
    Rc::new(move |item: &S| {
        let (many_selector, item) = (many_selector.clone(), item.clone());
        Observable::defer(move || {
            let cache = many_selector(&item);
            let subsequent_changes = cache.connect();
            if cache.count() > 0 {
                return subsequent_changes;
            }
            Observable::return_(ChangeSet::<D, DK>::empty()).concat(subsequent_changes)
        })
    })
}
