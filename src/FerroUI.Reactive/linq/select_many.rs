//! `SelectMany` (`Linq/Observable/SelectMany.cs`).
//!
//! The forms that project to sequences (with or without an index, with or without a result
//! selector, with separate selectors for the values, the error and the completion of the source)
//! share one sink, as the classes of upstream share one algorithm: each projected sequence is
//! subscribed to as it arrives, its values are forwarded (through the result selector, which keeps
//! the source value and both indexes), and the result ends when the source and every projected
//! sequence have ended. The forms that project to iterators share another sink, which iterates
//! each projection to its end while it handles the source value. The forms on tasks are not
//! ported (phase 4).

use crate::disposables::{CompositeDisposable, SingleAssignmentDisposableValue};
use crate::internal::{unary, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The projection of the values of one projected sequence to results, with the index of each value
/// in that sequence (the result selector bound to the source value and its index, or the identity).
pub(crate) type Project<C, R> = Box<dyn Fn(C, usize) -> Result<R, ObservableError>>;

/// A projected sequence with the projection of its values.
pub(crate) type Inner<C, R> = (Rc<dyn IObservable<C>>, Project<C, R>);

type CollectionSelector<T, C, R> = Rc<dyn Fn(T, usize) -> Result<Inner<C, R>, ObservableError>>;
type ErrorSelector<C, R> = Rc<dyn Fn(ObservableError) -> Result<Inner<C, R>, ObservableError>>;
type CompletedSelector<C, R> = Rc<dyn Fn() -> Result<Inner<C, R>, ObservableError>>;

/// The projection of the values of a projected sequence of the forms without a result selector.
pub(crate) fn identity<U: 'static>() -> Project<U, U> {
    Box::new(|value, _| Ok(value))
}

/// The sink of the forms on sequences (`SelectMany<TSource, TCollection, TResult>.ObservableSelector._`
/// and `ObservableSelectorIndexed._`, `SelectMany<TSource, TResult>.ObservableSelector._`,
/// `ObservableSelectorIndexed._`, `ObservableSelectors._` and `ObservableSelectorsIndexed._`): a
/// selector that fails ends the sequence with its error.
struct SelectManySink<T, C, R> {
    this: Weak<SelectManySink<T, C, R>>,
    sink: Sink<R>,
    group: Rc<CompositeDisposable>,
    selector: CollectionSelector<T, C, R>,
    on_error_selector: Option<ErrorSelector<C, R>>,
    on_completed_selector: Option<CompletedSelector<C, R>>,
    index: Cell<usize>,
    is_stopped: Cell<bool>,
}

impl<T: 'static, C: 'static, R: 'static> SelectManySink<T, C, R> {
    /// Ends the sequence when no projected sequence is running, or stops observing the source
    /// otherwise (`Final`).
    fn final_(&self) {
        self.is_stopped.set(true);
        if self.group.count() == 0 {
            self.sink.forward_on_completed();
        } else {
            self.sink.dispose_upstream();
        }
    }

    /// Subscribes to a projected sequence (`SubscribeInner`).
    fn subscribe_inner(&self, inner: Inner<C, R>) {
        let (collection, project) = inner;
        let parent = self.this.upgrade().expect("the sink is alive while it receives notifications");
        let observer = Rc::new_cyclic(|this| SelectManyInnerObserver {
            this: this.clone(),
            parent,
            project,
            index: Cell::new(0),
            resource: SingleAssignmentDisposableValue::new(),
        });
        self.group.add(observer.clone());
        let subscription = collection.subscribe(observer.clone());
        observer.resource.set_disposable(Some(subscription));
    }
}

impl<T: 'static, C: 'static, R: 'static> OperatorSink<T, R> for SelectManySink<T, C, R> {
    fn sink(&self) -> &Sink<R> {
        &self.sink
    }
}

