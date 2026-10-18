//! `Merge` (`Linq/Observable/Merge.cs`).
//!
//! The forms on an enumerable of sequences go through the conversion of the enumerable to a sequence
//! on a scheduler (`ToObservable`, `Linq/Observable/ToObservable.cs`, the recursive form), as
//! upstream's overloads do.

use crate::concurrency::{IScheduler, SchedulerDefaults};
use crate::disposables::{CompositeDisposable, Disposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

/// A value that is an observable sequence of `T`: the inner sequences of the operators on a
/// sequence of sequences (`Merge`, `Concat` and `Switch` of `IObservable<IObservable<T>>`, and the
/// windows of `Window`). Implemented for every `Rc` of an observable, `Rc<dyn IObservable<T>>` among
/// them, so that a sequence of subjects or of test observables is a sequence of sequences too.
pub trait ObservableSource<T> {
    /// Subscribes `observer` to the sequence.
    fn subscribe_source(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable>;
}

impl<T, O: IObservable<T> + ?Sized> ObservableSource<T> for Rc<O> {
    fn subscribe_source(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        (**self).subscribe(observer)
    }
}

/// The sink of `Merge` (`Merge<TSource>.Observables._` and, with a limit of concurrent inner
/// subscriptions, `Merge<TSource>.ObservablesMaxConcurrency._`): it subscribes to each inner
/// sequence as it arrives (or queues it while the limit is reached), and ends when the outer
/// sequence and every inner one have ended.
struct MergeSink<S, T> {
    this: Weak<MergeSink<S, T>>,
    sink: Sink<T>,
    group: Rc<CompositeDisposable>,
    max_concurrent: Option<usize>,
    queue: RefCell<VecDeque<S>>,
    active_count: Cell<usize>,
    is_stopped: Cell<bool>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> MergeSink<S, T> {
    fn subscribe_inner(&self, inner_source: S) {
        let parent = self.this.upgrade().expect("the sink is alive while it receives notifications");
        let inner = Rc::new_cyclic(|this| MergeInnerObserver { this: this.clone(), parent, resource: SingleAssignmentDisposableValue::new() });
        self.group.add(inner.clone());
        let subscription = inner_source.subscribe_source(inner.clone());
        inner.resource.set_disposable(Some(subscription));
    }
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObserver<S> for MergeSink<S, T> {
    fn on_next(&self, value: S) {
        match self.max_concurrent {
            None => self.subscribe_inner(value),
            Some(max_concurrent) => {
                if self.active_count.get() < max_concurrent {
                    self.active_count.set(self.active_count.get() + 1);
                    self.subscribe_inner(value);
                } else {
                    self.queue.borrow_mut().push_back(value);
                }
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.is_stopped.set(true);
        let idle = match self.max_concurrent {
            None => self.group.count() == 0,
            Some(_) => self.active_count.get() == 0,
        };
        if idle {
            self.sink.forward_on_completed();
        } else {
            self.sink.dispose_upstream();
        }
    }
}

impl<S: 'static, T: 'static> IDisposable for MergeSink<S, T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.group.dispose();
        }
    }
}

/// The observer of an inner sequence of `Merge` (`InnerObserver`, a `SafeObserver<TSource>`): it is
/// the disposable of its subscription in the group of the sink.
struct MergeInnerObserver<S, T> {
    this: Weak<MergeInnerObserver<S, T>>,
    parent: Rc<MergeSink<S, T>>,
    resource: SingleAssignmentDisposableValue,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObserver<T> for MergeInnerObserver<S, T> {
    fn on_next(&self, value: T) {
        self.parent.sink.forward_on_next(value);
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
        match parent.max_concurrent {
            None => {
                if parent.is_stopped.get() && parent.group.count() == 0 {
                    parent.sink.forward_on_completed();
                }
            }
            Some(_) => {
                let next = parent.queue.borrow_mut().pop_front();
                match next {
                    Some(source) => parent.subscribe_inner(source),
                    None => {
                        let active_count = parent.active_count.get().saturating_sub(1);
                        parent.active_count.set(active_count);
                        if parent.is_stopped.get() && active_count == 0 {
                            parent.sink.forward_on_completed();
                        }
                    }
                }
            }
        }
    }
}

impl<S, T> IDisposable for MergeInnerObserver<S, T> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `Merge` on a sequence of sequences (`Merge<TSource>.Observables` and
/// `Merge<TSource>.ObservablesMaxConcurrency`).
struct MergeObservable<S, T> {
    sources: Rc<dyn IObservable<S>>,
    max_concurrent: Option<usize>,
    _values: PhantomData<fn() -> T>,
}

impl<S: ObservableSource<T> + 'static, T: 'static> IObservable<T> for MergeObservable<S, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sources = self.sources.clone();
        let max_concurrent = self.max_concurrent;
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new_cyclic(|this| MergeSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    group: CompositeDisposable::new(),
                    max_concurrent,
                    queue: RefCell::new(VecDeque::new()),
                    active_count: Cell::new(0),
                    is_stopped: Cell::new(false),
                })
            },
            move |sink: &Rc<MergeSink<S, T>>| {
                let upstream = sources.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

/// `Merge` of a sequence of sequences, with an optional limit of concurrent inner subscriptions.
/// Panics when the limit is zero (upstream throws `ArgumentOutOfRangeException`).
pub(crate) fn merge_observables<S: ObservableSource<T> + 'static, T: 'static>(
    sources: Rc<dyn IObservable<S>>,
    max_concurrent: Option<usize>,
) -> Rc<dyn IObservable<T>> {
    if max_concurrent == Some(0) {
        panic!("Specified argument was out of the range of valid values. (Parameter 'maxConcurrent')");
    }
    Rc::new(MergeObservable { sources, max_concurrent, _values: PhantomData })
}

/// `Merge` of an enumerable of sequences: the enumerable as a sequence on `scheduler`
/// (`sources.ToObservable(scheduler)`), merged.
pub(crate) fn merge_enumerable<T: 'static, I>(sources: I, max_concurrent: Option<usize>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>
where
    I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
    I::IntoIter: 'static,
{
    merge_observables(enumerable_to_observable(sources, scheduler), max_concurrent)
}

/// The sink of the conversion of an enumerable to a sequence (`ToObservableRecursive<TSource>._`):
/// each value is produced by work of its own on the scheduler, which is never cancelled; a disposed
/// sink drops the enumerator the next time the work runs.
struct EnumerableSink<T> {
    sink: Sink<T>,
    iterator: RefCell<Option<Box<dyn Iterator<Item = T>>>>,
    disposed: Cell<bool>,
}

impl<T: 'static> EnumerableSink<T> {
    fn loop_rec(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        if self.disposed.get() {
            self.iterator.borrow_mut().take();
            return Disposable::empty();
        }
        let taken = self.iterator.borrow_mut().take();
        let Some(mut iterator) = taken else {
            return Disposable::empty();
        };
        match iterator.next() {
            None => {
                drop(iterator);
                self.sink.forward_on_completed();
            }
            Some(current) => {
                *self.iterator.borrow_mut() = Some(iterator);
                self.sink.forward_on_next(current);
                let this = self.clone();
                scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
            }
        }
        Disposable::empty()
    }
}

impl<T: 'static> IDisposable for EnumerableSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.disposed.set(true);
        }
    }
}

