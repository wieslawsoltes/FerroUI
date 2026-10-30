// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{DeferredContentPresentationSettings, DeferredContentPresentationTimeline};
use ferroui_base::animation::TimeSpan;
use ferroui_base::{
    ferro_properties, AttachedProperty, FerroObject, FerroProperty, StyledElement,
    StyledPropertyOptions,
};
use std::rc::Rc;
/// Inheritable timeline scope and per-host scheduling metadata.
pub struct DeferredContentScheduling;
ferroui_base::ferro_static_type!(DeferredContentScheduling);
ferroui_base::ferro_markup_type!(static DeferredContentScheduling{type_info:DeferredContentScheduling});
ferro_properties! {impl DeferredContentScheduling {
    /// Identifies the inheritable optional timeline scope.
    pub fn timeline_property()->AttachedProperty<Option<Rc<DeferredContentPresentationTimeline>>>{FerroProperty::register_attached_with::<DeferredContentScheduling,FerroObject,_>("Timeline",StyledPropertyOptions::new(None).inherits(true))}
    /// Identifies the inheritable per-host delay, initially zero.
    pub fn delay_property()->AttachedProperty<TimeSpan>{FerroProperty::register_attached_with::<DeferredContentScheduling,FerroObject,_>("Delay",StyledPropertyOptions::new(TimeSpan::ZERO).inherits(true))}
    /// Identifies the inheritable per-host presentation order, initially zero.
    pub fn order_property()->AttachedProperty<i32>{FerroProperty::register_attached_with::<DeferredContentScheduling,FerroObject,_>("Order",StyledPropertyOptions::new(0).inherits(true))}
}}
impl DeferredContentScheduling {
    /// Gets the optional timeline scope.
    pub fn get_timeline(target: &FerroObject) -> Option<Rc<DeferredContentPresentationTimeline>> {
        target.get_value(Self::timeline_property())
    }
    /// Sets the optional timeline scope.
    pub fn set_timeline(
        target: &FerroObject,
        value: Option<Rc<DeferredContentPresentationTimeline>>,
    ) {
        target.set_value(Self::timeline_property(), value);
    }
    /// Gets the inherited or local delay without clamping.
    pub fn get_delay(target: &FerroObject) -> TimeSpan {
        target.get_value(Self::delay_property())
    }
    /// Sets the inherited delay; realization clamps negative values to zero.
    pub fn set_delay(target: &FerroObject, value: TimeSpan) {
        target.set_value(Self::delay_property(), value);
    }
    /// Gets the inherited or local presentation order.
    pub fn get_order(target: &FerroObject) -> i32 {
        target.get_value(Self::order_property())
    }
    /// Sets the inherited presentation order.
    pub fn set_order(target: &FerroObject, value: i32) {
        target.set_value(Self::order_property(), value);
    }
}
pub(crate) fn resolve_timeline(target: &FerroObject) -> Rc<DeferredContentPresentationTimeline> {
    if let Some(timeline) = DeferredContentScheduling::get_timeline(target) {
        return timeline;
    }
    if !target.is_set(DeferredContentScheduling::timeline_property()) {
        if let Some(parent) = target
            .to_ref()
            .cast::<StyledElement>()
            .and_then(|s| s.templated_parent())
        {
            return resolve_timeline(&parent);
        }
    }
    DeferredContentPresentationSettings::default_timeline()
}
pub(crate) fn resolve_delay(target: &FerroObject) -> TimeSpan {
    if !target.is_set(DeferredContentScheduling::delay_property()) {
        if let Some(parent) = target
            .to_ref()
            .cast::<StyledElement>()
            .and_then(|s| s.templated_parent())
        {
            return resolve_delay(&parent);
        }
    }
    DeferredContentScheduling::get_delay(target).max(TimeSpan::ZERO)
}
pub(crate) fn resolve_order(target: &FerroObject) -> i32 {
    if !target.is_set(DeferredContentScheduling::order_property()) {
        if let Some(parent) = target
            .to_ref()
            .cast::<StyledElement>()
            .and_then(|s| s.templated_parent())
        {
            return resolve_order(&parent);
        }
    }
    DeferredContentScheduling::get_order(target)
}
