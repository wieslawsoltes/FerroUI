//! `Amb` (`Linq/Observable/Amb.cs`), and `Amb` of many sequences (`Linq/Observable/AmbMany.cs`).

use crate::disposables::Disposable;
use crate::internal::{subscribe_basic, subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The sink of `Amb` of two sequences (`Amb<TSource>.AmbCoordinator`): the first sequence to notify
/// wins the race, and the subscription to the other is disposed.
struct AmbCoordinator<T> {
    left: Rc<AmbObserver<T>>,
    right: Rc<AmbObserver<T>>,
    /// 0 while no sequence has won, 1 when the left one has, 2 when the right one has.
    winner: Cell<u8>,
}

impl<T: 'static> AmbCoordinator<T> {
    /// Tries to win the race for the right of emission (`TryWin`); true when the contender has won.
    fn try_win(&self, is_left: bool) -> bool {
        let index = if is_left { 1 } else { 2 };
        if self.winner.get() == index {
            return true;
        }
        if self.winner.get() == 0 {
            self.winner.set(index);
            if is_left {
                self.right.dispose();
            } else {
                self.left.dispose();
            }
            return true;
        }
        false
    }
}

impl<T: 'static> IDisposable for AmbCoordinator<T> {
    fn dispose(&self) {
        self.left.dispose();
        self.right.dispose();
    }
}

/// The observer of one of the two sequences of `Amb` (`AmbCoordinator.AmbObserver`).
struct AmbObserver<T> {
    sink: Sink<T>,
    parent: Weak<AmbCoordinator<T>>,
    is_left: bool,
    /// Whether this observer won the race and now emits on a fast path (`_iwon`).
    won: Cell<bool>,
}

impl<T: 'static> AmbObserver<T> {
    fn try_win(&self) -> bool {
        self.parent.upgrade().is_some_and(|parent| parent.try_win(self.is_left))
    }

    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }
}

impl<T: 'static> IObserver<T> for AmbObserver<T> {
    fn on_next(&self, value: T) {
        if self.won.get() {
            self.sink.forward_on_next(value);
        } else if self.try_win() {
            self.won.set(true);
            self.sink.forward_on_next(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.won.get() {
            self.sink.forward_on_error(error);
        } else if self.try_win() {
            self.won.set(true);
            self.sink.forward_on_error(error);
        } else {
            self.sink.dispose();
        }
    }

    fn on_completed(&self) {
        if self.won.get() {
            self.sink.forward_on_completed();
        } else if self.try_win() {
            self.won.set(true);
            self.sink.forward_on_completed();
        } else {
            self.sink.dispose();
        }
    }
}

impl<T: 'static> IDisposable for AmbObserver<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observable of `Amb` of two sequences (`Amb<TSource>`).
struct AmbObservable<T> {
    left: Rc<dyn IObservable<T>>,
    right: Rc<dyn IObservable<T>>,
}

impl<T: 'static> IObservable<T> for AmbObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (left, right) = (self.left.clone(), self.right.clone());
        subscribe_sink(
            observer,
            |observer| {
                Rc::new_cyclic(|parent: &Weak<AmbCoordinator<T>>| AmbCoordinator {
                    left: Rc::new(AmbObserver { sink: Sink::new(observer.clone()), parent: parent.clone(), is_left: true, won: Cell::new(false) }),
                    right: Rc::new(AmbObserver { sink: Sink::new(observer), parent: parent.clone(), is_left: false, won: Cell::new(false) }),
                    winner: Cell::new(0),
                })
            },
            move |coordinator: &Rc<AmbCoordinator<T>>| {
                coordinator.left.run(&left);
                coordinator.right.run(&right);
            },
        )
    }
}

pub(crate) fn amb<T: 'static>(left: Rc<dyn IObservable<T>>, right: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    Rc::new(AmbObservable { left, right })
}

/// The coordinator of `Amb` of many sequences (`AmbCoordinator<T>`): the first sequence to notify
/// wins, and the subscriptions to all the others are disposed.
struct AmbManyCoordinator<T> {
    observers: RefCell<Vec<Option<Rc<AmbManyObserver<T>>>>>,
    winner: Cell<Option<usize>>,
}

