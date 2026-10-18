//! `ForkJoin` (`Linq/QueryLanguageEx.cs`, the experimental `ObservableEx.ForkJoin`): the last
//! values of sequences that have all completed.

use crate::disposables::{CompositeDisposable, Disposable, SingleAssignmentDisposable, StableCompositeDisposable};
use crate::internal::subscribe_auto_detach;
use crate::linq::{Observable, ObservableExt};
use crate::notification::Notification;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ResultSelector<L, R, U> = Rc<dyn Fn(L, R) -> Result<U, ObservableError>>;

/// `ForkJoin<TFirst, TSecond, TResult>`, a `CombineObservable` (an `ObservableBase<TResult>`) of the
/// materialized sequences.
struct ForkJoinBinary<L, R, U> {
    first: Rc<dyn IObservable<L>>,
    second: Rc<dyn IObservable<R>>,
    result_selector: ResultSelector<L, R, U>,
}

/// The state of the combiner of a subscription (the closure of `ForkJoin`'s combiner selector).
struct BinaryState<L, R, U> {
    observer: Rc<dyn IObserver<U>>,
    left_subscription: Rc<SingleAssignmentDisposable>,
    right_subscription: Rc<SingleAssignmentDisposable>,
    result_selector: ResultSelector<L, R, U>,
    left_stopped: Cell<bool>,
    right_stopped: Cell<bool>,
    last_left: RefCell<Option<L>>,
    last_right: RefCell<Option<R>>,
}

impl<L: 'static, R: 'static, U: 'static> BinaryState<L, R, U> {
    /// The end of both sequences: the projection of their last values, or the completion when one
    /// of them had none.
    fn both_stopped(&self) {
        let left = self.last_left.borrow_mut().take();
        let right = self.last_right.borrow_mut().take();
        match (left, right) {
            (Some(left), Some(right)) => match (self.result_selector)(left, right) {
                Ok(result) => {
                    self.observer.on_next(result);
                    self.observer.on_completed();
                }
                Err(error) => self.observer.on_error(error),
            },
            _ => self.observer.on_completed(),
        }
    }

    fn left(&self, notification: Notification<L>) {
        match notification {
            Notification::OnNext(value) => *self.last_left.borrow_mut() = Some(value),
            Notification::OnError(error) => {
                self.right_subscription.dispose();
                self.observer.on_error(error);
            }
            Notification::OnCompleted => {
                self.left_stopped.set(true);
                if self.right_stopped.get() {
                    self.both_stopped();
                }
            }
        }
    }

    fn right(&self, notification: Notification<R>) {
        match notification {
            Notification::OnNext(value) => *self.last_right.borrow_mut() = Some(value),
            Notification::OnError(error) => {
                self.left_subscription.dispose();
                self.observer.on_error(error);
            }
            Notification::OnCompleted => {
                self.right_stopped.set(true);
                if self.left_stopped.get() {
                    self.both_stopped();
                }
            }
        }
    }
}

/// An observer of a materialized sequence that hands each notification to `handler` and ignores
/// the end of the materialized sequence (one side of `BinaryObserver<TLeft, TRight>`).
struct NotificationObserver<T> {
    handler: Box<dyn Fn(Notification<T>)>,
}

impl<T> IObserver<Notification<T>> for NotificationObserver<T> {
    fn on_next(&self, notification: Notification<T>) {
        (self.handler)(notification);
    }

    fn on_error(&self, _error: ObservableError) {}

    fn on_completed(&self) {}
}

impl<L: 'static, R: 'static, U: 'static> IObservable<U> for ForkJoinBinary<L, R, U> {
    fn subscribe(&self, observer: Rc<dyn IObserver<U>>) -> Rc<dyn IDisposable> {
        let (first, second, result_selector) = (self.first.clone(), self.second.clone(), self.result_selector.clone());
        subscribe_auto_detach(observer, move |observer: Rc<dyn IObserver<U>>| -> Rc<dyn IDisposable> {
            // `CombineObservable<TLeft, TRight, TResult>.SubscribeCore`; the `Synchronize(gate)` of
            // upstream has nothing to serialize on one thread.
            let left_subscription = SingleAssignmentDisposable::new();
            let right_subscription = SingleAssignmentDisposable::new();
            let state = Rc::new(BinaryState {
                observer,
                left_subscription: left_subscription.clone(),
                right_subscription: right_subscription.clone(),
                result_selector,
                left_stopped: Cell::new(false),
                right_stopped: Cell::new(false),
                last_left: RefCell::new(None),
                last_right: RefCell::new(None),
            });

            let left_state = state.clone();
            let left: Rc<dyn IObserver<Notification<L>>> =
                Rc::new(NotificationObserver { handler: Box::new(move |notification| left_state.left(notification)) });
            left_subscription.set_disposable(Some(first.materialize().subscribe(left)));

            let right_state = state;
            let right: Rc<dyn IObserver<Notification<R>>> =
                Rc::new(NotificationObserver { handler: Box::new(move |notification| right_state.right(notification)) });
            right_subscription.set_disposable(Some(second.materialize().subscribe(right)));

            let subscription: Rc<dyn IDisposable> = StableCompositeDisposable::create(left_subscription, right_subscription);
            subscription
        })
    }
}

