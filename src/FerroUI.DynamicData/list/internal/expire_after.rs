//! `ExpireAfter<T>` (`List/Internal/ExpireAfter.cs`): the items of a source list, each removed from
//! it once the time that a selector gives for it has passed.
//!
//! The state is released before the operator notifies, schedules or cancels, as for caches
//! (`cache::internal::expire_after`).

use crate::cache::internal::{Effects, ManagementMode, Notification, ScheduledManagement};
use crate::list::{ChangeSet, IExtendedList, ISourceList, ListChangeReason};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

struct Subscription<T, S> {
    expiration_due_times: Vec<Option<SystemTime>>,
    scheduler: Rc<dyn IScheduler>,
    source: S,
    time_selector: Rc<dyn Fn(&T) -> Option<Duration>>,
    next_scheduled_management: Option<ScheduledManagement>,
    mode: ManagementMode,
    _items: PhantomData<fn(T)>,
}

impl<T: Clone + PartialEq + 'static, S> Subscription<T, S> {
    /// `GetNextProposedExpirationDueTime`: the earliest due time.
    fn next_proposed_expiration_due_time(&self) -> Option<SystemTime> {
        self.expiration_due_times.iter().flatten().min().copied()
    }

    fn next_management_due_time(&self) -> Option<SystemTime> {
        self.mode.next_management_due_time(self.scheduler.now(), self.next_proposed_expiration_due_time())
    }

    /// `OnEditingSource`: the expired items are removed by `updater`, from the last.
    fn on_editing_source(&mut self, updater: &mut dyn IExtendedList<T>, effects: &mut Effects<Vec<T>>) {
        let Some(this_scheduled_management) = self.next_scheduled_management.take() else { return };
        let now = self.scheduler.now();

        // The due times stay in place until the removals come back as changes of the source.
        let mut expiring_indexes_buffer = Vec::new();
        for (index, due_time) in self.expiration_due_times.iter_mut().enumerate() {
            if due_time.is_some_and(|due_time| due_time <= now) {
                expiring_indexes_buffer.push(index);
                *due_time = None;
            }
        }

        if !expiring_indexes_buffer.is_empty() {
            expiring_indexes_buffer.sort_by(|x, y| y.cmp(x));
            let mut removed_items = Vec::with_capacity(expiring_indexes_buffer.len());
            for removed_index in expiring_indexes_buffer {
                removed_items.push(updater.get(removed_index));
                updater.remove_at(removed_index);
            }
            effects.notifications.push(Notification::Next(removed_items));
        }

        self.mode.on_expirations_managed(this_scheduled_management.due_time);
        self.on_expiration_due_times_changed(effects);
    }

    /// `OnExpirationDueTimesChanged`.
    fn on_expiration_due_times_changed(&mut self, effects: &mut Effects<Vec<T>>) {
        match self.next_management_due_time() {
            Some(next_management_due_time) => {
                if self.next_scheduled_management.as_ref().map(|management| management.due_time) != Some(next_management_due_time) {
                    effects.reschedule(&mut self.next_scheduled_management, next_management_due_time);
                }
            }
            None => effects.cancel(&mut self.next_scheduled_management),
        }
    }

    /// `OnSourceNext`: the due times follow the positions of the items.
    fn on_source_next(&mut self, changes: &ChangeSet<T>, effects: &mut Effects<Vec<T>>) {
        let now = self.scheduler.now();
        let mut have_expiration_due_times_changed = false;
        for change in changes {
            match change.reason() {
                ListChangeReason::Add => {
                    let item = change.item();
                    let due_time = (self.time_selector)(item.current()).map(|lifetime| now + lifetime);
                    let index = item.current_index().unwrap_or(self.expiration_due_times.len());
                    self.expiration_due_times.insert(index, due_time);
                    have_expiration_due_times_changed |= due_time.is_some();
                }
                ListChangeReason::AddRange => {
                    let range = change.range();
                    self.expiration_due_times.reserve(range.count());
                    let mut item_index = range.index().unwrap_or(self.expiration_due_times.len());
                    for item in range.iter() {
                        let due_time = (self.time_selector)(item).map(|lifetime| now + lifetime);
                        self.expiration_due_times.insert(item_index, due_time);
                        have_expiration_due_times_changed |= due_time.is_some();
                        item_index += 1;
                    }
                }
                ListChangeReason::Clear => {
                    if self.expiration_due_times.iter().any(Option::is_some) {
                        have_expiration_due_times_changed = true;
                    }
                    self.expiration_due_times.clear();
                }
                ListChangeReason::Moved => {
                    let item = change.item();
                    let previous_index = item.previous_index().expect("a move has its previous index");
                    let expiration_due_time = self.expiration_due_times.remove(previous_index);
                    self.expiration_due_times.insert(item.current_index().expect("a move has its index"), expiration_due_time);
                }
                ListChangeReason::Remove => {
                    let index = change.item().current_index().expect("the removal of an item of a source list has its index");
                    if self.expiration_due_times[index].is_some() {
                        have_expiration_due_times_changed = true;
                    }
                    self.expiration_due_times.remove(index);
                }
                ListChangeReason::RemoveRange => {
                    let range = change.range();
                    let index = range.index().expect("the removal of a range of a source list has its index");
                    if self.expiration_due_times[index..index + range.count()].iter().any(Option::is_some) {
                        have_expiration_due_times_changed = true;
                    }
                    self.expiration_due_times.drain(index..index + range.count());
                }
                ListChangeReason::Replace => {
                    let item = change.item();
                    let index = item.current_index().expect("the replacement of an item of a source list has its index");
                    let old_due_time = self.expiration_due_times[index];
                    let new_due_time = (self.time_selector)(item.current()).map(|lifetime| now + lifetime);
                    self.expiration_due_times[index] = new_due_time;
                    have_expiration_due_times_changed |= new_due_time != old_due_time;
                }
                ListChangeReason::Refresh => {}
            }
        }
        if have_expiration_due_times_changed {
            self.on_expiration_due_times_changed(effects);
        }
    }
}

