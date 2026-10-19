//! `Switch<TObject, TKey>` (`Cache/Internal/Switch.cs`): the change sets of the latest cache of a
//! sequence of caches, with the items of the previous one removed at each switch.

use super::Cache;
use crate::cache::{Change, ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::DeliveryQueue;
use ferroui_reactive::disposables::{Disposable, SerialDisposable, SingleAssignmentDisposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

/// `Switch.Run`.
///
/// Switching is done by hand rather than with `Observable.Switch`, which holds its gate across the
/// whole of downstream delivery. Here the gate is never held around user code, which can loop back
/// into this operator, so state read before such a call may be stale after it and every step
/// re-checks that its generation is still current.
///
/// The locks of upstream become nothing: its read locks, which publish state without delivering,
/// are plain reads of the state; its write locks are scopes of the delivery queue, whose end
/// delivers what they enqueued.
pub(crate) fn switch_cache_change_sets<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    sources: Rc<dyn IObservable<Rc<dyn IObservable<ChangeSet<T, K>>>>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let state = Rc::new(SwitchState {
            queue: DeliveryQueue::new(observer),
            current: RefCell::new(Cache::new()),
            current_subscription: SerialDisposable::new(),
            current_generation: RefCell::new(None),
            pending_source: RefCell::new(None),
            is_activating: Cell::new(false),
            has_outstanding_source: Cell::new(false),
            are_sources_complete: Cell::new(false),
            is_stopped: Cell::new(false),
        });

        let (select, fail, completed) = (state.clone(), state.clone(), state.clone());
        let sources_subscription = sources.subscribe_all(
            move |source| SwitchState::select_source(&select, source),
            move |error| fail.fail(error, None),
            move || completed.sources_completed(),
        );

        Disposable::create(move || {
            // Invalidate pending work before waiting for delivery; this scope does not drain.
            state.is_stopped.set(true);
            *state.pending_source.borrow_mut() = None;
            // Finish downstream delivery before tearing down the sources feeding it.
            state.queue.dispose();
            sources_subscription.dispose();
            state.current_subscription.dispose();
        })
    })
}

