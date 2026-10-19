//! `Filter.Static<TObject, TKey>` (`Cache/Internal/Filter.Static.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The items of the source that `filter` accepts (`Filter.Static.Create`): an update or a refresh
/// evaluates the item again, which may add or remove it downstream; moves are not supported.
pub(crate) fn filter_static<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    filter: Rc<dyn Fn(&T) -> bool>,
    suppress_empty_change_sets: bool,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |downstream: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let downstream_items = RefCell::new(ChangeAwareCache::<T, K>::new());
        let filter = filter.clone();
        source
            .select(move |upstream_changes: ChangeSet<T, K>| {
                let mut items = downstream_items.borrow_mut();
                for change in &upstream_changes {
                    match change.reason() {
                        ChangeReason::Add => {
                            if filter(change.current()) {
                                items.add(change.current().clone(), change.key().clone());
                            }
                        }
                        // Moves are not supported upstream: tracking the indexes is too much work.
                        ChangeReason::Moved => {}
                        ChangeReason::Refresh => {
                            let is_included = filter(change.current());
                            let was_included = items.lookup(change.key()).is_some();
                            if is_included && !was_included {
                                items.add(change.current().clone(), change.key().clone());
                            } else if is_included && was_included {
                                items.refresh(change.key());
                            } else if !is_included && was_included {
                                items.remove(change.key());
                            }
                        }
                        ChangeReason::Remove => items.remove(change.key()),
                        ChangeReason::Update => {
                            if filter(change.current()) {
                                items.add_or_update(change.current().clone(), change.key().clone());
                            } else {
                                items.remove(change.key());
                            }
                        }
                    }
                }
                items.capture_changes()
            })
            .where_(move |changes: &ChangeSet<T, K>| !suppress_empty_change_sets || changes.count() != 0)
            .subscribe(downstream)
    })
}