pub(crate) fn fork_join<L: 'static, R: 'static, U: 'static>(
    first: Rc<dyn IObservable<L>>,
    second: Rc<dyn IObservable<R>>,
    result_selector: ResultSelector<L, R, U>,
) -> Rc<dyn IObservable<U>> {
    Rc::new(ForkJoinBinary { first, second, result_selector })
}

/// `ForkJoinObservable<TSource>` (an `ObservableBase<TSource[]>`): each subscription takes the
/// sources of the enumerable (`ToArray`).
struct ForkJoinObservable<I> {
    sources: I,
}

/// The state of a subscription of [`ForkJoinObservable`].
struct NaryState<T> {
    observer: Rc<dyn IObserver<Vec<T>>>,
    group: Rc<CompositeDisposable>,
    finished: Cell<bool>,
    has_results: RefCell<Vec<bool>>,
    has_completed: RefCell<Vec<bool>>,
    results: RefCell<Vec<Option<T>>>,
}

impl<T: 'static> NaryState<T> {
    fn on_next(&self, index: usize, value: T) {
        if !self.finished.get() {
            self.has_results.borrow_mut()[index] = true;
            self.results.borrow_mut()[index] = Some(value);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.finished.set(true);
        self.observer.on_error(error);
        self.group.dispose();
    }

    fn on_completed(&self, index: usize) {
        if self.finished.get() {
            return;
        }
        if !self.has_results.borrow()[index] {
            self.observer.on_completed();
            return;
        }
        let all_completed = {
            let mut has_completed = self.has_completed.borrow_mut();
            has_completed[index] = true;
            has_completed.iter().all(|completed| *completed)
        };
        if !all_completed {
            return;
        }
        self.finished.set(true);
        let results: Vec<T> = self.results.borrow_mut().iter_mut().map(|result| result.take().expect("each source has a value")).collect();
        self.observer.on_next(results);
        self.observer.on_completed();
    }
}

impl<T: 'static, I> IObservable<Vec<T>> for ForkJoinObservable<I>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<Vec<T>>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        subscribe_auto_detach(observer, move |observer: Rc<dyn IObserver<Vec<T>>>| -> Rc<dyn IDisposable> {
            let all_sources: Vec<Rc<dyn IObservable<T>>> = sources.into_iter().collect();
            let count = all_sources.len();
            if count == 0 {
                observer.on_completed();
                return Disposable::empty();
            }

            let group = CompositeDisposable::with_capacity(count);
            let state = Rc::new(NaryState {
                observer,
                group: group.clone(),
                finished: Cell::new(false),
                has_results: RefCell::new(vec![false; count]),
                has_completed: RefCell::new(vec![false; count]),
                results: RefCell::new(Vec::with_capacity(count)),
            });

            for (index, source) in all_sources.into_iter().enumerate() {
                // Reserves the place of the value of the source.
                state.results.borrow_mut().push(None);
                let (on_next, on_error, on_completed) = (state.clone(), state.clone(), state.clone());
                group.add(source.subscribe_all(
                    move |value| on_next.on_next(index, value),
                    move |error| on_error.on_error(error),
                    move || on_completed.on_completed(index),
                ));
            }
            group
        })
    }
}

impl Observable {
    /// The last values of `sources`, in their order, once all of them have completed; a source that
    /// completes without a value completes the result without one, and the first error ends it
    /// (`ForkJoin<TSource>(params IObservable<TSource>[])`, `ForkJoin<TSource>(IEnumerable<IObservable<TSource>>)`).
    /// Each subscription takes the sources of `sources` anew.
    pub fn fork_join_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<Vec<T>>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        Rc::new(ForkJoinObservable { sources })
    }
}

/// The methods of `ForkJoin` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! fork_join_methods {
    () => {
        /// The projection of the last values of the sequence and of `second`, once both have
        /// completed; when one of them has no value the result completes without one, and the
        /// first error ends it (`ForkJoin<TSource1, TSource2, TResult>(IObservable<TSource1>,
        /// IObservable<TSource2>, Func<TSource1, TSource2, TResult>)`).
        fn fork_join<R: 'static, U: 'static>(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<R>>,
            result_selector: impl Fn(T, R) -> U + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::fork_join::fork_join(self.observable(), second, std::rc::Rc::new(move |left, right| Ok(result_selector(left, right))))
        }

        /// [`fork_join`](Self::fork_join) with a selector that may fail; a failure ends the
        /// sequence with its error (`ForkJoin` with a selector that throws).
        fn try_fork_join<R: 'static, U: 'static>(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<R>>,
            result_selector: impl Fn(T, R) -> Result<U, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::fork_join::fork_join(self.observable(), second, std::rc::Rc::new(result_selector))
        }
    };
}
pub(crate) use fork_join_methods;