impl<T: 'static, C: 'static, R: 'static> IObserver<T> for SelectManySink<T, C, R> {
    fn on_next(&self, value: T) {
        let index = self.index.get();
        self.index.set(index.wrapping_add(1));
        match (self.selector)(value, index) {
            Ok(inner) => self.subscribe_inner(inner),
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        match &self.on_error_selector {
            Some(selector) => match selector(error) {
                Ok(inner) => {
                    self.subscribe_inner(inner);
                    self.final_();
                }
                Err(error) => self.sink.forward_on_error(error),
            },
            None => self.sink.forward_on_error(error),
        }
    }

    fn on_completed(&self) {
        if let Some(selector) = &self.on_completed_selector {
            match selector() {
                Ok(inner) => self.subscribe_inner(inner),
                Err(error) => {
                    self.sink.forward_on_error(error);
                    return;
                }
            }
        }
        self.final_();
    }
}

impl<T: 'static, C: 'static, R: 'static> IDisposable for SelectManySink<T, C, R> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.group.dispose();
        }
    }
}

/// The observer of a projected sequence (`InnerObserver`, a `SafeObserver<TCollection>`): it is the
/// disposable of its subscription in the group of the sink.
struct SelectManyInnerObserver<T, C, R> {
    this: Weak<SelectManyInnerObserver<T, C, R>>,
    parent: Rc<SelectManySink<T, C, R>>,
    project: Project<C, R>,
    index: Cell<usize>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static, C: 'static, R: 'static> IObserver<C> for SelectManyInnerObserver<T, C, R> {
    fn on_next(&self, value: C) {
        let index = self.index.get();
        self.index.set(index.wrapping_add(1));
        match (self.project)(value, index) {
            Ok(result) => self.parent.sink.forward_on_next(result),
            Err(error) => self.parent.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let parent = &self.parent;
        if let Some(this) = self.this.upgrade() {
            let this: Rc<dyn IDisposable> = this;
            parent.group.remove(&this);
        }
        if parent.is_stopped.get() && parent.group.count() == 0 {
            parent.sink.forward_on_completed();
        }
    }
}

impl<T, C, R> IDisposable for SelectManyInnerObserver<T, C, R> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// `SelectMany` to sequences: `selector` projects each source value (with its index) to a sequence
/// and the projection of its values; `on_error` and `on_completed` project the terminal
/// notifications of the source to sequences too (the forms with three selectors).
pub(crate) fn select_many_observable<T: 'static, C: 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    selector: CollectionSelector<T, C, R>,
    on_error_selector: Option<ErrorSelector<C, R>>,
    on_completed_selector: Option<CompletedSelector<C, R>>,
) -> Rc<dyn IObservable<R>> {
    unary(source, move |observer| {
        let (selector, on_error_selector, on_completed_selector) = (selector.clone(), on_error_selector.clone(), on_completed_selector.clone());
        Rc::new_cyclic(|this| SelectManySink {
            this: this.clone(),
            sink: Sink::new(observer),
            group: CompositeDisposable::new(),
            selector,
            on_error_selector,
            on_completed_selector,
            index: Cell::new(0),
            is_stopped: Cell::new(false),
        })
    })
}

/// `SelectMany` of a selector to sequences, with the index of each source value.
pub(crate) fn select_many<T: 'static, U: 'static>(
    source: Rc<dyn IObservable<T>>,
    selector: impl Fn(T, usize) -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
) -> Rc<dyn IObservable<U>> {
    let selector: CollectionSelector<T, U, U> = Rc::new(move |value, index| -> Result<Inner<U, U>, ObservableError> { Ok((selector(value, index)?, identity())) });
    select_many_observable(source, selector, None, None)
}

/// `SelectMany` of a collection selector to sequences and a result selector of the source value,
/// its index, a value of its projected sequence and the index of that value.
pub(crate) fn select_many_with<T: Clone + 'static, C: 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    collection_selector: impl Fn(T, usize) -> Result<Rc<dyn IObservable<C>>, ObservableError> + 'static,
    result_selector: impl Fn(T, usize, C, usize) -> Result<R, ObservableError> + 'static,
) -> Rc<dyn IObservable<R>> {
    let result_selector = Rc::new(result_selector);
    select_many_observable(
        source,
        Rc::new(move |value: T, index| -> Result<Inner<C, R>, ObservableError> {
            let collection = collection_selector(value.clone(), index)?;
            let result_selector = result_selector.clone();
            let project: Project<C, R> = Box::new(move |item, item_index| result_selector(value.clone(), index, item, item_index));
            Ok((collection, project))
        }),
        None,
        None,
    )
}

