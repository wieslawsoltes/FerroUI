//! `ExpireAfter.ForStream<TObject, TKey>` (`Cache/Internal/ExpireAfter.ForStream.cs`): the items of
//! a stream, each removed once the time that a selector gives for it has passed.

use super::expire_after::{Effects, ManagementMode, Notification, ProposedExpiration, ScheduledManagement};
use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use crate::kernel::dotnet_binary_search;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

struct Subscription<T, K> {
    expiration_due_times_by_key: HashMap<K, SystemTime>,
    items_cache: ChangeAwareCache<T, K>,
    proposed_expirations_queue: Vec<ProposedExpiration<K>>,
    scheduler: Rc<dyn IScheduler>,
    time_selector: Rc<dyn Fn(&T) -> Option<Duration>>,
    has_source_completed: bool,
    next_scheduled_management: Option<ScheduledManagement>,
    mode: ManagementMode,
}

type Shared<T, K> = Rc<RefCell<Subscription<T, K>>>;

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Subscription<T, K> {
    fn next_management_due_time(&self) -> Option<SystemTime> {
        let next_proposed = self.proposed_expirations_queue.first().map(|expiration| expiration.due_time);
        self.mode.next_management_due_time(self.scheduler.now(), next_proposed)
    }

    /// `ClearExpiration`: the proposed expiration stays in the queue and is dropped when it comes
    /// up, as finding it would cost more.
    fn clear_expiration(&mut self, key: &K) {
        self.expiration_due_times_by_key.remove(key);
    }

    /// `ManageExpirations`.
    fn manage_expirations(&mut self, effects: &mut Effects<ChangeSet<T, K>>) {
        // The scheduler only promises its best to cancel, so the management is checked.
        let Some(this_scheduled_management) = self.next_scheduled_management.take() else { return };
        let now = self.scheduler.now();

        let mut proposed_expiration_index = 0;
        while proposed_expiration_index < self.proposed_expirations_queue.len() {
            let proposed_expiration = &self.proposed_expirations_queue[proposed_expiration_index];
            if proposed_expiration.due_time > now {
                break;
            }
            // The queue may be out of date with the due times, so the item is checked.
            let key = proposed_expiration.key.clone();
            if self.expiration_due_times_by_key.get(&key).is_some_and(|due_time| *due_time <= now) {
                self.expiration_due_times_by_key.remove(&key);
                self.items_cache.remove(&key);
            }
            proposed_expiration_index += 1;
        }
        self.proposed_expirations_queue.drain(0..proposed_expiration_index);

        // The scheduler may have run late or early, so there may be nothing to publish.
        let downstream_changes = self.items_cache.capture_changes();
        if downstream_changes.count() != 0 {
            effects.notifications.push(Notification::Next(downstream_changes));
        }
        self.mode.on_expirations_managed(this_scheduled_management.due_time);

        // The queue has changed: clean it up and schedule the next management.
        self.on_expirations_changed(effects);
    }

    /// `OnExpirationsChanged`.
    fn on_expirations_changed(&mut self, effects: &mut Effects<ChangeSet<T, K>>) {
        // The expirations at the front of the queue that are out of date go.
        let remove_to_index = self.proposed_expirations_queue.iter().position(|expiration| self.expiration_due_times_by_key.contains_key(&expiration.key));
        if let Some(remove_to_index) = remove_to_index.filter(|index| *index > 0) {
            self.proposed_expirations_queue.drain(0..remove_to_index);
        }

        // Without items to expire after the end of the source, nothing more can be published.
        if self.expiration_due_times_by_key.is_empty() && self.has_source_completed {
            effects.cancel(&mut self.next_scheduled_management);
            effects.notifications.push(Notification::Completed);
            return;
        }

        match self.next_management_due_time() {
            Some(next_management_due_time) => {
                if self.next_scheduled_management.as_ref().map(|management| management.due_time) != Some(next_management_due_time) {
                    effects.reschedule(&mut self.next_scheduled_management, next_management_due_time);
                }
            }
            None => effects.cancel(&mut self.next_scheduled_management),
        }
    }

    /// `OnSourceCompleted`: the completion waits for the pending expirations.
    fn on_source_completed(&mut self, effects: &mut Effects<ChangeSet<T, K>>) {
        self.has_source_completed = true;
        if self.expiration_due_times_by_key.is_empty() {
            effects.cancel(&mut self.next_scheduled_management);
            effects.notifications.push(Notification::Completed);
        }
    }

    /// `OnSourceNext`.
    fn on_source_next(&mut self, upstream_changes: &ChangeSet<T, K>, effects: &mut Effects<ChangeSet<T, K>>) {
        let now = self.scheduler.now();
        let mut have_expirations_changed = false;
        for change in upstream_changes {
            match change.reason() {
                ChangeReason::Add => {
                    if let Some(expire_after) = (self.time_selector)(change.current()) {
                        have_expirations_changed |= self.try_set_expiration(change.key().clone(), now + expire_after);
                    }
                    self.items_cache.add_or_update(change.current().clone(), change.key().clone());
                }
                // Moves are ignored: this is a filter, functionally.
                ChangeReason::Moved => {}
                ChangeReason::Remove => {
                    self.clear_expiration(change.key());
                    self.items_cache.remove(change.key());
                }
                ChangeReason::Refresh => self.items_cache.refresh(change.key()),
                ChangeReason::Update => {
                    match (self.time_selector)(change.current()) {
                        // Upstream assigns rather than combines here.
                        Some(expire_after) => have_expirations_changed = self.try_set_expiration(change.key().clone(), now + expire_after),
                        None => {
                            self.clear_expiration(change.key());
                            have_expirations_changed = true;
                        }
                    }
                    self.items_cache.add_or_update(change.current().clone(), change.key().clone());
                }
            }
        }

        if have_expirations_changed {
            self.on_expirations_changed(effects);
        }

        let downstream_changes = self.items_cache.capture_changes();
        if downstream_changes.count() != 0 {
            effects.notifications.push(Notification::Next(downstream_changes));
        }
    }

    /// `TrySetExpiration`: whether the due time of the key has changed.
    fn try_set_expiration(&mut self, key: K, due_time: SystemTime) -> bool {
        // The due time is stored whether it changes or not: the item may have.
        let old_due_time = self.expiration_due_times_by_key.insert(key.clone(), due_time);
        if old_due_time == Some(due_time) {
            return false;
        }
        let insertion_index = dotnet_binary_search(&self.proposed_expirations_queue, &due_time, |expiration, due_time| expiration.due_time.cmp(due_time)).unwrap_or_else(|index| index);
        // The old expiration of the item, if any, stays (see `clear_expiration`).
        self.proposed_expirations_queue.insert(insertion_index, ProposedExpiration { due_time, key });
        true
    }
}

