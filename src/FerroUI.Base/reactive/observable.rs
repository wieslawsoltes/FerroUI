use super::operators::{CombineLatest, CombineLatestEnumerable, Switch};
use super::{AnonymousObserver, Disposable, IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

/// Factory functions for observables.
pub struct Observable;

struct CreateObservable<T, F> {
    subscribe: F,
    _marker: PhantomData<fn(T)>,
}

impl<T, F> IObservable<T> for CreateObservable<T, F>
where
    F: Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        (self.subscribe)(observer)
    }
}

impl Observable {
    /// Creates an observable from a subscribe function.
    pub fn create<T: 'static>(
        subscribe: impl Fn(Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Rc::new(CreateObservable { subscribe, _marker: PhantomData })
    }

    /// An observable that produces a single value and completes.
    pub fn return_<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer| {
            observer.on_next(value.clone());
            observer.on_completed();
            Disposable::empty()
        })
    }

    /// An observable that produces a single value and never completes.
    pub fn single_value<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer| {
            observer.on_next(value.clone());
            Disposable::empty()
        })
    }

    /// An observable that never produces a value.
    pub fn never<T: 'static>() -> Rc<dyn IObservable<T>> {
        Self::create(|_| Disposable::empty())
    }

    /// An observable that completes immediately.
    pub fn empty<T: 'static>() -> Rc<dyn IObservable<T>> {
        Self::create(|observer| {
            observer.on_completed();
            Disposable::empty()
        })
    }

    /// Forwards the values of the most recent of the observables that
    /// `sources` produces.
    pub fn switch<TSource: 'static>(
        sources: Rc<dyn IObservable<Rc<dyn IObservable<TSource>>>>,
    ) -> Rc<dyn IObservable<TSource>> {
        Rc::new(Switch::new(sources))
    }

    /// Combines the latest values of a sequence of observables: once every
    /// one of them has produced a value, each new value produces the list of
    /// the latest ones.
    pub fn combine_latest_all<TInput: Clone + 'static>(
        inputs: impl IntoIterator<Item = Rc<dyn IObservable<TInput>>>,
    ) -> Rc<dyn IObservable<Vec<TInput>>> {
        Rc::new(CombineLatestEnumerable::<TInput, Vec<TInput>>::new(inputs, Rc::new(|items: &[TInput]| items.to_vec())))
    }

    /// An observable of the arguments of an event.
    ///
    /// `add_handler` adds the handler it is given to the event and returns
    /// the handle that removes it, as the events of this crate do (the
    /// original takes one function to add and one to remove a handler, for
    /// events with and without typed arguments).
    pub fn from_event_pattern<T: Clone + 'static>(
        add_handler: impl Fn(Rc<dyn Fn(&T)>) -> Rc<dyn IDisposable> + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Self::create(move |observer: Rc<dyn IObserver<T>>| {
            let converted: Rc<dyn Fn(&T)> = Rc::new(move |args: &T| observer.on_next(args.clone()));
            add_handler(converted)
        })
    }
}

struct MapObserver<T, U, F> {
    target: Rc<dyn IObserver<U>>,
    f: Rc<F>,
    _marker: PhantomData<fn(T)>,
}

impl<T, U, F: Fn(T) -> U> IObserver<T> for MapObserver<T, U, F> {
    fn on_next(&self, value: T) {
        self.target.on_next((self.f)(value))
    }
    fn on_error(&self, error: ObservableError) {
        self.target.on_error(error)
    }
    fn on_completed(&self) {
        self.target.on_completed()
    }
}

struct FilterObserver<T, F> {
    target: Rc<dyn IObserver<T>>,
    f: Rc<F>,
}

impl<T, F: Fn(&T) -> bool> IObserver<T> for FilterObserver<T, F> {
    fn on_next(&self, value: T) {
        if (self.f)(&value) {
            self.target.on_next(value)
        }
    }
    fn on_error(&self, error: ObservableError) {
        self.target.on_error(error)
    }
    fn on_completed(&self) {
        self.target.on_completed()
    }
}

