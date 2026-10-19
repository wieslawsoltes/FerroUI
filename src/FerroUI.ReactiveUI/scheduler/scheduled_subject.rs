//! `ScheduledSubject<T>` (`ReactiveUI.Shared/Scheduler/ScheduledSubject.cs`).

use crate::internal::subjects::SchedulingObserver;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// A subject that delivers its notifications to each observer on a scheduler
/// (`ScheduledSubject<T>`). A default observer receives the notifications while no other observer
/// is subscribed: the first subscription unsubscribes it, and it is subscribed again when the last
/// subscription is disposed.
pub struct ScheduledSubject<T> {
    this: Weak<ScheduledSubject<T>>,
    default_observer: Option<Rc<dyn IObserver<T>>>,
    scheduler: Rc<dyn IScheduler>,
    subject: Rc<dyn SubjectBase<T>>,
    observer_ref_count: Cell<usize>,
    default_observer_sub: RefCell<Rc<dyn IDisposable>>,
    is_disposed: Cell<bool>,
}

impl<T> fmt::Debug for ScheduledSubject<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScheduledSubject").field("observers", &self.observer_ref_count.get()).finish()
    }
}

impl<T: Clone + 'static> ScheduledSubject<T> {
    /// A subject that delivers on `scheduler` (`ScheduledSubject(IScheduler)`).
    pub fn new(scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Self::with_default_subject(scheduler, None, None)
    }

    /// A subject that delivers on `scheduler`, and to `default_observer` while nothing else
    /// subscribes (`ScheduledSubject(IScheduler, IObserver<T>?)`).
    pub fn with_default_observer(scheduler: Rc<dyn IScheduler>, default_observer: Option<Rc<dyn IObserver<T>>>) -> Rc<Self> {
        Self::with_default_subject(scheduler, default_observer, None)
    }

    /// A subject that delivers on `scheduler`, to `default_observer` while nothing else
    /// subscribes, through `default_subject` (a [`Subject`] when `None`)
    /// (`ScheduledSubject(IScheduler, IObserver<T>?, ISubject<T>?)`).
    pub fn with_default_subject(
        scheduler: Rc<dyn IScheduler>,
        default_observer: Option<Rc<dyn IObserver<T>>>,
        default_subject: Option<Rc<dyn SubjectBase<T>>>,
    ) -> Rc<Self> {
        let subject = default_subject.unwrap_or_else(|| Subject::<T>::new() as Rc<dyn SubjectBase<T>>);
        let default_observer_sub = match &default_observer {
            Some(observer) => subject.subscribe(SchedulingObserver::new(observer.clone(), scheduler.clone())),
            None => Disposable::empty(),
        };
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            default_observer,
            scheduler,
            subject,
            observer_ref_count: Cell::new(0),
            default_observer_sub: RefCell::new(default_observer_sub),
            is_disposed: Cell::new(false),
        })
    }

    /// Whether observers are subscribed (`HasObservers`).
    pub fn has_observers(&self) -> bool {
        self.subject.has_observers()
    }

    /// Whether the subject has been disposed (`IsDisposed`).
    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get() || self.subject.is_disposed()
    }

    /// The subject as the handle of an observable sequence.
    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }

    /// The subject as an observer.
    pub fn as_observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }

    fn on_unsubscribe(&self) {
        let count = self.observer_ref_count.get().saturating_sub(1);
        self.observer_ref_count.set(count);
        if count > 0 {
            return;
        }
        let Some(default_observer) = &self.default_observer else {
            return;
        };
        let subscription = self.subject.subscribe(SchedulingObserver::new(default_observer.clone(), self.scheduler.clone()));
        *self.default_observer_sub.borrow_mut() = subscription;
    }
}

impl<T: Clone + 'static> IObserver<T> for ScheduledSubject<T> {
    fn on_next(&self, value: T) {
        self.subject.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.subject.on_error(error);
    }

    fn on_completed(&self) {
        self.subject.on_completed();
    }
}

impl<T: Clone + 'static> IObservable<T> for ScheduledSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let default_subscription = std::mem::replace(&mut *self.default_observer_sub.borrow_mut(), Disposable::empty());
        default_subscription.dispose();
        self.observer_ref_count.set(self.observer_ref_count.get() + 1);
        let inner = self.subject.subscribe(SchedulingObserver::new(observer, self.scheduler.clone()));
        let parent = self.this.clone();
        Disposable::create(move || {
            inner.dispose();
            if let Some(parent) = parent.upgrade() {
                parent.on_unsubscribe();
            }
        })
    }
}

impl<T: Clone + 'static> IDisposable for ScheduledSubject<T> {
    fn dispose(&self) {
        self.is_disposed.set(true);
        self.subject.dispose();
        let default_subscription = self.default_observer_sub.borrow().clone();
        default_subscription.dispose();
    }
}
