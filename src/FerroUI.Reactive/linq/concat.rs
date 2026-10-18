//! `Concat` (`Linq/Observable/Concat.cs`, with `Internal/ConcatSink.cs` and
//! `Internal/TailRecursiveSink.cs`), and `Concat` of a sequence of sequences
//! (`Linq/Observable/ConcatMany.cs`).

use crate::disposables::{SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::linq::merge::ObservableSource;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

type Sources<T> = Box<dyn Iterator<Item = Rc<dyn IObservable<T>>>>;

/// The subscription to the current sequence of a [`ConcatSink`] (`_currentSubscription`): none, the
/// mark of a subscription being made (`ReadyToken.Ready`), the subscription, or disposed
/// (`BooleanDisposable.True`).
enum Current {
    Empty,
    Ready,
    Set(Rc<dyn IDisposable>),
    Disposed,
}

/// The sink of `Concat` of an enumerable of sequences (`Concat<TSource>._`, a `ConcatSink<TSource>`
/// and `TailRecursiveSink<TSource>`): it subscribes to the sequences one after another from a loop
/// that a completion re-enters through a trampoline, so that a long chain of sequences that end at
/// once does not grow the stack.
///
/// Deviation (DEVIATIONS.md, Reactive): upstream flattens a sequence that is itself a `Concat` into
/// the enumeration (`IConcatenatable`) and evaluates the sequences of `Defer`-like operators before
/// subscribing (`IEvaluatableObservable`); the port subscribes to each sequence as it is, which
/// notifies the same values at the same times.
struct ConcatSink<T> {
    this: Weak<ConcatSink<T>>,
    sink: Sink<T>,
    stack: RefCell<Vec<Sources<T>>>,
    is_disposed: Cell<bool>,
    trampoline: Cell<usize>,
    current: RefCell<Current>,
}

impl<T: 'static> ConcatSink<T> {
    fn run(&self, sources: Sources<T>) {
        self.stack.borrow_mut().push(sources);
        self.drain();
    }

    fn drain(&self) {
        let entered = self.trampoline.get() + 1;
        self.trampoline.set(entered);
        if entered != 1 {
            return;
        }
        loop {
            // A pass that asks for another one without leaving the trampoline is upstream's
            // `continue` of the loop.
            if self.drain_once() {
                continue;
            }
            let remaining = self.trampoline.get() - 1;
            self.trampoline.set(remaining);
            if remaining == 0 {
                break;
            }
        }
    }

    /// One pass of the loop of `Drain`; true to run another pass at once.
    fn drain_once(&self) -> bool {
        if self.is_disposed.get() {
            let stack = std::mem::take(&mut *self.stack.borrow_mut());
            drop(stack);
            let current = std::mem::replace(&mut *self.current.borrow_mut(), Current::Disposed);
            if let Current::Set(subscription) = current {
                subscription.dispose();
            }
            return false;
        }
        let top = self.stack.borrow_mut().pop();
        let Some(mut enumerator) = top else {
            self.is_disposed.set(true);
            self.sink.forward_on_completed();
            return false;
        };
        let Some(next) = enumerator.next() else {
            // The enumerator is done: it is disposed (dropped) and the loop goes on.
            drop(enumerator);
            return true;
        };
        self.stack.borrow_mut().push(enumerator);
        let ready = {
            let mut current = self.current.borrow_mut();
            if matches!(*current, Current::Empty) {
                *current = Current::Ready;
                true
            } else {
                false
            }
        };
        if !ready {
            return true;
        }
        let observer: Rc<dyn IObserver<T>> = self.this.upgrade().expect("the sink is alive while it runs");
        let subscription = next.subscribe(observer);
        let outcome = {
            let mut current = self.current.borrow_mut();
            if matches!(*current, Current::Ready) {
                *current = Current::Set(subscription.clone());
                None
            } else {
                Some(matches!(*current, Current::Disposed))
            }
        };
        match outcome {
            None => false,
            // The sequence ended (or the sink was disposed) while it was subscribed to.
            Some(disposed) => {
                subscription.dispose();
                disposed
            }
        }
    }

    /// Moves on to the next sequence (`Recurse`).
    fn recurse(&self) {
        let previous = {
            let mut current = self.current.borrow_mut();
            if matches!(*current, Current::Disposed) {
                None
            } else {
                Some(std::mem::replace(&mut *current, Current::Empty))
            }
        };
        if let Some(previous) = previous {
            if let Current::Set(subscription) = previous {
                subscription.dispose();
            }
            self.drain();
        }
    }
}

