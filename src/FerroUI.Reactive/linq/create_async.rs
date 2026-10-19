//! `Create` with an asynchronous subscription (`Linq/QueryLanguage.Creation.cs`, region
//! `CreateAsync`): the subscription is a function that returns a future, which runs as a task
//! (docs/porting/reactive.md, section 5.8).
//!
//! The forms of `Func<IObserver<TResult>, Task>` end the sequence when the task ends (an error of
//! the task is the error of the sequence); the forms of `Task<IDisposable>` and `Task<Action>`
//! give the resource that ends the subscription when the task ends, and disposing the
//! subscription before then disposes the resource as soon as it arrives. The forms with a token
//! cancel it when the subscription is disposed.

use crate::disposables::SingleAssignmentDisposableValue;
use crate::linq::select_many::future_as_task;
use crate::linq::task_observable::task_to_observable;
use crate::linq::Observable;
use crate::ObservableBase;
use ferroui_system::threading::{CancellationToken, CancellationTokenSource};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/// The observer of the task of the forms without a resource (`TaskCompletionObserver`): its end and
/// its error are those of the sequence.
struct TaskCompletionObserver<T> {
    observer: Rc<dyn IObserver<T>>,
}

impl<T> IObserver<()> for TaskCompletionObserver<T> {
    fn on_next(&self, _value: ()) {
        // Deliberately ignored, as upstream does.
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error);
    }

    fn on_completed(&self) {
        self.observer.on_completed();
    }
}

/// The subscription of the forms without a resource (`CreateWithTaskTokenObservable.Subscription`).
struct TaskTokenSubscription {
    cancel: CancellationTokenSource,
    subscription: Rc<dyn IDisposable>,
}

impl IDisposable for TaskTokenSubscription {
    fn dispose(&self) {
        self.cancel.cancel();
        self.subscription.dispose();
    }
}

/// The observer of the task of the forms with a disposable
/// (`CreateWithTaskDisposable.Subscription.TaskDisposeCompletionObserver`): the disposable is
/// assigned when the task ends, or disposed at once when the subscription was disposed before.
struct TaskDisposeCompletionObserver<T> {
    observer: Rc<dyn IObserver<T>>,
    disposable: SingleAssignmentDisposableValue,
}

impl<T> IObserver<Rc<dyn IDisposable>> for TaskDisposeCompletionObserver<T> {
    fn on_next(&self, value: Rc<dyn IDisposable>) {
        self.disposable.set_disposable(Some(value));
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error);
    }

    fn on_completed(&self) {}
}

impl<T> IDisposable for TaskDisposeCompletionObserver<T> {
    fn dispose(&self) {
        self.disposable.dispose();
    }
}

/// The subscription of the forms with a disposable or an action
/// (`CreateWithTaskDisposable.Subscription`, `CreateWithTaskActionObservable.Subscription`): the
/// subscription to the task is never disposed, so that the resource is always released.
struct TaskResourceSubscription {
    cancel: CancellationTokenSource,
    observer: Rc<dyn IDisposable>,
}

impl IDisposable for TaskResourceSubscription {
    fn dispose(&self) {
        self.cancel.cancel();
        self.observer.dispose();
    }
}

/// The action of the forms with an action: not arrived, arrived, or run (`Stubs.Nop` upstream).
enum ActionSlot<A> {
    Empty,
    Set(A),
    Done,
}

/// The observer of the task of the forms with an action
/// (`CreateWithTaskActionObservable.Subscription.TaskDisposeCompletionObserver`): the action
/// arrives in `result` when the task ends, and runs when the subscription is disposed, at once
/// when it was disposed before.
struct TaskActionCompletionObserver<T, A> {
    observer: Rc<dyn IObserver<T>>,
    result: Rc<RefCell<Option<A>>>,
    action: RefCell<ActionSlot<A>>,
}

impl<T, A: FnOnce()> IObserver<()> for TaskActionCompletionObserver<T, A> {
    fn on_next(&self, _value: ()) {
        let Some(value) = self.result.borrow_mut().take() else { return };
        let mut action = self.action.borrow_mut();
        if matches!(*action, ActionSlot::Empty) {
            *action = ActionSlot::Set(value);
        } else {
            drop(action);
            value();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error);
    }

    fn on_completed(&self) {}
}

impl<T, A: FnOnce()> IDisposable for TaskActionCompletionObserver<T, A> {
    fn dispose(&self) {
        let previous = std::mem::replace(&mut *self.action.borrow_mut(), ActionSlot::Done);
        if let ActionSlot::Set(action) = previous {
            action();
        }
    }
}

/// `CreateWithTaskTokenObservable`.
fn create_with_task_token<T: 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
    subscribe_async: impl Fn(Rc<dyn IObserver<T>>, CancellationToken) -> Fut + 'static,
) -> Rc<dyn IObservable<T>> {
    Rc::new(ObservableBase::from_fn(move |observer: Rc<dyn IObserver<T>>| -> Rc<dyn IDisposable> {
        let cancel = CancellationTokenSource::new();
        let task = future_as_task(subscribe_async(observer.clone(), cancel.token()));
        let subscription = task_to_observable(task, None).subscribe(Rc::new(TaskCompletionObserver { observer }));
        Rc::new(TaskTokenSubscription { cancel, subscription })
    }))
}

