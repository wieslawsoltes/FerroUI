//! `GroupByUntil` (`Linq/Observable/GroupByUntil.cs`): `GroupBy` where each group lasts until a
//! sequence its duration selector gives produces a value or ends; a value of the key after that
//! starts a new group.
//!
//! As for `GroupBy`, the capacity of the overloads that take one is not used, and every key
//! (`None` of an `Option` key among them) has its group in the map.

use super::group_by::{ElementSelector, GroupMap, GroupMapFactory, KeySelector};
use super::grouped_observable::{Group, GroupedObservable};
use crate::concurrency::AsyncLock;
use crate::disposables::{CompositeDisposable, RefCountDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) type DurationSelector<K, V, D> = Rc<dyn Fn(Group<K, V>) -> Result<Rc<dyn IObservable<D>>, ObservableError>>;

/// The writer of a group (`NewSubject`: `Subject.Create(new AsyncLockObserver<TElement>(sub, new
/// AsyncLock()), sub)`): a subject whose notifications go through a lock, so that the end of the
/// group that its duration notifies while a value is being written comes after that value; it
/// ignores everything after its end (`ObserverBase<T>`).
pub(crate) struct GroupWriter<V> {
    subject: Rc<Subject<V>>,
    gate: AsyncLock,
    is_stopped: Cell<bool>,
}

impl<V: Clone + 'static> GroupWriter<V> {
    fn new() -> Rc<Self> {
        Rc::new(Self { subject: Subject::new(), gate: AsyncLock::new(), is_stopped: Cell::new(false) })
    }
}

impl<V: Clone + 'static> IObserver<V> for GroupWriter<V> {
    fn on_next(&self, value: V) {
        if self.is_stopped.get() {
            return;
        }
        let subject = self.subject.clone();
        self.gate.wait(move || subject.on_next(value));
    }

    fn on_error(&self, error: ObservableError) {
        if self.is_stopped.replace(true) {
            return;
        }
        let subject = self.subject.clone();
        self.gate.wait(move || subject.on_error(error));
    }

    fn on_completed(&self) {
        if self.is_stopped.replace(true) {
            return;
        }
        let subject = self.subject.clone();
        self.gate.wait(move || subject.on_completed());
    }
}

impl<V: Clone + 'static> IObservable<V> for GroupWriter<V> {
    fn subscribe(&self, observer: Rc<dyn IObserver<V>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

/// The sink of `GroupByUntil` (`GroupByUntil<TSource, TKey, TElement, TDuration>._`): as the sink
/// of `GroupBy`, and each new group subscribes to its duration, which ends the group. The
/// subscriptions to the source and to the durations are a group of their own, which lasts while
/// the sequence of groups or a subscription to a group does (a `RefCountDisposable`).
struct GroupByUntilSink<T, K, V, D> {
    this: Weak<GroupByUntilSink<T, K, V, D>>,
    sink: Sink<Group<K, V>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, V>,
    duration_selector: DurationSelector<K, V, D>,
    map: RefCell<Box<dyn GroupMap<K, GroupWriter<V>>>>,
    group_disposable: Rc<CompositeDisposable>,
    ref_count: Rc<RefCountDisposable>,
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static, D: 'static> GroupByUntilSink<T, K, V, D> {
    /// Subscribes to the source (`Run`).
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        self.group_disposable.add(source.subscribe(self.clone()));
        self.sink.set_upstream(self.ref_count.clone());
    }

    /// Ends every group and the sequence with `error` (`Error`).
    fn error(&self, error: ObservableError) {
        let writers = self.map.borrow().values();
        for writer in writers {
            writer.on_error(error.clone());
        }
        self.sink.forward_on_error(error);
    }
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static, D: 'static> IObserver<T> for GroupByUntilSink<T, K, V, D> {
    fn on_next(&self, value: T) {
        let key = match (self.key_selector)(&value) {
            Ok(key) => key,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        let found = self.map.borrow().get(&key);
        let (writer, fire_new_map_entry) = match found {
            Ok(Some(writer)) => (writer, false),
            Ok(None) => {
                let writer = GroupWriter::new();
                self.map.borrow_mut().add(key.clone(), writer.clone());
                (writer, true)
            }
            Err(error) => {
                self.error(error);
                return;
            }
        };

        if fire_new_map_entry {
            let group = GroupedObservable::new(key.clone(), writer.clone(), Some(self.ref_count.clone()));
            let duration_group = GroupedObservable::new(key.clone(), writer.clone(), None);

            let duration = match (self.duration_selector)(duration_group) {
                Ok(duration) => duration,
                Err(error) => {
                    self.error(error);
                    return;
                }
            };

            self.sink.forward_on_next(group);

            if let Some(parent) = self.this.upgrade() {
                let duration_observer = DurationObserver::new(parent, key, writer.clone());
                self.group_disposable.add(duration_observer.clone());
                duration_observer.resource.set_disposable(Some(duration.subscribe(duration_observer.clone())));
            }
        }

        let element = match (self.element_selector)(value) {
            Ok(element) => element,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        // As upstream (the ISSUE note of `GroupByUntil.cs`), a group whose duration ends at once
        // (`Return`, `Empty`) has ended before its first value is written, and drops it; a group of
        // the same key started later does not see the value either.
        writer.on_next(element);
    }

    fn on_error(&self, error: ObservableError) {
        self.error(error);
    }

    /// Ends every group, going once through them, then the sequence (`OnCompleted`).
    fn on_completed(&self) {
        let writers = self.map.borrow().values();
        for writer in writers {
            writer.on_completed();
        }
        self.sink.forward_on_completed();
    }
}

impl<T: 'static, K: 'static, V: 'static, D: 'static> IDisposable for GroupByUntilSink<T, K, V, D> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observer of the duration of a group (`GroupByUntil<...>._.DurationObserver`, a
/// `SafeObserver<TDuration>`): its first value or its end removes the group from the map and ends
/// it; its error ends every group and the sequence.
struct DurationObserver<T, K, V, D> {
    this: Weak<DurationObserver<T, K, V, D>>,
    parent: Rc<GroupByUntilSink<T, K, V, D>>,
    key: K,
    writer: Rc<GroupWriter<V>>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static, D: 'static> DurationObserver<T, K, V, D> {
    fn new(parent: Rc<GroupByUntilSink<T, K, V, D>>, key: K, writer: Rc<GroupWriter<V>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), parent, key, writer, resource: SingleAssignmentDisposableValue::new() })
    }
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static, D: 'static> IObserver<D> for DurationObserver<T, K, V, D> {
    fn on_next(&self, _value: D) {
        self.on_completed();
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.error(error);
        self.dispose();
    }

    fn on_completed(&self) {
        let removed = self.parent.map.borrow_mut().remove(&self.key, &self.writer);
        if removed {
            self.writer.on_completed();
        }
        if let Some(this) = self.this.upgrade() {
            let this: Rc<dyn IDisposable> = this;
            self.parent.group_disposable.remove(&this);
        }
    }
}