/// Operators available on every shared observable.
pub trait ObservableExt<T: 'static> {
    /// Subscribes with a closure invoked for every value.
    fn subscribe_fn(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable>;

    /// Projects each value.
    fn select<U: 'static>(&self, f: impl Fn(T) -> U + 'static) -> Rc<dyn IObservable<U>>;

    /// Filters values.
    fn where_(&self, f: impl Fn(&T) -> bool + 'static) -> Rc<dyn IObservable<T>>;

    /// Produces `value` to each observer before the values of the source.
    fn start_with(&self, value: T) -> Rc<dyn IObservable<T>>
    where
        T: Clone;

    /// Combines the latest values of this observable and `second` with
    /// `result_selector`, once both have produced a value.
    fn combine_latest<TSecond: Clone + 'static, TResult: 'static>(
        &self,
        second: &Rc<dyn IObservable<TSecond>>,
        result_selector: impl Fn(T, TSecond) -> TResult + 'static,
    ) -> Rc<dyn IObservable<TResult>>
    where
        T: Clone;

    /// Leaves out the first `skip_count` values.
    ///
    /// Panics when `skip_count` is not bigger than zero.
    fn skip(&self, skip_count: i32) -> Rc<dyn IObservable<T>>;

    /// Produces the first `take_count` values and completes.
    fn take(&self, take_count: i32) -> Rc<dyn IObservable<T>>;
}

