//! `AggregationEx` (`Aggregation/AggregationEx.cs`): change sets shaped for aggregations
//! (`ForAggregation`), the invalidation of an aggregating sequence (`InvalidateWhen`) and the
//! accumulator of the aggregations (`Accumulate`), with the traits that carry every operator of
//! the namespace `DynamicData.Aggregation`.
//!
//! Upstream's static classes `AggregationEx`, `AvgEx`, `CountEx`, `MaxEx`, `StdDevEx` and `SumEx`
//! extend three kinds of sequences. The port has one trait per kind, each implemented for that
//! kind only:
//! - [`AggregationCacheEx`] for the sequences of change sets of caches (any [`cache::IChangeSet`]);
//! - [`AggregationListEx`] for those of lists (any [`list::IChangeSet`]);
//! - [`AggregateChangeSetEx`] for the sequences of aggregate change sets
//!   ([`AggregateChangeSets`]);
//! - and [`InvalidateWhenEx`] for every sequence.
//!
//! Naming (DEVIATIONS.md, DynamicData): `Count`, `IsEmpty` and `Sum` are names of operators of
//! `ferroui_reactive::ObservableExt`, which applies to the same sequences, so they are
//! `count_cache`, `is_empty_cache`, `sum_cache` (caches), `count_list`, `is_empty_list`,
//! `sum_list` (lists) and `count_aggregate`, `sum_aggregate` (aggregate change sets). `Avg`,
//! `Maximum`, `Minimum`, `StdDev` and `IsNotEmpty` keep their names. The default arguments of
//! upstream (`emptyValue`) are the `_with` forms.
//!
//! [`cache::IChangeSet`]: crate::cache::IChangeSet
//! [`list::IChangeSet`]: crate::list::IChangeSet

use super::{
    cache_aggregate, list_aggregate, AggregateChangeSets, AggregateNumber, AggregateType, AggregateValue, IAggregateChangeSet, MaxOrMin, StdDevValue,
};
use crate::cache::ObservableCacheEx;
use crate::list::ObservableListEx;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// `Accumulate<TObject, TResult>(IObservable<IAggregateChangeSet<TObject>>, ..)`: from `seed`,
/// `add_action` of the accessed value of each added item and `remove_action` of each removed one;
/// one value per change set.
pub(crate) fn accumulate<T: 'static, R: Clone + 'static>(
    source: AggregateChangeSets<T>,
    seed: R,
    accessor: impl Fn(&T) -> R + 'static,
    add_action: impl Fn(R, R) -> R + 'static,
    remove_action: impl Fn(R, R) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    source.scan_with(seed, move |state: R, changes: Rc<dyn IAggregateChangeSet<T>>| {
        changes.items().fold(state, |current, aggregate_item| match aggregate_item.aggregate_type() {
            AggregateType::Add => add_action(current, accessor(aggregate_item.item())),
            AggregateType::Remove => remove_action(current, accessor(aggregate_item.item())),
        })
    })
}

