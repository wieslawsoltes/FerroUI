// Code based on https://github.com/dotnet/reactive/blob/main/Rx.NET/Source/src/System.Reactive/Linq/Observable/CombineLatest.cs

use super::sink::{ISink, Sink, SubscriptionSlot};
use crate::reactive::{CompositeDisposable, IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Combines the latest values of two observables with a function.
pub(crate) struct CombineLatest<TFirst, TSecond, TResult> {
    first: Rc<dyn IObservable<TFirst>>,
    second: Rc<dyn IObservable<TSecond>>,
    result_selector: Rc<dyn Fn(TFirst, TSecond) -> TResult>,
}

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> CombineLatest<TFirst, TSecond, TResult> {
    pub fn new(
        first: Rc<dyn IObservable<TFirst>>,
        second: Rc<dyn IObservable<TSecond>>,
        result_selector: Rc<dyn Fn(TFirst, TSecond) -> TResult>,
    ) -> Self {
        Self { first, second, result_selector }
    }
}

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> IObservable<TResult>
    for CombineLatest<TFirst, TSecond, TResult>
{
    fn subscribe(&self, observer: Rc<dyn IObserver<TResult>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(CombineLatestSink::new(self.result_selector.clone(), observer));
        CombineLatestSink::run(&sink, &self.first, &self.second);
        Rc::new(SinkDisposable(sink))
    }
}

/// The handle of a subscription: disposing it disposes the sink.
struct SinkDisposable<S>(Rc<S>);

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> IDisposable
    for SinkDisposable<CombineLatestSink<TFirst, TSecond, TResult>>
{
    fn dispose(&self) {
        ISink::dispose(&*self.0);
    }
}

impl<TSource: Clone + 'static, TResult: 'static> IDisposable
    for SinkDisposable<CombineLatestEnumerableSink<TSource, TResult>>
{
    fn dispose(&self) {
        ISink::dispose(&*self.0);
    }
}

/// What one of the two observers of a [`CombineLatestSink`] has seen.
struct ObserverState<T> {
    has_value: Cell<bool>,
    value: RefCell<Option<T>>,
    done: Cell<bool>,
}

impl<T: Clone> ObserverState<T> {
    fn new() -> Self {
        Self { has_value: Cell::new(false), value: RefCell::new(None), done: Cell::new(false) }
    }

    fn set_value(&self, value: T) {
        self.has_value.set(true);
        *self.value.borrow_mut() = Some(value);
    }

    fn value(&self) -> Option<T> {
        self.value.borrow().clone()
    }
}

/// The subscription of a [`CombineLatest`] (the original's nested class `_`).
struct CombineLatestSink<TFirst, TSecond, TResult> {
    sink: Sink<TResult>,
    result_selector: Rc<dyn Fn(TFirst, TSecond) -> TResult>,
    first_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    second_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    first: ObserverState<TFirst>,
    second: ObserverState<TSecond>,
}

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> CombineLatestSink<TFirst, TSecond, TResult> {
    fn new(result_selector: Rc<dyn Fn(TFirst, TSecond) -> TResult>, observer: Rc<dyn IObserver<TResult>>) -> Self {
        Self {
            sink: Sink::new(observer),
            result_selector,
            first_disposable: RefCell::new(None),
            second_disposable: RefCell::new(None),
            first: ObserverState::new(),
            second: ObserverState::new(),
        }
    }

    fn run(this: &Rc<Self>, first: &Rc<dyn IObservable<TFirst>>, second: &Rc<dyn IObservable<TSecond>>) {
        let fst_o: Rc<dyn IObserver<TFirst>> = Rc::new(FirstObserver { parent: this.clone() });
        let snd_o: Rc<dyn IObserver<TSecond>> = Rc::new(SecondObserver { parent: this.clone() });

        let first_disposable = first.subscribe(fst_o);
        *this.first_disposable.borrow_mut() = Some(first_disposable);
        let second_disposable = second.subscribe(snd_o);
        *this.second_disposable.borrow_mut() = Some(second_disposable);
    }

    fn dispose_slot(slot: &RefCell<Option<Rc<dyn IDisposable>>>) {
        // A source that completes while it is being subscribed to has no
        // subscription yet. (The original fails on the missing subscription.)
        let disposable = slot.borrow().clone();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }
}

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> ISink<TResult>
    for CombineLatestSink<TFirst, TSecond, TResult>
{
    fn sink(&self) -> &Sink<TResult> {
        &self.sink
    }

    fn dispose(&self) {
        Self::dispose_slot(&self.first_disposable);
        Self::dispose_slot(&self.second_disposable);

        self.sink.dispose();
    }
}

struct FirstObserver<TFirst, TSecond, TResult> {
    parent: Rc<CombineLatestSink<TFirst, TSecond, TResult>>,
}

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> IObserver<TFirst>
    for FirstObserver<TFirst, TSecond, TResult>
{
    fn on_next(&self, value: TFirst) {
        let parent = &self.parent;
        parent.first.set_value(value.clone());

        if parent.second.has_value.get() {
            if let Some(other) = parent.second.value() {
                let res = (parent.result_selector)(value, other);
                parent.forward_on_next(res);
            }
        } else if parent.second.done.get() {
            parent.forward_on_completed();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        parent.first.done.set(true);

        if parent.second.done.get() {
            parent.forward_on_completed();
        } else {
            CombineLatestSink::<TFirst, TSecond, TResult>::dispose_slot(&parent.first_disposable);
        }
    }
}

struct SecondObserver<TFirst, TSecond, TResult> {
    parent: Rc<CombineLatestSink<TFirst, TSecond, TResult>>,
}

impl<TFirst: Clone + 'static, TSecond: Clone + 'static, TResult: 'static> IObserver<TSecond>
    for SecondObserver<TFirst, TSecond, TResult>
{
    fn on_next(&self, value: TSecond) {
        let parent = &self.parent;
        parent.second.set_value(value.clone());

        if parent.first.has_value.get() {
            if let Some(other) = parent.first.value() {
                let res = (parent.result_selector)(other, value);
                parent.forward_on_next(res);
            }
        } else if parent.first.done.get() {
            parent.forward_on_completed();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        parent.second.done.set(true);

        if parent.first.done.get() {
            parent.forward_on_completed();
        } else {
            CombineLatestSink::<TFirst, TSecond, TResult>::dispose_slot(&parent.second_disposable);
        }
    }
}

/// Combines the latest values of a sequence of observables with a function
/// (the original's `CombineLatest` with two type parameters).
pub(crate) struct CombineLatestEnumerable<TSource, TResult> {
    sources: Vec<Rc<dyn IObservable<TSource>>>,
    result_selector: Rc<dyn Fn(&[TSource]) -> TResult>,
}

impl<TSource: Clone + 'static, TResult: 'static> CombineLatestEnumerable<TSource, TResult> {
    pub fn new(
        sources: impl IntoIterator<Item = Rc<dyn IObservable<TSource>>>,
        result_selector: Rc<dyn Fn(&[TSource]) -> TResult>,
    ) -> Self {
        Self { sources: sources.into_iter().collect(), result_selector }
    }
}

impl<TSource: Clone + 'static, TResult: 'static> IObservable<TResult> for CombineLatestEnumerable<TSource, TResult> {
    fn subscribe(&self, observer: Rc<dyn IObserver<TResult>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(CombineLatestEnumerableSink::new(self.result_selector.clone(), observer));
        CombineLatestEnumerableSink::run(&sink, &self.sources);
        Rc::new(SinkDisposable(sink))
    }
}

/// The subscription of a [`CombineLatestEnumerable`] (the original's nested
/// class `_`).
struct CombineLatestEnumerableSink<TSource, TResult> {
    sink: Sink<TResult>,
    result_selector: Rc<dyn Fn(&[TSource]) -> TResult>,
    has_value: RefCell<Vec<bool>>,
    has_value_all: Cell<bool>,
    values: RefCell<Vec<Option<TSource>>>,
    is_done: RefCell<Vec<bool>>,
    subscriptions: RefCell<Vec<Rc<SubscriptionSlot>>>,
}

impl<TSource: Clone + 'static, TResult: 'static> CombineLatestEnumerableSink<TSource, TResult> {
    fn new(result_selector: Rc<dyn Fn(&[TSource]) -> TResult>, observer: Rc<dyn IObserver<TResult>>) -> Self {
        Self {
            sink: Sink::new(observer),
            result_selector,
            has_value: RefCell::new(Vec::new()),
            has_value_all: Cell::new(false),
            values: RefCell::new(Vec::new()),
            is_done: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
        }
    }

    fn run(this: &Rc<Self>, sources: &[Rc<dyn IObservable<TSource>>]) {
        let n = sources.len();

        *this.has_value.borrow_mut() = vec![false; n];
        this.has_value_all.set(false);

        *this.values.borrow_mut() = (0..n).map(|_| None).collect();

        *this.is_done.borrow_mut() = vec![false; n];

        *this.subscriptions.borrow_mut() = (0..n).map(|_| SubscriptionSlot::new()).collect();

        for (j, source) in sources.iter().enumerate() {
            let slot = this.subscriptions.borrow()[j].clone();
            let o: Rc<dyn IObserver<TSource>> = Rc::new(SourceObserver { parent: this.clone(), index: j });

            slot.set(source.subscribe(o));
        }

        let subscriptions: Vec<Rc<dyn IDisposable>> = this
            .subscriptions
            .borrow()
            .iter()
            .map(|slot| {
                let disposable: Rc<dyn IDisposable> = slot.clone();
                disposable
            })
            .collect();
        this.sink.set_upstream(Rc::new(CompositeDisposable::from_disposables(subscriptions)));
    }

    fn on_next(&self, index: usize, value: TSource) {
        self.values.borrow_mut()[index] = Some(value);

        self.has_value.borrow_mut()[index] = true;

        if !self.has_value_all.get() {
            let all = self.has_value.borrow().iter().all(|v| *v);
            self.has_value_all.set(all);
        }

        if self.has_value_all.get() {
            let values: Vec<TSource> = self.values.borrow().iter().filter_map(|value| value.clone()).collect();
            let res = (self.result_selector)(&values);

            self.forward_on_next(res);
        } else {
            let others_done = self.is_done.borrow().iter().enumerate().filter(|(i, _)| *i != index).all(|(_, d)| *d);
            if others_done {
                self.forward_on_completed();
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.forward_on_error(error);
    }

    fn on_completed(&self, index: usize) {
        self.is_done.borrow_mut()[index] = true;

        let all_done = self.is_done.borrow().iter().all(|d| *d);
        if all_done {
            self.forward_on_completed();
        } else {
            let subscription = self.subscriptions.borrow()[index].clone();
            subscription.dispose();
        }
    }
}

impl<TSource: Clone + 'static, TResult: 'static> ISink<TResult> for CombineLatestEnumerableSink<TSource, TResult> {
    fn sink(&self) -> &Sink<TResult> {
        &self.sink
    }
}

struct SourceObserver<TSource, TResult> {
    parent: Rc<CombineLatestEnumerableSink<TSource, TResult>>,
    index: usize,
}

impl<TSource: Clone + 'static, TResult: 'static> IObserver<TSource> for SourceObserver<TSource, TResult> {
    fn on_next(&self, value: TSource) {
        self.parent.on_next(self.index, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_error(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed(self.index);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the original has no tests of these types.
    use super::*;
    use crate::reactive::{AnonymousObserver, LightweightSubject};

    fn observer<T: 'static>() -> (Rc<RefCell<Vec<T>>>, Rc<Cell<bool>>, Rc<dyn IObserver<T>>) {
        let received = Rc::new(RefCell::new(Vec::new()));
        let completed = Rc::new(Cell::new(false));
        let (r, c) = (received.clone(), completed.clone());
        let observer: Rc<dyn IObserver<T>> = Rc::new(AnonymousObserver::new_with_completed(
            move |value: T| r.borrow_mut().push(value),
            move || c.set(true),
        ));
        (received, completed, observer)
    }

    #[test]
    fn combines_the_latest_values_of_two_sources() {
        let first = LightweightSubject::<i32>::new();
        let second = LightweightSubject::<&'static str>::new();
        let (received, completed, observer) = observer::<String>();
        let target = CombineLatest::<i32, &'static str, String>::new(
            Rc::new(first.clone()),
            Rc::new(second.clone()),
            Rc::new(|a: i32, b: &'static str| format!("{a}{b}")),
        );
        let _subscription = target.subscribe(observer);

        first.on_next(1);
        assert!(received.borrow().is_empty());
        second.on_next("a");
        first.on_next(2);
        second.on_next("b");
        assert_eq!(vec!["1a", "2a", "2b"], *received.borrow());

        first.on_completed();
        assert!(!completed.get());
        second.on_next("c");
        second.on_completed();
        assert!(completed.get());
        assert_eq!(vec!["1a", "2a", "2b", "2c"], *received.borrow());
    }

    #[test]
    fn disposing_the_subscription_unsubscribes_from_both_sources() {
        let first = LightweightSubject::<i32>::new();
        let second = LightweightSubject::<i32>::new();
        let (_, _, observer) = observer::<i32>();
        let target =
            CombineLatest::<i32, i32, i32>::new(Rc::new(first.clone()), Rc::new(second.clone()), Rc::new(|a: i32, b: i32| a + b));

        let subscription = target.subscribe(observer);
        assert!(first.has_observers() && second.has_observers());

        subscription.dispose();
        assert!(!first.has_observers() && !second.has_observers());
    }

    #[test]
    fn combines_the_latest_values_of_a_sequence_of_sources() {
        let subjects = [LightweightSubject::<i32>::new(), LightweightSubject::<i32>::new(), LightweightSubject::<i32>::new()];
        let sources: Vec<Rc<dyn IObservable<i32>>> = subjects
            .iter()
            .map(|subject| {
                let source: Rc<dyn IObservable<i32>> = Rc::new(subject.clone());
                source
            })
            .collect();
        let (received, completed, observer) = observer::<Vec<i32>>();
        let target = CombineLatestEnumerable::<i32, Vec<i32>>::new(sources, Rc::new(|items: &[i32]| items.to_vec()));
        let subscription = target.subscribe(observer);

        subjects[0].on_next(1);
        subjects[1].on_next(2);
        assert!(received.borrow().is_empty());
        subjects[2].on_next(3);
        subjects[0].on_next(4);
        assert_eq!(vec![vec![1, 2, 3], vec![4, 2, 3]], *received.borrow());

        subjects[0].on_completed();
        subjects[1].on_completed();
        assert!(!completed.get());
        subjects[2].on_completed();
        assert!(completed.get());

        subscription.dispose();
    }
}
