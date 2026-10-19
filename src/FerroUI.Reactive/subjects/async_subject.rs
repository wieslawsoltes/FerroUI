//! `AsyncSubject<T>` (`AsyncSubject.cs`).

use super::subject::{throw_disposed, ObserverList, SubjectBase, SubjectSubscription};
use crate::disposables::Disposable;
use crate::linq::SequenceError;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::threading::{current_synchronization_context, SynchronizationContext};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// A subject that keeps the last value and hands it, with the end, to every observer once the
/// sequence has ended (`AsyncSubject<T>`).
pub struct AsyncSubject<T> {
    this: Weak<AsyncSubject<T>>,
    observers: ObserverList<T>,
    inner: RefCell<Inner<T>>,
}

#[derive(Clone)]
enum Phase {
    Active,
    Terminated,
    Disposed,
}

struct Inner<T> {
    phase: Phase,
    value: Option<T>,
    error: Option<ObservableError>,
}

impl<T: Clone + 'static> AsyncSubject<T> {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            observers: ObserverList::new(),
            inner: RefCell::new(Inner { phase: Phase::Active, value: None, error: None }),
        })
    }

    /// Whether the subject has ended (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        matches!(self.inner.borrow().phase, Phase::Terminated)
    }

    /// The last value of an ended subject (`GetResult`): the error when it ended with one, `None`
    /// when it ended without a value (upstream throws `InvalidOperationException`). Panics when the
    /// subject has not ended, where upstream blocks until it does (a wait on the only thread would
    /// never end).
    pub fn get_result(&self) -> Result<Option<T>, ObservableError> {
        let inner = self.inner.borrow();
        match inner.phase {
            Phase::Disposed => throw_disposed(),
            Phase::Active => panic!("the subject has not completed; on a single thread waiting for it would never end"),
            Phase::Terminated => match &inner.error {
                Some(error) => Err(error.clone()),
                None => Ok(inner.value.clone()),
            },
        }
    }

    /// The awaiter of the subject (`GetAwaiter`): a task that ends when the subject ends, with its
    /// last value, with its error (a `TaskCanceledError` cancels it), or with
    /// [`SequenceError::NoElements`] when it ended without a value (`GetResult`'s
    /// `InvalidOperationException`). `async` code awaits it.
    ///
    /// Upstream's subject is its own awaiter; its members are those of the task: `IsCompleted` is
    /// [`Task::is_completed`], `OnCompleted(Action)` is [`Task::continue_with`] and `GetResult` is
    /// [`Task::result`] once the task has ended.
    ///
    /// The awaiter captures the current `SynchronizationContext` of the thread, as upstream's
    /// `AwaitObserver` does: when there is one, the end of the subject is posted to it (with
    /// [`post_local`](SynchronizationContext::post_local)), and the task ends, running its
    /// continuations, when the context runs that work; without one, the task ends where the subject
    /// does.
    ///
    /// Deviation (DEVIATIONS.md, Reactive): upstream posts only the continuation of `OnCompleted`
    /// and its `IsCompleted` is true as soon as the subject has ended; the task of the port ends
    /// when the posted work runs, so its [`is_completed`](Task::is_completed) is false until then.
    /// The context is captured when the awaiter is made, where upstream captures it when the
    /// continuation is given (`OnCompleted`).
    ///
    /// # Panics
    /// When the subject is disposed (`ObjectDisposedException`).
    pub fn get_awaiter(&self) -> Task<T> {
        let completion = TaskCompletionSource::new();
        let task = completion.task();
        let context = current_synchronization_context();
        self.subscribe(Rc::new(AwaitObserver { subject: self.this.clone(), completion, context }));
        task
    }

    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }

    pub fn as_observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }
}

/// The observer of [`AsyncSubject::get_awaiter`] (`AwaitObserver`): the end of the subject ends the
/// task with the outcome of the subject (`GetResult`), on the context it captured
/// (`InvokeOnOriginalContext`).
struct AwaitObserver<T> {
    subject: Weak<AsyncSubject<T>>,
    completion: TaskCompletionSource<T>,
    context: Option<Arc<dyn SynchronizationContext>>,
}

impl<T: Clone + 'static> AwaitObserver<T> {
    fn complete(&self) {
        let Some(subject) = self.subject.upgrade() else { return };
        let outcome = subject.get_result();
        let completion = self.completion.clone();
        let end = move || match outcome {
            Ok(Some(value)) => {
                completion.try_set_result(value);
            }
            Ok(None) => {
                completion.try_set_exception(Rc::new(SequenceError::NoElements));
            }
            Err(error) => {
                completion.try_set_exception(error);
            }
        };
        match &self.context {
            Some(context) => context.post_local(Box::new(end)),
            None => end(),
        }
    }
}

impl<T: Clone + 'static> IObserver<T> for AwaitObserver<T> {
    fn on_next(&self, _value: T) {}

    fn on_error(&self, _error: ObservableError) {
        self.complete();
    }

    fn on_completed(&self) {
        self.complete();
    }
}

impl<T: Clone + 'static> IObserver<T> for AsyncSubject<T> {
    fn on_next(&self, value: T) {
        let mut inner = self.inner.borrow_mut();
        match inner.phase {
            Phase::Disposed => throw_disposed(),
            Phase::Terminated => {}
            Phase::Active => inner.value = Some(value),
        }
    }

    fn on_error(&self, error: ObservableError) {
        {
            let mut inner = self.inner.borrow_mut();
            match inner.phase {
                Phase::Disposed => throw_disposed(),
                Phase::Terminated => return,
                Phase::Active => {}
            }
            inner.phase = Phase::Terminated;
            inner.value = None;
            inner.error = Some(error.clone());
        }
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                observer.on_error(error.clone());
            }
        }
    }

    fn on_completed(&self) {
        let value = {
            let mut inner = self.inner.borrow_mut();
            match inner.phase {
                Phase::Disposed => throw_disposed(),
                Phase::Terminated => return,
                Phase::Active => {}
            }
            inner.phase = Phase::Terminated;
            inner.value.clone()
        };
        for subscription in self.observers.take().iter() {
            if let Some(observer) = subscription.observer() {
                if let Some(value) = &value {
                    observer.on_next(value.clone());
                }
                observer.on_completed();
            }
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for AsyncSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (phase, value, error) = {
            let inner = self.inner.borrow();
            (inner.phase.clone(), inner.value.clone(), inner.error.clone())
        };
        match phase {
            Phase::Disposed => throw_disposed(),
            Phase::Terminated => {
                match error {
                    Some(error) => observer.on_error(error),
                    None => {
                        if let Some(value) = value {
                            observer.on_next(value);
                        }
                        observer.on_completed();
                    }
                }
                Disposable::empty()
            }
            Phase::Active => {
                let subject = self.this.clone();
                let subscription = SubjectSubscription::new(observer, move |subscription| {
                    if let Some(subject) = subject.upgrade() {
                        subject.observers.remove(subscription);
                    }
                });
                self.observers.add(subscription.clone());
                subscription
            }
        }
    }
}

impl<T: Clone + 'static> IDisposable for AsyncSubject<T> {
    fn dispose(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.phase = Phase::Disposed;
            inner.value = None;
            inner.error = None;
        }
        self.observers.take();
    }
}

impl<T: Clone + 'static> SubjectBase<T> for AsyncSubject<T> {
    fn has_observers(&self) -> bool {
        self.observers.len() != 0
    }

    fn is_disposed(&self) -> bool {
        matches!(self.inner.borrow().phase, Phase::Disposed)
    }
}
