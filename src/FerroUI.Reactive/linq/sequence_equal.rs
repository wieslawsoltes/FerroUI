//! `SequenceEqual` (`Linq/Observable/SequenceEqual.cs`).

use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

type Comparer<T> = Rc<dyn Fn(&T, &T) -> Result<bool, ObservableError>>;

/// An iterator over the second sequence, whose steps may fail (`IEnumerator<TSource>` whose
/// `MoveNext` may throw).
pub(crate) type Enumerator<T> = Box<dyn Iterator<Item = Result<T, ObservableError>>>;

/// Gets an iterator over the second sequence for a subscription, which may fail
/// (`IEnumerable<TSource>.GetEnumerator`).
pub(crate) type GetEnumerator<T> = Rc<dyn Fn() -> Result<Enumerator<T>, ObservableError>>;

/// The values of each side that the other has not matched yet, and which sides have ended.
struct State<T> {
    left: VecDeque<T>,
    right: VecDeque<T>,
    left_done: bool,
    right_done: bool,
}

/// `SequenceEqual` of two observable sequences (`SequenceEqual<TSource>.Observable`).
struct SequenceEqualObservable<T> {
    first: Rc<dyn IObservable<T>>,
    second: Rc<dyn IObservable<T>>,
    comparer: Comparer<T>,
}

/// The sink of `SequenceEqual` of two observable sequences (`SequenceEqual<TSource>.Observable._`):
/// each value is compared with the value of the other side at the same position, queued until the
/// other side has it; a difference, or a side that ends before the other, gives `false`; both sides
/// ending with all values equal gives `true`.
struct SequenceEqualSink<T> {
    sink: Sink<bool>,
    comparer: Comparer<T>,
    state: RefCell<State<T>>,
    second: SingleAssignmentDisposableValue,
}

impl<T: 'static> SequenceEqualSink<T> {
    fn forward_result(&self, result: bool) {
        self.sink.forward_on_next(result);
        self.sink.forward_on_completed();
    }

    /// A value of the first (`left`) or the second sequence (`FirstObserver.OnNext`,
    /// `SecondObserver.OnNext`).
    fn on_next_of(&self, left: bool, value: T) {
        let other_value = {
            let mut state = self.state.borrow_mut();
            let state = &mut *state;
            let (own, other, other_done) =
                if left { (&mut state.left, &mut state.right, state.right_done) } else { (&mut state.right, &mut state.left, state.left_done) };
            match other.pop_front() {
                Some(other_value) => Some(other_value),
                None if other_done => None,
                None => {
                    own.push_back(value);
                    return;
                }
            }
        };
        let Some(other_value) = other_value else {
            self.forward_result(false);
            return;
        };
        let equal = if left { (self.comparer)(&value, &other_value) } else { (self.comparer)(&other_value, &value) };
        match equal {
            Ok(true) => {}
            Ok(false) => self.forward_result(false),
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    /// The end of the first (`left`) or the second sequence (`FirstObserver.OnCompleted`,
    /// `SecondObserver.OnCompleted`).
    fn on_completed_of(&self, left: bool) {
        let result = {
            let mut state = self.state.borrow_mut();
            if left {
                state.left_done = true;
            } else {
                state.right_done = true;
            }
            let (own, other, other_done) = if left { (&state.left, &state.right, state.right_done) } else { (&state.right, &state.left, state.left_done) };
            if !own.is_empty() {
                None
            } else if !other.is_empty() {
                Some(false)
            } else if other_done {
                Some(true)
            } else {
                None
            }
        };
        if let Some(result) = result {
            self.forward_result(result);
        }
    }
}

impl<T: 'static> IDisposable for SequenceEqualSink<T> {
    fn dispose(&self) {
        self.second.dispose();
        self.sink.dispose();
    }
}

/// The observer of one of the two sequences (`FirstObserver` and `SecondObserver`).
struct SideObserver<T> {
    parent: Rc<SequenceEqualSink<T>>,
    left: bool,
}

impl<T: 'static> IObserver<T> for SideObserver<T> {
    fn on_next(&self, value: T) {
        self.parent.on_next_of(self.left, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed_of(self.left);
    }
}