/// `SelectMany` with a selector of the values (with the index of each), of the error and of the
/// completion of the source.
pub(crate) fn select_many_notifications<T: 'static, U: 'static>(
    source: Rc<dyn IObservable<T>>,
    on_next: impl Fn(T, usize) -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
    on_error: impl Fn(ObservableError) -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
    on_completed: impl Fn() -> Result<Rc<dyn IObservable<U>>, ObservableError> + 'static,
) -> Rc<dyn IObservable<U>> {
    let on_next: CollectionSelector<T, U, U> = Rc::new(move |value, index| -> Result<Inner<U, U>, ObservableError> { Ok((on_next(value, index)?, identity())) });
    let on_error: ErrorSelector<U, U> = Rc::new(move |error| -> Result<Inner<U, U>, ObservableError> { Ok((on_error(error)?, identity())) });
    let on_completed: CompletedSelector<U, U> = Rc::new(move || -> Result<Inner<U, U>, ObservableError> { Ok((on_completed()?, identity())) });
    select_many_observable(source, on_next, Some(on_error), Some(on_completed))
}

/// The iteration of a projection to an iterator: each step gives a result or an error.
pub(crate) type Enumerator<R> = Box<dyn Iterator<Item = Result<R, ObservableError>>>;

type IterSelector<T, R> = Rc<dyn Fn(T, usize) -> Result<Enumerator<R>, ObservableError>>;

/// The sink of the forms on iterators (`SelectMany<TSource, TResult>.EnumerableSelector._` and
/// `EnumerableSelectorIndexed._`, `SelectMany<TSource, TCollection, TResult>.EnumerableSelector._`
/// and `EnumerableSelectorIndexed._`): each projection is iterated to its end as the source value
/// is handled; a selector or a step that fails ends the sequence with its error, and the iterator
/// is dropped as the value has been handled (`using (e)`).
struct SelectManyIterSink<T, R> {
    sink: Sink<R>,
    selector: IterSelector<T, R>,
    index: Cell<usize>,
}

impl<T: 'static, R: 'static> OperatorSink<T, R> for SelectManyIterSink<T, R> {
    fn sink(&self) -> &Sink<R> {
        &self.sink
    }
}

