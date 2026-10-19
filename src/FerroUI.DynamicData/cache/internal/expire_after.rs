//! `ExpireAfter` (`Cache/Internal/ExpireAfter.cs`): the parts shared by the expiration of the
//! items of a stream (`expire_after_for_stream.rs`) and of a source (`expire_after_for_source.rs`),
//! which the expiration of the items of a source list uses too.
//!
//! Upstream holds a lock while it changes its state and notifies its observer; here the state is
//! in a `RefCell`, and what a step does to the world (the notifications, the schedulings and the
//! cancellations) is collected in [`Effects`] and done once the state is released, so that an
//! observer or a scheduler that runs work at once may come back into the operator.

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::{Disposable, SingleAssignmentDisposable};
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// A notification to deliver once the state is released.
pub(crate) enum Notification<O> {
    Next(O),
    Error(ObservableError),
    Completed,
}

/// The management of the expirations that is scheduled (`ScheduledManagement`).
pub(crate) struct ScheduledManagement {
    pub(crate) cancellation: Rc<SingleAssignmentDisposable>,
    pub(crate) due_time: SystemTime,
}

/// A proposed expiration of the item of a key (`ProposedExpiration`): it is out of date when the
/// due time of the key has changed since.
pub(crate) struct ProposedExpiration<K> {
    pub(crate) due_time: SystemTime,
    pub(crate) key: K,
}

/// When the expirations are managed (`OnDemandSubscription` and `PollingSubscription`).
pub(crate) enum ManagementMode {
    /// At the due time of the next proposed expiration.
    OnDemand,
    /// Every `interval`, from the due time of the last management; `None` before the first, as
    /// upstream's `_lastManagementDueTime` is `default` until its constructor ends.
    Polling { interval: Duration, last_management_due_time: Option<SystemTime> },
}

impl ManagementMode {
    /// `GetNextManagementDueTime`, given the due time of the next proposed expiration.
    pub(crate) fn next_management_due_time(&self, now: SystemTime, next_proposed_expiration_due_time: Option<SystemTime>) -> Option<SystemTime> {
        match self {
            ManagementMode::OnDemand => next_proposed_expiration_due_time,
            ManagementMode::Polling { interval, last_management_due_time } => {
                // Polls that take longer than the interval slow the polling down.
                let next_due_time = last_management_due_time.map(|last| last + *interval);
                match next_due_time {
                    Some(next_due_time) if next_due_time > now => Some(next_due_time),
                    _ => Some(now),
                }
            }
        }
    }

    /// `OnExpirationsManaged`.
    pub(crate) fn on_expirations_managed(&mut self, due_time: SystemTime) {
        if let ManagementMode::Polling { last_management_due_time, .. } = self {
            *last_management_due_time = Some(due_time);
        }
    }

    /// The end of the constructor of `PollingSubscription`: the last management is now.
    pub(crate) fn start(&mut self, now: SystemTime) {
        if let ManagementMode::Polling { last_management_due_time, .. } = self {
            *last_management_due_time = Some(now);
        }
    }
}

/// What a step does once the state is released: cancellations, then schedulings, then
/// notifications, each in order.
pub(crate) struct Effects<O> {
    pub(crate) to_cancel: Vec<Rc<SingleAssignmentDisposable>>,
    pub(crate) to_schedule: Vec<(SystemTime, Rc<SingleAssignmentDisposable>)>,
    pub(crate) notifications: Vec<Notification<O>>,
}

impl<O> Default for Effects<O> {
    fn default() -> Self {
        Self { to_cancel: Vec::new(), to_schedule: Vec::new(), notifications: Vec::new() }
    }
}

impl<O: 'static> Effects<O> {
    /// `TryCancelNextScheduledManagement`.
    pub(crate) fn cancel(&mut self, next_scheduled_management: &mut Option<ScheduledManagement>) {
        if let Some(management) = next_scheduled_management.take() {
            self.to_cancel.push(management.cancellation);
        }
    }

    /// Replaces the scheduled management with one at `due_time`.
    pub(crate) fn reschedule(&mut self, next_scheduled_management: &mut Option<ScheduledManagement>, due_time: SystemTime) {
        self.cancel(next_scheduled_management);
        let cancellation = SingleAssignmentDisposable::new();
        *next_scheduled_management = Some(ScheduledManagement { cancellation: cancellation.clone(), due_time });
        self.to_schedule.push((due_time, cancellation));
    }

    /// Does the effects; a scheduled management runs `manage`.
    pub(crate) fn apply(self, scheduler: &Rc<dyn IScheduler>, observer: &Rc<dyn IObserver<O>>, manage: &Rc<dyn Fn()>) {
        for cancellation in self.to_cancel {
            cancellation.dispose();
        }
        for (due_time, cancellation) in self.to_schedule {
            let manage = manage.clone();
            let scheduled = scheduler.schedule_at(
                due_time,
                Box::new(move |_| {
                    manage();
                    Disposable::empty()
                }),
            );
            cancellation.set_disposable(Some(scheduled));
        }
        for notification in self.notifications {
            match notification {
                Notification::Next(value) => observer.on_next(value),
                Notification::Error(error) => observer.on_error(error),
                Notification::Completed => observer.on_completed(),
            }
        }
    }
}
