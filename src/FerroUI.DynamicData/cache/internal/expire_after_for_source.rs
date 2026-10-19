//! `ExpireAfter.ForSource<TObject, TKey>` (`Cache/Internal/ExpireAfter.ForSource.cs`): the items
//! of a source cache, each removed from it once the time that a selector gives for it has passed.

use super::expire_after::{Effects, ManagementMode, Notification, ProposedExpiration, ScheduledManagement};
use crate::cache::{ChangeReason, ChangeSet, ISourceCache, ISourceUpdater};
use crate::kernel::dotnet_binary_search;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

struct Subscription<T, K, S> {
    expiration_due_times_by_key: HashMap<K, SystemTime>,
    proposed_expirations_queue: Vec<ProposedExpiration<K>>,
    scheduler: Rc<dyn IScheduler>,
    source: S,
    time_selector: Rc<dyn Fn(&T) -> Option<Duration>>,
    next_scheduled_management: Option<ScheduledManagement>,
    mode: ManagementMode,
    _items: PhantomData<fn(T)>,
}

type Removed<T, K> = Vec<(K, T)>;

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S> Subscription<T, K, S> {
    fn next_management_due_time(&self) -> Option<SystemTime> {
        let next_proposed = self.proposed_expirations_queue.first().map(|expiration| expiration.due_time);
        self.mode.next_management_due_time(self.scheduler.now(), next_proposed)
    }

    /// `ClearExpiration`.
    fn clear_expiration(&mut self, key: &K) {
        self.expiration_due_times_by_key.remove(key);
    }

    /// `OnEditingSource`: the expired items are removed from the source by `updater`.
    fn on_editing_source(&mut self, updater: &mut dyn ISourceUpdater<T, K>, effects: &mut Effects<Removed<T, K>>) {
        let Some(this_scheduled_management) = self.next_scheduled_management.take() else { return };
        let now = self.scheduler.now();

        let mut removed_items_buffer = Vec::new();
        let mut proposed_expiration_index = 0;
        while proposed_expiration_index < self.proposed_expirations_queue.len() {
            let proposed_expiration = &self.proposed_expirations_queue[proposed_expiration_index];
            if proposed_expiration.due_time > now {
                break;
            }
            let key = proposed_expiration.key.clone();
            if self.expiration_due_times_by_key.get(&key).is_some_and(|due_time| *due_time <= now) {
                self.expiration_due_times_by_key.remove(&key);
                if let Some(item) = updater.lookup(&key) {
                    if (self.time_selector)(&item).is_some() {
                        updater.remove_key(&key);
                        removed_items_buffer.push((key, item));
                    }
                }
            }
            proposed_expiration_index += 1;
        }
        self.proposed_expirations_queue.drain(0..proposed_expiration_index);

        if !removed_items_buffer.is_empty() {
            effects.notifications.push(Notification::Next(removed_items_buffer));
        }
        self.mode.on_expirations_managed(this_scheduled_management.due_time);
        self.on_expirations_changed(effects);
    }

    /// `OnExpirationsChanged`.
    fn on_expirations_changed(&mut self, effects: &mut Effects<Removed<T, K>>) {
        let remove_to_index = self.proposed_expirations_queue.iter().position(|expiration| self.expiration_due_times_by_key.contains_key(&expiration.key));
        if let Some(remove_to_index) = remove_to_index.filter(|index| *index > 0) {
            self.proposed_expirations_queue.drain(0..remove_to_index);
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

    /// `OnSourceNext`.
    fn on_source_next(&mut self, changes: &ChangeSet<T, K>, effects: &mut Effects<Removed<T, K>>) {
        let now = self.scheduler.now();
        let mut have_expirations_changed = false;
        for change in changes {
            match change.reason() {
                ChangeReason::Add => {
                    if let Some(expire_after) = (self.time_selector)(change.current()) {
                        have_expirations_changed |= self.try_set_expiration(change.key().clone(), now + expire_after);
                    }
                }
                ChangeReason::Remove => {
                    self.clear_expiration(change.key());
                    have_expirations_changed = true;
                }
                ChangeReason::Update => match (self.time_selector)(change.current()) {
                    Some(expire_after) => have_expirations_changed |= self.try_set_expiration(change.key().clone(), now + expire_after),
                    None => {
                        self.clear_expiration(change.key());
                        have_expirations_changed = true;
                    }
                },
                ChangeReason::Refresh | ChangeReason::Moved => {}
            }
        }
        if have_expirations_changed {
            self.on_expirations_changed(effects);
        }
    }

    /// `TrySetExpiration`.
    fn try_set_expiration(&mut self, key: K, due_time: SystemTime) -> bool {
        let old_due_time = self.expiration_due_times_by_key.insert(key.clone(), due_time);
        if old_due_time == Some(due_time) {
            return false;
        }
        let insertion_index = dotnet_binary_search(&self.proposed_expirations_queue, &due_time, |expiration, due_time| expiration.due_time.cmp(due_time)).unwrap_or_else(|index| index);
        self.proposed_expirations_queue.insert(insertion_index, ProposedExpiration { due_time, key });
        true
    }
}

struct Context<T: 'static, K: 'static, S> {
    state: RefCell<Subscription<T, K, S>>,
    has_source_completed: Cell<bool>,
    this: Weak<Context<T, K, S>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S: ISourceCache<T, K> + Clone + 'static> Context<T, K, S> {
    fn run(&self, observer: &Rc<dyn IObserver<Removed<T, K>>>, step: impl FnOnce(&mut Subscription<T, K, S>, &mut Effects<Removed<T, K>>)) {
        let mut effects = Effects::default();
        let scheduler = {
            let mut state = self.state.borrow_mut();
            step(&mut state, &mut effects);
            state.scheduler.clone()
        };
        let manage = self.manage_action(observer);
        effects.apply(&scheduler, observer, &manage);
    }

    /// The work of a scheduled management (`ManageExpirations`), which holds the subscription
    /// weakly as upstream's `WeakReference` does.
    fn manage_action(&self, observer: &Rc<dyn IObserver<Removed<T, K>>>) -> Rc<dyn Fn()> {
        let (this, observer) = (self.this.clone(), Rc::downgrade(observer));
        Rc::new(move || {
            let (Some(this), Some(observer)) = (this.upgrade(), observer.upgrade()) else { return };
            if this.has_source_completed.get() {
                return;
            }
            let source = this.state.borrow().source.clone();
            source.edit(&mut |updater: &mut dyn ISourceUpdater<T, K>| this.run(&observer, |state, effects| state.on_editing_source(updater, effects)));
        })
    }
}

/// `ExpireAfter.ForSource.Create`: the removed items, with their keys, each time some expire.
pub(crate) fn expire_after_for_source<T: Clone + 'static, K: Eq + Hash + Clone + 'static, S: ISourceCache<T, K> + Clone + 'static>(
    source: S,
    time_selector: Rc<dyn Fn(&T) -> Option<Duration>>,
    polling_interval: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<Removed<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<Removed<T, K>>>| {
        let scheduler = scheduler.clone().unwrap_or_else(SchedulerDefaults::time_based_operations);
        let mode = match polling_interval {
            Some(interval) => ManagementMode::Polling { interval, last_management_due_time: None },
            None => ManagementMode::OnDemand,
        };
        let context = Rc::new_cyclic(|this| Context {
            state: RefCell::new(Subscription {
                expiration_due_times_by_key: HashMap::new(),
                proposed_expirations_queue: Vec::new(),
                scheduler: scheduler.clone(),
                source: source.clone(),
                time_selector: time_selector.clone(),
                next_scheduled_management: None,
                mode,
                _items: PhantomData,
            }),
            has_source_completed: Cell::new(false),
            this: this.clone(),
        });

        let source_subscription = {
            let completed_flag = context.clone();
            let (next_context, next_observer) = (context.clone(), observer.clone());
            let (error_context, error_observer) = (context.clone(), observer.clone());
            let (completed_context, completed_observer) = (context.clone(), observer.clone());
            source.connect().finally(move || completed_flag.has_source_completed.set(true)).subscribe_all(
                move |changes: ChangeSet<T, K>| next_context.run(&next_observer, |state, effects| state.on_source_next(&changes, effects)),
                move |error| {
                    error_context.run(&error_observer, |state, effects| {
                        effects.cancel(&mut state.next_scheduled_management);
                        effects.notifications.push(Notification::Error(error));
                    })
                },
                move || {
                    completed_context.run(&completed_observer, |state, effects| {
                        effects.cancel(&mut state.next_scheduled_management);
                        effects.notifications.push(Notification::Completed);
                    })
                },
            )
        };
        context.state.borrow_mut().mode.start(scheduler.now());

        Disposable::create(move || {
            source_subscription.dispose();
            let next_scheduled_management = context.state.borrow_mut().next_scheduled_management.take();
            if let Some(management) = next_scheduled_management {
                management.cancellation.dispose();
            }
        })
    })
}
