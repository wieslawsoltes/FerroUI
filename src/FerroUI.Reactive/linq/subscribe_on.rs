//! `SubscribeOn` (`Concurrency/Synchronization.cs`, `SubscribeOnObservable<TSource>`; the operator
//! of `Linq/Observable.Concurrency.cs`).
//!
//! `SubscribeOn(SynchronizationContext)` is not ported yet (phase 4).

use crate::concurrency::IScheduler;
use crate::disposables::{Disposable, ScheduledDisposable, SerialDisposableValue};
use crate::internal::subscribe_auto_detach;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// The subscription of `SubscribeOn` (`SubscribeOnObservable<TSource>.Subscription`): the work that
/// subscribes to the source on the scheduler, then the subscription to the source, disposed on the
/// scheduler it was made on.
struct SubscribeOnSubscription {
    cancel: SerialDisposableValue,
}

impl SubscribeOnSubscription {
    fn new<T: 'static>(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>, observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        let this = Rc::new(Self { cancel: SerialDisposableValue::new() });
        let target = this.clone();
        let scheduled = scheduler.schedule(Box::new(move |closure_scheduler| {
            let subscription = source.subscribe(observer);
            let disposal: Rc<dyn IDisposable> = ScheduledDisposable::new(closure_scheduler.clone(), subscription);
            target.cancel.set_disposable(Some(disposal));
            Disposable::empty()
        }));
        // When the scheduler ran the work at once, the subscription is in place already and the
        // scheduled work is not kept.
        this.cancel.try_set_first(scheduled);
        this
    }
}

impl IDisposable for SubscribeOnSubscription {
    fn dispose(&self) {
        self.cancel.dispose();
    }
}

/// The observable of `SubscribeOn` (`SubscribeOnObservable<TSource>`, an `ObservableBase<TSource>`).
struct SubscribeOnObservable<T> {
    source: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObservable<T> for SubscribeOnObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let scheduler = self.scheduler.clone();
        subscribe_auto_detach(observer, move |observer| {
            let subscription: Rc<dyn IDisposable> = SubscribeOnSubscription::new(source, scheduler, observer);
            subscription
        })
    }
}

pub(crate) fn subscribe_on<T: 'static>(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(SubscribeOnObservable { source, scheduler })
}

/// The methods of `SubscribeOn` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! subscribe_on_methods {
    () => {
        /// Subscribes to the sequence, and disposes the subscription, on `scheduler`; the
        /// notifications are not moved (`SubscribeOn(IScheduler)`).
        fn subscribe_on(
            &self,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::subscribe_on::subscribe_on(self.observable(), scheduler)
        }
    };
}
pub(crate) use subscribe_on_methods;
