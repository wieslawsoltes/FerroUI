//! `Switch<T>` (`List/Internal/Switch.cs`): the change sets of the latest list of a sequence of
//! lists, with the items of the previous one cleared at each switch.

use crate::internal::DeliveryQueue;
use crate::list::{clone_changes, Change, ChangeSet, ListChangeReason};
use ferroui_reactive::disposables::{Disposable, SerialDisposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `Switch.Run`.
///
/// Switching is done by hand rather than with `Observable.Switch`, which holds its gate for the
/// whole of downstream delivery. The queue enqueues and returns instead, so a producer is never
/// held up by whatever a subscriber does with the notification. The locks of upstream are scopes
/// of the delivery queue, whose end delivers what they enqueued.
pub(crate) fn switch_list_change_sets<T: Clone + PartialEq + 'static>(sources: Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T>>>>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let state = Rc::new(SwitchState {
            queue: DeliveryQueue::new(observer),
            current: RefCell::new(Vec::new()),
            subscription: SerialDisposable::new(),
            active: Cell::new(0),
            is_source_running: Cell::new(false),
            are_sources_complete: Cell::new(false),
        });

        let (next, error, completed) = (state.clone(), state.clone(), state.clone());
        let outer = sources.subscribe_all(
            move |source| SwitchState::select_source(&next, source),
            move |e| {
                // `queue.OnError`.
                let scope = error.queue.acquire();
                scope.enqueue_error(e);
            },
            move || {
                let scope = completed.queue.acquire();
                completed.are_sources_complete.set(true);
                // The current source may still be running, and the result ends only once both
                // have.
                if !completed.is_source_running.get() {
                    scope.enqueue_completed();
                }
            },
        );

        // Disposal order matters. The queue goes first so that any delivery in flight is finished
        // before the subscriptions feeding it are torn down.
        Disposable::create(move || {
            state.queue.dispose();
            outer.dispose();
            state.subscription.dispose();
        })
    })
}

/// The state of a subscription (the locals of upstream's `Run`).
struct SwitchState<T: 'static> {
    queue: Rc<DeliveryQueue<ChangeSet<T>>>,
    /// What the current source has contributed, so that switching away can take it back out.
    current: RefCell<Vec<T>>,
    subscription: Rc<SerialDisposable>,
    /// Identifies the current source. A superseded one may still be mid-delivery, and anything it
    /// produces after this point belongs to a source that has already been switched away from.
    active: Cell<u64>,
    is_source_running: Cell<bool>,
    are_sources_complete: Cell<bool>,
}

impl<T: Clone + PartialEq + 'static> SwitchState<T> {
    /// The handler of a new source.
    fn select_source(this: &Rc<Self>, source: Rc<dyn IObservable<ChangeSet<T>>>) {
        let id = {
            let scope = this.queue.acquire();
            let id = this.active.get() + 1;
            this.active.set(id);
            this.is_source_running.set(true);
            let cleared = {
                let mut current = this.current.borrow_mut();
                if current.is_empty() {
                    None
                } else {
                    let items = std::mem::take(&mut *current);
                    Some(ChangeSet::from(vec![Change::range_change(ListChangeReason::Clear, items, None)]))
                }
            };
            if let Some(cleared) = cleared {
                scope.enqueue_next(cleared);
            }
            id
        };

        // Subscribed outside the lock. The source may deliver synchronously, and that delivery
        // takes the lock for itself.
        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let subscription = source.subscribe_all(
            move |changes: ChangeSet<T>| {
                let scope = next.queue.acquire();
                if id != next.active.get() {
                    return;
                }
                clone_changes(&mut *next.current.borrow_mut(), &changes);
                if changes.count() != 0 {
                    scope.enqueue_next(changes);
                }
            },
            move |e| {
                let scope = error.queue.acquire();
                if id != error.active.get() {
                    return;
                }
                scope.enqueue_error(e);
            },
            move || {
                let scope = completed.queue.acquire();
                if id != completed.active.get() {
                    return;
                }
                completed.is_source_running.set(false);
                if completed.are_sources_complete.get() {
                    scope.enqueue_completed();
                }
            },
        );
        this.subscription.set_disposable(Some(subscription));
    }
}
