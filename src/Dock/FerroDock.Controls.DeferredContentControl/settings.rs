// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{DeferredContentPresentationBudgetMode, DeferredContentPresentationTimeline};
use ferroui_base::animation::TimeSpan;
use std::rc::Rc;
ferroui_base::ferro_thread_local! {
    static DEFAULT_TIMELINE:Rc<DeferredContentPresentationTimeline>=DeferredContentPresentationTimeline::new();
}
/// Configures the shared default deferred presentation timeline.
pub struct DeferredContentPresentationSettings;
impl DeferredContentPresentationSettings {
    /// Gets the native UI thread's shared default presentation timeline.
    pub fn default_timeline() -> Rc<DeferredContentPresentationTimeline> {
        DEFAULT_TIMELINE.with(Clone::clone)
    }
    /// Gets the default timeline budget mode.
    pub fn budget_mode() -> DeferredContentPresentationBudgetMode {
        Self::default_timeline().budget_mode()
    }
    /// Sets the default timeline budget mode.
    pub fn set_budget_mode(value: DeferredContentPresentationBudgetMode) {
        Self::default_timeline().set_budget_mode(value);
    }
    /// Gets the default item-count limit.
    pub fn max_presentations_per_pass() -> i32 {
        Self::default_timeline().max_presentations_per_pass()
    }
    /// Sets the default item-count limit, clamping it to one.
    pub fn set_max_presentations_per_pass(value: i32) {
        Self::default_timeline().set_max_presentations_per_pass(value);
    }
    /// Gets the default realization-time budget.
    pub fn max_realization_time_per_pass() -> TimeSpan {
        Self::default_timeline().max_realization_time_per_pass()
    }
    /// Sets the default realization-time budget, clamping negative values to zero.
    pub fn set_max_realization_time_per_pass(value: TimeSpan) {
        Self::default_timeline().set_max_realization_time_per_pass(value);
    }
    /// Gets the default initial delay.
    pub fn initial_delay() -> TimeSpan {
        Self::default_timeline().initial_delay()
    }
    /// Sets the default initial delay, clamping negative values to zero.
    pub fn set_initial_delay(value: TimeSpan) {
        Self::default_timeline().set_initial_delay(value);
    }
    /// Gets the default follow-up delay.
    pub fn follow_up_delay() -> TimeSpan {
        Self::default_timeline().follow_up_delay()
    }
    /// Sets the default follow-up delay, clamping negative values to zero.
    pub fn set_follow_up_delay(value: TimeSpan) {
        Self::default_timeline().set_follow_up_delay(value);
    }
    /// Gets the default reveal duration.
    pub fn reveal_duration() -> TimeSpan {
        Self::default_timeline().reveal_duration()
    }
    /// Sets the default reveal duration, clamping negative values to zero.
    pub fn set_reveal_duration(value: TimeSpan) {
        Self::default_timeline().set_reveal_duration(value);
    }
}
ferroui_base::ferro_markup_type!(static DeferredContentPresentationSettings {
    static_properties:[
        DefaultTimeline:Rc<DeferredContentPresentationTimeline>{get:DeferredContentPresentationSettings::default_timeline},
        BudgetMode:DeferredContentPresentationBudgetMode{get:DeferredContentPresentationSettings::budget_mode,set:DeferredContentPresentationSettings::set_budget_mode},
        MaxPresentationsPerPass:i32{get:DeferredContentPresentationSettings::max_presentations_per_pass,set:DeferredContentPresentationSettings::set_max_presentations_per_pass},
        MaxRealizationTimePerPass:TimeSpan{get:DeferredContentPresentationSettings::max_realization_time_per_pass,set:DeferredContentPresentationSettings::set_max_realization_time_per_pass},
        InitialDelay:TimeSpan{get:DeferredContentPresentationSettings::initial_delay,set:DeferredContentPresentationSettings::set_initial_delay},
        FollowUpDelay:TimeSpan{get:DeferredContentPresentationSettings::follow_up_delay,set:DeferredContentPresentationSettings::set_follow_up_delay},
        RevealDuration:TimeSpan{get:DeferredContentPresentationSettings::reveal_duration,set:DeferredContentPresentationSettings::set_reveal_duration}
    ]
});
