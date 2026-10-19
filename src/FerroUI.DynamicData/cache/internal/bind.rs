//! The bodies of `Bind` (`Cache/ObservableCacheEx.Bind.cs`), and the choice between the forms of
//! upstream's overloads for sorted and plain change sets.
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream picks the overload of `Bind` for
//! `IObservable<ISortedChangeSet<TObject, TKey>>` when the source is sorted (the sorted, paged and
//! virtual change sets). The operators of the port are provided methods of one trait, so `bind` looks
//! at the type of the source when it runs, and binds a sorted source with the sorted adaptor. The
//! result is a sequence of plain change sets in both cases, where upstream's sorted form returns
//! the sorted change sets.

use crate::binding::{BindingOptions, IObservableCollection, IObservableCollectionAdaptor, ISortedObservableCollectionAdaptor, ObservableCollectionAdaptor, ObservableCollectionExtended, ReadOnlyObservableCollection, SortedObservableCollectionAdaptor};
use crate::cache::{ChangeSet, IChangeSet, ISortedChangeSet, PagedChangeSet, SortedChangeSet};
use crate::DynamicDataOptions;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::hash::Hash;
use std::rc::Rc;

/// The source as a sequence of sorted change sets, when it is one (sorted, paged or virtual).
pub(crate) fn as_sorted_stream<S: 'static, T: 'static, K: 'static>(source: &S) -> Option<Rc<dyn IObservable<SortedChangeSet<T, K>>>>
where
    T: Clone,
    K: Clone,
{
    let source = source as &dyn Any;
    if let Some(sorted) = source.downcast_ref::<Rc<dyn IObservable<SortedChangeSet<T, K>>>>() {
        return Some(sorted.clone());
    }
    if let Some(paged) = source.downcast_ref::<Rc<dyn IObservable<PagedChangeSet<T, K>>>>() {
        return Some(paged.select(|changes: PagedChangeSet<T, K>| changes.to_sorted_change_set()));
    }
    if let Some(virtualised) = source.downcast_ref::<Rc<dyn IObservable<crate::cache::VirtualChangeSet<T, K>>>>() {
        return Some(virtualised.select(|changes: crate::cache::VirtualChangeSet<T, K>| changes.to_sorted_change_set()));
    }
    None
}

/// The source as a sequence of sorted change sets.
///
/// # Panics
/// When the source is not sorted (the overload of upstream does not apply).
pub(crate) fn expect_sorted_stream<S: 'static, T: Clone + 'static, K: Clone + 'static>(source: &S) -> Rc<dyn IObservable<SortedChangeSet<T, K>>> {
    as_sorted_stream(source).expect("a sorted adaptor binds a sequence of sorted change sets (Sort, Page, Virtualise)")
}

/// The options of the forms that take a threshold: the defaults of `DynamicDataOptions.Binding`
/// unless the arguments differ from upstream's defaults.
pub(crate) fn binding_options(reset_threshold: usize, use_replace_for_updates: bool) -> BindingOptions {
    let defaults = DynamicDataOptions::binding();
    if reset_threshold == BindingOptions::DEFAULT_RESET_THRESHOLD && use_replace_for_updates == BindingOptions::DEFAULT_USE_REPLACE_FOR_UPDATES {
        defaults
    } else {
        defaults.with_reset_threshold(reset_threshold).with_use_replace_for_updates(use_replace_for_updates)
    }
}

/// `Bind(IObservableCollection<TObject>, IObservableCollectionAdaptor<TObject, TKey>)`.
pub(crate) fn bind_with_adaptor<T: Clone + PartialEq + 'static, K: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    destination: Rc<dyn IObservableCollection<T>>,
    updater: Rc<dyn IObservableCollectionAdaptor<T, K>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    source.select(move |changes: ChangeSet<T, K>| {
        updater.adapt(&changes, &*destination);
        changes
    })
}

/// `Bind(IObservableCollection<TObject>, ISortedObservableCollectionAdaptor<TObject, TKey>)`, as
/// plain change sets.
pub(crate) fn bind_sorted_with_adaptor<T: Clone + PartialEq + 'static, K: 'static>(
    source: Rc<dyn IObservable<SortedChangeSet<T, K>>>,
    destination: Rc<dyn IObservableCollection<T>>,
    updater: Rc<dyn ISortedObservableCollectionAdaptor<T, K>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    source.select(move |changes: SortedChangeSet<T, K>| {
        updater.adapt(&changes, &*destination);
        changes.change_set().clone()
    })
}

/// `Bind(IObservableCollection<TObject>, BindingOptions)` of a sorted or a plain source.
pub(crate) fn bind_with_options<S: 'static, T: Clone + PartialEq + 'static, K: Eq + Hash + Clone + 'static>(
    source: &S,
    plain: Rc<dyn IObservable<ChangeSet<T, K>>>,
    destination: Rc<dyn IObservableCollection<T>>,
    options: BindingOptions,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    match as_sorted_stream::<S, T, K>(source) {
        Some(sorted) => bind_sorted_with_adaptor(sorted, destination, Rc::new(SortedObservableCollectionAdaptor::with_options(options))),
        None => bind_with_adaptor(plain, destination, Rc::new(ObservableCollectionAdaptor::<T, K>::with_options(options))),
    }
}

/// A new collection and its read-only view (the `out ReadOnlyObservableCollection<TObject>` of
/// upstream).
pub(crate) fn new_read_only_target<T: Clone + PartialEq + 'static>(capacity: Option<usize>) -> (ObservableCollectionExtended<T>, ReadOnlyObservableCollection<T>) {
    let target = match capacity {
        Some(capacity) if capacity > 0 => ObservableCollectionExtended::with_capacity(capacity),
        _ => ObservableCollectionExtended::new(),
    };
    let read_only = ReadOnlyObservableCollection::from(&target);
    (target, read_only)
}

/// `Bind(IList<TObject>)` and `Bind(IList<TObject>, SortAndBindOptions)` of the results of
/// `SortAndPage` and `SortAndVirtualize` (`BindPaged`, `BindVirtualized`); `None` when the source
/// is not one of them.
pub(crate) fn bind_context_stream<S: 'static, T, K, L>(source: &S, target: L, options: Option<crate::binding::SortAndBindOptions>) -> Option<Rc<dyn IObservable<ChangeSet<T, K>>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    L: crate::list::IList<T> + Clone + 'static,
{
    use crate::cache::{ChangeSetWithContext, PageContext, VirtualContext};
    let source = source as &dyn Any;
    if let Some(paged) = source.downcast_ref::<Rc<dyn IObservable<ChangeSetWithContext<T, K, PageContext<T>>>>>() {
        return Some(crate::binding::bind_paged::bind_with_context(paged.clone(), target, options));
    }
    if let Some(virtualised) = source.downcast_ref::<Rc<dyn IObservable<ChangeSetWithContext<T, K, VirtualContext<T>>>>>() {
        return Some(crate::binding::bind_paged::bind_with_context(virtualised.clone(), target, options));
    }
    None
}

/// [`bind_context_stream`] for the forms that only apply to those sources.
///
/// # Panics
/// When the source is not a result of `SortAndPage` or `SortAndVirtualize`.
pub(crate) fn expect_bind_context_stream<S: 'static, T, K, L>(source: &S, target: L, options: Option<crate::binding::SortAndBindOptions>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    L: crate::list::IList<T> + Clone + 'static,
{
    bind_context_stream(source, target, options).expect("binding to a list needs the change sets of SortAndPage or SortAndVirtualize")
}
