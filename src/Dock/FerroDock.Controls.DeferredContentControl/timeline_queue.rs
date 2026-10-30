// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::presentation::IDeferredContentPresentationTarget;
use crate::{DeferredContentPresentationBudgetMode, DeferredContentPresentationTimeline};
use ferroui_base::animation::TimeSpan;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::{Instant, SystemTime};
struct PendingEntry {
    target: Rc<dyn IDeferredContentPresentationTarget>,
    due_at: Cell<SystemTime>,
    order: Cell<i32>,
    sequence: Cell<i64>,
}
pub(crate) struct TimelineQueue {
    timeline: Weak<DeferredContentPresentationTimeline>,
    pending: RefCell<HashMap<usize, Rc<PendingEntry>>>,
    due_entries: RefCell<Vec<Rc<PendingEntry>>>,
    due_version: Cell<u64>,
    is_scheduled: Cell<bool>,
    scheduled_due_at: Cell<Option<SystemTime>>,
    next_sequence: Cell<i64>,
    schedule_version: Cell<i64>,
    auto_schedule: Cell<bool>,
}
impl TimelineQueue {
    pub(crate) fn new(timeline: Weak<DeferredContentPresentationTimeline>) -> Self {
        Self {
            timeline,
            pending: RefCell::new(HashMap::new()),
            due_entries: RefCell::new(Vec::new()),
            due_version: Cell::new(0),
            is_scheduled: Cell::new(false),
            scheduled_due_at: Cell::new(None),
            next_sequence: Cell::new(0),
            schedule_version: Cell::new(0),
            auto_schedule: Cell::new(true),
        }
    }
    fn timeline(&self) -> Rc<DeferredContentPresentationTimeline> {
        self.timeline
            .upgrade()
            .expect("queue belongs to a live timeline")
    }
    pub(crate) fn auto_schedule(&self) -> bool {
        self.auto_schedule.get()
    }
    #[cfg(test)]
    pub(crate) fn set_auto_schedule(&self, value: bool) {
        self.auto_schedule.set(value);
    }
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.borrow().len()
    }
    pub(crate) fn enqueue(
        &self,
        target: Rc<dyn IDeferredContentPresentationTarget>,
        delay: TimeSpan,
        order: i32,
    ) {
        Dispatcher::ui_thread().verify_access();
        let timeline = self.timeline();
        let due_at = SystemTime::now()
            + normalize(timeline.initial_delay() + delay)
                .to_duration()
                .expect("normalized delay");
        let sequence = self.next_sequence.get().wrapping_add(1);
        self.next_sequence.set(sequence);
        let key = key(&target);
        {
            let mut pending = self.pending.borrow_mut();
            if let Some(entry) = pending.get(&key) {
                entry.due_at.set(due_at);
                entry.order.set(order);
                entry.sequence.set(sequence);
            } else {
                pending.insert(
                    key,
                    Rc::new(PendingEntry {
                        target: target.clone(),
                        due_at: Cell::new(due_at),
                        order: Cell::new(order),
                        sequence: Cell::new(sequence),
                    }),
                );
            }
        }
        target.set_enqueued_timeline(Some(timeline));
        if self.auto_schedule() {
            self.schedule_next_pending(SystemTime::now());
        }
    }
    pub(crate) fn remove(&self, target: &Rc<dyn IDeferredContentPresentationTarget>) {
        Dispatcher::ui_thread().verify_access();
        self.remove_pending(target);
        if self.pending_count() == 0 {
            self.cancel_scheduling();
        }
    }
    #[cfg(test)]
    pub(crate) fn flush_pending_batch_for_testing(&self) {
        self.flush_scheduled_batch(self.schedule_version.get());
    }
    fn clear_due_entries(&self) {
        self.due_entries.borrow_mut().clear();
        self.due_version.set(self.due_version.get().wrapping_add(1));
    }
    fn flush_scheduled_batch(&self, scheduled_version: i64) {
        Dispatcher::ui_thread().verify_access();
        if scheduled_version != self.schedule_version.get() {
            return;
        }
        self.is_scheduled.set(false);
        self.scheduled_due_at.set(None);
        if self.pending_count() == 0 {
            self.cancel_scheduling();
            return;
        }
        let now = SystemTime::now();
        self.clear_due_entries();
        {
            let mut due = self.due_entries.borrow_mut();
            for entry in self.pending.borrow().values() {
                if entry.due_at.get() <= now {
                    due.push(entry.clone());
                }
            }
        }
        if self.due_entries.borrow().is_empty() {
            if self.auto_schedule() {
                self.schedule_next_pending(now);
            }
            return;
        }
        self.due_entries.borrow_mut().sort_by(|a, b| {
            a.order
                .get()
                .cmp(&b.order.get())
                .then_with(|| a.sequence.get().cmp(&b.sequence.get()))
        });
        let timeline = self.timeline();
        let batch_size = usize::try_from(timeline.max_presentations_per_pass().max(1))
            .expect("positive item limit")
            .min(self.due_entries.borrow().len());
        let stopwatch = (timeline.budget_mode()
            == DeferredContentPresentationBudgetMode::RealizationTime)
            .then(Instant::now);
        let version = self.due_version.get();
        let mut completed_count = 0;
        let mut index = 0;
        loop {
            assert_eq!(
                version,
                self.due_version.get(),
                "source due-entry enumeration changed during callback"
            );
            let entry = self.due_entries.borrow().get(index).cloned();
            let Some(entry) = entry else {
                break;
            };
            index += 1;
            let current = self.pending.borrow().get(&key(&entry.target)).cloned();
            if !current.is_some_and(|current| Rc::ptr_eq(&current, &entry)) {
                continue;
            }
            if entry.target.apply_deferred_presentation() {
                self.remove_pending(&entry.target);
                completed_count += 1;
            } else if !entry.target.retain_pending_presentation_on_failure() {
                self.remove_pending(&entry.target);
            }
            let should_continue = match stopwatch {
                None => completed_count < batch_size,
                Some(start) => {
                    TimeSpan::from(start.elapsed()) < timeline.max_realization_time_per_pass()
                }
            };
            if !should_continue {
                break;
            }
        }
        self.clear_due_entries();
        if self.pending_count() == 0 {
            self.cancel_scheduling();
            return;
        }
        if !self.auto_schedule() {
            return;
        }
        let now = SystemTime::now();
        let has_due = self
            .pending
            .borrow()
            .values()
            .any(|entry| entry.due_at.get() <= now);
        self.schedule_flush(if has_due {
            timeline.follow_up_delay()
        } else {
            self.earliest_due_delay(now)
        });
    }
    fn schedule_next_pending(&self, now: SystemTime) {
        if self.pending_count() == 0 {
            self.cancel_scheduling();
            return;
        }
        self.schedule_flush(self.earliest_due_delay(now));
    }
    fn schedule_flush(&self, delay: TimeSpan) {
        let delay = normalize(delay);
        let due_at = SystemTime::now() + delay.to_duration().expect("normalized delay");
        if self.is_scheduled.get()
            && self
                .scheduled_due_at
                .get()
                .is_some_and(|scheduled| scheduled <= due_at)
        {
            return;
        }
        self.is_scheduled.set(true);
        self.scheduled_due_at.set(Some(due_at));
        let version = self.schedule_version.get().wrapping_add(1);
        self.schedule_version.set(version);
        let timeline = self.timeline();
        if delay == TimeSpan::ZERO {
            Dispatcher::ui_thread().post_local(
                move || timeline.queue().flush_scheduled_batch(version),
                DispatcherPriority::BACKGROUND,
            );
        } else {
            // Native timer wakes once; version checks invalidate superseded callbacks.
            DispatcherTimer::run_once(
                move || {
                    let timeline = timeline.clone();
                    Dispatcher::ui_thread().post_local(
                        move || timeline.queue().flush_scheduled_batch(version),
                        DispatcherPriority::BACKGROUND,
                    );
                },
                delay.to_duration().expect("positive delay"),
                DispatcherPriority::BACKGROUND,
            );
        }
    }
    fn cancel_scheduling(&self) {
        self.is_scheduled.set(false);
        self.scheduled_due_at.set(None);
        self.schedule_version
            .set(self.schedule_version.get().wrapping_add(1));
    }
    fn remove_pending(&self, target: &Rc<dyn IDeferredContentPresentationTarget>) {
        let removed = self.pending.borrow_mut().remove(&key(target));
        if removed.is_none() {
            return;
        }
        if target
            .enqueued_timeline()
            .is_some_and(|timeline| Rc::ptr_eq(&timeline, &self.timeline()))
        {
            target.set_enqueued_timeline(None);
        }
    }
    fn earliest_due_delay(&self, now: SystemTime) -> TimeSpan {
        let earliest = self
            .pending
            .borrow()
            .values()
            .map(|entry| entry.due_at.get())
            .min()
            .expect("nonempty pending queue");
        TimeSpan::from(earliest.duration_since(now).unwrap_or_default())
    }
}
fn key(target: &Rc<dyn IDeferredContentPresentationTarget>) -> usize {
    Rc::as_ptr(target) as *const () as usize
}
fn normalize(delay: TimeSpan) -> TimeSpan {
    delay.max(TimeSpan::ZERO)
}
