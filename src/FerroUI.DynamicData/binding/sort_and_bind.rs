//! `SortAndBind<TObject, TKey>` (`Binding/SortAndBind.cs`): a bind where the sort is part of the
//! binding, so that each change is applied at its sorted position in the target, and the
//! positions of `Cache/Internal/SortExtensions.cs`.

use super::{ObservableCollectionExtended, SortAndBindOptions};
use crate::cache::internal::Cache;
use crate::cache::{ChangeReason, ChangeSet, ICache, IQuery};
use crate::list::{binary_search_by, IList, SortException};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::hash::Hash;
use std::rc::Rc;

/// An `IComparer<T>`.
pub(crate) type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// The sorted items of `cache` (`cache.Items.OrderBy(t => t, comparer)`, a stable sort).
fn sorted_items<T: Clone, K: Eq + Hash + Clone>(cache: &Cache<T, K>, comparer: &Comparer<T>) -> Vec<T> {
    let mut items = cache.items();
    items.sort_by(|a, b| comparer(a, b));
    items
}

/// The index of `item` in `list` (`SortExtensions.GetCurrentPosition`).
///
/// Deviation (DEVIATIONS.md, DynamicData): the message of the error names the type of the item,
/// not the item, which would need `T: Debug`.
pub(crate) fn get_current_position<T: Clone + PartialEq>(list: &dyn IList<T>, item: &T, comparer: &Comparer<T>, use_binary_search: bool) -> Result<usize, ObservableError> {
    let index = if use_binary_search { binary_search_by(list, item, &|a: &T, b: &T| comparer(a, b)).ok() } else { list.index_of(item) };
    index.ok_or_else(|| SortException::new(format!("Cannot find item: {} from {} items", short_type_name::<T>(), list.count())).into_error())
}

/// The index at which `item` goes in `list` (`SortExtensions.GetInsertPosition`).
pub(crate) fn get_insert_position<T: Clone + PartialEq>(list: &dyn IList<T>, item: &T, comparer: &Comparer<T>, use_binary_search: bool) -> usize {
    if use_binary_search {
        get_insert_position_binary(list, item, comparer)
    } else {
        get_insert_position_linear(list, item, comparer)
    }
}

/// `SortExtensions.GetInsertPositionBinary`: when the list already has an item of the same value
/// (an update), the search does not give the position, and the linear search does.
pub(crate) fn get_insert_position_binary<T: Clone + PartialEq>(list: &dyn IList<T>, item: &T, comparer: &Comparer<T>) -> usize {
    match binary_search_by(list, item, &|a: &T, b: &T| comparer(a, b)) {
        Err(insert_index) => insert_index,
        Ok(_) => get_insert_position_linear(list, item, comparer),
    }
}

/// The index of the first item after which `item` sorts (`SortExtensions.GetInsertPositionLinear`).
pub(crate) fn get_insert_position_linear<T: Clone + PartialEq>(list: &dyn IList<T>, item: &T, comparer: &Comparer<T>) -> usize {
    (0..list.count()).find(|&index| comparer(item, &list.get(index)) == Ordering::Less).unwrap_or_else(|| list.count())
}

/// The name of the type without its path (`typeof(T).Name`).
fn short_type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    let name = name.split('<').next().unwrap_or(name);
    name.rsplit("::").next().unwrap_or(name)
}

/// The target of the binding with its state: the list, held so that the binding can change it.
struct Target<L> {
    list: RefCell<L>,
}

impl<L: 'static> Target<L> {
    /// The target as an `ObservableCollectionExtended<T>`, if it is one (`target is
    /// ObservableCollectionExtended<TObject>`).
    fn as_extended<T: 'static>(&self) -> Option<ObservableCollectionExtended<T>> {
        (&*self.list.borrow() as &dyn Any).downcast_ref::<ObservableCollectionExtended<T>>().cloned()
    }
}

