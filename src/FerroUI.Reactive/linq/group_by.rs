//! `GroupBy` (`Linq/Observable/GroupBy.cs`), with the map of the groups by key it shares with
//! `GroupByUntil` (`Internal/Grouping.cs`, `Internal/Map.cs`).
//!
//! The capacity of the overloads that take one is the initial size of upstream's dictionary of
//! groups; the port takes it and does not use it (the map grows as keys arrive).
//!
//! Deviation (DEVIATIONS.md, Reactive): upstream keeps the group of the `null` key apart, as its
//! dictionaries take no `null` key, and never hands `null` to a comparer; in the port a key is a
//! value of any type (`Option<K>` for a key that may be absent), and every key, `None` among them,
//! has its group in the map (and is compared by a comparer like any other).

use super::grouped_observable::{Group, GroupedObservable};
use crate::disposables::{RefCountDisposable, SingleAssignmentDisposable};
use crate::internal::{subscribe_sink, Sink};
use crate::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

pub(crate) type KeySelector<T, K> = Rc<dyn Fn(&T) -> Result<K, ObservableError>>;
pub(crate) type ElementSelector<T, V> = Rc<dyn Fn(T) -> Result<V, ObservableError>>;
pub(crate) type Comparer<K> = Rc<dyn Fn(&K, &K) -> Result<bool, ObservableError>>;
pub(crate) type GroupMapFactory<K, W> = Rc<dyn Fn() -> Box<dyn GroupMap<K, W>>>;

/// The writers of the groups by key (`Grouping<TKey, TElement>` of `GroupBy`, `Map<TKey, TValue>`
/// of `GroupByUntil`): the map keeps the order the keys were added in, which is the order upstream's
/// dictionaries enumerate the groups in (when nothing was removed from them).
pub(crate) trait GroupMap<K, W: ?Sized> {
    /// The writer of the group of `key`, if there is one (`TryGetValue`); a comparer that fails
    /// ends the sequence with its error.
    fn get(&self, key: &K) -> Result<Option<Rc<W>>, ObservableError>;

    /// Adds the writer of a new group (`Add`).
    fn add(&mut self, key: K, writer: Rc<W>);

    /// Removes the group of `key` when its writer is `writer`; false when the map does not hold
    /// it (`Remove`).
    fn remove(&mut self, key: &K, writer: &Rc<W>) -> bool;

    /// The writers, in the order their keys were added (`Values`): a copy, so that a writer may
    /// change the map while the operator goes through them.
    fn values(&self) -> Vec<Rc<W>>;
}

/// The groups by key with the default comparer (`EqualityComparer<TKey>.Default`): a hash map from
/// each key to the slot of its writer, the slots in the order the keys were added. A removed
/// group leaves an empty slot; the slots are compacted when more than half of them are empty, so
/// that every operation stays linear in the number of groups overall.
struct HashGroupMap<K, W: ?Sized> {
    index: HashMap<K, usize>,
    slots: Vec<Option<Rc<W>>>,
}

impl<K: Eq + Hash, W: ?Sized> HashGroupMap<K, W> {
    fn compact(&mut self) {
        let mut positions = vec![usize::MAX; self.slots.len()];
        let mut slots = Vec::with_capacity(self.index.len());
        for (position, slot) in std::mem::take(&mut self.slots).into_iter().enumerate() {
            if let Some(writer) = slot {
                positions[position] = slots.len();
                slots.push(Some(writer));
            }
        }
        for position in self.index.values_mut() {
            *position = positions[*position];
        }
        self.slots = slots;
    }
}

impl<K: Eq + Hash, W: ?Sized> GroupMap<K, W> for HashGroupMap<K, W> {
    fn get(&self, key: &K) -> Result<Option<Rc<W>>, ObservableError> {
        Ok(self.index.get(key).and_then(|&position| self.slots[position].clone()))
    }

    fn add(&mut self, key: K, writer: Rc<W>) {
        self.index.insert(key, self.slots.len());
        self.slots.push(Some(writer));
    }

    fn remove(&mut self, key: &K, writer: &Rc<W>) -> bool {
        let Some(&position) = self.index.get(key) else {
            return false;
        };
        match &self.slots[position] {
            Some(existing) if Rc::ptr_eq(existing, writer) => {}
            _ => return false,
        }
        self.slots[position] = None;
        self.index.remove(key);
        if self.slots.len() > 16 && self.index.len() * 2 < self.slots.len() {
            self.compact();
        }
        true
    }

    fn values(&self) -> Vec<Rc<W>> {
        self.slots.iter().flatten().cloned().collect()
    }
}

/// The groups by key compared with a comparer.
///
/// Deviation (DEVIATIONS.md, Reactive): an `IEqualityComparer<TKey>` is a function of equality in
/// the port, with no hash code; the keys are kept in a list that each new key is compared against,
/// in the order they were added (upstream compares a key only with the keys of the same hash
/// code). A group whose duration ends is removed by the identity of its writer, without the
/// comparer.
struct ComparerGroupMap<K, W: ?Sized> {
    entries: Vec<(K, Rc<W>)>,
    comparer: Comparer<K>,
}

