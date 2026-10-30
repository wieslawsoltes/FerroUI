// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::presentation::IDeferredContentPresentationTarget;
use crate::timeline_queue::TimelineQueue;
use ferroui_base::animation::TimeSpan;
use std::cell::Cell;
use std::rc::{Rc, Weak};
/// Determines which budget limits a timeline dispatcher pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DeferredContentPresentationBudgetMode {
    /// Count successful target realizations in a pass.
    #[default]
    ItemCount = 0,
    /// Limit elapsed realization time in a pass.
    RealizationTime = 1,
}
ferroui_base::ferro_markup_enum!(DeferredContentPresentationBudgetMode {
    ItemCount,
    RealizationTime
});
/// A reusable source presentation timeline shared by one or more deferred hosts.
pub struct DeferredContentPresentationTimeline {
    queue: TimelineQueue,
    budget_mode: Cell<DeferredContentPresentationBudgetMode>,
    max_presentations_per_pass: Cell<i32>,
    max_realization_time_per_pass: Cell<TimeSpan>,
    initial_delay: Cell<TimeSpan>,
    follow_up_delay: Cell<TimeSpan>,
    reveal_duration: Cell<TimeSpan>,
}
impl PartialEq for DeferredContentPresentationTimeline {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DeferredContentPresentationTimeline {
    /// Creates a timeline with the source defaults: eight items, 10ms realization,
    /// no initial delay, 1ms follow-up delay and 90ms reveal duration.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| Self {
            queue: TimelineQueue::new(this.clone()),
            budget_mode: Cell::new(DeferredContentPresentationBudgetMode::ItemCount),
            max_presentations_per_pass: Cell::new(8),
            max_realization_time_per_pass: Cell::new(TimeSpan::from_milliseconds(10.0)),
            initial_delay: Cell::new(TimeSpan::ZERO),
            follow_up_delay: Cell::new(TimeSpan::from_milliseconds(1.0)),
            reveal_duration: Cell::new(TimeSpan::from_milliseconds(90.0)),
        })
    }
    /// Gets the active dispatcher-pass budget mode.
    pub fn budget_mode(&self) -> DeferredContentPresentationBudgetMode {
        self.budget_mode.get()
    }
    /// Sets the active dispatcher-pass budget mode.
    pub fn set_budget_mode(&self, value: DeferredContentPresentationBudgetMode) {
        self.budget_mode.set(value);
    }
    /// Gets the maximum successful realizations in an item-count pass.
    pub fn max_presentations_per_pass(&self) -> i32 {
        self.max_presentations_per_pass.get()
    }
    /// Sets the item limit, clamping zero and negative values to one.
    pub fn set_max_presentations_per_pass(&self, value: i32) {
        self.max_presentations_per_pass.set(value.max(1));
    }
    /// Gets the realization-time budget for one pass.
    pub fn max_realization_time_per_pass(&self) -> TimeSpan {
        self.max_realization_time_per_pass.get()
    }
    /// Sets the realization-time budget, clamping negative intervals to zero.
    pub fn set_max_realization_time_per_pass(&self, value: TimeSpan) {
        self.max_realization_time_per_pass
            .set(value.max(TimeSpan::ZERO));
    }
    /// Gets the delay applied when a target joins the timeline.
    pub fn initial_delay(&self) -> TimeSpan {
        self.initial_delay.get()
    }
    /// Sets the initial delay, clamping negative intervals to zero.
    pub fn set_initial_delay(&self, value: TimeSpan) {
        self.initial_delay.set(value.max(TimeSpan::ZERO));
    }
    /// Gets the delay between due follow-up passes.
    pub fn follow_up_delay(&self) -> TimeSpan {
        self.follow_up_delay.get()
    }
    /// Sets the follow-up delay, clamping negative intervals to zero.
    pub fn set_follow_up_delay(&self, value: TimeSpan) {
        self.follow_up_delay.set(value.max(TimeSpan::ZERO));
    }
    /// Gets the reveal-animation duration.
    pub fn reveal_duration(&self) -> TimeSpan {
        self.reveal_duration.get()
    }
    /// Sets the reveal duration, clamping negative intervals to zero.
    pub fn set_reveal_duration(&self, value: TimeSpan) {
        self.reveal_duration.set(value.max(TimeSpan::ZERO));
    }
    pub(crate) fn queue(&self) -> &TimelineQueue {
        &self.queue
    }
    #[cfg(test)]
    pub(crate) fn auto_schedule(&self) -> bool {
        self.queue.auto_schedule()
    }
    #[cfg(test)]
    pub(crate) fn set_auto_schedule(&self, value: bool) {
        self.queue.set_auto_schedule(value);
    }
    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.queue.pending_count()
    }
    pub(crate) fn enqueue(
        &self,
        target: Rc<dyn IDeferredContentPresentationTarget>,
        delay: TimeSpan,
        order: i32,
    ) {
        self.queue.enqueue(target, delay, order);
    }
    pub(crate) fn remove(&self, target: &Rc<dyn IDeferredContentPresentationTarget>) {
        self.queue.remove(target);
    }
    #[cfg(test)]
    pub(crate) fn flush_pending_batch_for_testing(&self) {
        self.queue.flush_pending_batch_for_testing();
    }
}
ferroui_base::ferro_markup_type!(class DeferredContentPresentationTimeline {
    this: Rc<DeferredContentPresentationTimeline>,
    handles:[Rc<DeferredContentPresentationTimeline>,Option<Rc<DeferredContentPresentationTimeline>>],
    constructors:[()=>DeferredContentPresentationTimeline::new],
    properties:[
        BudgetMode:DeferredContentPresentationBudgetMode {get:|o:&Rc<DeferredContentPresentationTimeline>|o.budget_mode(),set:|o:&Rc<DeferredContentPresentationTimeline>,v:DeferredContentPresentationBudgetMode|o.set_budget_mode(v)},
        MaxPresentationsPerPass:i32 {get:|o:&Rc<DeferredContentPresentationTimeline>|o.max_presentations_per_pass(),set:|o:&Rc<DeferredContentPresentationTimeline>,v:i32|o.set_max_presentations_per_pass(v)},
        MaxRealizationTimePerPass:TimeSpan {get:|o:&Rc<DeferredContentPresentationTimeline>|o.max_realization_time_per_pass(),set:|o:&Rc<DeferredContentPresentationTimeline>,v:TimeSpan|o.set_max_realization_time_per_pass(v)},
        InitialDelay:TimeSpan {get:|o:&Rc<DeferredContentPresentationTimeline>|o.initial_delay(),set:|o:&Rc<DeferredContentPresentationTimeline>,v:TimeSpan|o.set_initial_delay(v)},
        FollowUpDelay:TimeSpan {get:|o:&Rc<DeferredContentPresentationTimeline>|o.follow_up_delay(),set:|o:&Rc<DeferredContentPresentationTimeline>,v:TimeSpan|o.set_follow_up_delay(v)},
        RevealDuration:TimeSpan {get:|o:&Rc<DeferredContentPresentationTimeline>|o.reveal_duration(),set:|o:&Rc<DeferredContentPresentationTimeline>,v:TimeSpan|o.set_reveal_duration(v)}
    ]
});
