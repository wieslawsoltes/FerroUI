//! `TransformMany<TSource, TDestination>` (`List/Internal/TransformMany.cs`).

use super::remove_index::without_index;
use super::transformer::transformer;
use crate::list::{clone_changes_with, Change, ChangeAwareList, ChangeSet, IObservableList, ListChangeReason};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The items an item of the source flattens into (`Func<TSource, IEnumerable<TDestination>>`).
pub(crate) type ListManySelector<S, D> = Rc<dyn Fn(&S) -> Vec<D>>;

/// The equality of the flattened items (`IEqualityComparer<TDestination>`).
pub(crate) type ListEqualityComparer<D> = Rc<dyn Fn(&D, &D) -> bool>;

/// The changes of the items of an item of the source (`Func<TSource, IObservable<IChangeSet<TDestination>>>`).
pub(crate) type ListChildChanges<S, D> = Rc<dyn Fn(&S) -> Rc<dyn IObservable<ChangeSet<D>>>>;

/// The items of an item of the source, and their later changes (`ManyContainer`). The items are
/// read when they are needed, as upstream's enumerables are: those of a list are its items at that
/// moment.
struct ManyContainer<D> {
    destination: Rc<dyn Fn() -> Vec<D>>,
    changes: Rc<dyn IObservable<ChangeSet<D>>>,
    /// The subscription to `changes` (`SubscribeMany` of `MergeMany`).
    subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
}

impl<D> Clone for ManyContainer<D> {
    fn clone(&self) -> Self {
        Self { destination: self.destination.clone(), changes: self.changes.clone(), subscription: self.subscription.clone() }
    }
}

impl<D: 'static> ManyContainer<D> {
    fn new(destination: impl Fn() -> Vec<D> + 'static, changes: Option<Rc<dyn IObservable<ChangeSet<D>>>>) -> Self {
        Self { destination: Rc::new(destination), changes: changes.unwrap_or_else(Observable::empty), subscription: Rc::new(RefCell::new(None)) }
    }
}

/// The items of each item of the source, as one list (`TransformMany.Run` without child
/// changes): the items of an item are taken when it is added, replaced or refreshed (the refresh
/// transforms again); a replacement removes the items the item no longer has and adds the new
/// ones, by `equality_comparer`. Moves are ignored: the index of an item of the source has no
/// bearing on those of its items. Empty change sets are dropped.
pub(crate) fn transform_many<S: Clone + PartialEq + 'static, D: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S>>>,
    many_selector: ListManySelector<S, D>,
    equality_comparer: ListEqualityComparer<D>,
) -> Rc<dyn IObservable<ChangeSet<D>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D>>>| {
        // `ChangeAwareList` captures the changes of the list so that later operators can replay
        // them.
        let result = RefCell::new(ChangeAwareList::<D>::new());
        let many_selector = many_selector.clone();
        let equality_comparer = equality_comparer.clone();
        let factory = move |item: &S, _previous: Option<&ManyContainer<D>>, _index: usize| -> Result<ManyContainer<D>, ObservableError> {
            let destination = many_selector(item);
            Ok(ManyContainer::new(move || destination.clone(), None))
        };
        transformer(source.clone(), factory, true)
            .select(move |changes: ChangeSet<ManyContainer<D>>| {
                let destination = destination_changes(&changes, &*equality_comparer);
                let mut result = result.borrow_mut();
                clone_changes_with(&mut *result, &destination, &*equality_comparer);
                result.capture_changes()
            })
            .where_(|changes: &ChangeSet<D>| changes.count() != 0)
            .subscribe_safe(observer)
    })
}