impl<T: 'static> IObserver<T> for ConcatSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.recurse();
    }
}

impl<T: 'static> IDisposable for ConcatSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            // `DisposeAll`: the loop disposes the current subscription and the enumerators.
            self.is_disposed.set(true);
            self.drain();
        }
    }
}

/// The observable of `Concat` of an enumerable of sequences (`Concat<TSource>`): each subscription
/// enumerates `sources` anew.
struct ConcatObservable<I> {
    sources: I,
}

impl<T: 'static, I> IObservable<T> for ConcatObservable<I>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    I::IntoIter: 'static,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new_cyclic(|this| ConcatSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    stack: RefCell::new(Vec::new()),
                    is_disposed: Cell::new(false),
                    trampoline: Cell::new(0),
                    current: RefCell::new(Current::Empty),
                })
            },
            move |sink: &Rc<ConcatSink<T>>| sink.run(Box::new(sources.into_iter())),
        )
    }
}

pub(crate) fn concat_enumerable<T: 'static, I>(sources: I) -> Rc<dyn IObservable<T>>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    I::IntoIter: 'static,
{
    Rc::new(ConcatObservable { sources })
}

/// The subscription of the inner sequence of `Concat` of a sequence of sequences
/// (`InnerObserver.Upstream`): none, the subscription, or disposed.
enum InnerSlot {
    Empty,
    Set(Rc<SingleAssignmentDisposable>),
    Disposed,
}

