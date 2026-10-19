//! `SwitchSubscribeMixins` (`ReactiveUI.Shared/Mixins/SwitchSubscribeMixins.cs`): subscribe to the
//! latest inner observable of an observable of observables (or of commands), switching when a new
//! one arrives and skipping the `None`s.

use crate::reactive_command::IReactiveCommandOf;
use crate::PanicException;
use ferroui_reactive::disposables::{SerialDisposable, SingleAssignmentDisposable};
use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// A value that is an observable of `T`: an observable, or a reactive command (whose results are
/// its values). It is the element of the sources [`SwitchSubscribeMixins::switch_subscribe`]
/// takes, which upstream overloads for `IObservable<IObservable<T>?>` and for
/// `IObservable<IReactiveCommand<TParam, TResult>?>`.
pub trait SwitchSource<T> {
    /// The value as an observable.
    fn as_switch_source(&self) -> Rc<dyn IObservable<T>>;
}

impl<T: 'static> SwitchSource<T> for Rc<dyn IObservable<T>> {
    fn as_switch_source(&self) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}

impl<P: 'static, R: 'static> SwitchSource<R> for Rc<dyn IReactiveCommandOf<P, R>> {
    fn as_switch_source(&self) -> Rc<dyn IObservable<R>> {
        let source: Rc<dyn IObservable<R>> = self.clone();
        source
    }
}

/// The extension methods of `SwitchSubscribeMixins` on an observable of optional values `S`.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream overloads `SwitchSubscribe` and `SwitchSelect`
/// on the element type and on the handlers; here the forms with a selector are
/// [`switch_select`](Self::switch_select) and `switch_subscribe_with*`, the forms on observables
/// and commands are [`switch_subscribe`](Self::switch_subscribe) and
/// [`switch_subscribe_all`](Self::switch_subscribe_all), and the forms with an error and an end
/// handler end in `_all`. The null checks of the arguments have no counterpart.
pub trait SwitchSubscribeMixins<S: 'static> {
    /// Projects each value that is not `None` to an observable and notifies the values of the
    /// latest one (`SwitchSelect<TResult>(Func<TSource, IObservable<TResult>>)`, and the form on
    /// commands).
    fn switch_select<U: 'static>(&self, selector: impl Fn(S) -> Rc<dyn IObservable<U>> + 'static) -> Rc<dyn IObservable<U>>;

    /// [`switch_select`](Self::switch_select) with a selector that may fail; a failure is the error
    /// of the sequence (the selector that throws upstream).
    fn try_switch_select<U: 'static>(
        &self,
        selector: impl Fn(S) -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<U>>;

    /// Subscribes `on_next` to [`switch_select`](Self::switch_select); an error is ignored
    /// (`SwitchSubscribe<TResult>(Func<TSource, IObservable<TResult>>, Action<TResult>)`).
    fn switch_subscribe_with<U: 'static>(
        &self,
        selector: impl Fn(S) -> Rc<dyn IObservable<U>> + 'static,
        on_next: impl Fn(U) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.switch_select(selector).subscribe(Observer::create_all(on_next, |_| {}, || {}))
    }

    /// Subscribes the three handlers to [`switch_select`](Self::switch_select)
    /// (`SwitchSubscribe<TResult>(Func<TSource, IObservable<TResult>>, Action<TResult>,
    /// Action<Exception>, Action)`).
    fn switch_subscribe_with_all<U: 'static>(
        &self,
        selector: impl Fn(S) -> Rc<dyn IObservable<U>> + 'static,
        on_next: impl Fn(U) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable> {
        self.switch_select(selector).subscribe(Observer::create_all(on_next, on_error, on_completed))
    }

    /// Subscribes `on_next` to the values of the latest observable (or command) the source
    /// notified; an error is ignored (`SwitchSubscribe(Action<T>)` on `IObservable<IObservable<T>?>`
    /// and on `IObservable<IReactiveCommand<TParam, TResult>?>`).
    fn switch_subscribe<T: 'static>(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable>
    where
        S: SwitchSource<T>,
    {
        self.switch_select(|source: S| source.as_switch_source()).subscribe(Observer::create_all(on_next, |_| {}, || {}))
    }

    /// [`switch_subscribe`](Self::switch_subscribe) with the error and end handlers
    /// (`SwitchSubscribe(Action<T>, Action<Exception>, Action)`).
    fn switch_subscribe_all<T: 'static>(
        &self,
        on_next: impl Fn(T) + 'static,
        on_error: impl Fn(ObservableError) + 'static,
        on_completed: impl Fn() + 'static,
    ) -> Rc<dyn IDisposable>
    where
        S: SwitchSource<T>,
    {
        self.switch_select(|source: S| source.as_switch_source()).subscribe(Observer::create_all(on_next, on_error, on_completed))
    }
}