/// [`transform_many`] for items whose items change later (`CreateWithChangeSet`): the
/// constructors of upstream for `ObservableCollection`, `ReadOnlyObservableCollection` and
/// `IObservableList` children.
///
/// The changes of the items of the source and those of the children (without their indexes,
/// which are those of the child lists) are applied to one list, whose changes are emitted. A child
/// subscription lives as long as its item of the source; the sequence ends when the source and
/// every child sequence have ended, and fails with an error of the source (the errors of the
/// children are ignored, as `MergeMany` does).
pub(crate) fn transform_many_with_child_changes<S: Clone + PartialEq + 'static, D: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S>>>,
    many_selector: ListManySelector<S, D>,
    equality_comparer: ListEqualityComparer<D>,
    child_changes: ListChildChanges<S, D>,
) -> Rc<dyn IObservable<ChangeSet<D>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D>>>| {
        let result = Rc::new(RefCell::new(ChangeAwareList::<D>::new()));
        let (many_selector, child_changes) = (many_selector.clone(), child_changes.clone());
        let factory = move |item: &S, _previous: Option<&ManyContainer<D>>, _index: usize| -> Result<ManyContainer<D>, ObservableError> {
            let changes = child_changes(item).skip(1);
            let (many_selector, item) = (many_selector.clone(), item.clone());
            Ok(ManyContainer::new(move || many_selector(&item), Some(changes)))
        };
        let transformed = transformer(source.clone(), factory, false);

        // Upstream publishes the transform and merges `initial` (the changes of the items of the
        // source) with `subsequent` (`MergeMany` of the changes of the children). One observer
        // does both here, in the order of upstream's subscriptions: the changes of a change set
        // of the source first, then the subscriptions to the children of the items it adds.
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

        let context = Rc::new(ChildContext { result: result.clone(), observer: observer.clone(), children: children.clone(), counter: counter.clone(), complete: complete.clone(), equality_comparer: equality_comparer.clone() });
        let next_context = context.clone();
        let (error_observer, error_terminated) = (observer.clone(), terminated.clone());
        let source_complete = complete.clone();
        let subscription = transformed.subscribe_all(
            move |changes: ChangeSet<ManyContainer<D>>| {
                let context = &next_context;
                let initial = destination_changes(&changes, &*context.equality_comparer);
                let captured = {
                    let mut result = context.result.borrow_mut();
                    clone_changes_with(&mut *result, &initial, &*context.equality_comparer);
                    result.capture_changes()
                };
                context.observer.on_next(captured);
                for change in &changes {
                    match change.reason() {
                        ListChangeReason::Add => subscribe_child(change.item().current(), context),
                        ListChangeReason::AddRange => {
                            for container in change.range() {
                                subscribe_child(container, context);
                            }
                        }
                        ListChangeReason::Replace => {
                            if let Some(previous) = change.item().previous() {
                                dispose_child(previous, &context.children);
                            }
                            subscribe_child(change.item().current(), context);
                        }
                        ListChangeReason::Remove => dispose_child(change.item().current(), &context.children),
                        ListChangeReason::RemoveRange | ListChangeReason::Clear => {
                            for container in change.range() {
                                dispose_child(container, &context.children);
                            }
                        }
                        ListChangeReason::Refresh | ListChangeReason::Moved => {}
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
        // complete the sequence.
        let stop_terminated = terminated.clone();
        let stop = Disposable::create(move || stop_terminated.set(true));
        let children: Rc<dyn IDisposable> = children;
        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([stop, subscription, children]);
        disposables
    })
}

/// What the subscriptions to the children share.
struct ChildContext<D> {
    result: Rc<RefCell<ChangeAwareList<D>>>,
    observer: Rc<dyn IObserver<ChangeSet<D>>>,
    children: Rc<CompositeDisposable>,
    counter: Rc<Cell<usize>>,
    complete: Rc<dyn Fn()>,
    equality_comparer: ListEqualityComparer<D>,
}

/// Subscribes to the changes of the children of `container` (`SubscribeMany` of `MergeMany`):
/// each change set, without its indexes (`RemoveIndex`), is applied to the result, whose changes
/// are emitted.
fn subscribe_child<D: Clone + PartialEq + 'static>(container: &ManyContainer<D>, context: &Rc<ChildContext<D>>) {
    context.counter.set(context.counter.get() + 1);
    let finally_complete = context.complete.clone();
    let next_context = context.clone();
    let subscription = container.changes.finally(move || finally_complete()).subscribe_all(
        move |changes: ChangeSet<D>| {
            let changes = without_index(&changes);
            let captured = {
                let mut result = next_context.result.borrow_mut();
                clone_changes_with(&mut *result, &changes, &*next_context.equality_comparer);
                result.capture_changes()
            };
            next_context.observer.on_next(captured);
        },
        |_| {},
        || {},
    );
    *container.subscription.borrow_mut() = Some(subscription.clone());
    context.children.add(subscription);
}

/// Ends the subscription to the children of `container` (`DisposeMany` of `SubscribeMany`).
fn dispose_child<D>(container: &ManyContainer<D>, children: &Rc<CompositeDisposable>) {
    if let Some(subscription) = container.subscription.borrow_mut().take() {
        children.remove(&subscription);
    }
}

/// The changes of the items of the items of the source (`DestinationEnumerator`).
fn destination_changes<D: Clone>(changes: &ChangeSet<ManyContainer<D>>, equality_comparer: &dyn Fn(&D, &D) -> bool) -> Vec<Change<D>> {
    let mut result = Vec::new();
    for change in changes {
        match change.reason() {
            ListChangeReason::Add | ListChangeReason::Remove => {
                for destination in (change.item().current().destination)() {
                    result.push(Change::new(change.reason(), destination, None));
                }
            }
            ListChangeReason::AddRange | ListChangeReason::Clear => {
                let items: Vec<D> = change.range().iter().flat_map(|many| (many.destination)()).collect();
                result.push(Change::range_change(change.reason(), items, None));
            }
            ListChangeReason::Replace | ListChangeReason::Refresh => {
                // Discover the additions and the removals.
                let current_items = (change.item().current().destination)();
                // Deviation (DEVIATIONS.md, DynamicData): a refresh that is not transformed has no
                // previous item, where upstream fails; it changes no item here.
                let Some(previous) = change.item().previous() else { continue };
                let previous_items = (previous.destination)();

                let adds = except(&current_items, &previous_items, equality_comparer);
                for destination in except(&previous_items, &current_items, equality_comparer) {
                    result.push(Change::new(ListChangeReason::Remove, destination, None));
                }
                for destination in adds {
                    result.push(Change::new(ListChangeReason::Add, destination, None));
                }
            }
            ListChangeReason::RemoveRange => {
                for destination in change.range().iter().flat_map(|many| (many.destination)()) {
                    result.push(Change::new(ListChangeReason::Remove, destination, None));
                }
            }
            // The index of an item of the source has no bearing on those of its items.
            ListChangeReason::Moved => {}
        }
    }
    result
}

/// `first.Except(second, comparer)`: the items of `first`, without repeats, that `second` does not
/// have.
fn except<D: Clone>(first: &[D], second: &[D], equality_comparer: &dyn Fn(&D, &D) -> bool) -> Vec<D> {
    let mut seen: Vec<&D> = second.iter().collect();
    let mut result = Vec::new();
    for item in first {
        if !seen.iter().any(|other| equality_comparer(other, item)) {
            seen.push(item);
            result.push(item.clone());
        }
    }
    result
}

/// The child changes of the items of a list of an item of the source (the constructor of
/// `TransformMany` for `IObservableList`): the changes of the list after the first; an empty list
/// gives an empty first change set, which is skipped.
pub(crate) fn observable_list_child_changes<S: Clone + 'static, D: Clone + PartialEq + 'static, C: IObservableList<D> + 'static>(
    many_selector: Rc<dyn Fn(&S) -> C>,
) -> ListChildChanges<S, D> {
    Rc::new(move |item: &S| {
        let (many_selector, item) = (many_selector.clone(), item.clone());
        Observable::defer(move || {
            let list = many_selector(&item);
            let subsequent_changes = list.connect();
            if list.count() > 0 {
                return subsequent_changes;
            }
            Observable::return_(ChangeSet::<D>::empty()).concat(subsequent_changes)
        })
    })
}