/// The observer of the outer sequence of `Concat` of a sequence of sequences
/// (`ConcatMany<T>.ConcatManyOuterObserver`): the inner sequences wait in a queue and are subscribed
/// to one at a time; an error of the outer sequence is delivered once the current inner sequence
/// has ended. It notifies the observer directly (upstream's `ConcatMany` is not a producer).
struct ConcatManyOuterObserver<S, T> {
    this: Weak<ConcatManyOuterObserver<S, T>>,
    downstream: Rc<dyn IObserver<T>>,
    queue: RefCell<VecDeque<S>>,
    upstream: SingleAssignmentDisposableValue,
    inner_upstream: RefCell<InnerSlot>,
    trampoline: Cell<usize>,
    error: RefCell<Option<ObservableError>>,
    done: Cell<bool>,
    active: Cell<bool>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> ConcatManyOuterObserver<S, T> {
    fn dispose_main(&self) {
        self.upstream.dispose();
    }

    /// `InnerObserver.SetDisposable`: false when the inner observer was disposed (the disposable is
    /// disposed then) or already holds a subscription.
    fn set_inner_disposable(&self, disposable: Rc<SingleAssignmentDisposable>) -> bool {
        let mut slot = self.inner_upstream.borrow_mut();
        if matches!(*slot, InnerSlot::Empty) {
            *slot = InnerSlot::Set(disposable);
            return true;
        }
        let disposed = matches!(*slot, InnerSlot::Disposed);
        drop(slot);
        if disposed {
            disposable.dispose();
        }
        false
    }

    /// `InnerObserver.Finish`: disposes the subscription of the inner sequence; false when the inner
    /// observer was disposed.
    fn finish_inner(&self) -> bool {
        let previous = {
            let mut slot = self.inner_upstream.borrow_mut();
            if matches!(*slot, InnerSlot::Disposed) {
                return false;
            }
            std::mem::replace(&mut *slot, InnerSlot::Empty)
        };
        if let InnerSlot::Set(subscription) = previous {
            subscription.dispose();
        }
        true
    }

    /// `InnerObserver.Dispose`.
    fn dispose_inner(&self) {
        let previous = std::mem::replace(&mut *self.inner_upstream.borrow_mut(), InnerSlot::Disposed);
        if let InnerSlot::Set(subscription) = previous {
            subscription.dispose();
        }
    }

    fn inner_error(&self, error: ObservableError) {
        if self.finish_inner() {
            let first = self.error.borrow().is_none();
            if first {
                *self.error.borrow_mut() = Some(error);
                self.done.set(true);
                self.active.set(false);
                self.drain();
            }
        }
    }

    fn inner_complete(&self) {
        if self.finish_inner() {
            self.active.set(false);
            self.drain();
        }
    }

    fn drain(&self) {
        let entered = self.trampoline.get() + 1;
        self.trampoline.set(entered);
        if entered != 1 {
            return;
        }
        loop {
            if self.upstream.is_disposed() {
                self.queue.borrow_mut().clear();
            } else if !self.active.get() {
                let is_done = self.done.get();
                let error = if is_done { self.error.borrow().clone() } else { None };
                if let Some(error) = error {
                    self.downstream.on_error(error);
                    self.dispose_main();
                } else {
                    let next = self.queue.borrow_mut().pop_front();
                    match next {
                        Some(source) => {
                            let subscription = SingleAssignmentDisposable::new();
                            if self.set_inner_disposable(subscription.clone()) {
                                self.active.set(true);
                                let parent = self.this.upgrade().expect("the observer is alive while it runs");
                                let inner = Rc::new(ConcatManyInnerObserver { parent });
                                subscription.set_disposable(Some(source.subscribe_source(inner)));
                            }
                        }
                        None => {
                            if is_done {
                                self.downstream.on_completed();
                                self.dispose_main();
                            }
                        }
                    }
                }
            }
            let remaining = self.trampoline.get() - 1;
            self.trampoline.set(remaining);
            if remaining == 0 {
                break;
            }
        }
    }
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObserver<S> for ConcatManyOuterObserver<S, T> {
    fn on_next(&self, value: S) {
        self.queue.borrow_mut().push_back(value);
        self.drain();
    }

    fn on_error(&self, error: ObservableError) {
        let first = self.error.borrow().is_none();
        if first {
            *self.error.borrow_mut() = Some(error);
            self.done.set(true);
            self.drain();
        }
    }

    fn on_completed(&self) {
        self.done.set(true);
        self.drain();
    }
}

impl<S: ObservableSource<T> + 'static, T: 'static> IDisposable for ConcatManyOuterObserver<S, T> {
    fn dispose(&self) {
        self.dispose_inner();
        self.dispose_main();
    }
}

/// The observer of the current inner sequence (`ConcatManyOuterObserver.InnerObserver`); its state
/// (the subscription) is kept by the outer observer, so that nothing refers back to the outer
/// observer once the inner subscription has ended.
struct ConcatManyInnerObserver<S, T> {
    parent: Rc<ConcatManyOuterObserver<S, T>>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObserver<T> for ConcatManyInnerObserver<S, T> {
    fn on_next(&self, value: T) {
        self.parent.downstream.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.inner_error(error);
    }

    fn on_completed(&self) {
        self.parent.inner_complete();
    }
}

/// The observable of `Concat` of a sequence of sequences (`ConcatMany<T>`).
struct ConcatManyObservable<S, T> {
    sources: Rc<dyn IObservable<S>>,
    _values: PhantomData<fn() -> T>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObservable<T> for ConcatManyObservable<S, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let parent = Rc::new_cyclic(|this| ConcatManyOuterObserver {
            this: this.clone(),
            downstream: observer,
            queue: RefCell::new(VecDeque::new()),
            upstream: SingleAssignmentDisposableValue::new(),
            inner_upstream: RefCell::new(InnerSlot::Empty),
            trampoline: Cell::new(0),
            error: RefCell::new(None),
            done: Cell::new(false),
            active: Cell::new(false),
        });
        let subscription = self.sources.subscribe(parent.clone());
        parent.upstream.set_disposable(Some(subscription));
        parent
    }
}

pub(crate) fn concat_observables<S: ObservableSource<T> + 'static, T: 'static>(sources: Rc<dyn IObservable<S>>) -> Rc<dyn IObservable<T>> {
    Rc::new(ConcatManyObservable { sources, _values: PhantomData })
}

impl crate::linq::Observable {
    /// The values of the sequences of an enumerable, one sequence after another
    /// (`Concat(IEnumerable<IObservable<TSource>>)` and `Concat(params IObservable<TSource>[])`).
    /// Each subscription enumerates `sources` anew, as each sequence ends.
    pub fn concat_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        concat_enumerable(sources)
    }
}

/// The methods of `Concat` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! concat_methods {
    () => {
        /// The values of the sequence, then those of `other`
        /// (`Concat(IObservable<TSource>, IObservable<TSource>)`).
        fn concat(&self, other: std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::concat::concat_enumerable(vec![self.observable(), other])
        }

        /// The values of the inner sequences of a sequence of sequences, one inner sequence after
        /// another (`Concat(IObservable<IObservable<TSource>>)`).
        fn concat_inner<U: 'static>(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            T: crate::linq::merge::ObservableSource<U>,
        {
            crate::linq::concat::concat_observables(self.observable())
        }
    };
}
pub(crate) use concat_methods;
