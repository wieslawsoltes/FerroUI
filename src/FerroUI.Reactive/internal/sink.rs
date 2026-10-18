//! `Sink<TTarget>`, `Producer` and the safeguards of a subscription (`Internal/Sink.cs`,
//! `Internal/Producer.cs`, `Internal/SafeObserver.cs`, `Internal/AutoDetachObserver.cs`,
//! `Internal/NopObserver.cs`).

use crate::concurrency::{CurrentThreadScheduler, SchedulerExt};
use crate::disposables::SingleAssignmentDisposableValue;
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An observer that ignores every notification (`NopObserver<T>`).
pub(crate) struct NopObserver;

impl<T> IObserver<T> for NopObserver {
    fn on_next(&self, _value: T) {}
    fn on_error(&self, _error: ObservableError) {}
    fn on_completed(&self) {}
}

pub(crate) fn nop_observer<T: 'static>() -> Rc<dyn IObserver<T>> {
    Rc::new(NopObserver)
}

/// The state every operator's sink is built on (`Sink<TTarget>`): the observer downstream, which is
/// replaced by one that ignores everything when the sink is disposed, and the subscription
/// upstream.
///
/// A notification is forwarded with a clone of the observer handle, so that the observer may
/// dispose the sink while it handles the notification.
pub(crate) struct Sink<T> {
    observer: RefCell<Rc<dyn IObserver<T>>>,
    upstream: SingleAssignmentDisposableValue,
    disposed: Cell<bool>,
}

impl<T: 'static> Sink<T> {
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>) -> Self {
        Self { observer: RefCell::new(observer), upstream: SingleAssignmentDisposableValue::new(), disposed: Cell::new(false) }
    }

    /// The observer downstream.
    pub(crate) fn observer(&self) -> Rc<dyn IObserver<T>> {
        self.observer.borrow().clone()
    }

    pub(crate) fn forward_on_next(&self, value: T) {
        let observer = self.observer();
        observer.on_next(value);
    }

    /// Forwards the completion and disposes the sink (`ForwardOnCompleted`).
    pub(crate) fn forward_on_completed(&self) {
        let observer = self.observer();
        observer.on_completed();
        self.dispose();
    }

    /// Forwards the error and disposes the sink (`ForwardOnError`).
    pub(crate) fn forward_on_error(&self, error: ObservableError) {
        let observer = self.observer();
        observer.on_error(error);
        self.dispose();
    }

    pub(crate) fn set_upstream(&self, upstream: Rc<dyn IDisposable>) {
        self.upstream.set_disposable(Some(upstream));
    }

    pub(crate) fn dispose_upstream(&self) {
        self.upstream.dispose();
    }

    pub(crate) fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// Disposes the sink: the observer is replaced by one that ignores everything and the
    /// subscription upstream is disposed. True the first time.
    pub(crate) fn dispose(&self) -> bool {
        if self.disposed.replace(true) {
            return false;
        }
        *self.observer.borrow_mut() = nop_observer();
        self.upstream.dispose();
        true
    }
}

/// Wraps an observer for a subscription (`SafeObserver<T>.Wrap`, the wrapping form): a terminal
/// notification, or a panic while the observer handles a value, disposes the subscription.
pub(crate) struct SafeObserver<T> {
    observer: Rc<dyn IObserver<T>>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static> SafeObserver<T> {
    pub(crate) fn wrap(observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        Rc::new(Self { observer, resource: SingleAssignmentDisposableValue::new() })
    }

    pub(crate) fn set_resource(&self, resource: Rc<dyn IDisposable>) {
        self.resource.set_disposable(Some(resource));
    }
}

/// Disposes a resource when it is dropped, or only while a panic unwinds.
struct DisposeGuard<'a> {
    resource: &'a SingleAssignmentDisposableValue,
    only_on_panic: bool,
}

impl Drop for DisposeGuard<'_> {
    fn drop(&mut self) {
        if !self.only_on_panic || std::thread::panicking() {
            self.resource.dispose();
        }
    }
}

impl<T: 'static> IObserver<T> for SafeObserver<T> {
    fn on_next(&self, value: T) {
        let _guard = DisposeGuard { resource: &self.resource, only_on_panic: true };
        self.observer.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        let _guard = DisposeGuard { resource: &self.resource, only_on_panic: false };
        self.observer.on_error(error);
    }

    fn on_completed(&self) {
        let _guard = DisposeGuard { resource: &self.resource, only_on_panic: false };
        self.observer.on_completed();
    }
}

