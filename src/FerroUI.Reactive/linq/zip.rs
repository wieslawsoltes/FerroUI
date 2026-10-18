//! `Zip` (`Linq/Observable/Zip.cs`), with the operators of three to sixteen sources
//! (`Linq/Observable/Zip.NAry.cs`).
//!
//! Each source has a queue of the values it has notified that are not combined yet; a result is
//! made of the first value of every queue once each queue has one. The sequence completes when every
//! source has completed, or when a source notifies a value while its queue cannot be combined and
//! all the other sources have completed.

use crate::disposables::{SingleAssignmentDisposableValue, StableCompositeDisposable};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

type ResultSelector<T, U, R> = Rc<dyn Fn(T, U) -> Result<R, ObservableError>>;

// ---------------------------------------------------------------------------------------------
// Binary: `Zip<TFirst, TSecond, TResult>.Observable`.
// ---------------------------------------------------------------------------------------------

/// The sink of the binary `Zip` (`Zip<TFirst, TSecond, TResult>.Observable._`): the queues and
/// completion flags of both observers (`FirstObserver`, `SecondObserver`), and the subscription to
/// each source.
struct ZipSink<T, U, R> {
    sink: Sink<R>,
    result_selector: ResultSelector<T, U, R>,
    first_queue: RefCell<VecDeque<T>>,
    first_done: Cell<bool>,
    first_disposable: SingleAssignmentDisposableValue,
    second_queue: RefCell<VecDeque<U>>,
    second_done: Cell<bool>,
    second_disposable: SingleAssignmentDisposableValue,
}

impl<T: 'static, U: 'static, R: 'static> ZipSink<T, U, R> {
    fn run(self: &Rc<Self>, first: &Rc<dyn IObservable<T>>, second: &Rc<dyn IObservable<U>>) {
        let first_observer = Rc::new(FirstObserver { parent: self.clone() });
        let second_observer = Rc::new(SecondObserver { parent: self.clone() });
        self.first_disposable.set_disposable(Some(first.subscribe(first_observer)));
        self.second_disposable.set_disposable(Some(second.subscribe(second_observer)));
    }

    fn forward_result(&self, result: Result<R, ObservableError>) {
        match result {
            Ok(result) => self.sink.forward_on_next(result),
            Err(error) => self.sink.forward_on_error(error),
        }
    }
}

impl<T: 'static, U: 'static, R: 'static> IDisposable for ZipSink<T, U, R> {
    fn dispose(&self) {
        if self.sink.is_disposed() {
            return;
        }
        self.first_disposable.dispose();
        self.second_disposable.dispose();

        // The queues are cleared (`FirstObserver.Dispose`, `SecondObserver.Dispose`), outside of
        // their borrows.
        let first_values = std::mem::take(&mut *self.first_queue.borrow_mut());
        let second_values = std::mem::take(&mut *self.second_queue.borrow_mut());
        drop(first_values);
        drop(second_values);

        self.sink.dispose();
    }
}

/// The observer of the first sequence (`FirstObserver`).
struct FirstObserver<T, U, R> {
    parent: Rc<ZipSink<T, U, R>>,
}

impl<T: 'static, U: 'static, R: 'static> IObserver<T> for FirstObserver<T, U, R> {
    fn on_next(&self, value: T) {
        let parent = &self.parent;
        let other = parent.second_queue.borrow_mut().pop_front();
        match other {
            Some(r) => parent.forward_result((parent.result_selector)(value, r)),
            None => {
                if parent.second_done.get() {
                    parent.sink.forward_on_completed();
                    return;
                }
                parent.first_queue.borrow_mut().push_back(value);
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        parent.first_done.set(true);
        if parent.second_done.get() {
            parent.sink.forward_on_completed();
        } else {
            parent.first_disposable.dispose();
        }
    }
}

/// The observer of the second sequence (`SecondObserver`).
struct SecondObserver<T, U, R> {
    parent: Rc<ZipSink<T, U, R>>,
}

impl<T: 'static, U: 'static, R: 'static> IObserver<U> for SecondObserver<T, U, R> {
    fn on_next(&self, value: U) {
        let parent = &self.parent;
        let other = parent.first_queue.borrow_mut().pop_front();
        match other {
            Some(l) => parent.forward_result((parent.result_selector)(l, value)),
            None => {
                if parent.first_done.get() {
                    parent.sink.forward_on_completed();
                    return;
                }
                parent.second_queue.borrow_mut().push_back(value);
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        parent.second_done.set(true);
        if parent.first_done.get() {
            parent.sink.forward_on_completed();
        } else {
            parent.second_disposable.dispose();
        }
    }
}

/// The observable of the binary `Zip` (`Zip<TFirst, TSecond, TResult>.Observable`).
struct ZipObservable<T, U, R> {
    first: Rc<dyn IObservable<T>>,
    second: Rc<dyn IObservable<U>>,
    result_selector: ResultSelector<T, U, R>,
}

impl<T: 'static, U: 'static, R: 'static> IObservable<R> for ZipObservable<T, U, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let (first, second, result_selector) = (self.first.clone(), self.second.clone(), self.result_selector.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(ZipSink {
                    sink: Sink::new(observer),
                    result_selector,
                    first_queue: RefCell::new(VecDeque::new()),
                    first_done: Cell::new(false),
                    first_disposable: SingleAssignmentDisposableValue::new(),
                    second_queue: RefCell::new(VecDeque::new()),
                    second_done: Cell::new(false),
                    second_disposable: SingleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<ZipSink<T, U, R>>| sink.run(&first, &second),
        )
    }
}

