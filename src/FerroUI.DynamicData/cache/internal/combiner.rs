//! `Combiner<TObject, TKey>` (`Cache/Internal/Combiner.cs`): the items of a fixed set of caches,
//! combined by a logical operator.

use super::{Cache, CombineOperator};
use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// Whether two items are the same object (`ReferenceEquals` of upstream's combiners).
pub(crate) type SameReference<T> = Rc<dyn Fn(&T, &T) -> bool>;

/// The items of `sources` that `kind` selects (`Combine(ICollection<IObservable<IChangeSet>>,
/// CombineOperator)` and `Combine(IObservable<IChangeSet>, CombineOperator, params ..)`): each
/// subscription has a combiner of its own, which subscribes the sources in order.
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream's callback catches an exception of the
/// observer and reports it as an error, as does the creation of the combiner; a panic of an
/// observer is not caught here (docs/porting/reactive.md, section 5.4).
pub(crate) fn combiner<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    sources: Vec<Rc<dyn IObservable<ChangeSet<T, K>>>>,
    kind: CombineOperator,
    same_reference: SameReference<T>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let combiner = Rc::new(Combiner {
            kind,
            same_reference: same_reference.clone(),
            updated_callback: observer,
            combined_cache: RefCell::new(ChangeAwareCache::new()),
            source_caches: RefCell::new(Vec::new()),
        });
        Combiner::subscribe(&combiner, &sources)
    })
}

/// The state of a subscription (`Combiner<TObject, TKey>`).
struct Combiner<T, K> {
    kind: CombineOperator,
    same_reference: SameReference<T>,
    updated_callback: Rc<dyn IObserver<ChangeSet<T, K>>>,
    combined_cache: RefCell<ChangeAwareCache<T, K>>,
    source_caches: RefCell<Vec<Cache<T, K>>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Combiner<T, K> {
    /// `Subscribe`: a copy of each source is made before any source is subscribed, so that the
    /// items a source has when it is subscribed are combined with all the copies.
    fn subscribe(this: &Rc<Self>, source: &[Rc<dyn IObservable<ChangeSet<T, K>>>]) -> Rc<dyn IDisposable> {
        let disposable = CompositeDisposable::new();
        this.source_caches.borrow_mut().extend((0..source.len()).map(|_| Cache::new()));
        for (index, item) in source.iter().enumerate() {
            let combiner = this.clone();
            // Upstream subscribes without an error handler: an error of a source is thrown.
            let subscription = item.subscribe_next(move |updates: ChangeSet<T, K>| combiner.update(index, &updates));
            disposable.add(subscription);
        }
        disposable
    }

    /// Whether the item of `key` belongs to the result (`MatchesConstraint`).
    fn matches_constraint(&self, key: &K) -> bool {
        let caches = self.source_caches.borrow();
        let has_key = |cache: &Cache<T, K>| cache.dictionary().contains_key(key);
        match self.kind {
            CombineOperator::And => caches.iter().all(has_key),
            CombineOperator::Or => caches.iter().any(has_key),
            CombineOperator::Xor => caches.iter().filter(|&cache| has_key(cache)).count() == 1,
            CombineOperator::Except => {
                let first = caches.iter().take(1).any(has_key);
                let others = caches.iter().skip(1).any(has_key);
                first && !others
            }
        }
    }

    /// `Update`: the copy of the source is updated, then the result.
    fn update(&self, index: usize, updates: &ChangeSet<T, K>) {
        // update cache for the individual source
        self.source_caches.borrow_mut()[index].clone_changes(updates);
        // update combined
        let notifications = self.update_combined(updates);
        if notifications.count() != 0 {
            self.updated_callback.on_next(notifications);
        }
    }

    /// `UpdateCombined`.
    fn update_combined(&self, updates: &ChangeSet<T, K>) -> ChangeSet<T, K> {
        let mut combined_cache = self.combined_cache.borrow_mut();
        // child caches have been updated before we reached this point.
        for update in updates {
            let key = update.key();
            match update.reason() {
                ChangeReason::Add | ChangeReason::Update => {
                    // get the current key.
                    // check whether the item should belong to the cache
                    let cached = combined_cache.lookup(key);
                    let matches = self.matches_constraint(key);
                    if matches {
                        match cached {
                            Some(cached) => {
                                if !(self.same_reference)(update.current(), &cached) {
                                    combined_cache.add_or_update(update.current().clone(), key.clone());
                                }
                            }
                            None => combined_cache.add_or_update(update.current().clone(), key.clone()),
                        }
                    } else if cached.is_some() {
                        combined_cache.remove(key);
                    }
                }
                ChangeReason::Remove => {
                    let cached = combined_cache.lookup(key);
                    let should_be_included = self.matches_constraint(key);
                    if should_be_included {
                        let first_one = self
                            .source_caches
                            .borrow()
                            .iter()
                            .find_map(|cache| cache.lookup(key))
                            .expect("a source holds the key that the constraint matches");
                        match cached {
                            None => combined_cache.add_or_update(first_one, key.clone()),
                            Some(cached) => {
                                if !(self.same_reference)(&first_one, &cached) {
                                    combined_cache.add_or_update(first_one, key.clone());
                                }
                            }
                        }
                    } else if cached.is_some() {
                        combined_cache.remove(key);
                    }
                }
                ChangeReason::Refresh => combined_cache.refresh(key),
                ChangeReason::Moved => {}
            }
        }
        combined_cache.capture_changes()
    }
}
