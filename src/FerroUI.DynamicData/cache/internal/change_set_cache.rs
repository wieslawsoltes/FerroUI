//! `ChangeSetCache<TObject, TKey>` (`Cache/Internal/ChangeSetCache.cs`): a sequence of change sets
//! with the cache of its items, which the sequence updates as it is observed.

use super::Cache;
use crate::cache::{ChangeSet, ICache};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The sequence and its cache (`ChangeSetCache<TObject, TKey>`).
pub(crate) struct ChangeSetCache<T, K> {
    cache: Rc<RefCell<Cache<T, K>>>,
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ChangeSetCache<T, K> {
    /// `ChangeSetCache(IObservable<IChangeSet<TObject, TKey>>)`: the source, which clones each of
    /// its change sets into the cache.
    pub(crate) fn new(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Rc<Self> {
        let cache = Rc::new(RefCell::new(Cache::new()));
        let target = cache.clone();
        let source = source.do_(move |changes: &ChangeSet<T, K>| target.borrow_mut().clone_changes(changes));
        Rc::new(Self { cache, source })
    }

    /// The items so far (`Cache`).
    pub(crate) fn cache(&self) -> std::cell::Ref<'_, Cache<T, K>> {
        self.cache.borrow()
    }

    /// The source (`Source`).
    pub(crate) fn source(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.source.clone()
    }
}