fn run<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    state: &Shared<T, K>,
    observer: &Rc<dyn IObserver<ChangeSet<T, K>>>,
    step: impl FnOnce(&mut Subscription<T, K>, &mut Effects<ChangeSet<T, K>>),
) {
    let mut effects = Effects::default();
    let scheduler = {
        let mut state = state.borrow_mut();
        step(&mut state, &mut effects);
        state.scheduler.clone()
    };
    let manage = manage_action(Rc::downgrade(state), observer.clone());
    effects.apply(&scheduler, observer, &manage);
}

/// The work of a scheduled management; the scheduler holds the subscription weakly, as upstream's
/// `WeakReference` does, so that a cancelled management that a scheduler keeps does not keep it.
fn manage_action<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(state: Weak<RefCell<Subscription<T, K>>>, observer: Rc<dyn IObserver<ChangeSet<T, K>>>) -> Rc<dyn Fn()> {
    let observer = Rc::downgrade(&observer);
    Rc::new(move || {
        if let (Some(state), Some(observer)) = (state.upgrade(), observer.upgrade()) {
            run(&state, &observer, |state, effects| state.manage_expirations(effects));
        }
    })
}

/// `ExpireAfter.ForStream.Create`.
pub(crate) fn expire_after_for_stream<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    time_selector: Rc<dyn Fn(&T) -> Option<Duration>>,
    polling_interval: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let scheduler = scheduler.clone().unwrap_or_else(SchedulerDefaults::time_based_operations);
        let mode = match polling_interval {
            Some(interval) => ManagementMode::Polling { interval, last_management_due_time: None },
            None => ManagementMode::OnDemand,
        };
        let state: Shared<T, K> = Rc::new(RefCell::new(Subscription {
            expiration_due_times_by_key: HashMap::new(),
            items_cache: ChangeAwareCache::new(),
            proposed_expirations_queue: Vec::new(),
            scheduler: scheduler.clone(),
            time_selector: time_selector.clone(),
            has_source_completed: false,
            next_scheduled_management: None,
            mode,
        }));

        let source_subscription = {
            let (next_state, next_observer) = (state.clone(), observer.clone());
            let (error_state, error_observer) = (state.clone(), observer.clone());
            let (completed_state, completed_observer) = (state.clone(), observer.clone());
            source.subscribe_all(
                move |changes: ChangeSet<T, K>| run(&next_state, &next_observer, |state, effects| state.on_source_next(&changes, effects)),
                move |error| {
                    run(&error_state, &error_observer, |state, effects| {
                        effects.cancel(&mut state.next_scheduled_management);
                        effects.notifications.push(Notification::Error(error));
                    })
                },
                move || run(&completed_state, &completed_observer, |state, effects| state.on_source_completed(effects)),
            )
        };
        state.borrow_mut().mode.start(scheduler.now());

        // The scheduled managements hold the observer weakly; the subscription keeps it.
        let keep_observer = observer.clone();
        Disposable::create(move || {
            source_subscription.dispose();
            let next_scheduled_management = state.borrow_mut().next_scheduled_management.take();
            if let Some(management) = next_scheduled_management {
                management.cancellation.dispose();
            }
            drop(keep_observer);
        })
    })
}