/// An enumerable as a sequence on a scheduler (`ToObservableRecursive<TSource>`): each subscription
/// enumerates the enumerable anew.
struct EnumerableObservable<I, T> {
    source: I,
    scheduler: Rc<dyn IScheduler>,
    _values: PhantomData<fn() -> T>,
}

impl<T: 'static, I> IObservable<T> for EnumerableObservable<I, T>
where
    I: IntoIterator<Item = T> + Clone + 'static,
    I::IntoIter: 'static,
{
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(EnumerableSink { sink: Sink::new(observer), iterator: RefCell::new(None), disposed: Cell::new(false) }),
            move |sink: &Rc<EnumerableSink<T>>| {
                *sink.iterator.borrow_mut() = Some(Box::new(source.into_iter()));
                let this = sink.clone();
                // The work is never cancelled: a disposed sink stops at its next run.
                scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
            },
        )
    }
}

/// An enumerable as a sequence on `scheduler` (`ToObservable(IEnumerable<TSource>, IScheduler)`).
pub(crate) fn enumerable_to_observable<T: 'static, I>(source: I, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>
where
    I: IntoIterator<Item = T> + Clone + 'static,
    I::IntoIter: 'static,
{
    Rc::new(EnumerableObservable { source, scheduler, _values: PhantomData })
}

