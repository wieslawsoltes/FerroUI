//! `Catch` (`Linq/Observable/Catch.cs`), and the sink of the operators over a sequence of sources
//! that it shares with `OnErrorResumeNext`, `Retry` and `Repeat` (`Internal/TailRecursiveSink.cs`,
//! `Internal/ConcatSink.cs`).

use crate::disposables::SerialDisposableValue;
use crate::internal::{subscribe_sink, Sink};
use crate::linq::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The enumerator of a sequence of sources (`IEnumerator<IObservable<TSource>>`): an item that is
/// an error is a `MoveNext` that throws. Dropping it is `Dispose`.
pub(crate) type SourceIterator<T> = Box<dyn Iterator<Item = Result<Rc<dyn IObservable<T>>, ObservableError>>>;

/// A sequence of sources (`IEnumerable<IObservable<TSource>>`): each subscription gets an
/// enumerator of its own; an error is a `GetEnumerator` that throws.
pub(crate) type Sources<T> = Rc<dyn Fn() -> Result<SourceIterator<T>, ObservableError>>;

/// The sources of a collection that can be enumerated again for each subscription.
pub(crate) fn sources_of<T: 'static, I>(sources: I) -> Sources<T>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    I::IntoIter: 'static,
{
    Rc::new(move || Ok::<_, ObservableError>(Box::new(sources.clone().into_iter().map(Ok::<_, ObservableError>)) as SourceIterator<T>))
}

/// The sources of a function that enumerates them and may fail, as may each step of the
/// enumeration.
pub(crate) fn try_sources_of<T: 'static, I>(sources: impl Fn() -> Result<I, ObservableError> + 'static) -> Sources<T>
where
    I: IntoIterator<Item = Result<Rc<dyn IObservable<T>>, ObservableError>>,
    I::IntoIter: 'static,
{
    Rc::new(move || sources().map(|sources| Box::new(sources.into_iter()) as SourceIterator<T>))
}

/// What the sink does with the notifications of a source, and at the end of the sources (the
/// overrides of the subclasses of `TailRecursiveSink<TSource>`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TailRecursiveMode {
    /// `Catch<TSource>._`: an error moves on to the next source; the end of the sources ends
    /// with the last error, else completes; the completion of a source completes.
    Catch,
    /// `OnErrorResumeNext<TSource>._`: an error or the completion of a source moves on to the next
    /// source; the end of the sources completes.
    OnErrorResumeNext,
    /// `ConcatSink<TSource>`: the completion of a source moves on to the next source; an error
    /// ends with the error; the end of the sources completes.
    Concat,
}

/// The subscription to the current source (`_currentSubscription`): `Empty` is `null`, `Ready` is
/// `ReadyToken.Ready`, `Disposed` is `BooleanDisposable.True`.
enum CurrentSubscription {
    Empty,
    Ready,
    Set(Rc<dyn IDisposable>),
    Disposed,
}

/// The sink that subscribes to one source after the other, without growing the stack
/// (`TailRecursiveSink<TSource>`).
///
/// Deviation (DEVIATIONS.md, Reactive): upstream flattens a source that is itself an operator of
/// the same kind over a sequence of sources (`Extract`) and evaluates deferred sources (`Unpack`
/// of `IEvaluatableObservable<T>`) before it subscribes. The observables of the port cannot be
/// inspected for their type, so each source is subscribed as it is; the notifications and their
/// order are the same, only a nesting of such operators uses the stack of each level.
pub(crate) struct TailRecursiveSink<T> {
    sink: Sink<T>,
    mode: TailRecursiveMode,
    stack: RefCell<Vec<SourceIterator<T>>>,
    is_disposed: Cell<bool>,
    trampoline: Cell<usize>,
    current_subscription: RefCell<CurrentSubscription>,
    last_error: RefCell<Option<ObservableError>>,
    this: Weak<Self>,
}

