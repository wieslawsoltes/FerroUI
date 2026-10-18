//! `CombineLatest` (`Linq/Observable/CombineLatest.cs`, and `Linq/Observable/CombineLatest.NAry.cs`).
//!
//! Upstream generates the forms of 3 to 16 sources (and their public overloads, with the tuple
//! overloads of `ObservableEx`) with T4 templates; here the `combine_latest_arity!` and
//! `combine_latest_arity_methods!` macros generate them, for 2 to 16 sources. The form of two
//! sources (upstream's hand-written `CombineLatest<TFirst, TSecond, TResult>`, with its
//! `FirstObserver` and `SecondObserver`) is the same algorithm as the generated ones, and is
//! generated with them: a source that produces a value before every source has one only stores it,
//! or ends the sequence when every other source has ended; a source that ends disposes its
//! subscription, and the sequence ends when every source has ended.
//!
//! The form of a list of sources (`CombineLatest<TSource, TResult>`, the `IEnumerable` and `params`
//! overloads) gives the selector the latest values as a slice.

use crate::disposables::{SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Helpers for the forms of a fixed number of sources.

/// The state of the sink of a fixed number of sources (`CombineLatestSink<TResult>`): which
/// sources have a value, which have ended, and the subscription to each (upstream's
/// `CombineLatestObserver`s, which are the subscriptions of the sink).
struct CombineLatestSink<R> {
    sink: Sink<R>,
    has_value_all: Cell<bool>,
    has_value: Box<[Cell<bool>]>,
    is_done: Box<[Cell<bool>]>,
    subscriptions: Box<[SingleAssignmentDisposableValue]>,
}

impl<R: 'static> CombineLatestSink<R> {
    fn new(arity: usize, observer: Rc<dyn IObserver<R>>) -> Self {
        Self {
            sink: Sink::new(observer),
            has_value_all: Cell::new(false),
            has_value: (0..arity).map(|_| Cell::new(false)).collect(),
            is_done: (0..arity).map(|_| Cell::new(false)).collect(),
            subscriptions: (0..arity).map(|_| SingleAssignmentDisposableValue::new()).collect(),
        }
    }

    /// Source `index` produced a value, which its observer has stored (`Next(int)`): once every
    /// source has a value, the result of the latest values is forwarded; before, the sequence ends
    /// when every other source has ended (no combination can be produced any more).
    fn next(&self, index: usize, get_result: impl FnOnce() -> Result<R, ObservableError>) {
        if !self.has_value_all.get() {
            self.has_value[index].set(true);
            let has_value_all = self.has_value.iter().all(|has_value| has_value.get());
            self.has_value_all.set(has_value_all);
        }

        if self.has_value_all.get() {
            match get_result() {
                Ok(result) => self.sink.forward_on_next(result),
                Err(error) => self.sink.forward_on_error(error),
            }
        } else {
            let all_others_done = self.is_done.iter().enumerate().all(|(i, is_done)| i == index || is_done.get());
            if all_others_done {
                self.sink.forward_on_completed();
            }
        }
    }

    /// A source failed (`Fail(Exception)`).
    fn fail(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    /// Source `index` ended (`Done(int)`): the sequence ends when every source has ended.
    fn done(&self, index: usize) {
        self.is_done[index].set(true);
        if self.is_done.iter().all(|is_done| is_done.get()) {
            self.sink.forward_on_completed();
        }
    }

    /// Disposes the sink and the subscription to every source (`Dispose(bool)` of the sink, with its
    /// upstream, the `StableCompositeDisposable` of the observers).
    fn dispose(&self) {
        self.sink.dispose();
        for subscription in self.subscriptions.iter() {
            subscription.dispose();
        }
    }
}

/// The sink of a fixed number of sources, as its observers see it (`ICombineLatest`, with the
/// `GetResult` of `CombineLatestSink<TResult>`).
trait CombineLatestArity: 'static {
    type Result: 'static;

    /// The shared state of the sink.
    fn base(&self) -> &CombineLatestSink<Self::Result>;

    /// The result of the latest values (`GetResult()`), called once every source has a value.
    fn get_result(&self) -> Result<Self::Result, ObservableError>;
}

