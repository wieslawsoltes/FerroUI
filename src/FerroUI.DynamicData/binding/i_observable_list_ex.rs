//! `IObservableListEx` (`Binding/IObservableListEx.cs`): `BindToObservableList`, the change sets
//! loaded into an observable list as they pass.

use crate::cache::internal::remove_key_enumerator::remove_key_changes as remove_key;
use crate::cache::{IChangeSet, SortReason, SortedChangeSet};
use crate::list::{clone_changes, IExtendedList, IList, IObservableList, SourceList};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// The changes of a cache applied to `list` without their keys (`editor.Clone(changes.RemoveKey(editor))`).
fn clone_without_keys<T: Clone + PartialEq + 'static, K>(editor: &mut dyn IExtendedList<T>, changes: &crate::cache::ChangeSet<T, K>) {
    let list_changes = remove_key(changes, Some(&*editor as &dyn IList<T>));
    clone_changes(editor, &crate::list::ChangeSet::from(list_changes));
}

/// A sequence that subscribes `source` and disposes `list` when the subscription ends
/// (`Observable.Create(observer => source.Finally(() => sourceList.Dispose()).SubscribeSafe(observer))`).
fn disposing_list<C: 'static, T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<C>>, list: SourceList<T>) -> Rc<dyn IObservable<C>> {
    Observable::create(move |observer: Rc<dyn IObserver<C>>| {
        let list = list.clone();
        source.finally(move || list.dispose()).subscribe(observer)
    })
}

/// `BindToObservableList<TObject>(this IObservable<IChangeSet<TObject>>, out IObservableList<TObject>)`.
pub(crate) fn bind_list_to_observable_list<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<crate::list::ChangeSet<T>>>,
) -> (Rc<dyn IObservable<crate::list::ChangeSet<T>>>, Rc<dyn IObservableList<T>>) {
    // Load the source list with the change sets.
    let source_list = SourceList::from_source(source.clone());

    // Output the read-only observable list, preventing the source list from being edited from
    // anywhere else, and connect to the source so that the list is disposed when the pipeline
    // ends.
    let observable_list: Rc<dyn IObservableList<T>> = Rc::new(source_list.clone());
    (disposing_list(source, source_list), observable_list)
}

/// `BindToObservableList<TObject, TKey>(this IObservable<IChangeSet<TObject, TKey>>, out
/// IObservableList<TObject>)`.
pub(crate) fn bind_cache_to_observable_list<T: Clone + PartialEq + 'static, K: 'static>(
    source: Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>,
) -> (Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>, Rc<dyn IObservableList<T>>) {
    let source_list = SourceList::<T>::new();
    let observable_list: Rc<dyn IObservableList<T>> = Rc::new(source_list.clone());

    let target = source_list.clone();
    let loading = source.do_(move |changes: &crate::cache::ChangeSet<T, K>| target.edit(|editor| clone_without_keys(editor, changes)));
    (disposing_list(loading, source_list), observable_list)
}

/// `BindToObservableList<TObject, TKey>(this IObservable<ISortedChangeSet<TObject, TKey>>, out
/// IObservableList<TObject>)`, as plain change sets.
pub(crate) fn bind_sorted_to_observable_list<T: Clone + PartialEq + 'static, K: 'static>(
    source: Rc<dyn IObservable<SortedChangeSet<T, K>>>,
) -> (Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>>, Rc<dyn IObservableList<T>>) {
    let source_list = SourceList::<T>::new();
    let observable_list: Rc<dyn IObservableList<T>> = Rc::new(source_list.clone());

    let target = source_list.clone();
    let loading = source.do_(move |changes: &SortedChangeSet<T, K>| match changes.sorted_items().sort_reason() {
        SortReason::InitialLoad => {
            let items: Vec<T> = changes.sorted_items().values().cloned().collect();
            target.edit(|editor| editor.add_range(items));
        }
        SortReason::ComparerChanged | SortReason::DataChanged | SortReason::Reorder => {
            target.edit(|editor| clone_without_keys(editor, changes.change_set()));
        }
        SortReason::Reset => {
            let items: Vec<T> = changes.sorted_items().values().cloned().collect();
            target.edit(|editor| {
                editor.clear();
                editor.add_range(items);
            });
        }
    });
    let plain = disposing_list(loading, source_list).select(|changes: SortedChangeSet<T, K>| changes.change_set().clone());
    (plain, observable_list)
}