impl<T: 'static> TailRecursiveSink<T> {
    fn new(observer: Rc<dyn IObserver<T>>, mode: TailRecursiveMode) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            sink: Sink::new(observer),
            mode,
            stack: RefCell::new(Vec::new()),
            is_disposed: Cell::new(false),
            trampoline: Cell::new(0),
            current_subscription: RefCell::new(CurrentSubscription::Empty),
            last_error: RefCell::new(None),
            this: this.clone(),
        })
    }

    /// Enumerates the sources and subscribes to the first one (`Run`).
    fn run(&self, sources: &Sources<T>) {
        match sources() {
            Ok(enumerator) => {
                self.stack.borrow_mut().push(enumerator);
                self.drain();
            }
            Err(error) => self.forward_on_error(error),
        }
    }

    fn forward_on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn forward_on_error(&self, error: ObservableError) {
        let observer = self.sink.observer();
        observer.on_error(error);
        self.dispose_sink();
    }

    fn forward_on_completed(&self) {
        let observer = self.sink.observer();
        observer.on_completed();
        self.dispose_sink();
    }

    /// `Dispose`: the observer is detached, then the enumerators and the current subscription
    /// are disposed (`Dispose(bool)`, `DisposeAll`).
    fn dispose_sink(&self) {
        if self.sink.dispose() {
            self.dispose_all();
        }
    }

    fn dispose_all(&self) {
        self.is_disposed.set(true);
        // The disposal of the current subscription is left to the drain loop, as upstream.
        self.drain();
    }

    fn drain(&self) {
        let trampoline = self.trampoline.get() + 1;
        self.trampoline.set(trampoline);
        if trampoline != 1 {
            return;
        }

        loop {
            if self.is_disposed.get() {
                loop {
                    let enumerator = self.stack.borrow_mut().pop();
                    match enumerator {
                        Some(enumerator) => drop(enumerator),
                        None => break,
                    }
                }
                let previous = std::mem::replace(&mut *self.current_subscription.borrow_mut(), CurrentSubscription::Disposed);
                if let CurrentSubscription::Set(subscription) = previous {
                    subscription.dispose();
                }
            } else {
                let top = self.stack.borrow_mut().pop();
                if let Some(mut enumerator) = top {
                    match enumerator.next() {
                        Some(Err(error)) => {
                            drop(enumerator);
                            self.forward_on_error(error);
                            self.is_disposed.set(true);
                            continue;
                        }
                        None => {
                            drop(enumerator);
                            continue;
                        }
                        Some(Ok(next)) => {
                            self.stack.borrow_mut().push(enumerator);

                            // Swap in the ready indicator, so that a disposal or a completion
                            // while the source is subscribed is seen.
                            let ready = {
                                let mut current = self.current_subscription.borrow_mut();
                                match *current {
                                    CurrentSubscription::Empty => {
                                        *current = CurrentSubscription::Ready;
                                        true
                                    }
                                    _ => false,
                                }
                            };
                            if !ready {
                                continue;
                            }

                            let Some(this) = self.this.upgrade() else {
                                return;
                            };
                            let observer: Rc<dyn IObserver<T>> = this;
                            let subscription = next.subscribe(observer);

                            // Swap the subscription in place of the ready indicator; anything else
                            // there means the source ended or the sink was disposed meanwhile.
                            let swapped = {
                                let mut current = self.current_subscription.borrow_mut();
                                match *current {
                                    CurrentSubscription::Ready => {
                                        *current = CurrentSubscription::Set(subscription.clone());
                                        None
                                    }
                                    CurrentSubscription::Disposed => Some(true),
                                    _ => Some(false),
                                }
                            };
                            if let Some(disposed) = swapped {
                                subscription.dispose();
                                if disposed {
                                    continue;
                                }
                            }
                        }
                    }
                } else {
                    self.is_disposed.set(true);
                    self.done();
                }
            }

            let trampoline = self.trampoline.get() - 1;
            self.trampoline.set(trampoline);
            if trampoline == 0 {
                break;
            }
        }
    }

    /// Disposes the subscription to the current source and moves on to the next one (`Recurse`).
    fn recurse(&self) {
        let previous = {
            let mut current = self.current_subscription.borrow_mut();
            if matches!(*current, CurrentSubscription::Disposed) {
                return;
            }
            std::mem::replace(&mut *current, CurrentSubscription::Empty)
        };
        if let CurrentSubscription::Set(subscription) = previous {
            subscription.dispose();
        }
        self.drain();
    }

    /// The end of the sources (`Done`).
    fn done(&self) {
        let last_error = match self.mode {
            TailRecursiveMode::Catch => self.last_error.borrow().clone(),
            TailRecursiveMode::OnErrorResumeNext | TailRecursiveMode::Concat => None,
        };
        match last_error {
            Some(error) => self.forward_on_error(error),
            None => self.forward_on_completed(),
        }
    }
}