impl<T: 'static, R: 'static> IObserver<T> for SelectManyIterSink<T, R> {
    fn on_next(&self, value: T) {
        let index = self.index.get();
        self.index.set(index.wrapping_add(1));
        let enumerator = match (self.selector)(value, index) {
            Ok(enumerator) => enumerator,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };
        for current in enumerator {
            match current {
                Ok(current) => self.sink.forward_on_next(current),
                Err(error) => {
                    self.sink.forward_on_error(error);
                    return;
                }
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

impl<T: 'static, R: 'static> IDisposable for SelectManyIterSink<T, R> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

fn select_many_enumerable<T: 'static, R: 'static>(source: Rc<dyn IObservable<T>>, selector: IterSelector<T, R>) -> Rc<dyn IObservable<R>> {
    unary(source, move |observer| Rc::new(SelectManyIterSink { sink: Sink::new(observer), selector: selector.clone(), index: Cell::new(0) }))
}

/// `SelectMany` to iterators: `selector` projects each source value (with its index) to an
/// iterable whose creation and steps may fail.
pub(crate) fn select_many_iter<T: 'static, U: 'static, I>(
    source: Rc<dyn IObservable<T>>,
    selector: impl Fn(T, usize) -> Result<I, ObservableError> + 'static,
) -> Rc<dyn IObservable<U>>
where
    I: IntoIterator<Item = Result<U, ObservableError>>,
    I::IntoIter: 'static,
{
    select_many_enumerable(
        source,
        Rc::new(move |value, index| -> Result<Enumerator<U>, ObservableError> {
            let enumerator: Enumerator<U> = Box::new(selector(value, index)?.into_iter());
            Ok(enumerator)
        }),
    )
}

/// `SelectMany` to iterators with a result selector of the source value, its index, a value of
/// its projection and the index of that value.
pub(crate) fn select_many_iter_with<T: Clone + 'static, C: 'static, R: 'static, I>(
    source: Rc<dyn IObservable<T>>,
    collection_selector: impl Fn(T, usize) -> Result<I, ObservableError> + 'static,
    result_selector: impl Fn(T, usize, C, usize) -> Result<R, ObservableError> + 'static,
) -> Rc<dyn IObservable<R>>
where
    I: IntoIterator<Item = Result<C, ObservableError>>,
    I::IntoIter: 'static,
{
    let result_selector = Rc::new(result_selector);
    select_many_enumerable(
        source,
        Rc::new(move |value: T, index| -> Result<Enumerator<R>, ObservableError> {
            let collection = collection_selector(value.clone(), index)?.into_iter();
            let result_selector = result_selector.clone();
            let enumerator: Enumerator<R> = Box::new(
                collection.enumerate().map(move |(item_index, item)| item.and_then(|item| result_selector(value.clone(), index, item, item_index))),
            );
            Ok(enumerator)
        }),
    )
}

/// The methods of `SelectMany` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! select_many_methods {
    () => {
        /// Projects each value to a sequence and merges the sequences
        /// (`SelectMany(Func<TSource, IObservable<TResult>>)`).
        fn select_many<U: 'static>(
            &self,
            selector: impl Fn(T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many(self.observable(), move |value, _| Ok(selector(value)))
        }

        /// [`select_many`](Self::select_many) with the index of each value
        /// (`SelectMany(Func<TSource, int, IObservable<TResult>>)`).
        fn select_many_with_index<U: 'static>(
            &self,
            selector: impl Fn(T, usize) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many(self.observable(), move |value, index| Ok(selector(value, index)))
        }

        /// [`select_many`](Self::select_many) with a selector that may fail; a failure ends the
        /// sequence with its error (`SelectMany(Func<TSource, IObservable<TResult>>)` with a
        /// selector that throws).
        fn try_select_many<U: 'static>(
            &self,
            selector: impl Fn(T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many(self.observable(), move |value, _| selector(value))
        }

        /// [`try_select_many`](Self::try_select_many) with the index of each value
        /// (`SelectMany(Func<TSource, int, IObservable<TResult>>)` with a selector that throws).
        fn try_select_many_with_index<U: 'static>(
            &self,
            selector: impl Fn(T, usize) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many(self.observable(), selector)
        }

        /// Subscribes to `other` for each value and merges the subscriptions
        /// (`SelectMany(IObservable<TSource>, IObservable<TOther>)`).
        fn select_many_to<U: 'static>(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<U>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many(self.observable(), move |_, _| Ok(other.clone()))
        }

        /// Projects each value to a sequence and each value of that sequence, with the source value,
        /// to a result (`SelectMany(Func<TSource, IObservable<TCollection>>, Func<TSource,
        /// TCollection, TResult>)`).
        fn select_many_with<C: 'static, R: 'static>(
            &self,
            collection_selector: impl Fn(T) -> std::rc::Rc<dyn ferroui_system::IObservable<C>> + 'static,
            result_selector: impl Fn(T, C) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::select_many::select_many_with(
                self.observable(),
                move |value, _| Ok(collection_selector(value)),
                move |value, _, item, _| Ok(result_selector(value, item)),
            )
        }

        /// [`select_many_with`](Self::select_many_with) with the index of each value and of each
        /// value of its sequence (`SelectMany(Func<TSource, int, IObservable<TCollection>>,
        /// Func<TSource, int, TCollection, int, TResult>)`).
        fn select_many_with_index_with<C: 'static, R: 'static>(
            &self,
            collection_selector: impl Fn(T, usize) -> std::rc::Rc<dyn ferroui_system::IObservable<C>> + 'static,
            result_selector: impl Fn(T, usize, C, usize) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::select_many::select_many_with(
                self.observable(),
                move |value, index| Ok(collection_selector(value, index)),
                move |value, index, item, item_index| Ok(result_selector(value, index, item, item_index)),
            )
        }

