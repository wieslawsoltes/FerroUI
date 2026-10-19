//! `EnumerableEx` (`EnumerableEx.cs`): a collection as a sequence of change sets.

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::Observable;
use ferroui_system::{IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// The collection as a sequence of one change set of additions (`EnumerableEx`), implemented for
/// every collection.
pub trait EnumerableEx<T: Clone + 'static>: IntoIterator<Item = T> + Sized {
    /// The items as one change set of additions to a list, at each subscription; the sequence does
    /// not end (`AsObservableChangeSet<TObject>(IEnumerable<TObject>, bool completable = false)`).
    fn as_observable_change_set(self) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        self.as_observable_change_set_with(false)
    }

    /// [`as_observable_change_set`](Self::as_observable_change_set) that ends after the change set
    /// when `completable` is true.
    fn as_observable_change_set_with(self, completable: bool) -> Rc<dyn IObservable<crate::list::ChangeSet<T>>> {
        let items: Vec<T> = self.into_iter().collect();
        Observable::create(move |observer: Rc<dyn IObserver<crate::list::ChangeSet<T>>>| {
            let changes: Vec<crate::list::Change<T>> = items.iter().map(|item| crate::list::Change::new(crate::list::ListChangeReason::Add, item.clone(), None)).collect();
            observer.on_next(crate::list::ChangeSet::from(changes));
            if completable {
                observer.on_completed();
            }
            Disposable::empty()
        })
    }

    /// The items as one change set of additions to a cache, keyed by `key_selector`
    /// (`AsObservableChangeSet<TObject, TKey>(IEnumerable<TObject>, Func<TObject, TKey>, bool
    /// completable = false)`).
    fn as_observable_change_set_cache<K: Eq + Hash + Clone + 'static>(self, key_selector: impl Fn(&T) -> K + 'static) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        self.as_observable_change_set_cache_with(key_selector, false)
    }

    /// [`as_observable_change_set_cache`](Self::as_observable_change_set_cache) that ends after the
    /// change set when `completable` is true.
    fn as_observable_change_set_cache_with<K: Eq + Hash + Clone + 'static>(self, key_selector: impl Fn(&T) -> K + 'static, completable: bool) -> Rc<dyn IObservable<crate::cache::ChangeSet<T, K>>> {
        let items: Vec<T> = self.into_iter().collect();
        Observable::create(move |observer: Rc<dyn IObserver<crate::cache::ChangeSet<T, K>>>| {
            let changes: crate::cache::ChangeSet<T, K> = items.iter().map(|item| crate::cache::Change::new(crate::cache::ChangeReason::Add, key_selector(item), item.clone())).collect();
            observer.on_next(changes);
            if completable {
                observer.on_completed();
            }
            Disposable::empty()
        })
    }
}

impl<T: Clone + 'static, I: IntoIterator<Item = T>> EnumerableEx<T> for I {}