impl<T: 'static> IObserver<T> for TailRecursiveSink<T> {
    fn on_next(&self, value: T) {
        self.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        match self.mode {
            TailRecursiveMode::Catch => {
                *self.last_error.borrow_mut() = Some(error);
                self.recurse();
            }
            TailRecursiveMode::OnErrorResumeNext => self.recurse(),
            TailRecursiveMode::Concat => self.forward_on_error(error),
        }
    }

    fn on_completed(&self) {
        match self.mode {
            TailRecursiveMode::Catch => self.forward_on_completed(),
            TailRecursiveMode::OnErrorResumeNext | TailRecursiveMode::Concat => self.recurse(),
        }
    }
}

impl<T: 'static> IDisposable for TailRecursiveSink<T> {
    fn dispose(&self) {
        self.dispose_sink();
    }
}

/// An operator over a sequence of sources (`Catch<TSource>`, `OnErrorResumeNext<TSource>`,
/// `Concat<TSource>`).
struct TailRecursiveObservable<T> {
    sources: Sources<T>,
    mode: TailRecursiveMode,
}

impl<T: 'static> IObservable<T> for TailRecursiveObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        let mode = self.mode;
        subscribe_sink(observer, move |observer| TailRecursiveSink::new(observer, mode), move |sink: &Rc<TailRecursiveSink<T>>| sink.run(&sources))
    }
}

/// The observable of an operator over the sources, of the kind `mode`.
pub(crate) fn tail_recursive<T: 'static>(sources: Sources<T>, mode: TailRecursiveMode) -> Rc<dyn IObservable<T>> {
    Rc::new(TailRecursiveObservable { sources, mode })
}

/// Continues with the next source when the current one fails (`Catch(IEnumerable<IObservable<TSource>>)`).
pub(crate) fn catch_sources<T: 'static>(sources: Sources<T>) -> Rc<dyn IObservable<T>> {
    tail_recursive(sources, TailRecursiveMode::Catch)
}

type Filter = Rc<dyn Fn(&ObservableError) -> bool>;
type Handler<T> = Rc<dyn Fn(ObservableError) -> Result<Rc<dyn IObservable<T>>, ObservableError>>;

/// `Catch` with a handler of the errors of a type (`Catch<TSource, TException>`): `filter` is the
/// test of the type of the error (`error is TException`).
struct CatchHandler<T> {
    source: Rc<dyn IObservable<T>>,
    filter: Filter,
    handler: Handler<T>,
}

impl<T: 'static> IObservable<T> for CatchHandler<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let (filter, handler) = (self.filter.clone(), self.handler.clone());
        subscribe_sink(
            observer,
            move |observer| CatchSink::new(observer, filter, handler),
            move |sink: &Rc<CatchSink<T>>| {
                let observer: Rc<dyn IObserver<T>> = sink.clone();
                sink.subscription.try_set_first(source.subscribe(observer));
            },
        )
    }
}

/// The sink of `Catch` with a handler (`Catch<TSource, TException>._`): the first error the filter
/// accepts is handed to the handler, whose sequence continues the source; a handler that fails
/// ends the sequence with its error.
struct CatchSink<T> {
    sink: Sink<T>,
    filter: Filter,
    handler: Handler<T>,
    once: Cell<bool>,
    subscription: SerialDisposableValue,
    this: Weak<Self>,
}

impl<T: 'static> CatchSink<T> {
    fn new(observer: Rc<dyn IObserver<T>>, filter: Filter, handler: Handler<T>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            sink: Sink::new(observer),
            filter,
            handler,
            once: Cell::new(false),
            subscription: SerialDisposableValue::new(),
            this: this.clone(),
        })
    }

    fn forward_on_error(&self, error: ObservableError) {
        let observer = self.sink.observer();
        observer.on_error(error);
        self.dispose();
    }

    fn forward_on_completed(&self) {
        let observer = self.sink.observer();
        observer.on_completed();
        self.dispose();
    }
}