impl<S: 'static> SwitchSubscribeMixins<S> for Rc<dyn IObservable<Option<S>>> {
    fn switch_select<U: 'static>(&self, selector: impl Fn(S) -> Rc<dyn IObservable<U>> + 'static) -> Rc<dyn IObservable<U>> {
        self.try_switch_select(move |value| Ok(selector(value)))
    }

    fn try_switch_select<U: 'static>(
        &self,
        selector: impl Fn(S) -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<U>> {
        Rc::new(SwitchSelectObservable { source: self.clone(), selector: Rc::new(selector) })
    }
}

type Selector<S, U> = Rc<dyn Fn(S) -> Result<Rc<dyn IObservable<U>>, ObservableError>>;

/// `SwitchSelectObservable<TSource, TResult>`.
struct SwitchSelectObservable<S, U> {
    source: Rc<dyn IObservable<Option<S>>>,
    selector: Selector<S, U>,
}

impl<S: 'static, U: 'static> IObservable<U> for SwitchSelectObservable<S, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<U>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new_cyclic(|this| SwitchSink {
            this: this.clone(),
            selector: self.selector.clone(),
            downstream: observer,
            outer: SingleAssignmentDisposable::new(),
            inner: SerialDisposable::new(),
            latest: Cell::new(0),
            has_inner: Cell::new(false),
            outer_completed: Cell::new(false),
            disposed: Cell::new(false),
        });
        // `Run(IObservable<TSource?>)`.
        let subscription = self.source.subscribe(sink.clone());
        sink.outer.set_disposable(Some(subscription));
        sink
    }
}

/// `SwitchSelectObservable<TSource, TResult>.Sink`.
struct SwitchSink<S, U> {
    this: Weak<SwitchSink<S, U>>,
    selector: Selector<S, U>,
    downstream: Rc<dyn IObserver<U>>,
    /// `_outer`.
    outer: Rc<SingleAssignmentDisposable>,
    /// `_inner`.
    inner: Rc<SerialDisposable>,
    /// `_latest`: the id of the latest inner subscription.
    latest: Cell<u64>,
    /// `_hasInner`.
    has_inner: Cell<bool>,
    /// `_outerCompleted`.
    outer_completed: Cell<bool>,
    /// `_disposed`.
    disposed: Cell<bool>,
}

impl<S: 'static, U: 'static> SwitchSink<S, U> {
    fn inner_on_next(&self, id: u64, value: U) {
        if self.disposed.get() || id != self.latest.get() {
            return;
        }
        self.downstream.on_next(value);
    }

    fn inner_on_error(&self, id: u64, error: ObservableError) {
        if self.disposed.get() || id != self.latest.get() {
            return;
        }
        self.downstream.on_error(error);
        self.dispose();
    }

    fn inner_on_completed(&self, id: u64) {
        if self.disposed.get() || id != self.latest.get() {
            return;
        }
        self.has_inner.set(false);
        if !self.outer_completed.get() {
            return;
        }
        self.downstream.on_completed();
        self.dispose();
    }
}

impl<S: 'static, U: 'static> IObserver<Option<S>> for SwitchSink<S, U> {
    fn on_next(&self, value: Option<S>) {
        let Some(value) = value else { return };
        // `try { inner = selector(value); } catch (Exception ex) { OnError(ex); }`.
        let inner = match PanicException::catch(|| (self.selector)(value)).unwrap_or_else(|(error, _)| Err(error)) {
            Ok(inner) => inner,
            Err(error) => {
                self.on_error(error);
                return;
            }
        };
        if self.disposed.get() {
            return;
        }
        let id = self.latest.get() + 1;
        self.latest.set(id);
        self.has_inner.set(true);
        let Some(parent) = self.this.upgrade() else { return };
        let subscription = inner.subscribe(Rc::new(InnerObserver { parent, id }));
        self.inner.set_disposable(Some(subscription));
    }

    fn on_error(&self, error: ObservableError) {
        if self.disposed.get() {
            return;
        }
        self.downstream.on_error(error);
        self.dispose();
    }

    fn on_completed(&self) {
        if self.disposed.get() {
            return;
        }
        self.outer_completed.set(true);
        if self.has_inner.get() {
            return;
        }
        self.downstream.on_completed();
        self.dispose();
    }
}

impl<S, U> IDisposable for SwitchSink<S, U> {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.outer.dispose();
        self.inner.dispose();
    }
}

/// `Sink.InnerObserver`.
struct InnerObserver<S, U> {
    parent: Rc<SwitchSink<S, U>>,
    id: u64,
}

impl<S: 'static, U: 'static> IObserver<U> for InnerObserver<S, U> {
    fn on_next(&self, value: U) {
        self.parent.inner_on_next(self.id, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.inner_on_error(self.id, error);
    }

    fn on_completed(&self) {
        self.parent.inner_on_completed(self.id);
    }
}
