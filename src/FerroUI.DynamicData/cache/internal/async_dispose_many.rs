//! `AsyncDisposeMany<TObject, TKey>` (`Cache/Internal/AsyncDisposeMany.cs`): disposes the items,
//! at once or by a task, when they are removed or replaced, and every item left when the sequence
//! ends or the subscription is disposed; the end of the disposals by tasks is signalled apart.

use super::Cache;
use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::synchronize_safe;
use crate::ItemDisposal;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::{ConnectableObservableExt, Observable, ObservableExt, Observer};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// How an item is disposed, if it is disposable (`item is IDisposable`, `item is
/// IAsyncDisposable`).
pub(crate) type ToItemDisposal<T> = Rc<dyn Fn(&T) -> Option<ItemDisposal>>;

/// The state of one subscription.
struct AsyncDisposeManyState<T, K> {
    items_by_key: RefCell<Cache<T, K>>,
    /// The disposals by tasks, each a sequence of the end of its task.
    disposals: Rc<Subject<Rc<dyn IObservable<()>>>>,
    to_item_disposal: ToItemDisposal<T>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> AsyncDisposeManyState<T, K> {
    /// Disposes every item left, and ends the disposals: once, while the disposals have their
    /// observer (`TearDown`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): upstream catches an exception of `Dispose` and
    /// fails the disposals with it; `IDisposable::dispose` cannot fail here, so only the tasks of
    /// the disposals by tasks fail them.
    fn tear_down(&self) {
        if self.disposals.has_observers() {
            let items = self.items_by_key.borrow().items();
            for item in &items {
                self.try_dispose_item(item);
            }
            self.disposals.on_completed();
            self.items_by_key.borrow_mut().clear();
        }
    }

    /// Disposes `item` at once, or starts its disposal by a task (`TryDisposeItem`).
    fn try_dispose_item(&self, item: &T) {
        match (self.to_item_disposal)(item) {
            Some(ItemDisposal::Disposable(disposable)) => disposable.dispose(),
            Some(ItemDisposal::AsyncDisposable(async_disposable)) => {
                self.disposals.on_next(Observable::from_async(move || async_disposable.dispose_async()));
            }
            None => {}
        }
    }
}

/// `AsyncDisposeMany.Create`: each change set is forwarded first, then the replaced items (those
/// not equal to their replacements) and the removed ones are disposed. `disposals_completed_accessor`
/// receives, once per subscription, a sequence of one value that ends when the sequence has ended
/// and every disposal by a task has ended; the error of such a disposal fails it.
pub(crate) fn async_dispose_many<T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    to_item_disposal: ToItemDisposal<T>,
    disposals_completed_accessor: Rc<dyn Fn(Rc<dyn IObservable<()>>)>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |downstream_observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let disposals = Subject::<Rc<dyn IObservable<()>>>::new();
        let disposals_completed = disposals
            .as_observable()
            .merge_inner::<()>()
            .ignore_elements()
            .concat(Observable::return_(()))
            .publish()
            .auto_connect_min(0);

        disposals_completed_accessor(disposals_completed);

        let state = Rc::new(AsyncDisposeManyState { items_by_key: RefCell::new(Cache::new()), disposals, to_item_disposal: to_item_disposal.clone() });

        let (next_state, error_state, completed_state) = (state.clone(), state.clone(), state.clone());
        let (next_observer, error_observer, completed_observer) = (downstream_observer.clone(), downstream_observer.clone(), downstream_observer);
        let source_subscription = synchronize_safe(source.clone()).subscribe_safe(Observer::create_all(
            move |upstream_changes: ChangeSet<T, K>| {
                next_observer.on_next(upstream_changes.clone());

                for change in &upstream_changes {
                    match change.reason() {
                        ChangeReason::Update => {
                            if let Some(previous) = change.previous() {
                                if change.current() != previous {
                                    next_state.try_dispose_item(previous);
                                }
                            }
                        }
                        ChangeReason::Remove => next_state.try_dispose_item(change.current()),
                        ChangeReason::Add | ChangeReason::Refresh | ChangeReason::Moved => {}
                    }
                }

                next_state.items_by_key.borrow_mut().clone_changes(&upstream_changes);
            },
            move |error| {
                error_observer.on_error(error);
                error_state.tear_down();
            },
            move || {
                completed_observer.on_completed();
                completed_state.tear_down();
            },
        ));

        Disposable::create(move || {
            source_subscription.dispose();
            state.tear_down();
        })
    })
}