impl<T: 'static> IObserver<T> for CatchSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        if !self.once.get() && (self.filter)(&error) {
            let result = match (self.handler)(error) {
                Ok(result) => result,
                Err(error) => {
                    self.forward_on_error(error);
                    return;
                }
            };
            self.once.set(true);
            let Some(this) = self.this.upgrade() else {
                return;
            };
            let observer: Rc<dyn IObserver<T>> = this;
            self.subscription.set_disposable(Some(result.subscribe(observer)));
        } else {
            self.forward_on_error(error);
        }
    }

    fn on_completed(&self) {
        self.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for CatchSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.subscription.dispose();
        }
    }
}

pub(crate) fn catch_handler<T: 'static>(source: Rc<dyn IObservable<T>>, filter: Filter, handler: Handler<T>) -> Rc<dyn IObservable<T>> {
    Rc::new(CatchHandler { source, filter, handler })
}

impl Observable {
    /// Continues with the next of `sources` when one fails; the last error ends the sequence when
    /// all of them fail (`Catch(IEnumerable<IObservable<TSource>>)`, `Catch(params
    /// IObservable<TSource>[])`). The sources are enumerated again for each subscription.
    pub fn catch_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        catch_sources(sources_of(sources))
    }

    /// [`catch_all`](Self::catch_all) over sources whose enumeration may fail: `sources` starts an
    /// enumeration (`GetEnumerator`, which fails with an `Err`), and each item is the next source
    /// or the error of the step (`MoveNext` that throws). A failure ends the sequence with its
    /// error.
    pub fn try_catch_all<T: 'static, I>(sources: impl Fn() -> Result<I, ObservableError> + 'static) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Result<Rc<dyn IObservable<T>>, ObservableError>>,
        I::IntoIter: 'static,
    {
        catch_sources(try_sources_of(sources))
    }
}

/// The methods of `Catch` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! catch_methods {
    () => {
        /// Continues with the sequence of `handler` when the source fails
        /// (`Catch<TSource, Exception>(Func<Exception, IObservable<TSource>>)`).
        fn catch(
            &self,
            handler: impl Fn(ferroui_system::ObservableError) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::catch::catch_handler(self.observable(), std::rc::Rc::new(|_: &ferroui_system::ObservableError| true), std::rc::Rc::new(move |error| Ok(handler(error))))
        }

        /// [`catch`](Self::catch) with a handler that may fail; a failure ends the sequence with its
        /// error (`Catch` with a handler that throws).
        fn try_catch(
            &self,
            handler: impl Fn(
                    ferroui_system::ObservableError,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<T>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::catch::catch_handler(self.observable(), std::rc::Rc::new(|_: &ferroui_system::ObservableError| true), std::rc::Rc::new(handler))
        }

        /// Continues with the sequence of `handler` when the source fails with an error of the type
        /// `E`; other errors end the sequence (`Catch<TSource, TException>(Func<TException,
        /// IObservable<TSource>>)`). The handler receives the error, which `downcast_ref::<E>` reads.
        fn catch_of<E: std::error::Error + 'static>(
            &self,
            handler: impl Fn(ferroui_system::ObservableError) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::catch::catch_handler(
                self.observable(),
                std::rc::Rc::new(|error: &ferroui_system::ObservableError| error.downcast_ref::<E>().is_some()),
                std::rc::Rc::new(move |error| Ok(handler(error))),
            )
        }

        /// [`catch_of`](Self::catch_of) with a handler that may fail; a failure ends the sequence
        /// with its error.
        fn try_catch_of<E: std::error::Error + 'static>(
            &self,
            handler: impl Fn(
                    ferroui_system::ObservableError,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<T>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::catch::catch_handler(
                self.observable(),
                std::rc::Rc::new(|error: &ferroui_system::ObservableError| error.downcast_ref::<E>().is_some()),
                std::rc::Rc::new(handler),
            )
        }

        /// Continues with `second` when the source fails (`Catch(IObservable<TSource>,
        /// IObservable<TSource>)`).
        fn catch_with(&self, second: std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::Observable::catch_all(vec![self.observable(), second])
        }
    };
}
pub(crate) use catch_methods;
