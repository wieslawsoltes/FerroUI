//! `IConnectableObservable<T>`, `ConnectableObservable<TSource, TResult>` and the subjects of
//! `Subject.Create` (`IConnectableObservable.cs`, `ConnectableObservable.cs`, `Subject.Extensions.cs`,
//! `AnonymousSubject.cs`).

use super::ISubject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// A sequence whose source is subscribed when it is connected (`IConnectableObservable<T>`).
pub trait IConnectableObservable<T>: IObservable<T> {
    /// Subscribes the subject to the source; disposing the result disconnects it (`Connect`).
    fn connect(&self) -> Rc<dyn IDisposable>;
}

/// Shares one subscription of a source among the observers of a subject (`ConnectableObservable`):
/// observers subscribe to the subject, and [`connect`](IConnectableObservable::connect) subscribes
/// the subject to the source.
pub struct ConnectableObservable<TSource, TResult> {
    source: Rc<dyn IObservable<TSource>>,
    subject: Rc<dyn ISubjectHandle<TSource, TResult>>,
    connection: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
}

/// A subject seen as both of its sides.
pub trait ISubjectHandle<TSource, TResult>: ISubject<TSource, TResult> {
    fn as_observer_handle(self: Rc<Self>) -> Rc<dyn IObserver<TSource>>;
}

impl<S: ISubject<A, B> + 'static, A, B> ISubjectHandle<A, B> for S {
    fn as_observer_handle(self: Rc<Self>) -> Rc<dyn IObserver<A>> {
        self
    }
}

impl<TSource: 'static, TResult: 'static> ConnectableObservable<TSource, TResult> {
    /// Shares `source` through `subject`.
    pub fn new(source: Rc<dyn IObservable<TSource>>, subject: Rc<dyn ISubjectHandle<TSource, TResult>>) -> Rc<Self> {
        // The source is subscribed through `AsObservable`, as upstream does: its sink ends the
        // subscription to the source when the source ends.
        let source = crate::ObservableExt::as_observable(&source);
        Rc::new(Self { source, subject, connection: Rc::new(RefCell::new(None)) })
    }
}

impl<TSource: 'static, TResult: 'static> IConnectableObservable<TResult> for ConnectableObservable<TSource, TResult> {
    fn connect(&self) -> Rc<dyn IDisposable> {
        if let Some(connection) = self.connection.borrow().clone() {
            return connection;
        }
        let subscription = self.source.subscribe(self.subject.clone().as_observer_handle());
        let connection: Rc<dyn IDisposable> = Rc::new(Connection { slot: self.connection.clone(), subscription: RefCell::new(Some(subscription)) });
        *self.connection.borrow_mut() = Some(connection.clone());
        connection
    }
}

impl<TSource: 'static, TResult: 'static> IObservable<TResult> for ConnectableObservable<TSource, TResult> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TResult>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

/// The connection of a connectable observable (`Connection`): disposing it unsubscribes the
/// subject from the source, and the next connect subscribes again.
struct Connection {
    slot: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl IDisposable for Connection {
    fn dispose(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            self.slot.borrow_mut().take();
            subscription.dispose();
        }
    }
}

/// A subject made of an observer and an observable (`AnonymousSubject<TSource, TResult>`).
struct AnonymousSubject<TSource, TResult> {
    observer: Rc<dyn IObserver<TSource>>,
    observable: Rc<dyn IObservable<TResult>>,
}

impl<TSource, TResult> IObserver<TSource> for AnonymousSubject<TSource, TResult> {
    fn on_next(&self, value: TSource) {
        self.observer.on_next(value)
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error)
    }

    fn on_completed(&self) {
        self.observer.on_completed()
    }
}

impl<TSource, TResult> IObservable<TResult> for AnonymousSubject<TSource, TResult> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TResult>>) -> Rc<dyn IDisposable> {
        self.observable.subscribe(observer)
    }
}

/// The factories of subjects (`Subject` static members).
pub struct SubjectFactory;

impl SubjectFactory {
    /// A subject whose observer side is `observer` and whose observable side is `observable`
    /// (`Subject.Create<TSource, TResult>`).
    pub fn create<TSource: 'static, TResult: 'static>(
        observer: Rc<dyn IObserver<TSource>>,
        observable: Rc<dyn IObservable<TResult>>,
    ) -> Rc<dyn ISubjectHandle<TSource, TResult>> {
        Rc::new(AnonymousSubject { observer, observable })
    }

    /// The subject itself (`Subject.Synchronize`): notifications on one thread are serialized
    /// already.
    pub fn synchronize<TSource: 'static, TResult: 'static>(
        subject: Rc<dyn ISubjectHandle<TSource, TResult>>,
    ) -> Rc<dyn ISubjectHandle<TSource, TResult>> {
        subject
    }
}
