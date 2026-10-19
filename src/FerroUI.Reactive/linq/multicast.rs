//! `Multicast` (`Linq/Observable/Multicast.cs`), and [`ConnectableObservableExt`], the operators of
//! a connectable observable (`RefCount`, `AutoConnect`).
//!
//! `Multicast(subject)` is upstream's `ConnectableObservable<TSource, TResult>` (the crate's
//! [`ConnectableObservable`]); `Multicast(subjectSelector, selector)` is the operator below, which
//! creates a subject and a connectable observable for each subscription.

use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{subscribe_sink, Sink};
use crate::subjects::{ConnectableObservable, IConnectableObservable, ISubjectHandle};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

type SubjectSelector<T, I> = Rc<dyn Fn() -> Result<Rc<dyn ISubjectHandle<T, I>>, ObservableError>>;
type Selector<I, R> = Rc<dyn Fn(Rc<dyn IObservable<I>>) -> Result<Rc<dyn IObservable<R>>, ObservableError>>;

/// `Multicast<TSource, TIntermediate, TResult>`: on each subscription, a subject from the subject
/// selector shares the source with the sequence the selector makes of it, and is connected once
/// the observer is subscribed to that sequence.
struct MulticastObservable<T, I, R> {
    source: Rc<dyn IObservable<T>>,
    subject_selector: SubjectSelector<T, I>,
    selector: Selector<I, R>,
}

/// The sink of `Multicast` (`Multicast<TSource, TIntermediate, TResult>._`, an `IdentitySink`):
/// it forwards the sequence of the selector and holds the connection of the source.
struct MulticastSink<R> {
    sink: Sink<R>,
    connection: SingleAssignmentDisposableValue,
}

impl<R: 'static> IObserver<R> for MulticastSink<R> {
    fn on_next(&self, value: R) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

impl<R: 'static> IDisposable for MulticastSink<R> {
    /// Disconnects the source, then disposes the sink (`Dispose(bool)`).
    fn dispose(&self) {
        self.connection.dispose();
        self.sink.dispose();
    }
}

impl<T: 'static, I: 'static, R: 'static> IObservable<R> for MulticastObservable<T, I, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let (source, subject_selector, selector) = (self.source.clone(), self.subject_selector.clone(), self.selector.clone());
        subscribe_sink(
            observer,
            |observer| Rc::new(MulticastSink { sink: Sink::new(observer), connection: SingleAssignmentDisposableValue::new() }),
            move |sink: &Rc<MulticastSink<R>>| {
                let subject = match subject_selector() {
                    Ok(subject) => subject,
                    Err(error) => {
                        sink.sink.forward_on_error(error);
                        return;
                    }
                };
                let connectable: Rc<dyn IConnectableObservable<I>> = ConnectableObservable::new(source, subject);
                let shared: Rc<dyn IObservable<I>> = connectable.clone();
                let observable = match selector(shared) {
                    Ok(observable) => observable,
                    Err(error) => {
                        sink.sink.forward_on_error(error);
                        return;
                    }
                };
                let upstream = observable.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
                sink.connection.set_disposable(Some(connectable.connect()));
            },
        )
    }
}

/// `Multicast(IObservable<TSource>, ISubject<TSource, TResult>)`: the source shared through
/// `subject` once connected.
pub(crate) fn multicast<T: 'static, U: 'static>(
    source: Rc<dyn IObservable<T>>,
    subject: Rc<dyn ISubjectHandle<T, U>>,
) -> Rc<dyn IConnectableObservable<U>> {
    ConnectableObservable::new(source, subject)
}

/// `Multicast(IObservable<TSource>, Func<ISubject<TSource, TIntermediate>>, Func<IObservable<TIntermediate>, IObservable<TResult>>)`.
pub(crate) fn multicast_with<T: 'static, I: 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    subject_selector: SubjectSelector<T, I>,
    selector: Selector<I, R>,
) -> Rc<dyn IObservable<R>> {
    Rc::new(MulticastObservable { source, subject_selector, selector })
}

