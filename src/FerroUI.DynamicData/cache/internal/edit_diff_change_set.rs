//! `EditDiffChangeSet<TObject, TKey>` (`Cache/Internal/EditDiffChangeSet.cs`): the changes of a
//! cache that holds the latest set of items of a sequence.

use super::edit_diff;
use crate::cache::{ChangeSet, IConnectableCache, SourceCache};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// `EditDiffChangeSet.Run`, with the `ObservableChangeSet.Create` it runs on: a cache keyed by
/// `key_selector` that each set of items edits by difference; the cache ends with the sequence.
pub(crate) fn edit_diff_change_set<T: Clone + 'static, K: Eq + Hash + Clone + 'static, I: IntoIterator<Item = T> + 'static>(
    source: Rc<dyn IObservable<I>>,
    key_selector: Rc<dyn Fn(&T) -> K>,
    equality_comparer: Rc<dyn Fn(&T, &T) -> bool>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let key_selector = key_selector.clone();
        let cache = SourceCache::new(move |item: &T| key_selector(item));
        let subscription = {
            let (edited, completed) = (cache.clone(), cache.clone());
            let equality_comparer = equality_comparer.clone();
            source.subscribe_next_completed(move |items: I| edit_diff(&edited, items.into_iter().collect(), &*equality_comparer), move || completed.dispose())
        };
        let completion = observer.clone();
        let connection = cache.connect().subscribe_safe(observer);
        Disposable::create(move || {
            subscription.dispose();
            completion.on_completed();
            connection.dispose();
            cache.dispose();
        })
    })
}