/// The aggregations of caches (the overloads of `AggregationEx`, `AvgEx`, `CountEx`, `MaxEx`,
/// `StdDevEx` and `SumEx` on `IObservable<IChangeSet<TObject, TKey>>`).
pub trait AggregationCacheEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
    /// The change sets as aggregate change sets: an add, a remove, or an update as the remove of
    /// the previous item and the add of the current one; refreshes and moves are left out
    /// (`ForAggregation<TObject, TKey>()`).
    fn for_aggregation(&self) -> AggregateChangeSets<T>;

    /// The average of the values of the items after each change set, zero when there are none
    /// (`Avg(Func<TObject, TValue>)`).
    fn avg<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>>;

    /// [`avg`](Self::avg) with the value when there are no items
    /// (`Avg(Func<TObject, TValue>, TValue emptyValue)`).
    fn avg_with<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static, empty_value: V::Number) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>>;

    /// The count of the items after each change set (`Count<TObject, TKey>()`, and
    /// `Count<TObject>(IObservable<IDistinctChangeSet<TObject>>)`).
    fn count_cache(&self) -> Rc<dyn IObservable<usize>>;

    /// True at once, then whether there are no items after each change set (`IsEmpty`).
    fn is_empty_cache(&self) -> Rc<dyn IObservable<bool>>;

    /// False at once, then whether there are items after each change set (`IsNotEmpty`).
    fn is_not_empty(&self) -> Rc<dyn IObservable<bool>>;

    /// The greatest value of the items, `R::default()` when there are none, each time it changes
    /// (`Maximum(Func<TObject, TResult>)`).
    fn maximum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>>;

    /// [`maximum`](Self::maximum) with the value when there are no items
    /// (`Maximum(Func<TObject, TResult>, TResult emptyValue)`).
    fn maximum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>>;

    /// The least value of the items, `R::default()` when there are none, each time it changes
    /// (`Minimum(Func<TObject, TResult>)`).
    fn minimum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>>;

    /// [`minimum`](Self::minimum) with the value when there are no items
    /// (`Minimum(Func<TObject, TResult>, TResult emptyValue)`).
    fn minimum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>>;

    /// The sample standard deviation of the values of the items after each change set,
    /// `fallback_value` for fewer than two items (`StdDev(Func<TObject, TValue>, TValue
    /// fallbackValue)`).
    fn std_dev<V: StdDevValue>(&self, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>>;

    /// The sum of the values of the items after each change set (`Sum(Func<TObject, TValue>)`).
    fn sum_cache<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>>;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: crate::cache::IChangeSet<T, K>> AggregationCacheEx<T, K> for Rc<dyn IObservable<C>> {
    fn for_aggregation(&self) -> AggregateChangeSets<T> {
        ObservableCacheEx::<T, K>::change_sets(self).select(cache_aggregate::<T, K>)
    }

    fn avg<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
        AggregationCacheEx::<T, K>::avg_with(self, value_selector, <V::Number as AggregateNumber>::ZERO)
    }

    fn avg_with<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static, empty_value: V::Number) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
        super::avg_ex::avg(AggregationCacheEx::<T, K>::for_aggregation(self), value_selector, empty_value)
    }

    fn count_cache(&self) -> Rc<dyn IObservable<usize>> {
        super::count_ex::count(AggregationCacheEx::<T, K>::for_aggregation(self))
    }

    fn is_empty_cache(&self) -> Rc<dyn IObservable<bool>> {
        super::count_ex::is_empty(AggregationCacheEx::<T, K>::for_aggregation(self))
    }

    fn is_not_empty(&self) -> Rc<dyn IObservable<bool>> {
        super::count_ex::is_not_empty(AggregationCacheEx::<T, K>::for_aggregation(self))
    }

    fn maximum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>> {
        AggregationCacheEx::<T, K>::maximum_with(self, value_selector, R::default())
    }

    fn maximum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>> {
        let source = super::max_ex::cache_changes_and_collection(ObservableCacheEx::<T, K>::change_sets(self));
        super::max_ex::calculate(source, Rc::new(value_selector), MaxOrMin::Max, empty_value)
    }

    fn minimum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>> {
        AggregationCacheEx::<T, K>::minimum_with(self, value_selector, R::default())
    }

    fn minimum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>> {
        let source = super::max_ex::cache_changes_and_collection(ObservableCacheEx::<T, K>::change_sets(self));
        super::max_ex::calculate(source, Rc::new(value_selector), MaxOrMin::Min, empty_value)
    }

    fn std_dev<V: StdDevValue>(&self, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>> {
        super::std_dev_ex::std_dev(AggregationCacheEx::<T, K>::for_aggregation(self), value_selector, fallback_value)
    }

    fn sum_cache<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>> {
        super::sum_ex::sum(AggregationCacheEx::<T, K>::for_aggregation(self), value_selector)
    }
}