/// [`multicast_with`] with a factory of a concrete subject and a selector that cannot fail: the
/// form the bindings of `Publish`, `PublishLast` and `Replay` use.
pub(crate) fn multicast_with_factory<T: 'static, I: 'static, R: 'static, S: ISubjectHandle<T, I> + 'static>(
    source: Rc<dyn IObservable<T>>,
    factory: impl Fn() -> Rc<S> + 'static,
    selector: impl Fn(Rc<dyn IObservable<I>>) -> Rc<dyn IObservable<R>> + 'static,
) -> Rc<dyn IObservable<R>> {
    multicast_with(
        source,
        Rc::new(move || {
            let subject: Rc<dyn ISubjectHandle<T, I>> = factory();
            Ok(subject)
        }),
        Rc::new(move |observable| Ok(selector(observable))),
    )
}

/// The operators of a connectable observable, as methods of its handle (the extension methods of
/// upstream's `Observable` on `IConnectableObservable<TSource>`: `RefCount`, `AutoConnect`).
///
/// The one required method gives the connectable observable as a handle; every operator is a
/// provided method, declared in the module of its family (`ref_count`, `auto_connect`). The trait
/// is implemented for `Rc<dyn IConnectableObservable<T>>`, which is also a handle of
/// [`ObservableExt`](super::ObservableExt), so the other operators apply to it as well.
pub trait ConnectableObservableExt<T: 'static> {
    /// The connectable observable as the handle the operators take.
    fn connectable(&self) -> Rc<dyn IConnectableObservable<T>>;

    super::ref_count::ref_count_methods!();
    super::auto_connect::auto_connect_methods!();
    super::run_async::run_async_connectable_methods!();
}

impl<T: 'static> ConnectableObservableExt<T> for Rc<dyn IConnectableObservable<T>> {
    fn connectable(&self) -> Rc<dyn IConnectableObservable<T>> {
        self.clone()
    }
}

impl<T: 'static> super::ObservableExt<T> for Rc<dyn IConnectableObservable<T>> {
    fn observable(&self) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}

/// The methods of `Multicast` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! multicast_methods {
    () => {
        /// Shares the sequence through `subject`: observers subscribe to the subject, and connecting
        /// subscribes the subject to the sequence (`Multicast(ISubject<TSource, TResult>)`).
        fn multicast<U: 'static>(
            &self,
            subject: std::rc::Rc<dyn crate::subjects::ISubjectHandle<T, U>>,
        ) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<U>> {
            crate::linq::multicast::multicast(self.observable(), subject)
        }

        /// On each subscription, shares the sequence through a subject from `subject_selector`
        /// with the sequence `selector` makes of it, and connects it once the observer is
        /// subscribed (`Multicast(Func<ISubject<TSource, TIntermediate>>, Func<IObservable<TIntermediate>, IObservable<TResult>>)`).
        fn multicast_with<I: 'static, R: 'static>(
            &self,
            subject_selector: impl Fn() -> std::rc::Rc<dyn crate::subjects::ISubjectHandle<T, I>> + 'static,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<I>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::multicast::multicast_with(
                self.observable(),
                std::rc::Rc::new(move || Ok(subject_selector())),
                std::rc::Rc::new(move |observable| Ok(selector(observable))),
            )
        }

        /// [`multicast_with`](Self::multicast_with) with a subject selector and a selector that may
        /// fail; a failure ends the sequence with its error (`Multicast` with a subject selector or
        /// a selector that throws).
        fn try_multicast_with<I: 'static, R: 'static>(
            &self,
            subject_selector: impl Fn() -> Result<std::rc::Rc<dyn crate::subjects::ISubjectHandle<T, I>>, ferroui_system::ObservableError> + 'static,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<I>>) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<R>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::multicast::multicast_with(self.observable(), std::rc::Rc::new(subject_selector), std::rc::Rc::new(selector))
        }
    };
}
pub(crate) use multicast_methods;