impl<K, W: ?Sized> GroupMap<K, W> for ComparerGroupMap<K, W> {
    fn get(&self, key: &K) -> Result<Option<Rc<W>>, ObservableError> {
        for (existing, writer) in &self.entries {
            if (self.comparer)(existing, key)? {
                return Ok(Some(writer.clone()));
            }
        }
        Ok(None)
    }

    fn add(&mut self, key: K, writer: Rc<W>) {
        self.entries.push((key, writer));
    }

    fn remove(&mut self, _key: &K, writer: &Rc<W>) -> bool {
        match self.entries.iter().position(|(_, existing)| Rc::ptr_eq(existing, writer)) {
            Some(position) => {
                self.entries.remove(position);
                true
            }
            None => false,
        }
    }

    fn values(&self) -> Vec<Rc<W>> {
        self.entries.iter().map(|(_, writer)| writer.clone()).collect()
    }
}

/// A map of the groups for each subscription with the default comparer.
pub(crate) fn hash_group_map<K: Eq + Hash + 'static, W: ?Sized + 'static>() -> GroupMapFactory<K, W> {
    Rc::new(|| Box::new(HashGroupMap { index: HashMap::new(), slots: Vec::new() }) as Box<dyn GroupMap<K, W>>)
}

/// A map of the groups for each subscription compared with `comparer`.
pub(crate) fn comparer_group_map<K: 'static, W: ?Sized + 'static>(comparer: Comparer<K>) -> GroupMapFactory<K, W> {
    Rc::new(move || Box::new(ComparerGroupMap { entries: Vec::new(), comparer: comparer.clone() }) as Box<dyn GroupMap<K, W>>)
}

/// The sink of `GroupBy` (`GroupBy<TSource, TKey, TElement>._`): a new key starts a group, handed
/// downstream before its first value; the end of the source ends every group, then the sequence of
/// groups. A key selector, element selector or comparer that fails ends the groups and the sequence
/// with its error.
struct GroupBySink<T, K, V> {
    sink: Sink<Group<K, V>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, V>,
    map: RefCell<Box<dyn GroupMap<K, Subject<V>>>>,
    source_subscription: Rc<SingleAssignmentDisposable>,
    ref_count: Rc<RefCountDisposable>,
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static> GroupBySink<T, K, V> {
    /// Subscribes to the source; the subscription lasts while the sequence of groups or a
    /// subscription to a group does (`Run`).
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        self.source_subscription.set_disposable(Some(source.subscribe(self.clone())));
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

impl<T: 'static, K: Clone + 'static, V: Clone + 'static> IObserver<T> for GroupBySink<T, K, V> {
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
                let writer = Subject::new();
                self.map.borrow_mut().add(key.clone(), writer.clone());
                (writer, true)
            }
            Err(error) => {
                self.error(error);
                return;
            }
        };

        if fire_new_map_entry {
            let group = GroupedObservable::new(key, writer.as_observable(), Some(self.ref_count.clone()));
            self.sink.forward_on_next(group);
        }

        let element = match (self.element_selector)(value) {
            Ok(element) => element,
            Err(error) => {
                self.error(error);
                return;
            }
        };

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

impl<T: 'static, K: 'static, V: 'static> IDisposable for GroupBySink<T, K, V> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observable of `GroupBy` (`GroupBy<TSource, TKey, TElement>`).
struct GroupByOperator<T, K, V> {
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, V>,
    map: GroupMapFactory<K, Subject<V>>,
}

impl<T: 'static, K: Clone + 'static, V: Clone + 'static> IObservable<Group<K, V>> for GroupByOperator<T, K, V> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Group<K, V>>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let key_selector = self.key_selector.clone();
        let element_selector = self.element_selector.clone();
        let map = self.map.clone();
        subscribe_sink(
            observer,
            move |observer| {
                let source_subscription = SingleAssignmentDisposable::new();
                let ref_count = RefCountDisposable::new(source_subscription.clone());
                Rc::new(GroupBySink {
                    sink: Sink::new(observer),
                    key_selector,
                    element_selector,
                    map: RefCell::new(map()),
                    source_subscription,
                    ref_count,
                })
            },
            move |sink: &Rc<GroupBySink<T, K, V>>| sink.run(&source),
        )
    }
}

pub(crate) fn group_by<T: 'static, K: Clone + 'static, V: Clone + 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: KeySelector<T, K>,
    element_selector: ElementSelector<T, V>,
    map: GroupMapFactory<K, Subject<V>>,
) -> Rc<dyn IObservable<Group<K, V>>> {
    Rc::new(GroupByOperator { source, key_selector, element_selector, map })
}