/// The aggregations of lists (the overloads of `AggregationEx`, `AvgEx`, `CountEx`, `MaxEx`,
/// `StdDevEx` and `SumEx` on `IObservable<IChangeSet<T>>`).
pub trait AggregationListEx<T: Clone + PartialEq + 'static> {
    /// The change sets as aggregate change sets: adds and ranges of adds, removes and ranges of
    /// removes and clears, or a replace as the remove of the previous item and the add of the
    /// current one; refreshes and moves are left out (`ForAggregation<TObject>()`).
    fn for_aggregation(&self) -> AggregateChangeSets<T>;

    /// The average of the values of the items after each change set, zero when there are none
    /// (`Avg(Func<T, TValue>)`).
    fn avg<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>>;

    /// [`avg`](Self::avg) with the value when there are no items
    /// (`Avg(Func<T, TValue>, TValue emptyValue)`).
    fn avg_with<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static, empty_value: V::Number) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>>;

    /// The count of the items after each change set (`Count<TObject>()`).
    fn count_list(&self) -> Rc<dyn IObservable<usize>>;

    /// True at once, then whether there are no items after each change set (`IsEmpty`).
    fn is_empty_list(&self) -> Rc<dyn IObservable<bool>>;

    /// False at once, then whether there are items after each change set (`IsNotEmpty`).
    fn is_not_empty(&self) -> Rc<dyn IObservable<bool>>;

    /// The greatest value of the items, `R::default()` when there are none, each time it changes
    /// (`Maximum(Func<TObject, TResult>)`).
    fn maximum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>>;

    /// [`maximum`](Self::maximum) with the value when there are no items
    /// (`Maximum(Func<TObject, TResult>, TResult emptyValue)`).
    fn maximum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>>;

    /// The least value of the items, `R::default()` when there are none, each time it changes
    /// (`Minimum(Func<TObject, TResult>)`).
    fn minimum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>>;

    /// [`minimum`](Self::minimum) with the value when there are no items
    /// (`Minimum(Func<TObject, TResult>, TResult emptyValue)`).
    fn minimum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>>;

    /// The sample standard deviation of the values of the items after each change set,
    /// `fallback_value` for fewer than two items (`StdDev(Func<T, TValue>, TValue fallbackValue)`).
    fn std_dev<V: StdDevValue>(&self, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>>;

    /// The sum of the values of the items after each change set (`Sum(Func<T, TValue>)`).
    fn sum_list<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>>;
}

impl<T: Clone + PartialEq + 'static, C: crate::list::IChangeSet<T>> AggregationListEx<T> for Rc<dyn IObservable<C>> {
    fn for_aggregation(&self) -> AggregateChangeSets<T> {
        ObservableListEx::<T>::change_sets(self).select(list_aggregate::<T>)
    }

    fn avg<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
        AggregationListEx::<T>::avg_with(self, value_selector, <V::Number as AggregateNumber>::ZERO)
    }

    fn avg_with<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static, empty_value: V::Number) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
        super::avg_ex::avg(AggregationListEx::<T>::for_aggregation(self), value_selector, empty_value)
    }

    fn count_list(&self) -> Rc<dyn IObservable<usize>> {
        super::count_ex::count(AggregationListEx::<T>::for_aggregation(self))
    }

    fn is_empty_list(&self) -> Rc<dyn IObservable<bool>> {
        super::count_ex::is_empty(AggregationListEx::<T>::for_aggregation(self))
    }

    fn is_not_empty(&self) -> Rc<dyn IObservable<bool>> {
        super::count_ex::is_not_empty(AggregationListEx::<T>::for_aggregation(self))
    }

    fn maximum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>> {
        AggregationListEx::<T>::maximum_with(self, value_selector, R::default())
    }

    fn maximum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>> {
        let source = super::max_ex::list_changes_and_collection(ObservableListEx::<T>::change_sets(self));
        super::max_ex::calculate(source, Rc::new(value_selector), MaxOrMin::Max, empty_value)
    }

    fn minimum<R: Copy + PartialOrd + Default + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static) -> Rc<dyn IObservable<R>> {
        AggregationListEx::<T>::minimum_with(self, value_selector, R::default())
    }

    fn minimum_with<R: Copy + PartialOrd + 'static>(&self, value_selector: impl Fn(&T) -> R + 'static, empty_value: R) -> Rc<dyn IObservable<R>> {
        let source = super::max_ex::list_changes_and_collection(ObservableListEx::<T>::change_sets(self));
        super::max_ex::calculate(source, Rc::new(value_selector), MaxOrMin::Min, empty_value)
    }

    fn std_dev<V: StdDevValue>(&self, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>> {
        super::std_dev_ex::std_dev(AggregationListEx::<T>::for_aggregation(self), value_selector, fallback_value)
    }

    fn sum_list<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>> {
        super::sum_ex::sum(AggregationListEx::<T>::for_aggregation(self), value_selector)
    }
}