pub(crate) fn zip<T: 'static, U: 'static, R: 'static>(
    first: Rc<dyn IObservable<T>>,
    second: Rc<dyn IObservable<U>>,
    result_selector: ResultSelector<T, U, R>,
) -> Rc<dyn IObservable<R>> {
    Rc::new(ZipObservable { first, second, result_selector })
}

// ---------------------------------------------------------------------------------------------
// Binary with an enumerable: `Zip<TFirst, TSecond, TResult>.Enumerable`.
// ---------------------------------------------------------------------------------------------

/// An enumerator of the second sequence: an item that is an error is a `MoveNext` (or a `Current`)
/// that throws. Dropping it is `Dispose`.
pub(crate) type Enumerator<U> = Box<dyn Iterator<Item = Result<U, ObservableError>>>;

/// An enumerable second sequence: each subscription gets an enumerator of its own; an error is a
/// `GetEnumerator` that throws.
pub(crate) type Enumerable<U> = Rc<dyn Fn() -> Result<Enumerator<U>, ObservableError>>;

/// An enumerable of a collection that can be enumerated again for each subscription.
pub(crate) fn enumerable_of<U: 'static, I>(second: I) -> Enumerable<U>
where
    I: IntoIterator<Item = U> + Clone + 'static,
    I::IntoIter: 'static,
{
    Rc::new(move || Ok::<_, ObservableError>(Box::new(second.clone().into_iter().map(Ok::<U, ObservableError>)) as Enumerator<U>))
}

/// An enumerable of a function that makes the enumeration and may fail, as may each of its steps.
pub(crate) fn try_enumerable_of<U: 'static, I>(second: impl Fn() -> Result<I, ObservableError> + 'static) -> Enumerable<U>
where
    I: IntoIterator<Item = Result<U, ObservableError>>,
    I::IntoIter: 'static,
{
    Rc::new(move || second().map(|second| Box::new(second.into_iter()) as Enumerator<U>))
}

/// The enumerator of the sink (`_rightEnumerator` and `_enumerationInProgress`): `Moving` while
/// the enumerator moves, out of its slot, so that a disposal meanwhile leaves the enumerator to be
/// disposed once it has moved; `Disposed` is the `DisposedEnumerator`.
enum EnumeratorState<U> {
    Unset,
    Ready(Enumerator<U>),
    Moving,
    Disposed,
}

/// The sink of `Zip` with an enumerable (`Zip<TFirst, TSecond, TResult>.Enumerable._`): each value of
/// the sequence moves the enumerator; the end of the enumeration completes.
struct ZipEnumerableSink<T, U, R> {
    sink: Sink<R>,
    result_selector: ResultSelector<T, U, R>,
    enumerator: RefCell<EnumeratorState<U>>,
}

impl<T: 'static, U: 'static, R: 'static> ZipEnumerableSink<T, U, R> {
    fn run(self: &Rc<Self>, first: &Rc<dyn IObservable<T>>, second: &Enumerable<U>) {
        // The enumerator is obtained before the subscription to the sequence, so that it is there as
        // soon as the observer can be called.
        let enumerator = match second() {
            Ok(enumerator) => enumerator,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };
        let rejected = {
            let mut state = self.enumerator.borrow_mut();
            if matches!(*state, EnumeratorState::Unset) {
                *state = EnumeratorState::Ready(enumerator);
                None
            } else {
                Some(enumerator)
            }
        };
        if rejected.is_some() {
            drop(rejected);
            return;
        }

        let upstream = first.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }
}

impl<T: 'static, U: 'static, R: 'static> IDisposable for ZipEnumerableSink<T, U, R> {
    fn dispose(&self) {
        if self.sink.is_disposed() {
            return;
        }
        // An enumerator that is moving is disposed by the move once it has moved.
        let previous = std::mem::replace(&mut *self.enumerator.borrow_mut(), EnumeratorState::Disposed);
        drop(previous);
        self.sink.dispose();
    }
}