impl<T: 'static> IObservable<bool> for SequenceEqualObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        let (first, second, comparer) = (self.first.clone(), self.second.clone(), self.comparer.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(SequenceEqualSink {
                    sink: Sink::new(observer),
                    comparer,
                    state: RefCell::new(State { left: VecDeque::new(), right: VecDeque::new(), left_done: false, right_done: false }),
                    second: SingleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<SequenceEqualSink<T>>| {
                let upstream = first.subscribe(Rc::new(SideObserver { parent: sink.clone(), left: true }));
                sink.sink.set_upstream(upstream);
                let second_subscription = second.subscribe(Rc::new(SideObserver { parent: sink.clone(), left: false }));
                sink.second.set_disposable(Some(second_subscription));
            },
        )
    }
}

/// `SequenceEqual` of an observable sequence and an iterable one (`SequenceEqual<TSource>.Enumerable`).
struct SequenceEqualIterObservable<T> {
    first: Rc<dyn IObservable<T>>,
    get_enumerator: GetEnumerator<T>,
    comparer: Comparer<T>,
}

/// The sink of `SequenceEqual` of an observable sequence and an iterable one
/// (`SequenceEqual<TSource>.Enumerable._`): each value is compared with the next of the iterator;
/// a difference, or an iterator that ends first, gives `false`; the end of the sequence gives
/// whether the iterator has ended too.
struct SequenceEqualIterSink<T> {
    sink: Sink<bool>,
    comparer: Comparer<T>,
    enumerator: RefCell<Option<Enumerator<T>>>,
}

impl<T: 'static> SequenceEqualIterSink<T> {
    /// The next item of the iterator; `None` once the sink is disposed (the disposed enumerator of
    /// upstream, which has no items).
    fn move_next(&self) -> Option<Result<T, ObservableError>> {
        self.enumerator.borrow_mut().as_mut().and_then(|enumerator| enumerator.next())
    }
}

impl<T: 'static> IDisposable for SequenceEqualIterSink<T> {
    fn dispose(&self) {
        let enumerator = self.enumerator.borrow_mut().take();
        drop(enumerator);
        self.sink.dispose();
    }
}

impl<T: 'static> IObserver<T> for SequenceEqualIterSink<T> {
    fn on_next(&self, value: T) {
        let equal = match self.move_next() {
            None => Ok(false),
            Some(Ok(current)) => (self.comparer)(&value, &current),
            Some(Err(error)) => Err(error),
        };
        match equal {
            Ok(true) => {}
            Ok(false) => {
                self.sink.forward_on_next(false);
                self.sink.forward_on_completed();
            }
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let has_next = match self.move_next() {
            None => false,
            Some(Ok(_)) => true,
            Some(Err(error)) => {
                self.sink.forward_on_error(error);
                return;
            }
        };
        self.sink.forward_on_next(!has_next);
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IObservable<bool> for SequenceEqualIterObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        let (first, get_enumerator, comparer) = (self.first.clone(), self.get_enumerator.clone(), self.comparer.clone());
        subscribe_sink(
            observer,
            move |observer| Rc::new(SequenceEqualIterSink { sink: Sink::new(observer), comparer, enumerator: RefCell::new(None) }),
            move |sink: &Rc<SequenceEqualIterSink<T>>| {
                // The iterator is obtained before the subscription, so that it is there when the
                // first value arrives (the order of upstream's `Run`).
                match get_enumerator() {
                    Err(error) => sink.sink.forward_on_error(error),
                    Ok(enumerator) => {
                        if sink.sink.is_disposed() {
                            return;
                        }
                        *sink.enumerator.borrow_mut() = Some(enumerator);
                        let upstream = first.subscribe(sink.clone());
                        sink.sink.set_upstream(upstream);
                    }
                }
            },
        )
    }
}

pub(crate) fn sequence_equal<T: 'static>(first: Rc<dyn IObservable<T>>, second: Rc<dyn IObservable<T>>, comparer: Comparer<T>) -> Rc<dyn IObservable<bool>> {
    Rc::new(SequenceEqualObservable { first, second, comparer })
}

pub(crate) fn sequence_equal_enumerator<T: 'static>(
    first: Rc<dyn IObservable<T>>,
    get_enumerator: GetEnumerator<T>,
    comparer: Comparer<T>,
) -> Rc<dyn IObservable<bool>> {
    Rc::new(SequenceEqualIterObservable { first, get_enumerator, comparer })
}