impl<T> IDisposable for SafeObserver<T> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// Subscribes through the sink of an operator (`Producer<TTarget, TSink>.SubscribeRaw`): wraps the
/// observer in a [`SafeObserver`], creates the sink for it, and runs the sink, through the
/// trampoline of the current thread when no work of it is running yet (so that work the
/// subscription schedules on it runs after the subscription has been set up). Returns the sink.
pub(crate) fn subscribe_sink<T: 'static, S: IDisposable + 'static>(
    observer: Rc<dyn IObserver<T>>,
    create_sink: impl FnOnce(Rc<dyn IObserver<T>>) -> Rc<S>,
    run: impl FnOnce(&Rc<S>) + 'static,
) -> Rc<dyn IDisposable> {
    let safe = SafeObserver::wrap(observer);
    let sink = create_sink(safe.clone());
    safe.set_resource(sink.clone());
    if CurrentThreadScheduler::is_schedule_required() {
        let running = sink.clone();
        CurrentThreadScheduler::instance().schedule_action(move || run(&running));
    } else {
        run(&sink);
    }
    sink
}

/// Subscribes through a function that returns the subscription (`BasicProducer<T>.SubscribeRaw`),
/// with the same safeguards as [`subscribe_sink`].
pub(crate) fn subscribe_basic<T: 'static>(
    observer: Rc<dyn IObserver<T>>,
    run: impl FnOnce(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static,
) -> Rc<dyn IDisposable> {
    let safe = SafeObserver::wrap(observer);
    let run_assignable = Rc::new(crate::disposables::SingleAssignmentDisposable::default());
    let result: Rc<dyn IDisposable> = if CurrentThreadScheduler::is_schedule_required() {
        let slot = run_assignable.clone();
        let target: Rc<dyn IObserver<T>> = safe.clone();
        CurrentThreadScheduler::instance().schedule_action(move || slot.set_disposable(Some(run(target))));
        run_assignable
    } else {
        run(safe.clone())
    };
    safe.set_resource(result.clone());
    result
}

/// Wraps an observer for a subscription made by user code (`AutoDetachObserver<T>`): it ignores
/// everything after a terminal notification, and a terminal notification, or a panic while the
/// observer handles a value, disposes the subscription.
pub(crate) struct AutoDetachObserver<T> {
    observer: Rc<dyn IObserver<T>>,
    resource: SingleAssignmentDisposableValue,
    is_stopped: Cell<bool>,
}

impl<T: 'static> AutoDetachObserver<T> {
    pub(crate) fn new(observer: Rc<dyn IObserver<T>>) -> Rc<Self> {
        Rc::new(Self { observer, resource: SingleAssignmentDisposableValue::new(), is_stopped: Cell::new(false) })
    }

    pub(crate) fn set_resource(&self, resource: Rc<dyn IDisposable>) {
        self.resource.set_disposable(Some(resource));
    }
}

impl<T: 'static> IObserver<T> for AutoDetachObserver<T> {
    fn on_next(&self, value: T) {
        if self.is_stopped.get() {
            return;
        }
        let _guard = DisposeGuard { resource: &self.resource, only_on_panic: true };
        self.observer.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        if self.is_stopped.replace(true) {
            return;
        }
        let _guard = DisposeGuard { resource: &self.resource, only_on_panic: false };
        self.observer.on_error(error);
    }

    fn on_completed(&self) {
        if self.is_stopped.replace(true) {
            return;
        }
        let _guard = DisposeGuard { resource: &self.resource, only_on_panic: false };
        self.observer.on_completed();
    }
}

impl<T> IDisposable for AutoDetachObserver<T> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// Subscribes through user code (`ObservableBase<T>.Subscribe`): the observer is wrapped in an
/// [`AutoDetachObserver`], and the subscription runs through the trampoline of the current thread
/// when no work of it is running yet.
pub(crate) fn subscribe_auto_detach<T: 'static>(
    observer: Rc<dyn IObserver<T>>,
    subscribe_core: impl FnOnce(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static,
) -> Rc<dyn IDisposable> {
    let auto_detach = AutoDetachObserver::new(observer);
    if CurrentThreadScheduler::is_schedule_required() {
        let target = auto_detach.clone();
        CurrentThreadScheduler::instance().schedule_action(move || {
            let resource = subscribe_core(target.clone());
            target.set_resource(resource);
        });
    } else {
        let resource = subscribe_core(auto_detach.clone());
        auto_detach.set_resource(resource);
    }
    auto_detach
}