impl<T: 'static> ObservableExt<T> for Rc<dyn IObservable<T>> {
    fn subscribe_fn(&self, on_next: impl Fn(T) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe(Rc::new(AnonymousObserver::new(on_next)))
    }

    fn select<U: 'static>(&self, f: impl Fn(T) -> U + 'static) -> Rc<dyn IObservable<U>> {
        let source = self.clone();
        let f = Rc::new(f);
        Observable::create(move |observer| {
            source.subscribe(Rc::new(MapObserver { target: observer, f: f.clone(), _marker: PhantomData }))
        })
    }

    fn where_(&self, f: impl Fn(&T) -> bool + 'static) -> Rc<dyn IObservable<T>> {
        let source = self.clone();
        let f = Rc::new(f);
        Observable::create(move |observer| {
            source.subscribe(Rc::new(FilterObserver { target: observer, f: f.clone() }))
        })
    }

    fn start_with(&self, value: T) -> Rc<dyn IObservable<T>>
    where
        T: Clone,
    {
        let source = self.clone();
        Observable::create(move |obs: Rc<dyn IObserver<T>>| {
            obs.on_next(value.clone());
            source.subscribe(obs)
        })
    }

    fn combine_latest<TSecond: Clone + 'static, TResult: 'static>(
        &self,
        second: &Rc<dyn IObservable<TSecond>>,
        result_selector: impl Fn(T, TSecond) -> TResult + 'static,
    ) -> Rc<dyn IObservable<TResult>>
    where
        T: Clone,
    {
        let result_selector: Rc<dyn Fn(T, TSecond) -> TResult> = Rc::new(result_selector);
        Rc::new(CombineLatest::new(self.clone(), second.clone(), result_selector))
    }

    fn skip(&self, skip_count: i32) -> Rc<dyn IObservable<T>> {
        if skip_count <= 0 {
            panic!("Skip count must be bigger than zero (Parameter 'skipCount')");
        }

        let source = self.clone();
        Observable::create(move |obs: Rc<dyn IObserver<T>>| {
            let remaining = Cell::new(skip_count);
            let (next, error, completed) = (obs.clone(), obs.clone(), obs);
            source.subscribe(Rc::new(AnonymousObserver::new_with_error_and_completed(
                move |input: T| {
                    if remaining.get() <= 0 {
                        next.on_next(input);
                    } else {
                        remaining.set(remaining.get() - 1);
                    }
                },
                move |e| error.on_error(e),
                move || completed.on_completed(),
            )))
        })
    }

    fn take(&self, take_count: i32) -> Rc<dyn IObservable<T>> {
        if take_count <= 0 {
            return Observable::empty();
        }

        let source = self.clone();
        Observable::create(move |obs: Rc<dyn IObserver<T>>| {
            let remaining = Cell::new(take_count);
            let sub: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
            let observer_sub = sub.clone();
            let (next, error, completed) = (obs.clone(), obs.clone(), obs);
            let subscription = source.subscribe(Rc::new(AnonymousObserver::new_with_error_and_completed(
                move |input: T| {
                    if remaining.get() > 0 {
                        remaining.set(remaining.get() - 1);
                        next.on_next(input);

                        if remaining.get() == 0 {
                            let sub = observer_sub.borrow_mut().take();
                            if let Some(sub) = sub {
                                sub.dispose();
                            }
                            next.on_completed();
                        }
                    }
                },
                move |e| error.on_error(e),
                move || completed.on_completed(),
            )));
            *sub.borrow_mut() = Some(subscription.clone());
            subscription
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of the operators.
    use super::*;
    use crate::reactive::LightweightSubject;

    type Collected<T> = (Rc<RefCell<Vec<T>>>, Rc<Cell<bool>>, Rc<dyn IDisposable>);

    fn collect<T: 'static>(source: &Rc<dyn IObservable<T>>) -> Collected<T> {
        let received = Rc::new(RefCell::new(Vec::new()));
        let completed = Rc::new(Cell::new(false));
        let (r, c) = (received.clone(), completed.clone());
        let subscription = source.subscribe(Rc::new(AnonymousObserver::new_with_completed(
            move |value: T| r.borrow_mut().push(value),
            move || c.set(true),
        )));
        (received, completed, subscription)
    }

    fn subject() -> (LightweightSubject<i32>, Rc<dyn IObservable<i32>>) {
        let subject = LightweightSubject::<i32>::new();
        let source: Rc<dyn IObservable<i32>> = Rc::new(subject.clone());
        (subject, source)
    }

    #[test]
    fn start_with_produces_the_value_first() {
        let (subject, source) = subject();
        let (received, _, _subscription) = collect(&source.start_with(0));

        subject.on_next(1);
        assert_eq!(vec![0, 1], *received.borrow());
    }

    #[test]
    fn skip_leaves_out_the_first_values() {
        let (subject, source) = subject();
        let (received, completed, _subscription) = collect(&source.skip(2));

        for value in 1..=4 {
            subject.on_next(value);
        }
        subject.on_completed();
        assert_eq!(vec![3, 4], *received.borrow());
        assert!(completed.get());
    }

    #[test]
    #[should_panic(expected = "Skip count must be bigger than zero")]
    fn skip_throws_for_a_count_of_zero() {
        let (_, source) = subject();
        source.skip(0);
    }

    #[test]
    fn take_produces_the_first_values_and_completes() {
        let (subject, source) = subject();
        let (received, completed, _subscription) = collect(&source.take(2));

        subject.on_next(1);
        assert!(!completed.get());
        subject.on_next(2);
        assert!(completed.get());
        assert!(!subject.has_observers());
        subject.on_next(3);
        assert_eq!(vec![1, 2], *received.borrow());
    }

    #[test]
    fn take_of_nothing_completes_at_once() {
        let (subject, source) = subject();
        let (received, completed, _subscription) = collect(&source.take(0));

        assert!(completed.get());
        assert!(!subject.has_observers());
        assert!(received.borrow().is_empty());
    }

    #[test]
    fn combine_latest_and_switch_are_reachable_as_operators() {
        let (first, first_source) = subject();
        let (second, second_source) = subject();
        let (sums, _, _sum_subscription) = collect(&first_source.combine_latest(&second_source, |a: i32, b: i32| a + b));

        let sources = LightweightSubject::<Rc<dyn IObservable<i32>>>::new();
        let outer: Rc<dyn IObservable<Rc<dyn IObservable<i32>>>> = Rc::new(sources.clone());
        let (switched, _, _switch_subscription) = collect(&Observable::switch(outer));

        let all = Observable::combine_latest_all([first_source.clone(), second_source.clone()]);
        let (lists, _, _list_subscription) = collect(&all);

        sources.on_next(first_source.clone());
        first.on_next(1);
        second.on_next(10);
        sources.on_next(second_source.clone());
        first.on_next(2);
        second.on_next(20);

        assert_eq!(vec![11, 12, 22], *sums.borrow());
        assert_eq!(vec![1, 20], *switched.borrow());
        assert_eq!(vec![vec![1, 10], vec![2, 10], vec![2, 20]], *lists.borrow());
    }

    #[test]
    fn from_event_pattern_adds_and_removes_a_handler() {
        type Handler = Rc<dyn Fn(&i32)>;
        let handlers: Rc<RefCell<Vec<Handler>>> = Rc::new(RefCell::new(Vec::new()));
        let h = handlers.clone();
        let source = Observable::from_event_pattern(move |handler: Handler| {
            h.borrow_mut().push(handler);
            let h = h.clone();
            Disposable::create(move || h.borrow_mut().clear())
        });
        let (received, _, subscription) = collect(&source);

        let handler = handlers.borrow()[0].clone();
        handler(&5);
        assert_eq!(vec![5], *received.borrow());

        subscription.dispose();
        assert!(handlers.borrow().is_empty());
    }
}