impl<T: 'static, U: 'static, R: 'static> IObserver<T> for ZipEnumerableSink<T, U, R> {
    fn on_next(&self, value: T) {
        let mut enumerator = {
            let mut state = self.enumerator.borrow_mut();
            match std::mem::replace(&mut *state, EnumeratorState::Moving) {
                EnumeratorState::Ready(enumerator) => enumerator,
                // A value notified while the enumerator moves (`_enumerationInProgress` above one)
                // has the enumerator disposed once it has moved, as upstream.
                EnumeratorState::Moving => {
                    *state = EnumeratorState::Disposed;
                    return;
                }
                other => {
                    *state = other;
                    return;
                }
            }
        };

        let next = enumerator.next();

        let disposed_meanwhile = {
            let mut state = self.enumerator.borrow_mut();
            if matches!(*state, EnumeratorState::Moving) {
                *state = EnumeratorState::Ready(enumerator);
                None
            } else {
                Some(enumerator)
            }
        };
        let was_disposed = disposed_meanwhile.is_some();
        drop(disposed_meanwhile);

        match next {
            Some(Err(error)) => self.sink.forward_on_error(error),
            _ if was_disposed => {}
            Some(Ok(right)) => match (self.result_selector)(value, right) {
                Ok(result) => self.sink.forward_on_next(result),
                Err(error) => self.sink.forward_on_error(error),
            },
            None => self.sink.forward_on_completed(),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

/// The observable of `Zip` with an enumerable (`Zip<TFirst, TSecond, TResult>.Enumerable`).
struct ZipEnumerableObservable<T, U, R> {
    first: Rc<dyn IObservable<T>>,
    second: Enumerable<U>,
    result_selector: ResultSelector<T, U, R>,
}

impl<T: 'static, U: 'static, R: 'static> IObservable<R> for ZipEnumerableObservable<T, U, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let (first, second, result_selector) = (self.first.clone(), self.second.clone(), self.result_selector.clone());
        subscribe_sink(
            observer,
            move |observer| Rc::new(ZipEnumerableSink { sink: Sink::new(observer), result_selector, enumerator: RefCell::new(EnumeratorState::Unset) }),
            move |sink: &Rc<ZipEnumerableSink<T, U, R>>| sink.run(&first, &second),
        )
    }
}

pub(crate) fn zip_enumerable<T: 'static, U: 'static, R: 'static>(
    first: Rc<dyn IObservable<T>>,
    second: Enumerable<U>,
    result_selector: ResultSelector<T, U, R>,
) -> Rc<dyn IObservable<R>> {
    Rc::new(ZipEnumerableObservable { first, second, result_selector })
}

// ---------------------------------------------------------------------------------------------
// [3,16]-ary: `Zip<T1, ..., TN, TResult>` (`Zip.NAry.cs`), on `ZipSink<TResult>` and
// `ZipObserver<T>`.
// ---------------------------------------------------------------------------------------------

/// What an observer of a source tells the sink (`IZip`).
trait IZip {
    /// The source at `index` has queued a value (`Next`).
    fn next(&self, index: usize);
    /// A source has failed (`Fail`).
    fn fail(&self, error: ObservableError);
    /// The source at `index` has completed (`Done`).
    fn done(&self, index: usize);
}

/// The state the sinks of three to sixteen sources share (`ZipSink<TResult>`): the completion flag
/// of each source. The queues are those of the observers.
struct ZipBase<R> {
    sink: Sink<R>,
    is_done: Box<[Cell<bool>]>,
}

impl<R: 'static> ZipBase<R> {
    fn new(observer: Rc<dyn IObserver<R>>, arity: usize) -> Self {
        Self { sink: Sink::new(observer), is_done: (0..arity).map(|_| Cell::new(false)).collect() }
    }

    /// `Next(int)`: with a value in every queue, the result of the first values; else, when all the
    /// other sources have completed, the completion.
    fn next(&self, index: usize, has_value_all: bool, get_result: impl FnOnce() -> Result<R, ObservableError>) {
        if has_value_all {
            match get_result() {
                Ok(result) => self.sink.forward_on_next(result),
                Err(error) => self.sink.forward_on_error(error),
            }
        } else {
            let all_others_done = self.is_done.iter().enumerate().all(|(i, done)| i == index || done.get());
            if all_others_done {
                self.sink.forward_on_completed();
            }
        }
    }

    /// `Fail(Exception)`.
    fn fail(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    /// `Done(int)`: the completion once every source has completed.
    fn done(&self, index: usize) {
        self.is_done[index].set(true);
        if self.is_done.iter().all(Cell::get) {
            self.sink.forward_on_completed();
        }
    }
}