        /// [`select_many_with`](Self::select_many_with) with selectors that may fail; a failure ends
        /// the sequence with its error (`SelectMany(Func<TSource, IObservable<TCollection>>,
        /// Func<TSource, TCollection, TResult>)` with selectors that throw).
        fn try_select_many_with<C: 'static, R: 'static>(
            &self,
            collection_selector: impl Fn(T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<C>>, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, C) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::select_many::select_many_with(
                self.observable(),
                move |value, _| collection_selector(value),
                move |value, _, item, _| result_selector(value, item),
            )
        }

        /// [`select_many_with_index_with`](Self::select_many_with_index_with) with selectors that may
        /// fail (`SelectMany(Func<TSource, int, IObservable<TCollection>>, Func<TSource, int,
        /// TCollection, int, TResult>)` with selectors that throw).
        fn try_select_many_with_index_with<C: 'static, R: 'static>(
            &self,
            collection_selector: impl Fn(T, usize) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<C>>, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, usize, C, usize) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::select_many::select_many_with(self.observable(), collection_selector, result_selector)
        }

        /// Projects each value, the error and the completion of the sequence to sequences and merges
        /// them (`SelectMany(Func<TSource, IObservable<TResult>>, Func<Exception,
        /// IObservable<TResult>>, Func<IObservable<TResult>>)`).
        fn select_many_notifications<U: 'static>(
            &self,
            on_next: impl Fn(T) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
            on_completed: impl Fn() -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many_notifications(
                self.observable(),
                move |value, _| Ok(on_next(value)),
                move |error| Ok(on_error(error)),
                move || Ok(on_completed()),
            )
        }

        /// [`select_many_notifications`](Self::select_many_notifications) with the index of each
        /// value (`SelectMany(Func<TSource, int, IObservable<TResult>>, Func<Exception,
        /// IObservable<TResult>>, Func<IObservable<TResult>>)`).
        fn select_many_notifications_with_index<U: 'static>(
            &self,
            on_next: impl Fn(T, usize) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
            on_completed: impl Fn() -> std::rc::Rc<dyn ferroui_system::IObservable<U>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many_notifications(
                self.observable(),
                move |value, index| Ok(on_next(value, index)),
                move |error| Ok(on_error(error)),
                move || Ok(on_completed()),
            )
        }

        /// [`select_many_notifications`](Self::select_many_notifications) with selectors that may
        /// fail; a failure ends the sequence with its error (the form with three selectors, with
        /// selectors that throw).
        fn try_select_many_notifications<U: 'static>(
            &self,
            on_next: impl Fn(T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
            on_completed: impl Fn() -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many_notifications(self.observable(), move |value, _| on_next(value), on_error, on_completed)
        }

        /// [`try_select_many_notifications`](Self::try_select_many_notifications) with the index of
        /// each value (the indexed form with three selectors, with selectors that throw).
        fn try_select_many_notifications_with_index<U: 'static>(
            &self,
            on_next: impl Fn(T, usize) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
            on_completed: impl Fn() -> Result<std::rc::Rc<dyn ferroui_system::IObservable<U>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>> {
            crate::linq::select_many::select_many_notifications(self.observable(), on_next, on_error, on_completed)
        }

        /// Projects each value to an iterable and notifies its values as the value is handled
        /// (`SelectMany(Func<TSource, IEnumerable<TResult>>)`).
        fn select_many_iter<U: 'static, I>(&self, selector: impl Fn(T) -> I + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            I: IntoIterator<Item = U>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter(self.observable(), move |value, _| {
                Ok(selector(value).into_iter().map(Ok::<U, ferroui_system::ObservableError>))
            })
        }

        /// [`select_many_iter`](Self::select_many_iter) with the index of each value
        /// (`SelectMany(Func<TSource, int, IEnumerable<TResult>>)`).
        fn select_many_iter_with_index<U: 'static, I>(&self, selector: impl Fn(T, usize) -> I + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            I: IntoIterator<Item = U>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter(self.observable(), move |value, index| {
                Ok(selector(value, index).into_iter().map(Ok::<U, ferroui_system::ObservableError>))
            })
        }

        /// [`select_many_iter`](Self::select_many_iter) of an iterable whose creation and steps may
        /// fail; a failure ends the sequence with its error (`SelectMany(Func<TSource,
        /// IEnumerable<TResult>>)` with a selector, `GetEnumerator`, `MoveNext` or `Current` that
        /// throws).
        fn try_select_many_iter<U: 'static, I>(
            &self,
            selector: impl Fn(T) -> Result<I, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            I: IntoIterator<Item = Result<U, ferroui_system::ObservableError>>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter(self.observable(), move |value, _| selector(value))
        }

        /// [`try_select_many_iter`](Self::try_select_many_iter) with the index of each value
        /// (`SelectMany(Func<TSource, int, IEnumerable<TResult>>)` with failures).
        fn try_select_many_iter_with_index<U: 'static, I>(
            &self,
            selector: impl Fn(T, usize) -> Result<I, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            I: IntoIterator<Item = Result<U, ferroui_system::ObservableError>>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter(self.observable(), selector)
        }

        /// Projects each value to an iterable and each value of it, with the source value, to a
        /// result (`SelectMany(Func<TSource, IEnumerable<TCollection>>, Func<TSource, TCollection,
        /// TResult>)`).
        fn select_many_iter_with<C: 'static, R: 'static, I>(
            &self,
            collection_selector: impl Fn(T) -> I + 'static,
            result_selector: impl Fn(T, C) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
            I: IntoIterator<Item = C>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter_with(
                self.observable(),
                move |value, _| Ok(collection_selector(value).into_iter().map(Ok::<C, ferroui_system::ObservableError>)),
                move |value, _, item, _| Ok(result_selector(value, item)),
            )
        }

        /// [`select_many_iter_with`](Self::select_many_iter_with) with the index of each value and
        /// of each value of its iterable (`SelectMany(Func<TSource, int, IEnumerable<TCollection>>,
        /// Func<TSource, int, TCollection, int, TResult>)`).
        fn select_many_iter_with_index_with<C: 'static, R: 'static, I>(
            &self,
            collection_selector: impl Fn(T, usize) -> I + 'static,
            result_selector: impl Fn(T, usize, C, usize) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
            I: IntoIterator<Item = C>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter_with(
                self.observable(),
                move |value, index| Ok(collection_selector(value, index).into_iter().map(Ok::<C, ferroui_system::ObservableError>)),
                move |value, index, item, item_index| Ok(result_selector(value, index, item, item_index)),
            )
        }

        /// [`select_many_iter_with`](Self::select_many_iter_with) with an iterable whose creation and
        /// steps may fail, and a result selector that may fail (`SelectMany(Func<TSource,
        /// IEnumerable<TCollection>>, Func<TSource, TCollection, TResult>)` with failures).
        fn try_select_many_iter_with<C: 'static, R: 'static, I>(
            &self,
            collection_selector: impl Fn(T) -> Result<I, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, C) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
            I: IntoIterator<Item = Result<C, ferroui_system::ObservableError>>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter_with(
                self.observable(),
                move |value, _| collection_selector(value),
                move |value, _, item, _| result_selector(value, item),
            )
        }

        /// [`try_select_many_iter_with`](Self::try_select_many_iter_with) with the index of each
        /// value and of each value of its iterable (`SelectMany(Func<TSource, int,
        /// IEnumerable<TCollection>>, Func<TSource, int, TCollection, int, TResult>)` with
        /// failures).
        fn try_select_many_iter_with_index_with<C: 'static, R: 'static, I>(
            &self,
            collection_selector: impl Fn(T, usize) -> Result<I, ferroui_system::ObservableError> + 'static,
            result_selector: impl Fn(T, usize, C, usize) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
            I: IntoIterator<Item = Result<C, ferroui_system::ObservableError>>,
            I::IntoIter: 'static,
        {
            crate::linq::select_many::select_many_iter_with(self.observable(), collection_selector, result_selector)
        }
    };
}
pub(crate) use select_many_methods;
