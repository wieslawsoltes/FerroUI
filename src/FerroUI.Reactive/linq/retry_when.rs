//! `RetryWhen` (`Linq/Observable/RetryWhen.cs`), with the parts it shares with `RepeatWhen`: the
//! serialized observer of the signals (`RedoSerializedObserver<X>`), the slot of the subscription
//! to the source, and the half serializer of the notifications (`Internal/HalfSerializer.cs`).

use crate::disposables::{Disposable, SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::Sink;
use crate::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// The forwarding of a sink, which the half serializer calls (`ISink<TTarget>`).
pub(crate) trait ForwardSink<T> {
    fn forward_on_next(&self, value: T);
    fn forward_on_error(&self, error: ObservableError);
    fn forward_on_completed(&self);
}

/// The terminal state of a half serializer (`ref Exception? error`): `Terminated` is
/// `ExceptionHelper.Terminated`.
enum HalfTerminal {
    None,
    Error(ObservableError),
    Terminated,
}

/// Serializes the values of one producer with the terminal notifications of others
/// (`HalfSerializer`, with its `wip` and `error` fields): a terminal notification that arrives
/// while a value is being forwarded is forwarded after it.
pub(crate) struct HalfSerializer {
    wip: Cell<usize>,
    error: RefCell<HalfTerminal>,
}

impl HalfSerializer {
    pub(crate) fn new() -> Self {
        Self { wip: Cell::new(0), error: RefCell::new(HalfTerminal::None) }
    }

    /// `HalfSerializer.ForwardOnNext`.
    pub(crate) fn forward_on_next<T>(&self, sink: &impl ForwardSink<T>, item: T) {
        if self.wip.get() == 0 {
            self.wip.set(1);
            sink.forward_on_next(item);
            let wip = self.wip.get() - 1;
            self.wip.set(wip);
            if wip != 0 {
                let error = std::mem::replace(&mut *self.error.borrow_mut(), HalfTerminal::Terminated);
                match error {
                    HalfTerminal::Error(error) => sink.forward_on_error(error),
                    _ => sink.forward_on_completed(),
                }
            }
        }
        // Upstream traces a warning for a value that arrives while another one is forwarded.
    }

    /// `HalfSerializer.ForwardOnError`.
    pub(crate) fn forward_on_error<T>(&self, sink: &impl ForwardSink<T>, error: ObservableError) {
        if !self.try_set_terminal(HalfTerminal::Error(error.clone())) {
            return;
        }
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip == 1 {
            *self.error.borrow_mut() = HalfTerminal::Terminated;
            sink.forward_on_error(error);
        }
    }

    /// `HalfSerializer.ForwardOnCompleted`.
    pub(crate) fn forward_on_completed<T>(&self, sink: &impl ForwardSink<T>) {
        if !self.try_set_terminal(HalfTerminal::Terminated) {
            return;
        }
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip == 1 {
            sink.forward_on_completed();
        }
    }

    /// `ExceptionHelper.TrySetException`: sets the terminal state unless one is set.
    fn try_set_terminal(&self, terminal: HalfTerminal) -> bool {
        let mut error = self.error.borrow_mut();
        if !matches!(*error, HalfTerminal::None) {
            return false;
        }
        *error = terminal;
        true
    }
}

/// The terminal state of a [`RedoSerializedObserver`] (`_terminalException`): `Completed` is
/// `ExceptionHelper.Terminated` and `Signaled` is `SignaledIndicator`.
enum RedoTerminal {
    None,
    Error(ObservableError),
    Completed,
    Signaled,
}

/// An observer that serializes the notifications to the subject of the signals: one that arrives
/// while another is delivered is queued and delivered after it (`RedoSerializedObserver<X>`).
pub(crate) struct RedoSerializedObserver<X> {
    downstream: Rc<dyn IObserver<X>>,
    queue: RefCell<VecDeque<X>>,
    wip: Cell<usize>,
    terminal: RefCell<RedoTerminal>,
}

impl<X: 'static> RedoSerializedObserver<X> {
    pub(crate) fn new(downstream: Rc<dyn IObserver<X>>) -> Self {
        Self { downstream, queue: RefCell::new(VecDeque::new()), wip: Cell::new(0), terminal: RefCell::new(RedoTerminal::None) }
    }

    fn set_terminal(&self, terminal: RedoTerminal) -> bool {
        let mut current = self.terminal.borrow_mut();
        if !matches!(*current, RedoTerminal::None) {
            return false;
        }
        *current = terminal;
        true
    }

    fn drain(&self) {
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip != 1 {
            return;
        }
        let mut missed = 1;
        loop {
            let terminal = {
                let mut current = self.terminal.borrow_mut();
                match &*current {
                    RedoTerminal::None => None,
                    RedoTerminal::Signaled => Some(None),
                    RedoTerminal::Error(error) => {
                        let error = error.clone();
                        *current = RedoTerminal::Signaled;
                        Some(Some(Some(error)))
                    }
                    RedoTerminal::Completed => {
                        *current = RedoTerminal::Signaled;
                        Some(Some(None))
                    }
                }
            };
            match terminal {
                Some(signal) => {
                    match signal {
                        Some(Some(error)) => self.downstream.on_error(error),
                        Some(None) => self.downstream.on_completed(),
                        None => {}
                    }
                    self.queue.borrow_mut().clear();
                }
                None => loop {
                    let item = self.queue.borrow_mut().pop_front();
                    match item {
                        Some(item) => self.downstream.on_next(item),
                        None => break,
                    }
                },
            }

            let wip = self.wip.get() - missed;
            self.wip.set(wip);
            missed = wip;
            if missed == 0 {
                break;
            }
        }
    }
}

