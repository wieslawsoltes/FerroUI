//! `Finally` (`Linq/Observable/Finally.cs`).

use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The subscription to the source of a [`FinallySink`] (`_sourceDisposable`: null, the
/// subscription, or `BooleanDisposable.True` once the sink is disposed).
enum SourceSubscription {
    Pending,
    Set(Rc<dyn IDisposable>),
    Disposed,
}

/// Runs the action of `Finally` when it is dropped, so that it runs after the subscription to the
/// source is disposed even when that panics (`try { d.Dispose(); } finally { _finallyAction(); }`).
struct RunFinally<'a>(&'a dyn Fn());

impl Drop for RunFinally<'_> {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// The sink of `Finally` (`Finally<TSource>._`): forwards every notification, and runs the action
/// once, after the subscription to the source is disposed, when the sink is disposed (by the end of
/// the sequence or by its subscriber).
struct FinallySink<T> {
    sink: Sink<T>,
    action: Rc<dyn Fn()>,
    source: RefCell<SourceSubscription>,
}

impl<T: 'static> FinallySink<T> {
    /// Subscribes the sink to the source (`Run`); when the sink was disposed during the
    /// subscription, disposes the subscription and runs the action.
    fn run(this: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        let subscription = source.subscribe(this.clone());
        let disposed = {
            let mut state = this.source.borrow_mut();
            match *state {
                SourceSubscription::Disposed => true,
                _ => {
                    *state = SourceSubscription::Set(subscription.clone());
                    false
                }
            }
        };
        if disposed {
            let _finally = RunFinally(&*this.action);
            subscription.dispose();
        }
    }
}

impl<T: 'static> IDisposable for FinallySink<T> {
    /// `Dispose(bool)`: disposes the sink, then the subscription to the source, then runs the
    /// action, the first time.
    fn dispose(&self) {
        self.sink.dispose();
        let previous = std::mem::replace(&mut *self.source.borrow_mut(), SourceSubscription::Disposed);
        if let SourceSubscription::Set(subscription) = previous {
            let _finally = RunFinally(&*self.action);
            subscription.dispose();
        }
    }
}

impl<T: 'static> IObserver<T> for FinallySink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        // `ForwardOnError` disposes the sink through `Dispose(bool)`, which runs the action.
        self.sink.forward_on_error(error);
        self.dispose();
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
        self.dispose();
    }
}

/// The observable of `Finally` (`Finally<TSource>`).
struct FinallyObservable<T> {
    source: Rc<dyn IObservable<T>>,
    action: Rc<dyn Fn()>,
}

impl<T: 'static> IObservable<T> for FinallyObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let action = self.action.clone();
        subscribe_sink(
            observer,
            move |observer| Rc::new(FinallySink { sink: Sink::new(observer), action, source: RefCell::new(SourceSubscription::Pending) }),
            move |sink: &Rc<FinallySink<T>>| FinallySink::run(sink, &source),
        )
    }
}

pub(crate) fn finally<T: 'static>(source: Rc<dyn IObservable<T>>, action: Rc<dyn Fn()>) -> Rc<dyn IObservable<T>> {
    Rc::new(FinallyObservable { source, action })
}

/// The methods of `Finally` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! finally_methods {
    () => {
        /// Runs `action` once when the subscription ends, by the end of the sequence or by its
        /// disposal, after the subscription to the source is disposed (`Finally(Action)`).
        fn finally(&self, action: impl Fn() + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::finally::finally(self.observable(), std::rc::Rc::new(action))
        }
    };
}
pub(crate) use finally_methods;