/// The state of a subscription (the locals of upstream's `Run`).
struct SwitchState<T: 'static, K: 'static> {
    queue: Rc<DeliveryQueue<ChangeSet<T, K>>>,
    /// What the current source has contributed, so that switching away can take it back out.
    current: RefCell<Cache<T, K>>,
    current_subscription: Rc<SerialDisposable>,
    /// Doubles as the generation token, published before any user code runs so that a reentrant
    /// selection can cancel a subscription whose Subscribe has not yet returned.
    current_generation: RefCell<Option<Rc<SingleAssignmentDisposable>>>,
    /// A selection made while an earlier Subscribe or Dispose frame is still on the stack cannot
    /// start yet: that frame owns resources it cannot release until it unwinds, so the activation
    /// already in progress owns the handoff loop and picks the latest selection up when it gets
    /// there.
    pending_source: RefCell<Option<Rc<dyn IObservable<ChangeSet<T, K>>>>>,
    is_activating: Cell<bool>,
    /// Selected but not yet completed, so the result cannot complete while a source is still
    /// pending.
    has_outstanding_source: Cell<bool>,
    are_sources_complete: Cell<bool>,
    is_stopped: Cell<bool>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> SwitchState<T, K> {
    /// `SelectSource`.
    fn select_source(this: &Rc<Self>, source: Rc<dyn IObservable<ChangeSet<T, K>>>) {
        // Publish without draining: nothing may get ahead of releasing the previous source's
        // resources.
        if this.is_stopped.get() || this.are_sources_complete.get() {
            return;
        }
        let generation = SingleAssignmentDisposable::new();
        *this.current_generation.borrow_mut() = Some(generation.clone());
        this.has_outstanding_source.set(true);
        // Latest selection always wins; only starting it can be deferred.
        *this.pending_source.borrow_mut() = Some(source);
        let owns_handoff = !this.is_activating.get();
        this.is_activating.set(true);

        // Disposing the outgoing subscription runs user teardown, so it happens outside the gate.
        let generation: Rc<dyn IDisposable> = generation;
        this.current_subscription.set_disposable(Some(generation));

        if !owns_handoff {
            // An activation further down the stack starts this selection when it unwinds.
            return;
        }

        Self::run_handoffs(this);
    }

    /// `RunHandoffs`: starts selections one at a time, so a replacement never begins while a
    /// superseded frame unwinds.
    fn run_handoffs(this: &Rc<Self>) {
        loop {
            let source = this.pending_source.borrow_mut().take();
            let generation = this.current_generation.borrow().clone();
            let (source, generation) = match (source, generation) {
                (Some(source), Some(generation)) if !this.is_stopped.get() => (source, generation),
                _ => {
                    this.is_activating.set(false);
                    return;
                }
            };
            Self::activate(this, source, generation);
        }
    }

    /// `Activate`.
    fn activate(this: &Rc<Self>, source: Rc<dyn IObservable<ChangeSet<T, K>>>, generation: Rc<SingleAssignmentDisposable>) {
        {
            let scope = this.queue.acquire();
            if !this.is_current(&generation) {
                return;
            }
            let removals = {
                let mut current = this.current.borrow_mut();
                if current.count() == 0 {
                    None
                } else {
                    let changes: Vec<Change<T, K>> = current.key_values().into_iter().map(|(key, value)| Change::new(ChangeReason::Remove, key, value)).collect();
                    current.clear();
                    Some(ChangeSet::from(changes))
                }
            };
            if let Some(removals) = removals {
                scope.enqueue_next(removals);
            }
        }

        // That scope delivered the clearing changeset, so a newer source may have been selected.
        // Skipping the subscribe is only an optimization; the handlers re-check anyway.
        if !this.is_current(&generation) {
            return;
        }

        // Subscribe outside the gate and assign only to this generation's holder: if a newer
        // source was selected synchronously, the holder is already disposed, so this assignment
        // releases the late subscription and the handoff loop then starts the newer source.
        let (next, error, completed) = (this.clone(), this.clone(), this.clone());
        let (next_generation, error_generation, completed_generation) = (generation.clone(), generation.clone(), generation.clone());
        let subscription = source.subscribe_all(
            move |changes: ChangeSet<T, K>| {
                let scope = next.queue.acquire();
                if !next.is_current(&next_generation) {
                    return;
                }
                next.current.borrow_mut().clone_changes(&changes);
                if changes.count() != 0 {
                    scope.enqueue_next(changes);
                }
            },
            move |e| error.fail(e, Some(&error_generation)),
            move || {
                let scope = completed.queue.acquire();
                if !completed.is_current(&completed_generation) {
                    return;
                }
                completed.has_outstanding_source.set(false);
                if completed.are_sources_complete.get() {
                    completed.is_stopped.set(true);
                    scope.enqueue_completed();
                }
            },
        );
        generation.set_disposable(Some(subscription));
    }

    /// `SourcesCompleted`.
    fn sources_completed(&self) {
        let scope = self.queue.acquire();
        if self.is_stopped.get() {
            return;
        }
        self.are_sources_complete.set(true);
        // No source outstanding also covers the case where none was ever selected.
        if !self.has_outstanding_source.get() {
            self.is_stopped.set(true);
            scope.enqueue_completed();
        }
    }

    /// `Fail`.
    fn fail(&self, error: ObservableError, generation: Option<&Rc<SingleAssignmentDisposable>>) {
        let scope = self.queue.acquire();
        let superseded = match generation {
            Some(generation) => !self.current_generation.borrow().as_ref().is_some_and(|current| Rc::ptr_eq(current, generation)),
            None => false,
        };
        if self.is_stopped.get() || superseded {
            return;
        }
        // Stop as soon as the terminal notification is queued, not only when it is delivered.
        self.is_stopped.set(true);
        scope.enqueue_error(error);
    }

    /// `IsCurrent`: all callers hold the queue gate; notification handlers and handoffs use the
    /// same state.
    fn is_current(&self, generation: &Rc<SingleAssignmentDisposable>) -> bool {
        !self.is_stopped.get() && self.current_generation.borrow().as_ref().is_some_and(|current| Rc::ptr_eq(current, generation))
    }
}