impl crate::linq::Observable {
    /// Merges the values of the sequences of an enumerable, enumerated on the immediate scheduler
    /// (`Merge(IEnumerable<IObservable<TSource>>)` and `Merge(params IObservable<TSource>[])`). Each
    /// subscription enumerates `sources` anew.
    pub fn merge_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        merge_enumerable(sources, None, SchedulerDefaults::constant_time_operations())
    }

    /// [`merge_all`](Self::merge_all) enumerating on `scheduler`
    /// (`Merge(IEnumerable<IObservable<TSource>>, IScheduler)` and
    /// `Merge(IScheduler, params IObservable<TSource>[])`).
    pub fn merge_all_on<T: 'static, I>(sources: I, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        merge_enumerable(sources, None, scheduler)
    }

    /// Merges the values of the sequences of an enumerable with at most `max_concurrent` inner
    /// subscriptions at a time (`Merge(IEnumerable<IObservable<TSource>>, int)`). Panics when
    /// `max_concurrent` is zero.
    pub fn merge_all_max<T: 'static, I>(sources: I, max_concurrent: usize) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        merge_enumerable(sources, Some(max_concurrent), SchedulerDefaults::constant_time_operations())
    }

    /// [`merge_all_max`](Self::merge_all_max) enumerating on `scheduler`
    /// (`Merge(IEnumerable<IObservable<TSource>>, int, IScheduler)`).
    pub fn merge_all_max_on<T: 'static, I>(sources: I, max_concurrent: usize, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        merge_enumerable(sources, Some(max_concurrent), scheduler)
    }
}

/// The methods of `Merge` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! merge_methods {
    () => {
        /// Merges the values of the sequence and of `other`
        /// (`Merge(IObservable<TSource>, IObservable<TSource>)`).
        fn merge(&self, other: std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::merge::merge_enumerable(
                vec![self.observable(), other],
                None,
                crate::concurrency::SchedulerDefaults::constant_time_operations(),
            )
        }

        /// [`merge`](Self::merge) with the two sequences enumerated on `scheduler`
        /// (`Merge(IObservable<TSource>, IObservable<TSource>, IScheduler)`).
        fn merge_on(
            &self,
            other: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::merge::merge_enumerable(vec![self.observable(), other], None, scheduler)
        }

        /// Merges the values of the inner sequences of a sequence of sequences
        /// (`Merge(IObservable<IObservable<TSource>>)`).
        fn merge_inner<U: 'static>(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            T: crate::linq::merge::ObservableSource<U>,
        {
            crate::linq::merge::merge_observables(self.observable(), None)
        }

        /// [`merge_inner`](Self::merge_inner) with at most `max_concurrent` inner subscriptions at a
        /// time; the others wait in order (`Merge(IObservable<IObservable<TSource>>, int)`). Panics
        /// when `max_concurrent` is zero.
        fn merge_inner_max<U: 'static>(&self, max_concurrent: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<U>>
        where
            T: crate::linq::merge::ObservableSource<U>,
        {
            crate::linq::merge::merge_observables(self.observable(), Some(max_concurrent))
        }
    };
}
pub(crate) use merge_methods;
