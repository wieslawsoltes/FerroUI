//! `SubscribeOn` (`Concurrency/Synchronization.cs`, `SubscribeOnObservable<TSource>`; the operator
//! of `Linq/Observable.Concurrency.cs`).
//!
//! `SubscribeOn(SynchronizationContext)` (`SubscribeOnCtxObservable<TSource>`) posts the
//! subscription to the context, and the disposal of the subscription through a
//! [`ContextDisposable`].
//!
//! Deviation (DEVIATIONS.md, Reactive): the subscription is not `Send`, so it is posted with
//! [`SynchronizationContext::post_local`](ferroui_system::threading::SynchronizationContext::post_local):
//! the sequence is subscribed to on the thread of the context, later, when the context runs the
//! work it was posted. Upstream posts from the thread that subscribes, which may be any thread.

use crate::concurrency::{post_with_start_complete, IScheduler};
use crate::disposables::{ContextDisposable, Disposable, ScheduledDisposable, SerialDisposableValue, SingleAssignmentDisposableValue};
use crate::internal::subscribe_auto_detach;
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;
use std::sync::Arc;

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

/// The subscription of `SubscribeOn(SynchronizationContext)`
/// (`SubscribeOnCtxObservable<TSource>.Subscription`): the subscription to the source, posted to
/// the context unless the subscription is disposed first, and disposed on the context.
struct SubscribeOnContextSubscription {
    cancel: SingleAssignmentDisposableValue,
}

impl SubscribeOnContextSubscription {
    fn new<T: 'static>(source: Rc<dyn IObservable<T>>, context: Arc<dyn SynchronizationContext>, observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        let this = Rc::new(Self { cancel: SingleAssignmentDisposableValue::new() });
        let target = this.clone();
        let disposal_context = context.clone();
        post_with_start_complete(&context, move || {
            if !target.cancel.is_disposed() {
                let subscription = source.subscribe(observer);
                let disposal: Rc<dyn IDisposable> = ContextDisposable::new(disposal_context, subscription);
                target.cancel.set_disposable(Some(disposal));
            }
        });
        this
    }
}

impl IDisposable for SubscribeOnContextSubscription {
    fn dispose(&self) {
        self.cancel.dispose();
    }
}

/// The observable of `SubscribeOn(SynchronizationContext)` (`SubscribeOnCtxObservable<TSource>`, an
/// `ObservableBase<TSource>`).
struct SubscribeOnContextObservable<T> {
    source: Rc<dyn IObservable<T>>,
    context: Arc<dyn SynchronizationContext>,
}

impl<T: 'static> IObservable<T> for SubscribeOnContextObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let context = self.context.clone();
        subscribe_auto_detach(observer, move |observer| {
            let subscription: Rc<dyn IDisposable> = SubscribeOnContextSubscription::new(source, context, observer);
            subscription
        })
    }
}

pub(crate) fn subscribe_on_context<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    context: Arc<dyn SynchronizationContext>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(SubscribeOnContextObservable { source, context })
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

        /// Subscribes to the sequence, and disposes the subscription, on `context`: both are posted
        /// to it; the notifications are not moved (`SubscribeOn(SynchronizationContext)`).
        fn subscribe_on_context(
            &self,
            context: std::sync::Arc<dyn ferroui_system::threading::SynchronizationContext>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::subscribe_on::subscribe_on_context(self.observable(), context)
        }
    };
}
pub(crate) use subscribe_on_methods;