/// The observer of one source (`ZipObserver<T>`, a `SafeObserver<T>`): it queues the values and
/// holds the subscription to its source (`SetResource`). Disposing it disposes the subscription and
/// clears the queue.
struct ZipObserver<T> {
    parent: Weak<dyn IZip>,
    index: usize,
    values: RefCell<VecDeque<T>>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static> ZipObserver<T> {
    fn new(parent: Weak<dyn IZip>, index: usize) -> Rc<Self> {
        Rc::new(Self { parent, index, values: RefCell::new(VecDeque::new()), resource: SingleAssignmentDisposableValue::new() })
    }

    fn set_resource(&self, resource: Rc<dyn IDisposable>) {
        self.resource.set_disposable(Some(resource));
    }

    fn is_empty(&self) -> bool {
        self.values.borrow().is_empty()
    }

    fn dequeue(&self) -> T {
        self.values.borrow_mut().pop_front().expect("every queue holds a value when a result is made")
    }
}

impl<T: 'static> IObserver<T> for ZipObserver<T> {
    fn on_next(&self, value: T) {
        self.values.borrow_mut().push_back(value);
        if let Some(parent) = self.parent.upgrade() {
            parent.next(self.index);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.dispose();
        if let Some(parent) = self.parent.upgrade() {
            parent.fail(error);
        }
    }

    fn on_completed(&self) {
        // Only the subscription is disposed: the queue keeps the values not combined yet.
        self.resource.dispose();
        if let Some(parent) = self.parent.upgrade() {
            parent.done(self.index);
        }
    }
}

impl<T: 'static> IDisposable for ZipObserver<T> {
    fn dispose(&self) {
        self.resource.dispose();
        let values = std::mem::take(&mut *self.values.borrow_mut());
        drop(values);
    }
}