impl<T: 'static> AmbManyCoordinator<T> {
    /// The subscription to the race of `sources` (`AmbCoordinator<T>.Create`): none for no source,
    /// the source itself for one.
    fn create(observer: Rc<dyn IObserver<T>>, sources: Vec<Rc<dyn IObservable<T>>>) -> Rc<dyn IDisposable> {
        let n = sources.len();
        if n == 0 {
            observer.on_completed();
            return Disposable::empty();
        }
        if n == 1 {
            return sources[0].subscribe(observer);
        }
        let parent = Rc::new_cyclic(|parent: &Weak<AmbManyCoordinator<T>>| AmbManyCoordinator {
            observers: RefCell::new(
                (0..n)
                    .map(|index| {
                        Some(Rc::new(AmbManyObserver { sink: Sink::new(observer.clone()), parent: parent.clone(), index, won: Cell::new(false) }))
                    })
                    .collect(),
            ),
            winner: Cell::new(None),
        });
        parent.subscribe(&sources);
        parent
    }

    fn subscribe(&self, sources: &[Rc<dyn IObservable<T>>]) {
        for (index, source) in sources.iter().enumerate() {
            let inner = self.observers.borrow()[index].clone();
            let Some(inner) = inner else {
                break;
            };
            let upstream = source.subscribe(inner.clone());
            inner.sink.set_upstream(upstream);
        }
    }

    fn try_win(&self, index: usize) -> bool {
        if self.winner.get().is_some() {
            return false;
        }
        self.winner.set(Some(index));
        let losers: Vec<Rc<AmbManyObserver<T>>> = {
            let mut observers = self.observers.borrow_mut();
            observers.iter_mut().enumerate().filter(|(i, _)| *i != index).filter_map(|(_, slot)| slot.take()).collect()
        };
        for loser in losers {
            loser.dispose();
        }
        true
    }
}

impl<T: 'static> IDisposable for AmbManyCoordinator<T> {
    fn dispose(&self) {
        let observers: Vec<Rc<AmbManyObserver<T>>> = self.observers.borrow_mut().iter_mut().filter_map(Option::take).collect();
        for observer in observers {
            observer.dispose();
        }
    }
}

/// The observer of one of the sequences of `Amb` of many sequences (`AmbCoordinator<T>.InnerObserver`):
/// unlike the two-sequence form, a value that loses the race disposes the observer.
struct AmbManyObserver<T> {
    sink: Sink<T>,
    parent: Weak<AmbManyCoordinator<T>>,
    index: usize,
    won: Cell<bool>,
}

impl<T: 'static> AmbManyObserver<T> {
    fn try_win(&self) -> bool {
        self.parent.upgrade().is_some_and(|parent| parent.try_win(self.index))
    }
}

impl<T: 'static> IObserver<T> for AmbManyObserver<T> {
    fn on_next(&self, value: T) {
        if self.won.get() {
            self.sink.forward_on_next(value);
        } else if self.try_win() {
            self.won.set(true);
            self.sink.forward_on_next(value);
        } else {
            self.sink.dispose();
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.won.get() {
            self.sink.forward_on_error(error);
        } else if self.try_win() {
            self.won.set(true);
            self.sink.forward_on_error(error);
        } else {
            self.sink.dispose();
        }
    }

    fn on_completed(&self) {
        if self.won.get() {
            self.sink.forward_on_completed();
        } else if self.try_win() {
            self.won.set(true);
            self.sink.forward_on_completed();
        } else {
            self.sink.dispose();
        }
    }
}

impl<T: 'static> IDisposable for AmbManyObserver<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observable of `Amb` of many sequences (`AmbManyEnumerable<T>`, and `AmbManyArray<T>`): each
/// subscription enumerates `sources` anew.
struct AmbManyObservable<I> {
    sources: I,
}

impl<T: 'static, I> IObservable<T> for AmbManyObservable<I>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        subscribe_basic(observer, move |observer| AmbManyCoordinator::create(observer, sources.into_iter().collect()))
    }
}

impl crate::linq::Observable {
    /// The sequence of an enumerable that notifies first; the subscriptions to the others are
    /// disposed (`Amb(IEnumerable<IObservable<TSource>>)` and `Amb(params IObservable<TSource>[])`).
    /// No sequence ends at once; one sequence is the sequence itself.
    pub fn amb_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        Rc::new(AmbManyObservable { sources })
    }
}

/// The methods of `Amb` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! amb_methods {
    () => {
        /// The sequence, or `other`, whichever notifies first; the subscription to the other is
        /// disposed (`Amb(IObservable<TSource>, IObservable<TSource>)`).
        fn amb(&self, other: std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::amb::amb(self.observable(), other)
        }
    };
}
pub(crate) use amb_methods;