/// The methods of `GroupBy` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! group_by_methods {
    () => {
        /// Groups the values by key (`GroupBy(Func<TSource, TKey>)`).
        fn group_by<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|value: T| Ok(value)),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// Groups the values by key, compared with `comparer`
        /// (`GroupBy(Func<TSource, TKey>, IEqualityComparer<TKey>)`).
        fn group_by_comparer<K: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(|value: T| Ok(value)),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(move |a: &K, b: &K| Ok(comparer(a, b)))),
            )
        }

        /// Groups the values, projected with `element_selector`, by key
        /// (`GroupBy(Func<TSource, TKey>, Func<TSource, TElement>)`).
        fn group_by_with<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |value: T| Ok(element_selector(value))),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// Groups the values, projected with `element_selector`, by key, compared with `comparer`
        /// (`GroupBy(Func<TSource, TKey>, Func<TSource, TElement>, IEqualityComparer<TKey>)`).
        fn group_by_with_comparer<K: Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(move |value: &T| Ok(key_selector(value))),
                std::rc::Rc::new(move |value: T| Ok(element_selector(value))),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(move |a: &K, b: &K| Ok(comparer(a, b)))),
            )
        }

        /// [`group_by`](Self::group_by) with an initial capacity, which the port does not use
        /// (`GroupBy(Func<TSource, TKey>, int)`).
        fn group_by_capacity<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.group_by(key_selector)
        }

        /// [`group_by_comparer`](Self::group_by_comparer) with an initial capacity, which the port
        /// does not use (`GroupBy(Func<TSource, TKey>, int, IEqualityComparer<TKey>)`).
        fn group_by_capacity_comparer<K: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.group_by_comparer(key_selector, comparer)
        }

        /// [`group_by_with`](Self::group_by_with) with an initial capacity, which the port does not
        /// use (`GroupBy(Func<TSource, TKey>, Func<TSource, TElement>, int)`).
        fn group_by_with_capacity<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.group_by_with(key_selector, element_selector)
        }

        /// [`group_by_with_comparer`](Self::group_by_with_comparer) with an initial capacity, which
        /// the port does not use
        /// (`GroupBy(Func<TSource, TKey>, Func<TSource, TElement>, int, IEqualityComparer<TKey>)`).
        fn group_by_with_capacity_comparer<K: Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> K + 'static,
            element_selector: impl Fn(T) -> V + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> bool + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.group_by_with_comparer(key_selector, element_selector, comparer)
        }

        /// [`group_by`](Self::group_by) with a key selector that may fail; a failure ends the
        /// groups and the sequence with its error (`GroupBy` with a selector that throws).
        fn try_group_by<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(|value: T| Ok(value)),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// [`group_by_comparer`](Self::group_by_comparer) with a key selector and a comparer that
        /// may fail; a failure ends the groups and the sequence with its error.
        fn try_group_by_comparer<K: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(|value: T| Ok(value)),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(comparer)),
            )
        }

        /// [`group_by_with`](Self::group_by_with) with a key selector and an element selector that
        /// may fail; a failure ends the groups and the sequence with its error.
        fn try_group_by_with<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(element_selector),
                crate::linq::group_by::hash_group_map(),
            )
        }

        /// [`group_by_with_comparer`](Self::group_by_with_comparer) with a key selector, an element
        /// selector and a comparer that may fail; a failure ends the groups and the sequence with
        /// its error.
        fn try_group_by_with_comparer<K: Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            crate::linq::group_by::group_by(
                self.observable(),
                std::rc::Rc::new(key_selector),
                std::rc::Rc::new(element_selector),
                crate::linq::group_by::comparer_group_map(std::rc::Rc::new(comparer)),
            )
        }

        /// [`try_group_by`](Self::try_group_by) with an initial capacity, which the port does not
        /// use (`GroupBy(Func<TSource, TKey>, int)` with a selector that throws).
        fn try_group_by_capacity<K: Eq + std::hash::Hash + Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.try_group_by(key_selector)
        }

        /// [`try_group_by_comparer`](Self::try_group_by_comparer) with an initial capacity, which
        /// the port does not use.
        fn try_group_by_capacity_comparer<K: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, T>>>>
        where
            T: Clone,
        {
            self.try_group_by_comparer(key_selector, comparer)
        }

        /// [`try_group_by_with`](Self::try_group_by_with) with an initial capacity, which the port
        /// does not use.
        fn try_group_by_with_capacity<K: Eq + std::hash::Hash + Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            _capacity: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.try_group_by_with(key_selector, element_selector)
        }

        /// [`try_group_by_with_comparer`](Self::try_group_by_with_comparer) with an initial
        /// capacity, which the port does not use.
        fn try_group_by_with_capacity_comparer<K: Clone + 'static, V: Clone + 'static>(
            &self,
            key_selector: impl Fn(&T) -> Result<K, ferroui_system::ObservableError> + 'static,
            element_selector: impl Fn(T) -> Result<V, ferroui_system::ObservableError> + 'static,
            _capacity: usize,
            comparer: impl Fn(&K, &K) -> Result<bool, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn crate::linq::grouped_observable::IGroupedObservable<K, V>>>> {
            self.try_group_by_with_comparer(key_selector, element_selector, comparer)
        }
    };
}
pub(crate) use group_by_methods;