/// The sink, the observable and the function of `Zip` of `N` sources (`Zip<T1, ..., TN, TResult>`):
/// the sink subscribes the observers in order, then holds them as its subscription upstream
/// (`SetUpstream(StableCompositeDisposable.CreateTrusted(...))`).
macro_rules! zip_arity {
    ($sink:ident, $observable:ident, $function:ident, $arity:literal; $(($t:ident, $source:ident, $observer:ident, $value:ident, $index:literal)),+ $(,)?) => {
        /// The sink of `Zip` of this arity (`Zip<T1, ..., TN, TResult>._`).
        #[allow(clippy::type_complexity)]
        struct $sink<$($t,)+ R> {
            base: ZipBase<R>,
            result_selector: Rc<dyn Fn($($t),+) -> Result<R, ObservableError>>,
            $($observer: Rc<ZipObserver<$t>>,)+
        }

        impl<$($t: 'static,)+ R: 'static> $sink<$($t,)+ R> {
            #[allow(clippy::too_many_arguments)]
            fn run(self: &Rc<Self>, $($source: &Rc<dyn IObservable<$t>>),+) {
                let mut disposables: Vec<Rc<dyn IDisposable>> = Vec::with_capacity($arity);
                $(
                    let disposable: Rc<dyn IDisposable> = self.$observer.clone();
                    disposables.push(disposable);
                )+

                $(
                    let observer: Rc<dyn IObserver<$t>> = self.$observer.clone();
                    self.$observer.set_resource($source.subscribe(observer));
                )+

                let upstream: Rc<dyn IDisposable> = StableCompositeDisposable::create_from(disposables);
                self.base.sink.set_upstream(upstream);
            }
        }

        impl<$($t: 'static,)+ R: 'static> IZip for $sink<$($t,)+ R> {
            fn next(&self, index: usize) {
                let has_value_all = $(!self.$observer.is_empty())&&+;
                self.base.next(index, has_value_all, || {
                    $(let $value = self.$observer.dequeue();)+
                    (self.result_selector)($($value),+)
                });
            }

            fn fail(&self, error: ObservableError) {
                self.base.fail(error);
            }

            fn done(&self, index: usize) {
                self.base.done(index);
            }
        }

        impl<$($t: 'static,)+ R: 'static> IDisposable for $sink<$($t,)+ R> {
            fn dispose(&self) {
                self.base.sink.dispose();
            }
        }

        /// The observable of `Zip` of this arity (`Zip<T1, ..., TN, TResult>`).
        #[allow(clippy::type_complexity)]
        struct $observable<$($t,)+ R> {
            $($source: Rc<dyn IObservable<$t>>,)+
            result_selector: Rc<dyn Fn($($t),+) -> Result<R, ObservableError>>,
        }

        impl<$($t: 'static,)+ R: 'static> IObservable<R> for $observable<$($t,)+ R> {
            fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
                $(let $source = self.$source.clone();)+
                let result_selector = self.result_selector.clone();
                subscribe_sink(
                    observer,
                    move |observer| {
                        Rc::new_cyclic(|this: &Weak<$sink<$($t,)+ R>>| {
                            let parent: Weak<dyn IZip> = this.clone();
                            $sink {
                                base: ZipBase::new(observer, $arity),
                                result_selector,
                                $($observer: ZipObserver::new(parent.clone(), $index),)+
                            }
                        })
                    },
                    move |sink: &Rc<$sink<$($t,)+ R>>| sink.run($(&$source),+),
                )
            }
        }

        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        pub(crate) fn $function<$($t: 'static,)+ R: 'static>(
            $($source: Rc<dyn IObservable<$t>>,)+
            result_selector: Rc<dyn Fn($($t),+) -> Result<R, ObservableError>>,
        ) -> Rc<dyn IObservable<R>> {
            Rc::new($observable { $($source,)+ result_selector })
        }
    };
}

zip_arity!(Zip3Sink, Zip3Observable, zip3, 3; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2));
zip_arity!(Zip4Sink, Zip4Observable, zip4, 4; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3));
zip_arity!(Zip5Sink, Zip5Observable, zip5, 5; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4));
zip_arity!(Zip6Sink, Zip6Observable, zip6, 6; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5));
zip_arity!(Zip7Sink, Zip7Observable, zip7, 7; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6));
zip_arity!(Zip8Sink, Zip8Observable, zip8, 8; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7));
zip_arity!(Zip9Sink, Zip9Observable, zip9, 9; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8));
zip_arity!(Zip10Sink, Zip10Observable, zip10, 10; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9));
zip_arity!(Zip11Sink, Zip11Observable, zip11, 11; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9), (T11, source11, observer11, value11, 10));
zip_arity!(Zip12Sink, Zip12Observable, zip12, 12; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9), (T11, source11, observer11, value11, 10), (T12, source12, observer12, value12, 11));
zip_arity!(Zip13Sink, Zip13Observable, zip13, 13; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9), (T11, source11, observer11, value11, 10), (T12, source12, observer12, value12, 11), (T13, source13, observer13, value13, 12));
zip_arity!(Zip14Sink, Zip14Observable, zip14, 14; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9), (T11, source11, observer11, value11, 10), (T12, source12, observer12, value12, 11), (T13, source13, observer13, value13, 12), (T14, source14, observer14, value14, 13));
zip_arity!(Zip15Sink, Zip15Observable, zip15, 15; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9), (T11, source11, observer11, value11, 10), (T12, source12, observer12, value12, 11), (T13, source13, observer13, value13, 12), (T14, source14, observer14, value14, 13), (T15, source15, observer15, value15, 14));
zip_arity!(Zip16Sink, Zip16Observable, zip16, 16; (T1, source1, observer1, value1, 0), (T2, source2, observer2, value2, 1), (T3, source3, observer3, value3, 2), (T4, source4, observer4, value4, 3), (T5, source5, observer5, value5, 4), (T6, source6, observer6, value6, 5), (T7, source7, observer7, value7, 6), (T8, source8, observer8, value8, 7), (T9, source9, observer9, value9, 8), (T10, source10, observer10, value10, 9), (T11, source11, observer11, value11, 10), (T12, source12, observer12, value12, 11), (T13, source13, observer13, value13, 12), (T14, source14, observer14, value14, 13), (T15, source15, observer15, value15, 14), (T16, source16, observer16, value16, 15));

// ---------------------------------------------------------------------------------------------
// N-ary: `Zip<TSource>` of an enumerable of sources.
// ---------------------------------------------------------------------------------------------

/// The subscriptions of the sink (`_subscriptions`): `NotRun` is `null`, `Disposed` is the empty
/// array a disposal leaves.
enum ZipAllSubscriptions {
    NotRun,
    Running(Rc<[SingleAssignmentDisposableValue]>),
    Disposed,
}

/// The sink of `Zip` of an enumerable of sources (`Zip<TSource>._`): a queue and a completion flag
/// for each source, and the subscription to each.
struct ZipAllSink<T> {
    sink: Sink<Vec<T>>,
    queues: RefCell<Vec<VecDeque<T>>>,
    is_done: RefCell<Vec<bool>>,
    subscriptions: RefCell<ZipAllSubscriptions>,
}