/// The observer of one source of a fixed number of sources (`CombineLatestObserver<T>`): it stores
/// the value in the sink (upstream keeps it in the observer), and an end disposes its subscription
/// before the sink handles it.
struct CombineLatestObserver<S, T> {
    parent: Rc<S>,
    index: usize,
    store: fn(&S, T),
}

impl<S: CombineLatestArity, T: 'static> IObserver<T> for CombineLatestObserver<S, T> {
    fn on_next(&self, value: T) {
        (self.store)(&self.parent, value);
        let parent = &self.parent;
        parent.base().next(self.index, || parent.get_result());
    }

    fn on_error(&self, error: ObservableError) {
        let base = self.parent.base();
        base.subscriptions[self.index].dispose();
        base.fail(error);
    }

    fn on_completed(&self) {
        let base = self.parent.base();
        base.subscriptions[self.index].dispose();
        base.done(self.index);
    }
}

/// Generates the sink, the observable and the function of `CombineLatest` of a fixed number of
/// sources (the T4 template of `CombineLatest.NAry.tt`): `$sink` and `$observable` are the names of
/// the types, `$function` the function, `$arity` the number of sources, and each source is given as
/// its type parameter, its name and its index.
macro_rules! combine_latest_arity {
    ($sink:ident, $observable:ident, $function:ident, $arity:literal; $($T:ident $source:ident $index:tt),+) => {
        /// The sink of `CombineLatest` of a fixed number of sources (`CombineLatest<T1, ..., TResult>._`).
        #[allow(clippy::type_complexity)]
        struct $sink<$($T,)+ R> {
            base: CombineLatestSink<R>,
            result_selector: Rc<dyn Fn($($T),+) -> Result<R, ObservableError>>,
            values: ($(RefCell<Option<$T>>,)+),
        }

        impl<$($T: Clone + 'static,)+ R: 'static> $sink<$($T,)+ R> {
            /// Subscribes to the sources in order (`Run`).
            #[allow(clippy::too_many_arguments)]
            fn run(self: &Rc<Self>, $($source: &Rc<dyn IObservable<$T>>),+) {
                $(
                    let observer = Rc::new(CombineLatestObserver::<Self, $T> {
                        parent: self.clone(),
                        index: $index,
                        store: |parent: &Self, value: $T| {
                            *parent.values.$index.borrow_mut() = Some(value);
                        },
                    });
                    self.base.subscriptions[$index].set_disposable(Some($source.subscribe(observer)));
                )+
            }
        }

        impl<$($T: Clone + 'static,)+ R: 'static> CombineLatestArity for $sink<$($T,)+ R> {
            type Result = R;

            fn base(&self) -> &CombineLatestSink<R> {
                &self.base
            }

            fn get_result(&self) -> Result<R, ObservableError> {
                let values = ($(self.values.$index.borrow().clone().expect("every source has a value once the result is taken"),)+);
                (self.result_selector)($(values.$index),+)
            }
        }

        impl<$($T: 'static,)+ R: 'static> IDisposable for $sink<$($T,)+ R> {
            fn dispose(&self) {
                self.base.dispose();
            }
        }

        /// The observable of `CombineLatest` of a fixed number of sources (`CombineLatest<T1, ..., TResult>`).
        #[allow(clippy::type_complexity)]
        struct $observable<$($T,)+ R> {
            $($source: Rc<dyn IObservable<$T>>,)+
            result_selector: Rc<dyn Fn($($T),+) -> Result<R, ObservableError>>,
        }

        impl<$($T: Clone + 'static,)+ R: 'static> IObservable<R> for $observable<$($T,)+ R> {
            fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
                let ($($source,)+) = ($(self.$source.clone(),)+);
                let result_selector = self.result_selector.clone();
                subscribe_sink(
                    observer,
                    move |observer| {
                        Rc::new($sink {
                            base: CombineLatestSink::new($arity, observer),
                            result_selector,
                            values: ($(RefCell::new(None::<$T>),)+),
                        })
                    },
                    move |sink: &Rc<$sink<$($T,)+ R>>| sink.run($(&$source),+),
                )
            }
        }

        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        pub(crate) fn $function<$($T: Clone + 'static,)+ R: 'static>(
            $($source: Rc<dyn IObservable<$T>>,)+
            result_selector: Rc<dyn Fn($($T),+) -> Result<R, ObservableError>>,
        ) -> Rc<dyn IObservable<R>> {
            Rc::new($observable { $($source,)+ result_selector })
        }
    };
}

// The forms of 2 to 16 sources.

combine_latest_arity!(CombineLatest2Sink, CombineLatest2, combine_latest2, 2; T1 source1 0, T2 source2 1);
combine_latest_arity!(CombineLatest3Sink, CombineLatest3, combine_latest3, 3; T1 source1 0, T2 source2 1, T3 source3 2);
combine_latest_arity!(CombineLatest4Sink, CombineLatest4, combine_latest4, 4; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3);
combine_latest_arity!(CombineLatest5Sink, CombineLatest5, combine_latest5, 5; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4);
combine_latest_arity!(CombineLatest6Sink, CombineLatest6, combine_latest6, 6; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5);
combine_latest_arity!(CombineLatest7Sink, CombineLatest7, combine_latest7, 7; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6);
combine_latest_arity!(CombineLatest8Sink, CombineLatest8, combine_latest8, 8; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7);
combine_latest_arity!(CombineLatest9Sink, CombineLatest9, combine_latest9, 9; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8);
combine_latest_arity!(CombineLatest10Sink, CombineLatest10, combine_latest10, 10; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9);
combine_latest_arity!(CombineLatest11Sink, CombineLatest11, combine_latest11, 11; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9, T11 source11 10);
combine_latest_arity!(CombineLatest12Sink, CombineLatest12, combine_latest12, 12; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9, T11 source11 10, T12 source12 11);
combine_latest_arity!(CombineLatest13Sink, CombineLatest13, combine_latest13, 13; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9, T11 source11 10, T12 source12 11, T13 source13 12);
combine_latest_arity!(CombineLatest14Sink, CombineLatest14, combine_latest14, 14; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9, T11 source11 10, T12 source12 11, T13 source13 12, T14 source14 13);
combine_latest_arity!(CombineLatest15Sink, CombineLatest15, combine_latest15, 15; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9, T11 source11 10, T12 source12 11, T13 source13 12, T14 source14 13, T15 source15 14);
combine_latest_arity!(CombineLatest16Sink, CombineLatest16, combine_latest16, 16; T1 source1 0, T2 source2 1, T3 source3 2, T4 source4 3, T5 source5 4, T6 source6 5, T7 source7 6, T8 source8 7, T9 source9 8, T10 source10 9, T11 source11 10, T12 source12 11, T13 source13 12, T14 source14 13, T15 source15 14, T16 source16 15);

// The form of a list of sources.

type ManyResultSelector<T, R> = Rc<dyn Fn(&[T]) -> Result<R, ObservableError>>;

/// The sink of `CombineLatest` of a list of sources (`CombineLatest<TSource, TResult>._`).
///
/// Upstream keeps the latest values in an array of defaults; here they are options until every
/// source has a value (`partial`), and then the values themselves (`values`), which the selector
/// gets as a slice.
struct CombineLatestManySink<T, R> {
    sink: Sink<R>,
    result_selector: ManyResultSelector<T, R>,
    has_value: RefCell<Vec<bool>>,
    has_value_all: Cell<bool>,
    partial: RefCell<Vec<Option<T>>>,
    values: RefCell<Vec<T>>,
    is_done: RefCell<Vec<bool>>,
    subscriptions: RefCell<Vec<Rc<SingleAssignmentDisposable>>>,
}

impl<T: 'static, R: 'static> CombineLatestManySink<T, R> {
    /// Subscribes to the sources in order (`Run`).
    fn run(self: &Rc<Self>, sources: Vec<Rc<dyn IObservable<T>>>) {
        let n = sources.len();

        *self.has_value.borrow_mut() = vec![false; n];
        self.has_value_all.set(false);

        *self.partial.borrow_mut() = (0..n).map(|_| None).collect();

        *self.is_done.borrow_mut() = vec![false; n];

        let subscriptions: Vec<Rc<SingleAssignmentDisposable>> = (0..n).map(|_| SingleAssignmentDisposable::new()).collect();
        *self.subscriptions.borrow_mut() = subscriptions.clone();

        for (index, source) in sources.iter().enumerate() {
            let observer = Rc::new(CombineLatestManyObserver { parent: self.clone(), index });
            subscriptions[index].set_disposable(Some(source.subscribe(observer)));
        }
    }

    fn on_next(&self, index: usize, value: T) {
        if self.has_value_all.get() {
            self.values.borrow_mut()[index] = value;
        } else {
            self.partial.borrow_mut()[index] = Some(value);
            self.has_value.borrow_mut()[index] = true;
            if self.has_value.borrow().iter().all(|&has_value| has_value) {
                let partial = std::mem::take(&mut *self.partial.borrow_mut());
                *self.values.borrow_mut() =
                    partial.into_iter().map(|value| value.expect("every source has a value once all have one")).collect();
                self.has_value_all.set(true);
            }
        }

        if self.has_value_all.get() {
            let result = {
                let values = self.values.borrow();
                (self.result_selector)(values.as_slice())
            };
            match result {
                Ok(result) => self.sink.forward_on_next(result),
                Err(error) => self.sink.forward_on_error(error),
            }
        } else {
            let all_others_done = self.is_done.borrow().iter().enumerate().all(|(i, &done)| i == index || done);
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
            is_done.iter().all(|&done| done)
        };
        if all_done {
            self.sink.forward_on_completed();
        } else {
            let subscription = self.subscriptions.borrow()[index].clone();
            subscription.dispose();
        }
    }
}

impl<T: 'static, R: 'static> IDisposable for CombineLatestManySink<T, R> {
    /// Disposes the sink and the subscription to every source (upstream's upstream, the
    /// `StableCompositeDisposable` of the observers).
    fn dispose(&self) {
        self.sink.dispose();
        let subscriptions = self.subscriptions.borrow().clone();
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

/// The observer of one source of a list (`SourceObserver`).
struct CombineLatestManyObserver<T, R> {
    parent: Rc<CombineLatestManySink<T, R>>,
    index: usize,
}

impl<T: 'static, R: 'static> IObserver<T> for CombineLatestManyObserver<T, R> {
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

/// The observable of `CombineLatest` of a list of sources (`CombineLatest<TSource, TResult>`): each
/// subscription enumerates `sources` anew.
struct CombineLatestMany<I, T, R> {
    sources: I,
    result_selector: ManyResultSelector<T, R>,
}

impl<T: 'static, R: 'static, I> IObservable<R> for CombineLatestMany<I, T, R>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        let result_selector = self.result_selector.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(CombineLatestManySink {
                    sink: Sink::new(observer),
                    result_selector,
                    has_value: RefCell::new(Vec::new()),
                    has_value_all: Cell::new(false),
                    partial: RefCell::new(Vec::new()),
                    values: RefCell::new(Vec::new()),
                    is_done: RefCell::new(Vec::new()),
                    subscriptions: RefCell::new(Vec::new()),
                })
            },
            move |sink: &Rc<CombineLatestManySink<T, R>>| sink.run(sources.into_iter().collect()),
        )
    }
}

pub(crate) fn combine_latest_many<T: 'static, R: 'static, I>(sources: I, result_selector: ManyResultSelector<T, R>) -> Rc<dyn IObservable<R>>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
{
    Rc::new(CombineLatestMany { sources, result_selector })
}

impl crate::linq::Observable {
    /// The lists of the latest values of `sources`, whenever any of them produces a value, once each
    /// has one; a source that ends keeps its last value until all have ended
    /// (`CombineLatest(IEnumerable<IObservable<TSource>>)` and
    /// `CombineLatest(params IObservable<TSource>[])`). No sources: the sequence never notifies.
    pub fn combine_latest_all<T: Clone + 'static, I>(sources: I) -> Rc<dyn IObservable<Vec<T>>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        combine_latest_many(sources, Rc::new(|values: &[T]| -> Result<Vec<T>, ObservableError> { Ok(values.to_vec()) }))
    }

    /// [`combine_latest_all`](Self::combine_latest_all) with `result_selector` applied to the latest
    /// values (`CombineLatest(IEnumerable<IObservable<TSource>>, Func<IList<TSource>, TResult>)`).
    pub fn combine_latest_all_with<T: 'static, R: 'static, I>(sources: I, result_selector: impl Fn(&[T]) -> R + 'static) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        combine_latest_many(sources, Rc::new(move |values: &[T]| -> Result<R, ObservableError> { Ok(result_selector(values)) }))
    }

    /// [`combine_latest_all_with`](Self::combine_latest_all_with) with a selector that may fail; a
    /// failure ends the sequence with its error (`CombineLatest` of a list with a selector that
    /// throws).
    pub fn try_combine_latest_all_with<T: 'static, R: 'static, I>(
        sources: I,
        result_selector: impl Fn(&[T]) -> Result<R, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    {
        combine_latest_many(sources, Rc::new(result_selector))
    }
}

/// Generates the methods of `CombineLatest` of a fixed number of sources (the T4 templates of
/// `Observable.Multiple.CombineLatest.tt`): the form with a selector, the form with a selector that
/// may fail, and the form of tuples (`ObservableEx`). The sequence itself is the first source; each
/// other source is given as its type parameter, its name and the name of its value.
macro_rules! combine_latest_arity_methods {
    ($name:ident, $try_name:ident, $tuple_name:ident, $function:ident; $doc:literal, $try_doc:literal, $tuple_doc:literal; $($T:ident $source:ident $value:ident),+) => {
        #[doc = $doc]
        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        fn $name<$($T: Clone + 'static,)+ R: 'static>(
            &self,
            $($source: std::rc::Rc<dyn ferroui_system::IObservable<$T>>,)+
            result_selector: impl Fn(T, $($T),+) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            let result_selector: std::rc::Rc<dyn Fn(T, $($T),+) -> Result<R, ferroui_system::ObservableError>> =
                std::rc::Rc::new(move |value1: T, $($value: $T),+| Ok(result_selector(value1, $($value),+)));
            crate::linq::combine_latest::$function(self.observable(), $($source,)+ result_selector)
        }

        #[doc = $try_doc]
        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        fn $try_name<$($T: Clone + 'static,)+ R: 'static>(
            &self,
            $($source: std::rc::Rc<dyn ferroui_system::IObservable<$T>>,)+
            result_selector: impl Fn(T, $($T),+) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            let result_selector: std::rc::Rc<dyn Fn(T, $($T),+) -> Result<R, ferroui_system::ObservableError>> = std::rc::Rc::new(result_selector);
            crate::linq::combine_latest::$function(self.observable(), $($source,)+ result_selector)
        }

        #[doc = $tuple_doc]
        #[allow(clippy::too_many_arguments, clippy::type_complexity)]
        fn $tuple_name<$($T: Clone + 'static),+>(
            &self,
            $($source: std::rc::Rc<dyn ferroui_system::IObservable<$T>>),+
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<(T, $($T),+)>>
        where
            T: Clone,
        {
            let result_selector: std::rc::Rc<dyn Fn(T, $($T),+) -> Result<(T, $($T),+), ferroui_system::ObservableError>> =
                std::rc::Rc::new(|value1: T, $($value: $T),+| Ok((value1, $($value),+)));
            crate::linq::combine_latest::$function(self.observable(), $($source,)+ result_selector)
        }
    };
}
pub(crate) use combine_latest_arity_methods;

/// The methods of `CombineLatest` (provided methods of [`ObservableExt`](super::ObservableExt)):
/// `combine_latest` (two sources), `combine_latest3` to `combine_latest16`, their `try_` forms and
/// their tuple forms (`combine_latest_tuple`, `combine_latest3_tuple` to `combine_latest16_tuple`).
/// The forms of a list of sources are associated functions of
/// [`Observable`](crate::linq::Observable) (`combine_latest_all`, `combine_latest_all_with`,
/// `try_combine_latest_all_with`).
macro_rules! combine_latest_methods {
    () => {
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest, try_combine_latest, combine_latest_tuple, combine_latest2;
            "Combines the latest value of the sequence and of `second` with `result_selector` whenever either produces a value, once both have one; a source that ends keeps its last value until both have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, Func<TSource1, TSource2, TResult>)`).",
            "[`combine_latest`](Self::combine_latest) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest`](Self::combine_latest) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>)`).";
            T2 second value2
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest3, try_combine_latest3, combine_latest3_tuple, combine_latest3;
            "Combines the latest values of the sequence and of the 2 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, Func<TSource1, TSource2, TSource3, TResult>)`).",
            "[`combine_latest3`](Self::combine_latest3) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest3`](Self::combine_latest3) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>)`).";
            T2 source2 value2, T3 source3 value3
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest4, try_combine_latest4, combine_latest4_tuple, combine_latest4;
            "Combines the latest values of the sequence and of the 3 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, Func<TSource1, TSource2, TSource3, TSource4, TResult>)`).",
            "[`combine_latest4`](Self::combine_latest4) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest4`](Self::combine_latest4) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest5, try_combine_latest5, combine_latest5_tuple, combine_latest5;
            "Combines the latest values of the sequence and of the 4 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TResult>)`).",
            "[`combine_latest5`](Self::combine_latest5) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest5`](Self::combine_latest5) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest6, try_combine_latest6, combine_latest6_tuple, combine_latest6;
            "Combines the latest values of the sequence and of the 5 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TResult>)`).",
            "[`combine_latest6`](Self::combine_latest6) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest6`](Self::combine_latest6) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest7, try_combine_latest7, combine_latest7_tuple, combine_latest7;
            "Combines the latest values of the sequence and of the 6 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TResult>)`).",
            "[`combine_latest7`](Self::combine_latest7) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest7`](Self::combine_latest7) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest8, try_combine_latest8, combine_latest8_tuple, combine_latest8;
            "Combines the latest values of the sequence and of the 7 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TResult>)`).",
            "[`combine_latest8`](Self::combine_latest8) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest8`](Self::combine_latest8) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest9, try_combine_latest9, combine_latest9_tuple, combine_latest9;
            "Combines the latest values of the sequence and of the 8 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TResult>)`).",
            "[`combine_latest9`](Self::combine_latest9) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest9`](Self::combine_latest9) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest10, try_combine_latest10, combine_latest10_tuple, combine_latest10;
            "Combines the latest values of the sequence and of the 9 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TResult>)`).",
            "[`combine_latest10`](Self::combine_latest10) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest10`](Self::combine_latest10) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest11, try_combine_latest11, combine_latest11_tuple, combine_latest11;
            "Combines the latest values of the sequence and of the 10 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, IObservable<TSource11>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TResult>)`).",
            "[`combine_latest11`](Self::combine_latest11) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest11`](Self::combine_latest11) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>, IObservable<TEleventh>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10, T11 source11 value11
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest12, try_combine_latest12, combine_latest12_tuple, combine_latest12;
            "Combines the latest values of the sequence and of the 11 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, IObservable<TSource11>, IObservable<TSource12>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TResult>)`).",
            "[`combine_latest12`](Self::combine_latest12) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest12`](Self::combine_latest12) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>, IObservable<TEleventh>, IObservable<TTwelfth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10, T11 source11 value11, T12 source12 value12
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest13, try_combine_latest13, combine_latest13_tuple, combine_latest13;
            "Combines the latest values of the sequence and of the 12 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, IObservable<TSource11>, IObservable<TSource12>, IObservable<TSource13>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TResult>)`).",
            "[`combine_latest13`](Self::combine_latest13) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest13`](Self::combine_latest13) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>, IObservable<TEleventh>, IObservable<TTwelfth>, IObservable<TThirteenth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10, T11 source11 value11, T12 source12 value12, T13 source13 value13
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest14, try_combine_latest14, combine_latest14_tuple, combine_latest14;
            "Combines the latest values of the sequence and of the 13 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, IObservable<TSource11>, IObservable<TSource12>, IObservable<TSource13>, IObservable<TSource14>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TResult>)`).",
            "[`combine_latest14`](Self::combine_latest14) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest14`](Self::combine_latest14) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>, IObservable<TEleventh>, IObservable<TTwelfth>, IObservable<TThirteenth>, IObservable<TFourteenth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10, T11 source11 value11, T12 source12 value12, T13 source13 value13, T14 source14 value14
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest15, try_combine_latest15, combine_latest15_tuple, combine_latest15;
            "Combines the latest values of the sequence and of the 14 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, IObservable<TSource11>, IObservable<TSource12>, IObservable<TSource13>, IObservable<TSource14>, IObservable<TSource15>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TSource15, TResult>)`).",
            "[`combine_latest15`](Self::combine_latest15) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest15`](Self::combine_latest15) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>, IObservable<TEleventh>, IObservable<TTwelfth>, IObservable<TThirteenth>, IObservable<TFourteenth>, IObservable<TFifteenth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10, T11 source11 value11, T12 source12 value12, T13 source13 value13, T14 source14 value14, T15 source15 value15
        );
        crate::linq::combine_latest::combine_latest_arity_methods!(
            combine_latest16, try_combine_latest16, combine_latest16_tuple, combine_latest16;
            "Combines the latest values of the sequence and of the 15 other sources with `result_selector` whenever any of them produces a value, once each has one; a source that ends keeps its last value until all have ended (`CombineLatest(IObservable<TSource1>, IObservable<TSource2>, IObservable<TSource3>, IObservable<TSource4>, IObservable<TSource5>, IObservable<TSource6>, IObservable<TSource7>, IObservable<TSource8>, IObservable<TSource9>, IObservable<TSource10>, IObservable<TSource11>, IObservable<TSource12>, IObservable<TSource13>, IObservable<TSource14>, IObservable<TSource15>, IObservable<TSource16>, Func<TSource1, TSource2, TSource3, TSource4, TSource5, TSource6, TSource7, TSource8, TSource9, TSource10, TSource11, TSource12, TSource13, TSource14, TSource15, TSource16, TResult>)`).",
            "[`combine_latest16`](Self::combine_latest16) with a selector that may fail; a failure ends the sequence with its error (`CombineLatest` with a selector that throws).",
            "[`combine_latest16`](Self::combine_latest16) with the latest values as a tuple (`ObservableEx.CombineLatest(IObservable<TFirst>, IObservable<TSecond>, IObservable<TThird>, IObservable<TFourth>, IObservable<TFifth>, IObservable<TSixth>, IObservable<TSeventh>, IObservable<TEighth>, IObservable<TNinth>, IObservable<TTenth>, IObservable<TEleventh>, IObservable<TTwelfth>, IObservable<TThirteenth>, IObservable<TFourteenth>, IObservable<TFifteenth>, IObservable<TSixteenth>)`).";
            T2 source2 value2, T3 source3 value3, T4 source4 value4, T5 source5 value5, T6 source6 value6, T7 source7 value7, T8 source8 value8, T9 source9 value9, T10 source10 value10, T11 source11 value11, T12 source12 value12, T13 source13 value13, T14 source14 value14, T15 source15 value15, T16 source16 value16
        );
    };
}
pub(crate) use combine_latest_methods;