impl<T: 'static, K: 'static, V: 'static, D: 'static> IDisposable for DurationObserver<T, K, V, D> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// The observable of `GroupByUntil` (`GroupByUntil<TSource, TKey, TElement, TDuration>`).
struct GroupByUntilOperator<T, K, V, D> {
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, V>,
    duration_selector: DurationSelector<K, V, D>,
    map: GroupMapFactory<K, GroupWriter<V>>,
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static, D: 'static> IObservable<Group<K, V>> for GroupByUntilOperator<T, K, V, D> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Group<K, V>>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let key_selector = self.key_selector.clone();
        let element_selector = self.element_selector.clone();
        let duration_selector = self.duration_selector.clone();
        let map = self.map.clone();
        subscribe_sink(
            observer,
            move |observer| {
                let group_disposable = CompositeDisposable::new();
                let ref_count = RefCountDisposable::new(group_disposable.clone());
                Rc::new_cyclic(|this| GroupByUntilSink {
                    this: this.clone(),
                    sink: Sink::new(observer),
                    key_selector,
                    element_selector,
                    duration_selector,
                    map: RefCell::new(map()),
                    group_disposable,
                    ref_count,
                })
            },
            move |sink: &Rc<GroupByUntilSink<T, K, V, D>>| sink.run(&source),
        )
    }
}

pub(crate) fn group_by_until<T: 'static, K: Clone + 'static, V: Clone + 'static, D: 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, V>,
    duration_selector: DurationSelector<K, V, D>,
    map: GroupMapFactory<K, GroupWriter<V>>,
) -> Rc<dyn IObservable<Group<K, V>>> {
    Rc::new(GroupByUntilOperator { source, key_selector, element_selector, duration_selector, map })
}