impl<T: 'static> ZipAllSink<T> {
    fn run<I>(self: &Rc<Self>, sources: I)
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>>,
    {
        let sources: Vec<Rc<dyn IObservable<T>>> = sources.into_iter().collect();
        let n = sources.len();

        *self.queues.borrow_mut() = (0..n).map(|_| VecDeque::new()).collect();
        *self.is_done.borrow_mut() = vec![false; n];

        let subscriptions: Rc<[SingleAssignmentDisposableValue]> = (0..n).map(|_| SingleAssignmentDisposableValue::new()).collect();

        let run = {
            let mut slot = self.subscriptions.borrow_mut();
            if matches!(*slot, ZipAllSubscriptions::NotRun) {
                *slot = ZipAllSubscriptions::Running(subscriptions.clone());
                true
            } else {
                false
            }
        };
        if run {
            for (index, source) in sources.into_iter().enumerate() {
                let observer = Rc::new(ZipAllSourceObserver { parent: self.clone(), index });
                subscriptions[index].set_disposable(Some(source.subscribe(observer)));
            }
        }
    }

    fn on_next(&self, index: usize, value: T) {
        self.queues.borrow_mut()[index].push_back(value);

        let has_value_all = self.queues.borrow().iter().all(|queue| !queue.is_empty());
        if has_value_all {
            let result: Vec<T> = self
                .queues
                .borrow_mut()
                .iter_mut()
                .map(|queue| queue.pop_front().expect("every queue holds a value when a result is made"))
                .collect();
            self.sink.forward_on_next(result);
        } else {
            let all_others_done = self.is_done.borrow().iter().enumerate().all(|(i, done)| i == index || *done);
            if all_others_done {
                self.sink.forward_on_completed();
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self, index: usize) {
        let all_done = {
            let mut is_done = self.is_done.borrow_mut();
            is_done[index] = true;
            is_done.iter().all(|done| *done)
        };
        if all_done {
            self.sink.forward_on_completed();
        } else {
            let subscriptions = match &*self.subscriptions.borrow() {
                ZipAllSubscriptions::Running(subscriptions) => Some(subscriptions.clone()),
                _ => None,
            };
            if let Some(subscriptions) = subscriptions {
                subscriptions[index].dispose();
            }
        }
    }
}

impl<T: 'static> IDisposable for ZipAllSink<T> {
    fn dispose(&self) {
        if self.sink.is_disposed() {
            return;
        }
        let previous = std::mem::replace(&mut *self.subscriptions.borrow_mut(), ZipAllSubscriptions::Disposed);
        if let ZipAllSubscriptions::Running(subscriptions) = previous {
            for subscription in subscriptions.iter() {
                subscription.dispose();
            }
            let queues = std::mem::take(&mut *self.queues.borrow_mut());
            drop(queues);
        }
        self.sink.dispose();
    }
}

/// The observer of one source (`SourceObserver`).
struct ZipAllSourceObserver<T> {
    parent: Rc<ZipAllSink<T>>,
    index: usize,
}

impl<T: 'static> IObserver<T> for ZipAllSourceObserver<T> {
    fn on_next(&self, value: T) {
        self.parent.on_next(self.index, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_error(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed(self.index);
    }
}

/// The observable of `Zip` of an enumerable of sources (`Zip<TSource>`): each subscription
/// enumerates `sources` anew.
struct ZipAllObservable<T, I> {
    sources: I,
    _values: PhantomData<fn() -> T>,
}

impl<T: 'static, I> IObservable<Vec<T>> for ZipAllObservable<T, I>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<Vec<T>>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        subscribe_sink(
            observer,
            |observer| {
                Rc::new(ZipAllSink {
                    sink: Sink::new(observer),
                    queues: RefCell::new(Vec::new()),
                    is_done: RefCell::new(Vec::new()),
                    subscriptions: RefCell::new(ZipAllSubscriptions::NotRun),
                })
            },
            move |sink: &Rc<ZipAllSink<T>>| sink.run(sources),
        )
    }
}

impl crate::linq::Observable {
    /// The lists of the values of the sequences of an enumerable, by position: the n-th list holds
    /// the n-th value of each sequence (`Zip(IEnumerable<IObservable<TSource>>)` and
    /// `Zip(params IObservable<TSource>[])`). Each subscription enumerates `sources` anew.
    pub fn zip_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<Vec<T>>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        Rc::new(ZipAllObservable { sources, _values: PhantomData })
    }

    /// [`zip_all`](Self::zip_all) with each list projected by `result_selector`
    /// (`Zip(IEnumerable<IObservable<TSource>>, Func<IList<TSource>, TResult>)`).
    pub fn zip_all_with<T: 'static, R: 'static, I>(sources: I, result_selector: impl Fn(Vec<T>) -> R + 'static) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        crate::linq::select::select(Self::zip_all(sources), Rc::new(move |values: Vec<T>, _: usize| Ok::<R, ObservableError>(result_selector(values))))
    }

    /// [`zip_all_with`](Self::zip_all_with) with a selector that may fail; a failure ends the
    /// sequence with its error (`Zip(IEnumerable<IObservable<TSource>>, Func<IList<TSource>, TResult>)`
    /// with a selector that throws).
    pub fn try_zip_all_with<T: 'static, R: 'static, I>(
        sources: I,
        result_selector: impl Fn(Vec<T>) -> Result<R, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        crate::linq::select::select(Self::zip_all(sources), Rc::new(move |values: Vec<T>, _: usize| result_selector(values)))
    }
}

