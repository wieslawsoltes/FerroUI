//! `Defer` and `DeferAsync` with an asynchronous factory (`Linq/QueryLanguage.Creation.cs`, region
//! `DeferAsync`): `Defer(() => StartAsync(observableFactoryAsync).Merge())`, the factory returning a
//! future, which runs as a task (docs/porting/reactive.md, section 5.8).
//!
//! The overloads with `ignoreExceptionsAfterUnsubscribe` are not ported: the option concerns the
//! unobserved exceptions of the task scheduler of .NET, which tasks here do not have (see
//! `TaskObservationOptions` in `linq/task_observable.rs`).

use crate::linq::merge::merge_observables;
use crate::linq::task_observable::{start_async, start_async_with_token};
use crate::linq::Observable;
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IObservable, ObservableError};
use std::future::Future;
use std::rc::Rc;

impl Observable {
    /// Runs the future the factory gives for each subscription, as a task, and subscribes to the
    /// sequence it gives; an error of the task is the error of the sequence
    /// (`Defer<TResult>(Func<Task<IObservable<TResult>>>)`).
    pub fn defer_async<T: 'static, Fut: Future<Output = Result<Rc<dyn IObservable<T>>, ObservableError>> + 'static>(
        observable_factory_async: impl Fn() -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Observable::defer(move || merge_observables(start_async(observable_factory_async(), None), None))
    }

    /// [`defer_async`](Self::defer_async) with a token, which disposing the subscription cancels
    /// (`DeferAsync<TResult>(Func<CancellationToken, Task<IObservable<TResult>>>)`).
    pub fn defer_async_with_token<T: 'static, Fut: Future<Output = Result<Rc<dyn IObservable<T>>, ObservableError>> + 'static>(
        observable_factory_async: impl Fn(CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Observable::defer(move || merge_observables(start_async_with_token(&observable_factory_async, None), None))
    }
}