/// The aggregations of aggregate change sets (the overloads of `AvgEx`, `CountEx`, `StdDevEx` and
/// `SumEx` on `IObservable<IAggregateChangeSet<T>>`), to which the forms of caches and lists
/// forward.
pub trait AggregateChangeSetEx<T: 'static> {
    /// The average of the values of the items after each change set, zero when there are none
    /// (`Avg(Func<T, TValue>)`).
    fn avg<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>>;

    /// [`avg`](Self::avg) with the value when there are no items
    /// (`Avg(Func<T, TValue>, TValue emptyValue)`).
    fn avg_with<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static, empty_value: V::Number) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>>;

    /// The count of the items after each change set (`Count<TObject>()`).
    fn count_aggregate(&self) -> Rc<dyn IObservable<usize>>;

    /// The sample standard deviation of the values of the items after each change set,
    /// `fallback_value` for fewer than two items (`StdDev(Func<T, TValue>, TValue fallbackValue)`).
    fn std_dev<V: StdDevValue>(&self, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>>;

    /// The sum of the values of the items after each change set (`Sum(Func<T, TValue>)`).
    fn sum_aggregate<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>>;
}

impl<T: 'static> AggregateChangeSetEx<T> for AggregateChangeSets<T> {
    fn avg<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
        AggregateChangeSetEx::<T>::avg_with(self, value_selector, <V::Number as AggregateNumber>::ZERO)
    }

    fn avg_with<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static, empty_value: V::Number) -> Rc<dyn IObservable<<V::Number as AggregateNumber>::Average>> {
        super::avg_ex::avg(self.clone(), value_selector, empty_value)
    }

    fn count_aggregate(&self) -> Rc<dyn IObservable<usize>> {
        super::count_ex::count(self.clone())
    }

    fn std_dev<V: StdDevValue>(&self, value_selector: impl Fn(&T) -> V + 'static, fallback_value: V) -> Rc<dyn IObservable<f64>> {
        super::std_dev_ex::std_dev(self.clone(), value_selector, fallback_value)
    }

    fn sum_aggregate<V: AggregateValue>(&self, value_selector: impl Fn(&T) -> V + 'static) -> Rc<dyn IObservable<V::Number>> {
        super::sum_ex::sum(self.clone(), value_selector)
    }
}

/// The invalidation of an aggregating sequence (`InvalidateWhen`), for every sequence.
pub trait InvalidateWhenEx<T> {
    /// The sequence, subscribed again each time `invalidate` notifies, without repeating a value:
    /// used when an item changes inline, so that an aggregation is computed again
    /// (`InvalidateWhen(IObservable<Unit>)` and `InvalidateWhen<T, TTrigger>(IObservable<TTrigger?>)`,
    /// which are one method: the trigger's values are not read).
    fn invalidate_when<U: 'static>(&self, invalidate: Rc<dyn IObservable<U>>) -> Rc<dyn IObservable<T>>;
}

impl<T: Clone + PartialEq + 'static> InvalidateWhenEx<T> for Rc<dyn IObservable<T>> {
    fn invalidate_when<U: 'static>(&self, invalidate: Rc<dyn IObservable<U>>) -> Rc<dyn IObservable<T>> {
        let source = self.clone();
        invalidate
            .select(|_: U| ())
            .start_with([()])
            .select(move |_: ()| source.clone())
            .switch::<T>()
            .distinct_until_changed()
    }
}
