//! `IdleTimeoutObservable` (`Shared/Platform/IdleTimeoutObservable.cs`).

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// An idle timer that restarts on each deactivation (`IdleTimeoutObservable`): it notifies once
/// each time the application stays deactivated for the idle timeout without being reactivated.
/// Each deactivation starts the timer again, a reactivation cancels it, and when it elapses it
/// notifies an empty disposable. It replaces
/// `deactivated.SelectMany(_ => Timer(idle)).TakeUntil(reactivated).Repeat()`.
pub struct IdleTimeoutObservable {
    deactivated: Rc<dyn IObservable<()>>,
    reactivated: Rc<dyn IObservable<()>>,
    idle_timeout: Rc<dyn Fn() -> Duration>,
    scheduler: Rc<dyn IScheduler>,
}

impl IdleTimeoutObservable {
    /// The idle timer of the application (`IdleTimeoutObservable(IObservable<RxVoid>,
    /// IObservable<RxVoid>, Func<TimeSpan>, ISequencer)`): `idle_timeout` is read each time the
    /// timer starts, and the timer runs on `scheduler`.
    pub fn new(
        deactivated: Rc<dyn IObservable<()>>,
        reactivated: Rc<dyn IObservable<()>>,
        idle_timeout: impl Fn() -> Duration + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<Self> {
        Rc::new(Self { deactivated, reactivated, idle_timeout: Rc::new(idle_timeout), scheduler })
    }
}

impl IObservable<Rc<dyn IDisposable>> for IdleTimeoutObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<Rc<dyn IDisposable>>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(IdleTimeoutSink {
            downstream: observer,
            idle_timeout: self.idle_timeout.clone(),
            scheduler: self.scheduler.clone(),
            timer: RefCell::new(None),
            deactivated_subscription: RefCell::new(None),
            reactivated_subscription: RefCell::new(None),
            disposed: Cell::new(false),
        });
        // `Run`. The sources hold the sink until it is disposed, as their observers do upstream.
        let on_deactivated = sink.clone();
        let deactivated = self.deactivated.subscribe(Observer::create_with_error(
            move |_: ()| on_deactivated.on_deactivated(&on_deactivated),
            |_| {},
        ));
        *sink.deactivated_subscription.borrow_mut() = Some(deactivated);
        let on_reactivated = sink.clone();
        let reactivated = self.reactivated.subscribe(Observer::create_with_error(move |_: ()| on_reactivated.cancel_timer(), |_| {}));
        *sink.reactivated_subscription.borrow_mut() = Some(reactivated);
        sink
    }
}

/// `IdleTimeoutObservable.Sink`: the pending timer and the subscriptions to the two sources.
struct IdleTimeoutSink {
    downstream: Rc<dyn IObserver<Rc<dyn IDisposable>>>,
    idle_timeout: Rc<dyn Fn() -> Duration>,
    scheduler: Rc<dyn IScheduler>,
    /// `_timer`: the pending timer, if any.
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_deactivatedSubscription`.
    deactivated_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_reactivatedSubscription`.
    reactivated_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_disposed`.
    disposed: Cell<bool>,
}

impl IdleTimeoutSink {
    /// `OnDeactivated`: starts the timer again.
    fn on_deactivated(&self, this: &Rc<IdleTimeoutSink>) {
        if self.disposed.get() {
            return;
        }

        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
        let elapsed = this.clone();
        let timer = self.scheduler.schedule_action_after((self.idle_timeout)(), move || elapsed.on_elapsed());
        *self.timer.borrow_mut() = Some(timer);
    }

    /// `CancelTimer`: cancels the pending timer on a reactivation.
    fn cancel_timer(&self) {
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
    }

    /// `OnElapsed`: notifies the idle timeout, unless the sink was disposed.
    fn on_elapsed(&self) {
        if self.disposed.get() {
            return;
        }
        *self.timer.borrow_mut() = None;
        self.downstream.on_next(Disposable::empty());
    }
}

impl IDisposable for IdleTimeoutSink {
    fn dispose(&self) {
        self.disposed.set(true);
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }

        if let Some(subscription) = self.deactivated_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.reactivated_subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}
