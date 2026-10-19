//! `SpecifiedGrouper<TObject, TKey, TGroupKey>` (`Cache/Internal/SpecifiedGrouper.cs`).

use super::group_on::{dispose_many, group_on};
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, Group, GroupChangeSet, ICache, IChangeSet, IConnectableCache, IObservableCache, ObservableCache};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The groups of the items of `source` by `group_selector`, limited to the group keys of
/// `result_group_source`: a group exists while its key is there, even without items
/// (`SpecifiedGrouper.Run`).
///
/// The keys come from a change set whose items are their own keys, as `DistinctValues` gives
/// (`IDistinctChangeSet<TGroupKey>`); any such change set is taken.
pub(crate) fn specified_grouper<T, K, G, C>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    group_selector: Rc<dyn Fn(&T) -> G>,
    result_group_source: Rc<dyn IObservable<C>>,
) -> Rc<dyn IObservable<GroupChangeSet<T, K, G>>>
where
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
    C: IChangeSet<G, G>,
{
    Observable::create(move |observer: Rc<dyn IObserver<GroupChangeSet<T, K, G>>>| {
        // The `SharedDeliveryQueue` of upstream serializes the two sources; on one thread they are
        // already serial.

        // The groups of the source.
        let source_groups: ObservableCache<Group<T, K, G>, G> = ObservableCache::from_source(dispose_many(group_on(source.clone(), group_selector.clone(), None), Group::ptr_eq));

        // The groups of the result (`Transform(..).DisposeMany().AsObservableCache()`): a group of
        // a key is filled with the items of the group of the source, if there is one.
        let parent_groups: ObservableCache<Group<T, K, G>, G> = {
            let transformed = Rc::new(RefCell::new(ChangeAwareCache::<Group<T, K, G>, G>::new()));
            let lookup = source_groups.clone();
            let transformed_groups = result_group_source.select(move |changes: C| {
                let mut transformed = transformed.borrow_mut();
                for change in changes.change_set() {
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            let group_key = change.current().clone();
                            let result = Group::new(group_key.clone());
                            if let Some(child) = lookup.lookup(&group_key) {
                                let initial_updates = child.get_initial_updates();
                                result.update(|updater| updater.clone_changes(&initial_updates));
                            }
                            transformed.add_or_update(result, change.key().clone());
                        }
                        ChangeReason::Remove => transformed.remove(change.key()),
                        ChangeReason::Refresh => transformed.refresh(change.key()),
                        ChangeReason::Moved => {}
                    }
                }
                transformed.capture_changes()
            });
            ObservableCache::from_source(dispose_many(transformed_groups, Group::ptr_eq))
        };

        // Each group of the source updates the group of the result of its key
        // (`SubscribeMany(..).DisposeMany().Subscribe()`).
        let updates_from_children = {
            let child_subscriptions = Rc::new(RefCell::new(Dictionary::<G, Rc<dyn IDisposable>>::new()));
            let (subscriptions, targets) = (child_subscriptions.clone(), parent_groups.clone());
            let subscription = source_groups.connect().subscribe_all(
                move |changes: ChangeSet<Group<T, K, G>, G>| {
                    for change in &changes {
                        match change.reason() {
                            ChangeReason::Add | ChangeReason::Update => {
                                let group = change.current().clone();
                                let (targets, group_key) = (targets.clone(), group.key().clone());
                                let child = group.observable_cache().connect().subscribe_all(
                                    move |updates: ChangeSet<T, K>| {
                                        if let Some(group_to_update) = targets.lookup(&group_key) {
                                            group_to_update.update(|updater| updater.clone_changes(&updates));
                                        }
                                    },
                                    |_| {},
                                    || {},
                                );
                                let replaced = subscriptions.borrow_mut().insert(change.key().clone(), child);
                                if let Some(replaced) = replaced {
                                    replaced.dispose();
                                }
                            }
                            ChangeReason::Remove => {
                                let removed = subscriptions.borrow_mut().remove(change.key());
                                if let Some(removed) = removed {
                                    removed.dispose();
                                }
                            }
                            ChangeReason::Refresh | ChangeReason::Moved => {}
                        }
                    }
                },
                |_| {},
                || {},
            );
            Disposable::create(move || {
                subscription.dispose();
                let remaining: Vec<Rc<dyn IDisposable>> = child_subscriptions.borrow().values().cloned().collect();
                child_subscriptions.borrow_mut().clear();
                for child in remaining {
                    child.dispose();
                }
            })
        };

        let notifier = parent_groups
            .connect()
            .select(|changes: ChangeSet<Group<T, K, G>, G>| {
                let groups: Vec<Change<Group<T, K, G>, G>> = changes
                    .iter()
                    .map(|change| Change::with_previous(change.reason(), change.key().clone(), change.current().clone(), change.previous().cloned(), None, None))
                    .collect();
                GroupChangeSet::from(groups)
            })
            .subscribe(observer);

        Disposable::create(move || {
            notifier.dispose();
            source_groups.dispose();
            parent_groups.dispose();
            updates_from_children.dispose();
        })
    })
}