// ---------------------------------------------------------------------------------------------
// The methods.
// ---------------------------------------------------------------------------------------------

/// The methods of `Zip` of one arity of three to sixteen sources: the form with a selector, the
/// form with a selector that may fail, and the form of tuples (`ObservableEx.Zip`).
macro_rules! zip_arity_methods {
    ($function:ident, $try_function:ident, $tuple_function:ident, $others:literal, $overload:literal, $sources:literal; $(($t:ident, $source:ident, $value:ident)),+ $(,)?) => {
        #[doc = concat!(
            "Combines the values of the sequence and of ", $others, " other sequences by position with `result_selector`: ",
            "the n-th result combines the n-th value of each sequence (`", $overload, "`)."
        )]
        #[allow(clippy::too_many_arguments)]
        fn $function<$($t: 'static,)+ R: 'static>(
            &self,
            $($source: std::rc::Rc<dyn ferroui_system::IObservable<$t>>,)+
            result_selector: impl Fn(T, $($t),+) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::zip::$function(
                self.observable(),
                $($source,)+
                std::rc::Rc::new(move |value1: T, $($value: $t),+| Ok::<R, ferroui_system::ObservableError>(result_selector(value1, $($value),+))),
            )
        }

        #[doc = concat!(
            "[`", stringify!($function), "`](Self::", stringify!($function), ") with a selector that may fail; a failure ends ",
            "the sequence with its error (`", $overload, "` with a selector that throws)."
        )]
        #[allow(clippy::too_many_arguments)]
        fn $try_function<$($t: 'static,)+ R: 'static>(
            &self,
            $($source: std::rc::Rc<dyn ferroui_system::IObservable<$t>>,)+
            result_selector: impl Fn(T, $($t),+) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::zip::$function(self.observable(), $($source,)+ std::rc::Rc::new(result_selector))
        }

        #[doc = concat!(
            "The tuples of the values of the sequence and of ", $others, " other sequences by position ",
            "(`ObservableEx.Zip<", $sources, ">`)."
        )]
        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        fn $tuple_function<$($t: 'static),+>(
            &self,
            $($source: std::rc::Rc<dyn ferroui_system::IObservable<$t>>),+
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<(T, $($t),+)>> {
            crate::linq::zip::$function(
                self.observable(),
                $($source,)+
                std::rc::Rc::new(|value1: T, $($value: $t),+| Ok::<(T, $($t),+), ferroui_system::ObservableError>((value1, $($value),+))),
            )
        }
    };
}