/// `SequenceEqual` with an iterable that is iterated anew (a clone of it) for each subscription.
pub(crate) fn sequence_equal_iter<T: 'static, I>(first: Rc<dyn IObservable<T>>, second: I, comparer: Comparer<T>) -> Rc<dyn IObservable<bool>>
where
    I: IntoIterator<Item = T> + Clone + 'static,
    I::IntoIter: 'static,
{
    let get_enumerator: GetEnumerator<T> = Rc::new(move || {
        let enumerator: Enumerator<T> = Box::new(second.clone().into_iter().map(Ok::<T, ObservableError>));
        Ok(enumerator)
    });
    sequence_equal_enumerator(first, get_enumerator, comparer)
}

/// `SequenceEqual` with an iterator that `get_iterator` gives for each subscription, whose
/// creation and steps may fail.
pub(crate) fn try_sequence_equal_iter<T: 'static, I, F>(first: Rc<dyn IObservable<T>>, get_iterator: F, comparer: Comparer<T>) -> Rc<dyn IObservable<bool>>
where
    F: Fn() -> Result<I, ObservableError> + 'static,
    I: IntoIterator<Item = Result<T, ObservableError>>,
    I::IntoIter: 'static,
{
    let get_enumerator: GetEnumerator<T> = Rc::new(move || {
        let enumerator: Enumerator<T> = Box::new(get_iterator()?.into_iter());
        Ok(enumerator)
    });
    sequence_equal_enumerator(first, get_enumerator, comparer)
}

/// The methods of `SequenceEqual` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! sequence_equal_methods {
    () => {
        /// Whether the sequence and `second` have equal values in the same order
        /// (`SequenceEqual(IObservable<TSource>)`).
        fn sequence_equal(&self, second: std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>>
        where
            T: PartialEq,
        {
            crate::linq::sequence_equal::sequence_equal(self.observable(), second, std::rc::Rc::new(|x: &T, y: &T| Ok(x == y)))
        }

        /// Whether the sequence and `second` have values that `comparer` finds equal, in the same
        /// order; the comparer takes a value of the sequence and one of `second`
        /// (`SequenceEqual(IObservable<TSource>, IEqualityComparer<TSource>)`).
        fn sequence_equal_with(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
            comparer: impl Fn(&T, &T) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::sequence_equal::sequence_equal(self.observable(), second, std::rc::Rc::new(move |x: &T, y: &T| Ok(comparer(x, y))))
        }

        /// [`sequence_equal_with`](Self::sequence_equal_with) with a comparer that may fail; a
        /// failure ends the sequence with its error (`SequenceEqual` with a comparer that throws).
        fn try_sequence_equal_with(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
            comparer: impl Fn(&T, &T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>> {
            crate::linq::sequence_equal::sequence_equal(self.observable(), second, std::rc::Rc::new(comparer))
        }

        /// Whether the sequence and the iterable `second` have equal values in the same order;
        /// each subscription iterates a clone of `second` (`SequenceEqual(IEnumerable<TSource>)`).
        fn sequence_equal_iter<I>(&self, second: I) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>>
        where
            T: PartialEq,
            I: IntoIterator<Item = T> + Clone + 'static,
            I::IntoIter: 'static,
        {
            crate::linq::sequence_equal::sequence_equal_iter(self.observable(), second, std::rc::Rc::new(|x: &T, y: &T| Ok(x == y)))
        }

        /// Whether the sequence and the iterable `second` have values that `comparer` finds equal,
        /// in the same order (`SequenceEqual(IEnumerable<TSource>, IEqualityComparer<TSource>)`).
        fn sequence_equal_iter_with<I>(&self, second: I, comparer: impl Fn(&T, &T) -> bool + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>>
        where
            I: IntoIterator<Item = T> + Clone + 'static,
            I::IntoIter: 'static,
        {
            crate::linq::sequence_equal::sequence_equal_iter(self.observable(), second, std::rc::Rc::new(move |x: &T, y: &T| Ok(comparer(x, y))))
        }

        /// [`sequence_equal_iter_with`](Self::sequence_equal_iter_with) where everything may fail:
        /// `get_iterator` gives the iterator for each subscription or fails
        /// (`GetEnumerator` that throws), each item of the iterator may be an error (`MoveNext`
        /// that throws), and so may the comparer; a failure ends the sequence with its error.
        fn try_sequence_equal_iter_with<I>(
            &self,
            get_iterator: impl Fn() -> Result<I, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&T, &T) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<bool>>
        where
            I: IntoIterator<Item = Result<T, ferroui_system::ObservableError>>,
            I::IntoIter: 'static,
        {
            crate::linq::sequence_equal::try_sequence_equal_iter(self.observable(), get_iterator, std::rc::Rc::new(comparer))
        }
    };
}
pub(crate) use sequence_equal_methods;