/// The methods of `GroupByUntil` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! group_by_until_methods {
    () => {
        /// Groups the values by key, each group lasting until its duration produces a value or
        /// ends (`GroupByUntil(Func<TSource, TKey>, Func<IGroupedObservable<TKey, TSource>,
        /// IObservable<TDuration>>)`).
        fn group_by_until<K: Eq + std::hash::Hash + Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|value: T| Ok(value)),
                std::rc::Rc::new(move |group: std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>| Ok(duration_selector(group))),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// [`group_by_until`](Self::group_by_until) with the keys compared with `comparer`
        /// (`GroupByUntil(Func<TSource, TKey>, Func<IGroupedObservable<TKey, TSource>,
        /// IObservable<TDuration>>, IEqualityComparer<TKey>)`).
        fn group_by_until_comparer<K: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|value: T| Ok(value)),
                std::rc::Rc::new(move |group: std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>| Ok(duration_selector(group))),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(move |a: &K, b: &K| Ok(comparer(a, b)))),
            )
        }

        /// [`group_by_until`](Self::group_by_until) of the values projected with
        /// `element_selector` (`GroupByUntil(Func<TSource, TKey>, Func<TSource, TElement>,
        /// Func<IGroupedObservable<TKey, TElement>, IObservable<TDuration>>)`).
        fn group_by_until_with<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |value: T| Ok(element_selector(value))),
                std::rc::Rc::new(move |group: std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>| Ok(duration_selector(group))),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// [`group_by_until_with`](Self::group_by_until_with) with the keys compared with
        /// `comparer` (`GroupByUntil(Func<TSource, TKey>, Func<TSource, TElement>,
        /// Func<IGroupedObservable<TKey, TElement>, IObservable<TDuration>>,
        /// IEqualityComparer<TKey>)`).
        fn group_by_until_with_comparer<K: Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |value: T| Ok(element_selector(value))),
                std::rc::Rc::new(move |group: std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>| Ok(duration_selector(group))),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(move |a: &K, b: &K| Ok(comparer(a, b)))),
            )
        }

        /// [`group_by_until`](Self::group_by_until) with an initial capacity, which the port does
        /// not use (`GroupByUntil(Func<TSource, TKey>, Func<IGroupedObservable<TKey, TSource>,
        /// IObservable<TDuration>>, int)`).
        fn group_by_until_capacity<K: Eq + std::hash::Hash + Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.group_by_until(key_selector, duration_selector)
        }

        /// [`group_by_until_comparer`](Self::group_by_until_comparer) with an initial capacity,
        /// which the port does not use (`GroupByUntil(Func<TSource, TKey>,
        /// Func<IGroupedObservable<TKey, TSource>, IObservable<TDuration>>, int,
        /// IEqualityComparer<TKey>)`).
        fn group_by_until_capacity_comparer<K: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.group_by_until_comparer(key_selector, duration_selector, comparer)
        }

        /// [`group_by_until_with`](Self::group_by_until_with) with an initial capacity, which the
        /// port does not use (`GroupByUntil(Func<TSource, TKey>, Func<TSource, TElement>,
        /// Func<IGroupedObservable<TKey, TElement>, IObservable<TDuration>>, int)`).
        fn group_by_until_with_capacity<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.group_by_until_with(key_selector, element_selector, duration_selector)
        }

        /// [`group_by_until_with_comparer`](Self::group_by_until_with_comparer) with an initial
        /// capacity, which the port does not use (`GroupByUntil(Func<TSource, TKey>,
        /// Func<TSource, TElement>, Func<IGroupedObservable<TKey, TElement>,
        /// IObservable<TDuration>>, int, IEqualityComparer<TKey>)`).
        fn group_by_until_with_capacity_comparer<K: Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            duration_selector: impl Fn(std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>) -> std::rc::Rc<dyn ferroui_system::IObservable<D>> + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.group_by_until_with_comparer(key_selector, element_selector, duration_selector, comparer)
        }

        /// [`group_by_until`](Self::group_by_until) with a key selector and a duration selector
        /// that may fail; a failure ends the groups and the sequence with its error
        /// (`GroupByUntil` with a selector that throws).
        fn try_group_by_until<K: Eq + std::hash::Hash + Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(|value: T| Ok(value)),
                std::rc::Rc::new(duration_selector),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// [`group_by_until_comparer`](Self::group_by_until_comparer) with a key selector, a
        /// duration selector and a comparer that may fail; a failure ends the groups and the
        /// sequence with its error.
        fn try_group_by_until_comparer<K: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(|value: T| Ok(value)),
                std::rc::Rc::new(duration_selector),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(comparer)),
            )
        }

        /// [`group_by_until_with`](Self::group_by_until_with) with a key selector, an element
        /// selector and a duration selector that may fail; a failure ends the groups and the
        /// sequence with its error.
        fn try_group_by_until_with<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(element_selector),
                std::rc::Rc::new(duration_selector),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// [`group_by_until_with_comparer`](Self::group_by_until_with_comparer) with a key
        /// selector, an element selector, a duration selector and a comparer that may fail; a
        /// failure ends the groups and the sequence with its error.
        fn try_group_by_until_with_comparer<K: Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by_until::group_by_until(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(element_selector),
                std::rc::Rc::new(duration_selector),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(comparer)),
            )
        }

        /// [`try_group_by_until`](Self::try_group_by_until) with an initial capacity, which the
        /// port does not use.
        fn try_group_by_until_capacity<K: Eq + std::hash::Hash + Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.try_group_by_until(key_selector, duration_selector)
        }

        /// [`try_group_by_until_comparer`](Self::try_group_by_until_comparer) with an initial
        /// capacity, which the port does not use.
        fn try_group_by_until_capacity_comparer<K: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.try_group_by_until_comparer(key_selector, duration_selector, comparer)
        }

        /// [`try_group_by_until_with`](Self::try_group_by_until_with) with an initial capacity,
        /// which the port does not use.
        fn try_group_by_until_with_capacity<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.try_group_by_until_with(key_selector, element_selector, duration_selector)
        }

        /// [`try_group_by_until_with_comparer`](Self::try_group_by_until_with_comparer) with an
        /// initial capacity, which the port does not use.
        fn try_group_by_until_with_capacity_comparer<K: Clone + 'static, V: Clone + 'static, D: 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            duration_selector: impl Fn(
                    std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>,
                ) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<D>>, ferroui_system::ObservableError>
                + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.try_group_by_until_with_comparer(key_selector, element_selector, duration_selector, comparer)
        }
    };
}
pub(crate) use group_by_until_methods;