impl<X: 'static> IObserver<X> for RedoSerializedObserver<X> {
    fn on_next(&self, value: X) {
        self.queue.borrow_mut().push_back(value);
        self.drain();
    }

    fn on_error(&self, error: ObservableError) {
        if self.set_terminal(RedoTerminal::Error(error)) {
            self.drain();
        }
    }

    fn on_completed(&self) {
        if self.set_terminal(RedoTerminal::Completed) {
            self.drain();
        }
    }
}

/// What the slot of the subscription to the source holds (`IDisposable? _upstream`): `Empty` is
/// `null` and `Disposed` is `BooleanDisposable.True`.
enum UpstreamState {
    Empty,
    Set(Rc<dyn IDisposable>),
    Disposed,
}

/// The slot of the subscription to the source of `RetryWhen` and `RepeatWhen` (`_upstream` with
/// the helpers of `Disposable` that update it).
pub(crate) struct RedoUpstream {
    state: RefCell<UpstreamState>,
}

impl RedoUpstream {
    pub(crate) fn new() -> Self {
        Self { state: RefCell::new(UpstreamState::Empty) }
    }

    /// Empties the slot and disposes what it held; false when it was disposed
    /// (`Disposable.TrySetSerial(ref _upstream, null)`).
    pub(crate) fn try_clear(&self) -> bool {
        let previous = {
            let mut state = self.state.borrow_mut();
            if matches!(*state, UpstreamState::Disposed) {
                return false;
            }
            std::mem::replace(&mut *state, UpstreamState::Empty)
        };
        if let UpstreamState::Set(previous) = previous {
            previous.dispose();
        }
        true
    }

    /// Assigns the subscription when the slot is empty (`Disposable.TrySetSingle`, which succeeds
    /// only then); a subscription assigned to a disposed slot is disposed.
    pub(crate) fn try_set_single(&self, value: Rc<dyn IDisposable>) -> bool {
        let disposed = {
            let mut state = self.state.borrow_mut();
            match *state {
                UpstreamState::Empty => {
                    *state = UpstreamState::Set(value);
                    return true;
                }
                UpstreamState::Set(_) => false,
                UpstreamState::Disposed => true,
            }
        };
        if disposed {
            value.dispose();
        }
        false
    }

    /// Assigns the subscription when the slot is empty (`Interlocked.CompareExchange(ref _upstream,
    /// value, null) == null`); nothing else happens otherwise.
    pub(crate) fn try_set_if_empty(&self, value: Rc<dyn IDisposable>) -> bool {
        let mut state = self.state.borrow_mut();
        if matches!(*state, UpstreamState::Empty) {
            *state = UpstreamState::Set(value);
            true
        } else {
            false
        }
    }

    /// Disposes the slot and what it holds (`Disposable.Dispose(ref _upstream)`).
    pub(crate) fn dispose(&self) {
        let previous = std::mem::replace(&mut *self.state.borrow_mut(), UpstreamState::Disposed);
        if let UpstreamState::Set(previous) = previous {
            previous.dispose();
        }
    }
}

type Handler<U> = Rc<dyn Fn(Rc<dyn IObservable<ObservableError>>) -> Result<Rc<dyn IObservable<U>>, ObservableError>>;