struct Context<T: 'static, S> {
    state: RefCell<Subscription<T, S>>,
    has_source_completed: Cell<bool>,
    this: Weak<Context<T, S>>,
}

impl<T: Clone + PartialEq + 'static, S: ISourceList<T> + Clone + 'static> Context<T, S> {
    fn run(&self, observer: &Rc<dyn IObserver<Vec<T>>>, step: impl FnOnce(&mut Subscription<T, S>, &mut Effects<Vec<T>>)) {
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
    fn manage_action(&self, observer: &Rc<dyn IObserver<Vec<T>>>) -> Rc<dyn Fn()> {
        let (this, observer) = (self.this.clone(), Rc::downgrade(observer));
        Rc::new(move || {
            let (Some(this), Some(observer)) = (this.upgrade(), observer.upgrade()) else { return };
            if this.has_source_completed.get() {
                return;
            }
            let source = this.state.borrow().source.clone();
            source.edit_dyn(&mut |updater: &mut dyn IExtendedList<T>| this.run(&observer, |state, effects| state.on_editing_source(updater, effects)));
        })
    }
}

/// `ExpireAfter.Create`: the removed items each time some expire.
pub(crate) fn expire_after<T: Clone + PartialEq + 'static, S: ISourceList<T> + Clone + 'static>(
    source: S,
    time_selector: Rc<dyn Fn(&T) -> Option<Duration>>,
    polling_interval: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<Vec<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<Vec<T>>>| {
        let scheduler = scheduler.clone().unwrap_or_else(SchedulerDefaults::time_based_operations);
        let mode = match polling_interval {
            Some(interval) => ManagementMode::Polling { interval, last_management_due_time: None },
            None => ManagementMode::OnDemand,
        };
        let context = Rc::new_cyclic(|this| Context {
            state: RefCell::new(Subscription {
                expiration_due_times: Vec::new(),
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
                move |changes: ChangeSet<T>| next_context.run(&next_observer, |state, effects| state.on_source_next(&changes, effects)),
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
