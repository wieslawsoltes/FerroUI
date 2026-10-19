//! `ObservableMixins` (`ReactiveUI.Shared/Mixins/ObservableMixins.cs`): `WhereNotNull`, and the
//! asynchronous executions with cancellation that the commands made from tasks run.

use crate::PanicException;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::threading::tasks::Task;
use ferroui_system::threading::{CancellationToken, CancellationTokenRegistration, CancellationTokenSource};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// The extension methods of `ObservableMixins` on an observable of optional values.
pub trait ObservableMixins<T> {
    /// The values that are not `None` (`WhereNotNull()`); the error and the end pass through.
    fn where_not_null(&self) -> Rc<dyn IObservable<T>>;
}

impl<T: 'static> ObservableMixins<T> for Rc<dyn IObservable<Option<T>>> {
    fn where_not_null(&self) -> Rc<dyn IObservable<T>> {
        Rc::new(WhereNotNullObservable { source: self.clone() })
    }
}

/// An asynchronous execution as the commands made from tasks run it: its results, and the action
/// that cancels it (`(IObservable<TResult> Result, Action Cancel)`).
pub type AsyncExecution<T> = (Rc<dyn IObservable<T>>, Rc<dyn Fn()>);

/// A boxed future of the outcome of an asynchronous execution.
type BoxedOutcome<T> = Pin<Box<dyn Future<Output = Result<T, ObservableError>>>>;

/// For each subscription, a new execution of `action_async`: the observable that runs it with a
/// token, and the action that cancels that token (`FromAsyncWithAllNotifications(Func<CancellationToken,
/// Task<TResult>>)`, and `FromAsyncWithAllNotifications(Func<CancellationToken, Task>)` for
/// `T = ()`).
///
/// Upstream it is internal (the factories of `ReactiveCommand` use it), and its tests reach it
/// through `InternalsVisibleTo`; here it is public and hidden from the documentation.
#[doc(hidden)]
pub fn from_async_with_all_notifications<T: 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
    action_async: impl Fn(CancellationToken) -> Fut + 'static,
) -> Rc<dyn IObservable<AsyncExecution<T>>> {
    let action_async: Rc<dyn Fn(CancellationToken) -> BoxedOutcome<T>> = Rc::new(move |token| Box::pin(action_async(token)));
    Rc::new(DeferredValueObservable::new(move || {
        let cts = CancellationTokenSource::new();
        let result: Rc<dyn IObservable<T>> = Rc::new(FromAsyncObservable { factory: action_async.clone(), outer_cancellation: cts.clone() });
        let cancel: Rc<dyn Fn()> = Rc::new(move || request_cancel(&cts));
        Ok((result, cancel))
    }))
}

/// [`from_async_with_all_notifications`] with a parameter handed to the action
/// (`FromAsyncWithAllNotifications<TParam, TResult>(Func<TParam, CancellationToken, Task<TResult>>, TParam)`,
/// and its `Task` form for `T = ()`).
#[doc(hidden)]
pub fn from_async_with_all_notifications_param<P: Clone + 'static, T: 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
    action_async: impl Fn(P, CancellationToken) -> Fut + 'static,
    param: P,
) -> Rc<dyn IObservable<AsyncExecution<T>>> {
    from_async_with_all_notifications(move |token| action_async(param.clone(), token))
}

/// `RequestCancel(CancellationTokenSource)`.
fn request_cancel(source: &CancellationTokenSource) {
    source.cancel();
}

/// `WhereNotNullObservable<T>`.
struct WhereNotNullObservable<T> {
    source: Rc<dyn IObservable<Option<T>>>,
}

impl<T: 'static> IObservable<T> for WhereNotNullObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(WhereNotNullSink { downstream: observer }))
    }
}

struct WhereNotNullSink<T> {
    downstream: Rc<dyn IObserver<T>>,
}

impl<T> IObserver<Option<T>> for WhereNotNullSink<T> {
    fn on_next(&self, value: Option<T>) {
        if let Some(value) = value {
            self.downstream.on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}

/// An observable that, for each subscription, notifies the value of a factory and ends, or the
/// error of the factory (`DeferredValueObservable<T>`).
///
/// Upstream catches the exception of the factory; here the factory returns a `Result`, whose
/// `Err` is notified, and a panic of the factory is caught and notified as a [`PanicException`].
#[doc(hidden)]
pub struct DeferredValueObservable<T> {
    factory: Box<dyn Fn() -> Result<T, ObservableError>>,
}

impl<T> DeferredValueObservable<T> {
    /// An observable of the value of `factory` (`new DeferredValueObservable<T>(Func<T>)`).
    pub fn new(factory: impl Fn() -> Result<T, ObservableError> + 'static) -> Self {
        Self { factory: Box::new(factory) }
    }
}

impl<T> IObservable<T> for DeferredValueObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        match PanicException::catch(|| (self.factory)()) {
            Ok(Ok(value)) => {
                observer.on_next(value);
                observer.on_completed();
            }
            Ok(Err(error)) | Err((error, _)) => observer.on_error(error),
        }
        Disposable::empty()
    }
}

/// `FromAsyncObservable<T>`: each subscription runs the factory, as a task of the thread, with a
/// token that the outer source and the disposal of the subscription cancel.
struct FromAsyncObservable<T> {
    factory: Rc<dyn Fn(CancellationToken) -> BoxedOutcome<T>>,
    outer_cancellation: CancellationTokenSource,
}

impl<T: 'static> IObservable<T> for FromAsyncObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let run = Rc::new(Run {
            observer,
            emitted: Cell::new(false),
            disposed: Cell::new(false),
            linked: CancellationTokenSource::new(),
            registration: RefCell::new(None),
        });
        run.clone().start(&self.factory, &self.outer_cancellation);
        run
    }
}

/// `FromAsyncObservable<T>.Run`.
///
/// Upstream links the outer source and a source of the subscription
/// (`CancellationTokenSource.CreateLinkedTokenSource`); here the linked source is cancelled by a
/// registration on the outer one and by the disposal of the subscription, which is the same.
struct Run<T> {
    observer: Rc<dyn IObserver<T>>,
    /// `_emitted`.
    emitted: Cell<bool>,
    /// `_disposed`.
    disposed: Cell<bool>,
    /// `_linked`: the source of the token the factory receives.
    linked: CancellationTokenSource,
    /// The registration that cancels the linked source when the outer one is.
    registration: RefCell<Option<CancellationTokenRegistration>>,
}

impl<T: 'static> Run<T> {
    /// `Start()`: runs the factory as a task (`RunAsync`).
    fn start(self: Rc<Self>, factory: &Rc<dyn Fn(CancellationToken) -> BoxedOutcome<T>>, outer: &CancellationTokenSource) {
        let linked = self.linked.clone();
        let registration = outer.token().register(move || linked.cancel());
        *self.registration.borrow_mut() = Some(registration);
        let outcome = factory(self.linked.token());
        let run = self;
        let _ = Task::spawn_local(async move {
            let outcome = outcome.await;
            if !run.emitted.replace(true) {
                match outcome {
                    Ok(value) => {
                        run.observer.on_next(value);
                        run.observer.on_completed();
                    }
                    Err(error) => run.observer.on_error(error),
                }
            }
            run.dispose();
            Ok::<(), ObservableError>(())
        });
    }
}

impl<T> IDisposable for Run<T> {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.linked.cancel();
        let registration = self.registration.borrow_mut().take();
        if let Some(registration) = registration {
            registration.dispose();
        }
    }
}
