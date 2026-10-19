//! `Sort<TObject, TKey>` (`Cache/Internal/Sort.cs`).

use super::{Comparer, IndexCalculator, KeyValueCollection, KeyValueComparer};
use crate::cache::{ChangeAwareCache, ChangeSet, IQuery, SortOptimisations, SortReason, SortedChangeSet};
use crate::list::SortException;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The sorted change sets of `source` (`Sort<TObject, TKey>.Run`): sorted by `comparer`, or by the
/// latest comparer of `comparer_changed`, again whenever `resorter` notifies.
///
/// A change set of at least `reset_threshold` changes (when it is not 0) rebuilds the sorted
/// items rather than computing the indexes of each change.
///
/// Deviation (DEVIATIONS.md, DynamicData): the three sources are merged without the shared
/// delivery queue of upstream (single-threaded, section 5.1); a `SortException` ends the sequence
/// with that error, as upstream's exception reaches `OnError`.
///
/// # Panics
/// When neither `comparer` nor `comparer_changed` is given (`ArgumentException`).
pub(crate) fn sort<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    comparer: Option<Comparer<T>>,
    sort_optimisations: SortOptimisations,
    comparer_changed: Option<Rc<dyn IObservable<Comparer<T>>>>,
    resorter: Option<Rc<dyn IObservable<()>>>,
    reset_threshold: usize,
) -> Rc<dyn IObservable<SortedChangeSet<T, K>>> {
    assert!(comparer.is_some() || comparer_changed.is_some(), "Must specify comparer or comparerChanged");
    Observable::create(move |observer: Rc<dyn IObserver<SortedChangeSet<T, K>>>| {
        let sorter = Rc::new(RefCell::new(Sorter::new(sort_optimisations, comparer.clone(), reset_threshold)));

        if comparer_changed.is_none() && resorter.is_none() {
            let data_sorter = sorter.clone();
            let data_changed = source.try_select(move |changes: ChangeSet<T, K>| data_sorter.borrow_mut().sort_changes(&changes).map_err(SortException::into_error));
            return where_some(data_changed).subscribe(observer);
        }

        let comparer_sorter = sorter.clone();
        let comparer_changed = comparer_changed
            .clone()
            .unwrap_or_else(Observable::never)
            .try_select(move |comparer: Comparer<T>| comparer_sorter.borrow_mut().sort_comparer(comparer).map_err(SortException::into_error));
        let reorder_sorter = sorter.clone();
        let sort_again = resorter.clone().unwrap_or_else(Observable::never).try_select(move |_: ()| reorder_sorter.borrow_mut().sort_again().map_err(SortException::into_error));
        let data_sorter = sorter.clone();
        let data_changed = source.try_select(move |changes: ChangeSet<T, K>| data_sorter.borrow_mut().sort_changes(&changes).map_err(SortException::into_error));
        where_some(Observable::merge_all(vec![comparer_changed, data_changed, sort_again])).subscribe(observer)
    })
}

/// The values of `source` that are `Some` (`Where(result => result is not null).Select(x => x!)`).
pub(crate) fn where_some<U: Clone + 'static>(source: Rc<dyn IObservable<Option<U>>>) -> Rc<dyn IObservable<U>> {
    source.where_(Option::is_some).select(|value: Option<U>| value.expect("the values that are not Some are filtered out"))
}

/// The state of one subscription of [`sort`] (`Sort<TObject, TKey>.Sorter`).
struct Sorter<T, K> {
    optimisations: SortOptimisations,
    reset_threshold: usize,
    cache: ChangeAwareCache<T, K>,
    calculator: Option<IndexCalculator<T, K>>,
    comparer: KeyValueComparer<T, K>,
    have_received_data: bool,
    initialised: bool,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Sorter<T, K> {
    fn new(optimisations: SortOptimisations, comparer: Option<Comparer<T>>, reset_threshold: usize) -> Self {
        Self {
            optimisations,
            reset_threshold,
            cache: ChangeAwareCache::new(),
            calculator: None,
            comparer: KeyValueComparer::new(comparer),
            have_received_data: false,
            initialised: false,
        }
    }

    /// Sorts the changes of the data (`Sort(IChangeSet)`).
    fn sort_changes(&mut self, changes: &ChangeSet<T, K>) -> Result<Option<SortedChangeSet<T, K>>, SortException> {
        self.do_sort(SortReason::DataChanged, Some(changes))
    }

    /// Sorts with a new comparer (`Sort(IComparer<TObject>)`).
    fn sort_comparer(&mut self, comparer: Comparer<T>) -> Result<Option<SortedChangeSet<T, K>>, SortException> {
        self.comparer = KeyValueComparer::new(Some(comparer));
        self.do_sort(SortReason::ComparerChanged, None)
    }

    /// Sorts again (`Sort()`).
    fn sort_again(&mut self) -> Result<Option<SortedChangeSet<T, K>>, SortException> {
        self.do_sort(SortReason::Reorder, None)
    }

    fn do_sort(&mut self, mut sort_reason: SortReason, changes: Option<&ChangeSet<T, K>>) -> Result<Option<SortedChangeSet<T, K>>, SortException> {
        let changes = changes.map(|changes| {
            crate::cache::ICache::clone_changes(&mut self.cache, changes);
            self.have_received_data = true;
            self.cache.capture_changes()
        });

        // Without data there is nothing to sort.
        if !self.have_received_data {
            return Ok(None);
        }

        if !self.initialised {
            sort_reason = SortReason::InitialLoad;
            self.initialised = true;
        } else if let Some(changes) = &changes {
            if self.reset_threshold > 0 && changes.count() >= self.reset_threshold {
                sort_reason = SortReason::Reset;
            }
        }

        let change_set = match sort_reason {
            SortReason::InitialLoad => {
                // For the first batch, changes may have arrived before the comparer was set, so
                // the first batch of changes is inferred from the cache.
                let mut calculator = IndexCalculator::new(self.comparer.clone(), self.optimisations);
                let change_set = calculator.load(&self.cache);
                self.calculator = Some(calculator);
                change_set
            }
            SortReason::Reset => {
                let Self { calculator, cache, .. } = &mut *self;
                calculator.as_mut().expect("The calculator has not been initialized").reset(cache);
                changes.expect("a reset of the data has its changes")
            }
            SortReason::DataChanged => {
                let changes = changes.expect("Data has been indicated as changed, but changes is null.");
                self.calculator_mut().calculate(&changes)?
            }
            SortReason::ComparerChanged => {
                let comparer = self.comparer.clone();
                let mut change_set = self.calculator_mut().change_comparer(comparer);
                if self.reset_threshold > 0 && self.cache.count() >= self.reset_threshold {
                    sort_reason = SortReason::Reset;
                    let Self { calculator, cache, .. } = &mut *self;
                    calculator.as_mut().expect("The calculator has not been initialized").reset(cache);
                } else {
                    sort_reason = SortReason::Reorder;
                    change_set = self.calculator_mut().reorder();
                }
                change_set
            }
            SortReason::Reorder => self.calculator_mut().reorder(),
        };

        if sort_reason == SortReason::Reorder && change_set.count() == 0 {
            return Ok(None);
        }

        let calculator = self.calculator.as_ref().expect("The calculator has not been initialized");
        let sorted = KeyValueCollection::new(calculator.list().to_vec(), self.comparer.clone(), sort_reason, self.optimisations);
        Ok(Some(SortedChangeSet::new(sorted, change_set)))
    }

    fn calculator_mut(&mut self) -> &mut IndexCalculator<T, K> {
        self.calculator.as_mut().expect("The calculator has not been initialized")
    }
}
