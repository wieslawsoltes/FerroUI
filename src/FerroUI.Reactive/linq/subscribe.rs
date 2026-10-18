//! The subscription overloads of `ObservableExtensions` (`Observable.Extensions.cs`).

use crate::internal::SafeObserver;
use ferroui_system::threading::{CancellationToken, CancellationTokenRegistration};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

ferroui_system::ferro_thread_local! {
    /// The subscriptions that the registrations of cancellation tokens dispose, by registration:
    /// the callback a token keeps must be `Send`, the subscriptions are not, so the callback names
    /// its entry here.
    static PENDING_SUBSCRIPTIONS: RefCell<HashMap<u64, Rc<dyn IDisposable>>> = RefCell::new(HashMap::new());
}

/// The numbers of the registrations, unique in the process so that a callback run on another
/// thread finds no entry of that thread.
static NEXT_REGISTRATION: AtomicU64 = AtomicU64::new(0);

/// The registration of the disposal of a subscription with a cancellation token
/// (`CancellationTokenRegistration` as a disposable): disposing it removes the registration.
struct SubscriptionRegistration {
    registration: CancellationTokenRegistration,
    id: u64,
}

impl SubscriptionRegistration {
    /// Registers the disposal of `subscription` to run when `token` is canceled on this thread; a
    /// token canceled already disposes it at once.
    fn register(token: &CancellationToken, subscription: Rc<dyn IDisposable>) -> Rc<Self> {
        let id = NEXT_REGISTRATION.fetch_add(1, Ordering::Relaxed);
        PENDING_SUBSCRIPTIONS.with(|pending| pending.borrow_mut().insert(id, subscription));
        let registration = token.register(move || {
            let subscription = PENDING_SUBSCRIPTIONS.try_with(|pending| pending.borrow_mut().remove(&id)).ok().flatten();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        });
        Rc::new(Self { registration, id })
    }
}

impl IDisposable for SubscriptionRegistration {
    fn dispose(&self) {
        self.registration.dispose();
        let id = self.id;
        let removed = PENDING_SUBSCRIPTIONS.try_with(|pending| pending.borrow_mut().remove(&id)).ok().flatten();
        drop(removed);
    }
}

/// Subscribes `observer` to `source` until `token` is canceled (`Subscribe_` of
/// `Observable.Extensions.cs`): a token that cannot be canceled subscribes as it is; a token
/// canceled already does not subscribe; else the observer is wrapped in a safe observer, and the
/// cancellation of the token disposes the subscription. A terminal notification removes the
/// registration (the resource of the safe observer), not the subscription.
pub(crate) fn subscribe_with_token<T: 'static>(source: Rc<dyn IObservable<T>>, observer: Rc<dyn IObserver<T>>, token: CancellationToken) {
    if token.can_be_canceled() {
        if !token.is_cancellation_requested() {
            let consumer = SafeObserver::wrap(observer);
            let subscription = source.subscribe(consumer.clone());
            consumer.set_resource(SubscriptionRegistration::register(&token, subscription));
        }
    } else {
        source.subscribe(observer);
    }
}

/// The subscription methods of `ObservableExtensions` (the provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! subscribe_methods {
    () => {
        /// Subscribes an observer, with the safeguards of a subscription (`SubscribeSafe`): an
        /// observer that panics while it handles a notification disposes the subscription first.
        fn subscribe_safe(&self, observer: std::rc::Rc<dyn ferroui_system::IObserver<T>>) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            // Every observable of the crate subscribes through the safeguards of a sink or of user
            // code (`SubscribeSafe` short-cuts producers the same way upstream).
            self.observable().subscribe(observer)
        }

        /// Subscribes and ignores every notification but errors, which panic (`Subscribe()`).
        fn subscribe_ignore(&self) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(|_| {}, crate::observer::throw, || {})))
        }

        /// Subscribes a function of values; an error panics (`Subscribe(Action<T>)`).
        fn subscribe_next(&self, on_next: impl Fn(T) + 'static) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, crate::observer::throw, || {})))
        }

        /// Subscribes functions of values and errors (`Subscribe(Action<T>, Action<Exception>)`).
        fn subscribe_next_error(
            &self,
            on_next: impl Fn(T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, on_error, || {})))
        }

        /// Subscribes functions of values and the end; an error panics (`Subscribe(Action<T>, Action)`).
        fn subscribe_next_completed(&self, on_next: impl Fn(T) + 'static, on_completed: impl Fn() + 'static) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, crate::observer::throw, on_completed)))
        }

        /// Subscribes functions of all three notifications (`Subscribe(Action<T>, Action<Exception>, Action)`).
        fn subscribe_all(
            &self,
            on_next: impl Fn(T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
            on_completed: impl Fn() + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IDisposable> {
            self.observable().subscribe(std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, on_error, on_completed)))
        }

        /// Subscribes an observer until `token` is canceled
        /// (`Subscribe(IObserver<T>, CancellationToken)`): the cancellation disposes the
        /// subscription; a token canceled already does not subscribe.
        fn subscribe_with_token(
            &self,
            observer: std::rc::Rc<dyn ferroui_system::IObserver<T>>,
            token: ferroui_system::threading::CancellationToken,
        ) {
            crate::linq::subscribe::subscribe_with_token(self.observable(), observer, token);
        }

        /// Subscribes until `token` is canceled and ignores every notification but errors, which
        /// panic (`Subscribe(CancellationToken)`).
        fn subscribe_ignore_with_token(&self, token: ferroui_system::threading::CancellationToken) {
            crate::linq::subscribe::subscribe_with_token(
                self.observable(),
                std::rc::Rc::new(crate::observer::AnonymousObserver::new(|_| {}, crate::observer::throw, || {})),
                token,
            );
        }

        /// Subscribes a function of values until `token` is canceled; an error panics
        /// (`Subscribe(Action<T>, CancellationToken)`).
        fn subscribe_next_with_token(&self, on_next: impl Fn(T) + 'static, token: ferroui_system::threading::CancellationToken) {
            crate::linq::subscribe::subscribe_with_token(
                self.observable(),
                std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, crate::observer::throw, || {})),
                token,
            );
        }

        /// Subscribes functions of values and errors until `token` is canceled
        /// (`Subscribe(Action<T>, Action<Exception>, CancellationToken)`).
        fn subscribe_next_error_with_token(
            &self,
            on_next: impl Fn(T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) {
            crate::linq::subscribe::subscribe_with_token(
                self.observable(),
                std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, on_error, || {})),
                token,
            );
        }

        /// Subscribes functions of values and the end until `token` is canceled; an error panics
        /// (`Subscribe(Action<T>, Action, CancellationToken)`).
        fn subscribe_next_completed_with_token(
            &self,
            on_next: impl Fn(T) + 'static,
            on_completed: impl Fn() + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) {
            crate::linq::subscribe::subscribe_with_token(
                self.observable(),
                std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, crate::observer::throw, on_completed)),
                token,
            );
        }

        /// Subscribes functions of all three notifications until `token` is canceled
        /// (`Subscribe(Action<T>, Action<Exception>, Action, CancellationToken)`).
        fn subscribe_all_with_token(
            &self,
            on_next: impl Fn(T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
            on_completed: impl Fn() + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) {
            crate::linq::subscribe::subscribe_with_token(
                self.observable(),
                std::rc::Rc::new(crate::observer::AnonymousObserver::new(on_next, on_error, on_completed)),
                token,
            );
        }
    };
}
pub(crate) use subscribe_methods;
