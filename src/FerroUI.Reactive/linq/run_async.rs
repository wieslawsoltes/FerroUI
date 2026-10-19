//! `RunAsync` and `GetAwaiter` (`Linq/Observable.Awaiter.cs`, `Linq/QueryLanguage.Awaiter.cs`): a
//! sequence as a task of its last value, through an [`AsyncSubject`] as upstream does
//! (docs/porting/reactive.md, section 5.8).
//!
//! Deviation (DEVIATIONS.md, Reactive): upstream returns the `AsyncSubject<TSource>`, which is its
//! own awaiter (`IsCompleted`, `OnCompleted`, `GetResult`); here the methods return the task of
//! that subject ([`AsyncSubject::get_awaiter`]), whose `is_completed`, `continue_with` and `result`
//! are those members, and which `async` code awaits. The cancellation ends the subject with a
//! `TaskCanceledError` (upstream's `OperationCanceledException`), which cancels the task.

use crate::disposables::StableCompositeDisposable;
use crate::internal::TokenRegistration;
use crate::observer::AnonymousObserver;
use crate::subjects::{AsyncSubject, IConnectableObservable};
use ferroui_system::threading::tasks::{Task, TaskCanceledError};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// `Cancel`: ends the subject with a cancellation (`OperationCanceledException`).
fn cancel<T: Clone + 'static>(subject: &AsyncSubject<T>) {
    subject.on_error(Rc::new(TaskCanceledError::default()));
}

/// `RegisterCancelation`: the cancellation of `token` disposes `subscription` and cancels the
/// subject; the end of the subject removes the registration.
fn register_cancelation<T: Clone + 'static>(subject: &Rc<AsyncSubject<T>>, subscription: Rc<dyn IDisposable>, token: &CancellationToken) {
    let target = subject.clone();
    let registration = TokenRegistration::register(token, move || {
        subscription.dispose();
        cancel(&target);
    });
    let on_error = registration.clone();
    let on_completed = registration;
    subject.subscribe(Rc::new(AnonymousObserver::new(|_: T| {}, move |_| on_error.dispose(), move || on_completed.dispose())));
}

/// `RunAsync(IObservable<TSource>, CancellationToken)`: the subject that the sequence is
/// subscribed to; a token canceled already cancels it without subscribing.
pub(crate) fn run_async_subject<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, token: &CancellationToken) -> Rc<AsyncSubject<T>> {
    let subject = AsyncSubject::new();
    if token.is_cancellation_requested() {
        cancel(&subject);
        return subject;
    }
    let subscription = source.subscribe(subject.as_observer());
    if token.can_be_canceled() {
        register_cancelation(&subject, subscription, token);
    }
    subject
}

/// `RunAsync(IConnectableObservable<TSource>, CancellationToken)`: as
/// [`run_async_subject`], and the sequence is connected once the subject is subscribed.
pub(crate) fn run_async_connectable_subject<T: Clone + 'static>(
    source: Rc<dyn IConnectableObservable<T>>,
    token: &CancellationToken,
) -> Rc<AsyncSubject<T>> {
    let subject = AsyncSubject::new();
    if token.is_cancellation_requested() {
        cancel(&subject);
        return subject;
    }
    let subscription = source.subscribe(subject.as_observer());
    let connection = source.connect();
    if token.can_be_canceled() {
        let both: Rc<dyn IDisposable> = StableCompositeDisposable::create(subscription, connection);
        register_cancelation(&subject, both, token);
    }
    subject
}

/// The task of [`run_async_subject`].
pub(crate) fn run_async<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, token: CancellationToken) -> Task<T> {
    run_async_subject(source, &token).get_awaiter()
}

/// The task of [`run_async_connectable_subject`].
pub(crate) fn run_async_connectable<T: Clone + 'static>(source: Rc<dyn IConnectableObservable<T>>, token: CancellationToken) -> Task<T> {
    run_async_connectable_subject(source, &token).get_awaiter()
}

/// The methods of `RunAsync` and `GetAwaiter` on a sequence (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! run_async_methods {
    () => {
        /// Subscribes to the sequence and gives a task of its last value: the task ends when the
        /// sequence ends, with its last value, with its error, or with
        /// `SequenceError::NoElements` when it had no value (`GetAwaiter(IObservable<TSource>)`,
        /// the subject `await` awaits).
        fn get_awaiter(&self) -> ferroui_system::threading::tasks::Task<T>
        where
            T: Clone,
        {
            crate::linq::run_async::run_async(self.observable(), ferroui_system::threading::CancellationToken::none())
        }

        /// [`get_awaiter`](Self::get_awaiter) that the cancellation of `token` cancels, disposing
        /// the subscription; a token canceled already cancels the task without subscribing
        /// (`RunAsync(IObservable<TSource>, CancellationToken)`).
        fn run_async(&self, token: ferroui_system::threading::CancellationToken) -> ferroui_system::threading::tasks::Task<T>
        where
            T: Clone,
        {
            crate::linq::run_async::run_async(self.observable(), token)
        }
    };
}
pub(crate) use run_async_methods;

/// The methods of `RunAsync` and `GetAwaiter` on a connectable sequence (provided methods of
/// [`ConnectableObservableExt`](super::ConnectableObservableExt)), which also connect it.
///
/// They take the handle by value: the handle of a connectable sequence has the methods of
/// [`ObservableExt`](super::ObservableExt) too, and method resolution picks a method that takes
/// the receiver by value before one that borrows it, so `connectable.run_async(token)` calls these,
/// as upstream's overload resolution prefers the `IConnectableObservable<TSource>` overloads.
macro_rules! run_async_connectable_methods {
    () => {
        /// Subscribes to the sequence, connects it, and gives a task of its last value
        /// (`GetAwaiter(IConnectableObservable<TSource>)`); see
        /// [`ObservableExt::get_awaiter`](crate::linq::ObservableExt::get_awaiter).
        fn get_awaiter(self) -> ferroui_system::threading::tasks::Task<T>
        where
            Self: Sized,
            T: Clone,
        {
            crate::linq::run_async::run_async_connectable(self.connectable(), ferroui_system::threading::CancellationToken::none())
        }

        /// [`get_awaiter`](Self::get_awaiter) that the cancellation of `token` cancels, disposing
        /// the subscription and the connection; a token canceled already cancels the task without
        /// subscribing or connecting (`RunAsync(IConnectableObservable<TSource>, CancellationToken)`).
        fn run_async(self, token: ferroui_system::threading::CancellationToken) -> ferroui_system::threading::tasks::Task<T>
        where
            Self: Sized,
            T: Clone,
        {
            crate::linq::run_async::run_async_connectable(self.connectable(), token)
        }
    };
}
pub(crate) use run_async_connectable_methods;