/// The methods of `Zip` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! zip_methods {
    () => {
        /// Combines the values of the sequence and of `second` by position with `result_selector`:
        /// the n-th result combines the n-th value of each
        /// (`Zip(IObservable<TSource1>, IObservable<TSource2>, Func<TSource1, TSource2, TResult>)`).
        fn zip<U: 'static, R: 'static>(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            result_selector: impl Fn(T, U) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::zip::zip(self.observable(), second, std::rc::Rc::new(move |left: T, right: U| Ok::<R, ferroui_system::ObservableError>(result_selector(left, right))))
        }

        /// [`zip`](Self::zip) with a selector that may fail; a failure ends the sequence with its
        /// error (`Zip(IObservable<TSource1>, IObservable<TSource2>, Func<TSource1, TSource2, TResult>)`
        /// with a selector that throws).
        fn try_zip<U: 'static, R: 'static>(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
            result_selector: impl Fn(T, U) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> {
            crate::linq::zip::zip(self.observable(), second, std::rc::Rc::new(result_selector))
        }

        /// The pairs of the values of the sequence and of `second` by position
        /// (`ObservableEx.Zip<TFirst, TSecond>(IObservable<TFirst>, IObservable<TSecond>)`).
        fn zip_tuple<U: 'static>(&self, second: std::rc::Rc<dyn ferroui_system::IObservable<U>>) -> std::rc::Rc<dyn ferroui_system::IObservable<(T, U)>> {
            crate::linq::zip::zip(self.observable(), second, std::rc::Rc::new(|left: T, right: U| Ok::<(T, U), ferroui_system::ObservableError>((left, right))))
        }

        /// Combines each value of the sequence with the next value of `second`, an enumerable that
        /// each subscription iterates anew, with `result_selector`; the end of `second` completes
        /// the sequence (`Zip(IObservable<TSource1>, IEnumerable<TSource2>, Func<TSource1, TSource2, TResult>)`).
        fn zip_iter<U: 'static, R: 'static, I>(&self, second: I, result_selector: impl Fn(T, U) -> R + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            I: IntoIterator<Item = U> + Clone + 'static,
            I::IntoIter: 'static,
        {
            crate::linq::zip::zip_enumerable(
                self.observable(),
                crate::linq::zip::enumerable_of(second),
                std::rc::Rc::new(move |left: T, right: U| Ok::<R, ferroui_system::ObservableError>(result_selector(left, right))),
            )
        }

        /// [`zip_iter`](Self::zip_iter) of an enumerable whose enumeration may fail, as may each of
        /// its steps, and with a selector that may fail: `second` makes the enumeration of each
        /// subscription (`GetEnumerator`, which may throw), whose errors are a `MoveNext` or a
        /// `Current` that throws; each failure ends the sequence with its error.
        fn try_zip_iter<U: 'static, R: 'static, I>(
            &self,
            second: impl Fn() -> Result<I, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, U) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            I: IntoIterator<Item = Result<U, ferroui_system::ObservableError>>,
            I::IntoIter: 'static,
        {
            crate::linq::zip::zip_enumerable(self.observable(), crate::linq::zip::try_enumerable_of(second), std::rc::Rc::new(result_selector))
        }

        /// The pairs of each value of the sequence and the next value of `second`
        /// (`ObservableEx.Zip<TFirst, TSecond>(IObservable<TFirst>, IEnumerable<TSecond>)`).
        fn zip_iter_tuple<U: 'static, I>(&self, second: I) -> std::rc::Rc<dyn ferroui_system::IObservable<(T, U)>>
        where
            I: IntoIterator<Item = U> + Clone + 'static,
            I::IntoIter: 'static,
        {
            crate::linq::zip::zip_enumerable(
                self.observable(),
                crate::linq::zip::enumerable_of(second),
                std::rc::Rc::new(|left: T, right: U| Ok::<(T, U), ferroui_system::ObservableError>((left, right))),
            )
        }

        crate::linq::zip::zip_arity_methods!(zip3, try_zip3, zip3_tuple, "two", "Zip<TSource1, TSource2, TSource3, TResult>", "TSource1, TSource2, TSource3"; (T2, source2, value2), (T3, source3, value3));
        crate::linq::zip::zip_arity_methods!(zip4, try_zip4, zip4_tuple, "three", "Zip<TSource1, TSource2, TSource3, TSource4, TResult>", "TSource1, TSource2, TSource3, TSource4"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4));
        crate::linq::zip::zip_arity_methods!(zip5, try_zip5, zip5_tuple, "four", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5));
        crate::linq::zip::zip_arity_methods!(zip6, try_zip6, zip6_tuple, "five", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6));
        crate::linq::zip::zip_arity_methods!(zip7, try_zip7, zip7_tuple, "six", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7));
        crate::linq::zip::zip_arity_methods!(zip8, try_zip8, zip8_tuple, "seven", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8));
        crate::linq::zip::zip_arity_methods!(zip9, try_zip9, zip9_tuple, "eight", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9));
        crate::linq::zip::zip_arity_methods!(zip10, try_zip10, zip10_tuple, "nine", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10));
        crate::linq::zip::zip_arity_methods!(zip11, try_zip11, zip11_tuple, "ten", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10), (T11, source11, value11));
        crate::linq::zip::zip_arity_methods!(zip12, try_zip12, zip12_tuple, "eleven", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10), (T11, source11, value11), (T12, source12, value12));
        crate::linq::zip::zip_arity_methods!(zip13, try_zip13, zip13_tuple, "twelve", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10), (T11, source11, value11), (T12, source12, value12), (T13, source13, value13));
        crate::linq::zip::zip_arity_methods!(zip14, try_zip14, zip14_tuple, "thirteen", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10), (T11, source11, value11), (T12, source12, value12), (T13, source13, value13), (T14, source14, value14));
        crate::linq::zip::zip_arity_methods!(zip15, try_zip15, zip15_tuple, "fourteen", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TSource15, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TSource15"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10), (T11, source11, value11), (T12, source12, value12), (T13, source13, value13), (T14, source14, value14), (T15, source15, value15));
        crate::linq::zip::zip_arity_methods!(zip16, try_zip16, zip16_tuple, "fifteen", "Zip<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TSource15, TSource16, TResult>", "TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TSource15, TSource16"; (T2, source2, value2), (T3, source3, value3), (T4, source4, value4), (T5, source5, value5), (T6, source6, value6), (T7, source7, value7), (T8, source8, value8), (T9, source9, value9), (T10, source10, value10), (T11, source11, value11), (T12, source12, value12), (T13, source13, value13), (T14, source14, value14), (T15, source15, value15), (T16, source16, value16));
    };
}

pub(crate) use zip_arity_methods;
pub(crate) use zip_methods;