/// `CreateWithTaskDisposable`.
fn create_with_task_disposable<T: 'static, Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static>(
    subscribe_async: impl Fn(Rc<dyn IObserver<T>>, CancellationToken) -> Fut + 'static,
) -> Rc<dyn IObservable<T>> {
    Rc::new(ObservableBase::from_fn(move |observer: Rc<dyn IObserver<T>>| -> Rc<dyn IDisposable> {
        let cancel = CancellationTokenSource::new();
        let task = future_as_task(subscribe_async(observer.clone(), cancel.token()));
        let completion = Rc::new(TaskDisposeCompletionObserver { observer, disposable: SingleAssignmentDisposableValue::new() });
        // Never disposed: the disposable has to arrive to be released.
        let _ = task_to_observable(task, None).subscribe(completion.clone());
        Rc::new(TaskResourceSubscription { cancel, observer: completion })
    }))
}

/// `CreateWithTaskActionObservable`. The action is taken out of the outcome of the future, so that
/// it need not be `Clone` as the result of a task is.
fn create_with_task_action<T: 'static, A: FnOnce() + 'static, Fut: Future<Output = Result<A, ObservableError>> + 'static>(
    subscribe_async: impl Fn(Rc<dyn IObserver<T>>, CancellationToken) -> Fut + 'static,
) -> Rc<dyn IObservable<T>> {
    Rc::new(ObservableBase::from_fn(move |observer: Rc<dyn IObserver<T>>| -> Rc<dyn IDisposable> {
        let cancel = CancellationTokenSource::new();
        let result = Rc::new(RefCell::new(None));
        let future = subscribe_async(observer.clone(), cancel.token());
        let slot = result.clone();
        let task = future_as_task(async move {
            let action = future.await?;
            *slot.borrow_mut() = Some(action);
            Ok::<(), ObservableError>(())
        });
        let completion = Rc::new(TaskActionCompletionObserver { observer, result, action: RefCell::new(ActionSlot::Empty) });
        // Never disposed: the action has to arrive to run.
        let _ = task_to_observable(task, None).subscribe(completion.clone());
        Rc::new(TaskResourceSubscription { cancel, observer: completion })
    }))
}

impl Observable {
    /// A sequence whose subscription is an asynchronous function: the future it returns runs as a
    /// task (a `Task` is used as it is), and the sequence ends when the task ends, with its error
    /// when it fails, or a `TaskCanceledError` when it is canceled
    /// (`Create<TResult>(Func<IObserver<TResult>, Task>)`).
    pub fn create_async<T: 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        subscribe_async: impl Fn(Rc<dyn IObserver<T>>) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        create_with_task_token(move |observer, _| subscribe_async(observer))
    }

    /// [`create_async`](Self::create_async) with a token that disposing the subscription cancels
    /// (`Create<TResult>(Func<IObserver<TResult>, CancellationToken, Task>)`).
    pub fn create_async_with_token<T: 'static, Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        subscribe_async: impl Fn(Rc<dyn IObserver<T>>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        create_with_task_token(subscribe_async)
    }

    /// A sequence whose subscription is an asynchronous function whose task gives the disposable
    /// that ends the subscription; disposing the subscription before the task ends disposes the
    /// disposable as soon as it arrives. An error of the task is the error of the sequence; its end
    /// does not end the sequence (`Create<TResult>(Func<IObserver<TResult>, Task<IDisposable>>)`).
    pub fn create_async_with_disposable<T: 'static, Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static>(
        subscribe_async: impl Fn(Rc<dyn IObserver<T>>) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        create_with_task_disposable(move |observer, _| subscribe_async(observer))
    }

    /// [`create_async_with_disposable`](Self::create_async_with_disposable) with a token that
    /// disposing the subscription cancels
    /// (`Create<TResult>(Func<IObserver<TResult>, CancellationToken, Task<IDisposable>>)`).
    pub fn create_async_with_disposable_with_token<T: 'static, Fut: Future<Output = Result<Rc<dyn IDisposable>, ObservableError>> + 'static>(
        subscribe_async: impl Fn(Rc<dyn IObserver<T>>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        create_with_task_disposable(subscribe_async)
    }

    /// A sequence whose subscription is an asynchronous function whose task gives the action that
    /// ends the subscription; disposing the subscription before the task ends runs the action as
    /// soon as it arrives. An error of the task is the error of the sequence; its end does not end
    /// the sequence (`Create<TResult>(Func<IObserver<TResult>, Task<Action>>)`).
    pub fn create_async_with_action<T: 'static, A: FnOnce() + 'static, Fut: Future<Output = Result<A, ObservableError>> + 'static>(
        subscribe_async: impl Fn(Rc<dyn IObserver<T>>) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        create_with_task_action(move |observer, _| subscribe_async(observer))
    }

    /// [`create_async_with_action`](Self::create_async_with_action) with a token that disposing
    /// the subscription cancels (`Create<TResult>(Func<IObserver<TResult>, CancellationToken,
    /// Task<Action>>)`).
    pub fn create_async_with_action_with_token<T: 'static, A: FnOnce() + 'static, Fut: Future<Output = Result<A, ObservableError>> + 'static>(
        subscribe_async: impl Fn(Rc<dyn IObserver<T>>, CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        create_with_task_action(subscribe_async)
    }
}