/// Applies the changes to the target at their sorted positions (`SortAndBind.SortApplicator`).
struct SortApplicator<T, K, L> {
    cache: Rc<RefCell<Cache<T, K>>>,
    target: Rc<Target<L>>,
    comparer: Comparer<T>,
    options: SortAndBindOptions,
}

impl<T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static, L: IList<T> + 'static> SortApplicator<T, K, L> {
    /// Sorts the whole target again, after a change of comparer (`ApplySort`).
    fn apply_sort(&self) {
        if self.cache.borrow().count() == 0 {
            return;
        }
        let fire_reset = self.options.reset_threshold > 0 && self.options.reset_threshold < self.target.list.borrow().count();
        let sorted = sorted_items(&self.cache.borrow(), &self.comparer);
        self.reset(sorted, fire_reset);
    }

    /// Applies the sorted changes to the target as a side effect of the stream (`ProcessChanges`).
    fn process_changes(&self, change_set: &ChangeSet<T, K>, is_first_time_load: bool) -> Result<(), ObservableError> {
        let force_reset = is_first_time_load && self.options.reset_on_first_time_load;

        if force_reset || (self.options.reset_threshold > 0 && self.options.reset_threshold < change_set.count()) {
            let sorted = sorted_items(&self.cache.borrow(), &self.comparer);
            self.reset(sorted, true);
            Ok(())
        } else if let Some(extended) = self.target.as_extended::<T>() {
            // Suspend count as it can result in a flood of binding updates.
            let suspension = extended.suspend_count();
            let result = self.apply_changes(change_set);
            suspension.dispose();
            result
        } else {
            self.apply_changes(change_set)
        }
    }

    fn reset(&self, sorted: Vec<T>, fire_reset: bool) {
        match self.target.as_extended::<T>() {
            Some(extended) if fire_reset => {
                let suspension = extended.suspend_notifications();
                extended.load(sorted);
                suspension.dispose();
            }
            _ => {
                let mut target = self.target.list.borrow_mut();
                target.clear();
                for item in sorted {
                    target.add(item);
                }
            }
        }
    }

    /// Finds the sorted position of each change and applies it (`ApplyChanges`).
    fn apply_changes(&self, changes: &ChangeSet<T, K>) -> Result<(), ObservableError> {
        let use_binary_search = self.options.use_binary_search;
        let comparer = &self.comparer;
        for change in changes {
            let item = change.current();
            let mut target = self.target.list.borrow_mut();
            match change.reason() {
                ChangeReason::Add => {
                    let index = get_insert_position(&*target, item, comparer, use_binary_search);
                    target.insert(index, item.clone());
                }
                ChangeReason::Update => {
                    let previous = change.previous().expect("an update has a previous item");
                    if !self.options.use_replace_for_updates {
                        // If using binary search, it works best when we remove then add, so
                        // let's optimise for that first.
                        let current_index = get_current_position(&*target, previous, comparer, use_binary_search)?;
                        target.remove_at(current_index);

                        let updated_index = get_insert_position(&*target, item, comparer, use_binary_search);
                        target.insert(updated_index, item.clone());
                    } else {
                        let current_index = get_current_position(&*target, previous, comparer, use_binary_search)?;
                        let mut updated_index = get_insert_position(&*target, item, comparer, use_binary_search);

                        // We need to recalibrate as GetCurrentPosition includes the current item.
                        if current_index < updated_index {
                            updated_index -= 1;
                        }

                        // Some control suites and platforms do not support replace, whiles others
                        // do, so we opt in.
                        if current_index == updated_index {
                            target.set(current_index, item.clone());
                        } else {
                            target.remove_at(current_index);
                            target.insert(updated_index, item.clone());
                        }
                    }
                }
                ChangeReason::Remove => {
                    let current_index = get_current_position(&*target, item, comparer, use_binary_search)?;
                    target.remove_at(current_index);
                }
                ChangeReason::Refresh => {
                    // Look up the current location and the new location. The linear methods are
                    // used, as a binary search does not work on a list that is not sorted yet.
                    let Some(current_index) = target.index_of(item) else {
                        return Err(SortException::new(format!("Cannot find item: {}", short_type_name::<T>())).into_error());
                    };
                    let mut updated_index = get_insert_position_linear(&*target, item, comparer);

                    // We need to recalibrate as GetInsertPosition includes the current item.
                    if current_index < updated_index {
                        updated_index -= 1;
                    }
                    if updated_index != current_index {
                        // `SortExtensions.Move`: the list's own move where it has one.
                        target.move_item(current_index, updated_index);
                    }
                }
                ChangeReason::Moved => {}
            }
        }
        Ok(())
    }
}