/// `RetryWhen<T, U>`: subscribes to the source again each time the sequence of the handler
/// signals a value after an error of the source.
struct RetryWhen<T, U> {
    source: Rc<dyn IObservable<T>>,
    handler: Handler<U>,
}

impl<T: 'static, U: 'static> IObservable<T> for RetryWhen<T, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let error_signals = Subject::<ObservableError>::new();
        let redo = match (self.handler)(error_signals.as_observable()) {
            Ok(redo) => redo,
            Err(error) => {
                observer.on_error(error);
                return Disposable::empty();
            }
        };

        let parent = MainObserver::new(observer, self.source.clone(), Rc::new(RedoSerializedObserver::new(error_signals.as_observer())));
        let consumer: Rc<dyn IObserver<U>> = Rc::new(HandlerObserver { main: parent.clone() });
        let subscription = redo.subscribe(consumer);
        parent.handler_upstream.set_disposable(Some(subscription));
        parent.handler_next();
        parent
    }
}

/// The observer of the source, and the subscription (`RetryWhen<T, U>.MainObserver`).
struct MainObserver<T> {
    sink: Sink<T>,
    source: Rc<dyn IObservable<T>>,
    error_signal: Rc<dyn IObserver<ObservableError>>,
    upstream: RedoUpstream,
    handler_upstream: SingleAssignmentDisposableValue,
    trampoline: Cell<usize>,
    half_serializer: HalfSerializer,
    this: Weak<Self>,
}

impl<T: 'static> MainObserver<T> {
    fn new(downstream: Rc<dyn IObserver<T>>, source: Rc<dyn IObservable<T>>, error_signal: Rc<dyn IObserver<ObservableError>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            sink: Sink::new(downstream),
            source,
            error_signal,
            upstream: RedoUpstream::new(),
            handler_upstream: SingleAssignmentDisposableValue::new(),
            trampoline: Cell::new(0),
            half_serializer: HalfSerializer::new(),
            this: this.clone(),
        })
    }

    fn handler_error(&self, error: ObservableError) {
        self.half_serializer.forward_on_error(self, error);
    }

    fn handler_complete(&self) {
        self.half_serializer.forward_on_completed(self);
    }

    /// Subscribes to the source again, through a trampoline (`HandlerNext`).
    fn handler_next(&self) {
        let trampoline = self.trampoline.get() + 1;
        self.trampoline.set(trampoline);
        if trampoline == 1 {
            loop {
                let assignment = SingleAssignmentDisposable::new();
                let slot: Rc<dyn IDisposable> = assignment.clone();
                if !self.upstream.try_set_single(slot) {
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

impl<T: 'static> ForwardSink<T> for MainObserver<T> {
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
        self.half_serializer.forward_on_next(self, value);
    }

    fn on_error(&self, error: ObservableError) {
        if self.upstream.try_clear() {
            self.error_signal.on_next(error);
        }
    }

    fn on_completed(&self) {
        self.half_serializer.forward_on_completed(self);
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

pub(crate) fn retry_when<T: 'static, U: 'static>(source: Rc<dyn IObservable<T>>, handler: Handler<U>) -> Rc<dyn IObservable<T>> {
    Rc::new(RetryWhen { source, handler })
}

/// The methods of `RetryWhen` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! retry_when_methods {
    () => {
        /// Subscribes to the source again each time the sequence `handler` makes of the errors of
        /// the source signals a value; the end or the error of that sequence ends the sequence
        /// (`RetryWhen<TSource, TSignal>(Func<IObservable<Exception>, IObservable<TSignal>>)`).
        fn retry_when<U: 'static>(
            &self,
            handler: impl Fn(
                    std::rc::Rc<dyn ferroui_system::IObservable<ferroui_system::ObservableError>>,
                ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::retry_when::retry_when(self.observable(), std::rc::Rc::new(move |signals| Ok(handler(signals))))
        }

        /// [`retry_when`](Self::retry_when) with a handler that may fail; a failure ends the
        /// sequence with its error when it is subscribed (`RetryWhen` with a handler that throws).
        fn try_retry_when<U: 'static>(
            &self,
            handler: impl Fn(
                    std::rc::Rc<dyn ferroui_system::IObservable<ferroui_system::ObservableError>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::retry_when::retry_when(self.observable(), std::rc::Rc::new(handler))
        }
    };
}
pub(crate) use retry_when_methods;
