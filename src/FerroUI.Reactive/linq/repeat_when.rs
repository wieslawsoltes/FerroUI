//! `RepeatWhen` (`Linq/Observable/RepeatWhen.cs`).

use crate::disposables::{Disposable, SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::{HalfSerializedSink, HalfSerializer, Sink};
use crate::linq::retry_when::{RedoSerializedObserver, RedoUpstream};
use crate::subjects::Subject;
use crate::Unit;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::{Rc, Weak};

type Handler<U> = Rc<dyn Fn(Rc<dyn IObservable<Unit>>) -> Result<Rc<dyn IObservable<U>>, ObservableError>>;

/// `RepeatWhen<T, U>`: subscribes to the source again each time the sequence of the handler
/// signals a value after the source completed.
///
/// Deviation (DEVIATIONS.md, Reactive): the signals of the completions are `Unit` values, where
/// upstream sends `null` of the type `object` (kept there for compatibility).
struct RepeatWhen<T, U> {
    source: Rc<dyn IObservable<T>>,
    handler: Handler<U>,
}

impl<T: 'static, U: 'static> IObservable<T> for RepeatWhen<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let complete_signals = Subject::<Unit>::new();
        let redo = match (self.handler)(complete_signals.as_observable()) {
            Ok(redo) => redo,
            Err(error) => {
                observer.on_error(error);
                return Disposable::empty();
            }
        };

        let parent = MainObserver::new(observer, self.source.clone(), Rc::new(RedoSerializedObserver::new(complete_signals.as_observer())));
        let consumer: Rc<dyn IObserver<U>> = Rc::new(HandlerObserver { main: parent.clone() });
        let subscription = redo.subscribe(consumer);
        parent.handler_upstream.set_disposable(Some(subscription));
        parent.handler_next();
        parent
    }
}

/// The observer of the source, and the subscription (`RepeatWhen<T, U>.MainObserver`).
struct MainObserver<T> {
    sink: Sink<T>,
    source: Rc<dyn IObservable<T>>,
    complete_signal: Rc<dyn IObserver<Unit>>,
    upstream: RedoUpstream,
    handler_upstream: SingleAssignmentDisposableValue,
    trampoline: Cell<usize>,
    half_serializer: HalfSerializer,
    this: Weak<Self>,
}

impl<T: 'static> MainObserver<T> {
    fn new(downstream: Rc<dyn IObserver<T>>, source: Rc<dyn IObservable<T>>, complete_signal: Rc<dyn IObserver<Unit>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            sink: Sink::new(downstream),
            source,
            complete_signal,
            upstream: RedoUpstream::new(),
            handler_upstream: SingleAssignmentDisposableValue::new(),
            trampoline: Cell::new(0),
            half_serializer: HalfSerializer::new(),
            this: this.clone(),
        })
    }

    fn handler_error(&self, error: ObservableError) {
        self.half_serializer.on_error(self, error);
    }

    fn handler_complete(&self) {
        self.half_serializer.on_completed(self);
    }

    /// Subscribes to the source again, through a trampoline (`HandlerNext`).
    fn handler_next(&self) {
        let trampoline = self.trampoline.get() + 1;
        self.trampoline.set(trampoline);
        if trampoline == 1 {
            loop {
                let assignment = SingleAssignmentDisposable::new();
                let slot: Rc<dyn IDisposable> = assignment.clone();
                if !self.upstream.try_set_if_empty(slot) {
                    return;
                }
                let Some(this) = self.this.upgrade() else {
                    return;
                };
                let observer: Rc<dyn IObserver<T>> = this;
                assignment.set_disposable(Some(self.source.subscribe(observer)));

                let trampoline = self.trampoline.get() - 1;
                self.trampoline.set(trampoline);
                if trampoline == 0 {
                    break;
                }
            }
        }
    }
}

impl<T: 'static> HalfSerializedSink<T> for MainObserver<T> {
    fn forward_on_next(&self, value: T) {
        self.sink.forward_on_next(value);
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

impl<T: 'static> IObserver<T> for MainObserver<T> {
    fn on_next(&self, value: T) {
        self.half_serializer.on_next(self, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.half_serializer.on_error(self, error);
    }

    fn on_completed(&self) {
        if self.upstream.try_clear() {
            self.complete_signal.on_next(());
        }
    }
}

impl<T: 'static> IDisposable for MainObserver<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.upstream.dispose();
            self.handler_upstream.dispose();
        }
    }
}

/// The observer of the sequence of the handler (`MainObserver.HandlerObserver`).
struct HandlerObserver<T> {
    main: Rc<MainObserver<T>>,
}

impl<T: 'static, U> IObserver<U> for HandlerObserver<T> {
    fn on_next(&self, _value: U) {
        self.main.handler_next();
    }

    fn on_error(&self, error: ObservableError) {
        self.main.handler_error(error);
    }

    fn on_completed(&self) {
        self.main.handler_complete();
    }
}

pub(crate) fn repeat_when<T: 'static, U: 'static>(source: Rc<dyn IObservable<T>>, handler: Handler<U>) -> Rc<dyn IObservable<T>> {
    Rc::new(RepeatWhen { source, handler })
}

/// The methods of `RepeatWhen` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! repeat_when_methods {
    () => {
        /// Subscribes to the source again each time the sequence `handler` makes of the completions
        /// of the source signals a value; the end or the error of that sequence ends the sequence
        /// (`RepeatWhen<TSource, TSignal>(Func<IObservable<object>, IObservable<TSignal>>)`).
        fn repeat_when<U: 'static>(
            &self,
            handler: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<crate::Unit>>) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::repeat_when::repeat_when(self.observable(), std::rc::Rc::new(move |signals| Ok(handler(signals))))
        }

        /// [`repeat_when`](Self::repeat_when) with a handler that may fail; a failure ends the
        /// sequence with its error when it is subscribed (`RepeatWhen` with a handler that throws).
        fn try_repeat_when<U: 'static>(
            &self,
            handler: impl Fn(
                    std::rc::Rc<dyn ferroui_system::IObservable<crate::Unit>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::repeat_when::repeat_when(self.observable(), std::rc::Rc::new(handler))
        }
    };
}
pub(crate) use repeat_when_methods;