/// `new SortAndBind<TObject, TKey>(source, comparer, options, target).Run()`: the comparer is
/// fixed.
pub(crate) fn sort_and_bind<T, K, L>(source: Rc<dyn IObservable<ChangeSet<T, K>>>, comparer: Comparer<T>, options: SortAndBindOptions, target: L) -> Rc<dyn IObservable<ChangeSet<T, K>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    L: IList<T> + 'static,
{
    let cache = Rc::new(RefCell::new(Cache::<T, K>::new()));
    let target = Rc::new(Target { list: RefCell::new(target) });

    // A static, one time comparer.
    let applicator = SortApplicator { cache: cache.clone(), target, comparer, options: options.clone() };

    let source = match &options.scheduler {
        Some(scheduler) => source.observe_on(scheduler.clone()),
        None => source,
    };

    source.try_select_with_index(move |changes: ChangeSet<T, K>, index| {
        // Clone to the local cache so that the entire set can be sorted when the threshold is
        // over a certain size.
        cache.borrow_mut().clone_changes(&changes);
        applicator.process_changes(&changes, index == 0)?;
        Ok(changes)
    })
}

/// `new SortAndBind<TObject, TKey>(source, comparerChanged, options, target).Run()`: each
/// comparer that `comparer_changed` gives sorts the target again; the changes before the first
/// comparer only fill the cache.
pub(crate) fn sort_and_bind_dynamic<T, K, L>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    comparer_changed: Rc<dyn IObservable<Comparer<T>>>,
    options: SortAndBindOptions,
    target: L,
) -> Rc<dyn IObservable<ChangeSet<T, K>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    L: IList<T> + 'static,
{
    let cache = Rc::new(RefCell::new(Cache::<T, K>::new()));
    let target = Rc::new(Target { list: RefCell::new(target) });

    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let (source, comparer_changed) = match &options.scheduler {
            Some(scheduler) => (source.observe_on(scheduler.clone()), comparer_changed.observe_on(scheduler.clone())),
            None => (source.clone(), comparer_changed.clone()),
        };

        let sort_applicator: Rc<RefCell<Option<Rc<SortApplicator<T, K, L>>>>> = Rc::new(RefCell::new(None));

        // Create a new sort applicator each time.
        let latest_comparer = {
            let sort_applicator = sort_applicator.clone();
            let cache = cache.clone();
            let target = target.clone();
            let options = options.clone();
            comparer_changed.subscribe_next(move |comparer: Comparer<T>| {
                let applicator = Rc::new(SortApplicator { cache: cache.clone(), target: target.clone(), comparer, options: options.clone() });
                *sort_applicator.borrow_mut() = Some(applicator.clone());
                applicator.apply_sort();
            })
        };

        // Listen to changes and apply the sorting.
        let cache = cache.clone();
        let subscriber = source
            .try_select_with_index(move |changes: ChangeSet<T, K>, index| {
                cache.borrow_mut().clone_changes(&changes);

                // The sort applicator is none until the observable of comparers gives one.
                let applicator = sort_applicator.borrow().clone();
                if let Some(applicator) = applicator {
                    applicator.process_changes(&changes, index == 0)?;
                }
                Ok(changes)
            })
            .subscribe(observer);

        let subscriptions: Vec<Rc<dyn IDisposable>> = vec![latest_comparer, subscriber];
        CompositeDisposable::from_disposables(subscriptions) as Rc<dyn IDisposable>
    })
}
