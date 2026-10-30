// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::{IndexBasedLayoutOrientation, Layout, LinedFlowLayout};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_static_type, FerroObject, Ref};
use std::rc::Rc;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinedFlowLayoutInvalidationTrigger {
    InvalidateLayoutCall = 0,
    SnappedAverageItemsPerLineChange = 1,
    ItemDesiredWidthChange = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs {
    invalidation_trigger: LinedFlowLayoutInvalidationTrigger,
}
impl LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs {
    pub fn invalidation_trigger(&self) -> LinedFlowLayoutInvalidationTrigger {
        self.invalidation_trigger
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs {
    item_index: i32,
    line_index: i32,
}
impl LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs {
    pub fn item_index(&self) -> i32 {
        self.item_index
    }
    pub fn line_index(&self) -> i32 {
        self.line_index
    }
}

struct Hooks {
    anchor_index: HandlerList<dyn Fn(&Ref<FerroObject>)>,
    anchor_offset: HandlerList<dyn Fn(&Ref<FerroObject>)>,
    average: HandlerList<dyn Fn(&Ref<FerroObject>)>,
    invalidated: HandlerList<
        dyn Fn(&Ref<FerroObject>, &LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs),
    >,
    locked:
        HandlerList<dyn Fn(&Ref<FerroObject>, &LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs)>,
}
ferroui_base::ferro_thread_local! {
    static HOOKS: Hooks = Hooks {
        anchor_index: HandlerList::new(), anchor_offset: HandlerList::new(), average: HandlerList::new(), invalidated: HandlerList::new(), locked: HandlerList::new(),
    };
}

/// UI-thread-local diagnostic controls and observations of the layout engine.
/// The forced values are applied to the actual production algorithms.
pub struct LayoutsTestHooks;
ferro_static_type!(LayoutsTestHooks);

macro_rules! simple_event {
    ($subscribe:ident, $notify:ident, $field:ident) => {
        pub fn $subscribe(handler: impl Fn(&Ref<FerroObject>) + 'static) -> Rc<dyn IDisposable> {
            let token = HOOKS.with(|hooks| hooks.$field.add(Rc::new(handler)));
            Disposable::create(move || {
                let _ = HOOKS.try_with(|hooks| hooks.$field.remove(token));
            })
        }
        pub(crate) fn $notify(layout: &Ref<FerroObject>) {
            let handlers = HOOKS.with(|hooks| hooks.$field.snapshot());
            for (_, handler) in handlers.iter() {
                handler(layout);
            }
        }
    };
}
impl LayoutsTestHooks {
    simple_event!(
        layout_anchor_index_changed,
        notify_anchor_index_changed,
        anchor_index
    );
    simple_event!(
        layout_anchor_offset_changed,
        notify_anchor_offset_changed,
        anchor_offset
    );
    simple_event!(
        lined_flow_layout_snapped_average_items_per_line_changed,
        notify_average_changed,
        average
    );
    pub fn lined_flow_layout_invalidated(
        handler: impl Fn(&Ref<FerroObject>, &LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs)
            + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = HOOKS.with(|hooks| hooks.invalidated.add(Rc::new(handler)));
        Disposable::create(move || {
            let _ = HOOKS.try_with(|hooks| hooks.invalidated.remove(token));
        })
    }
    pub fn lined_flow_layout_item_locked(
        handler: impl Fn(&Ref<FerroObject>, &LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs)
            + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = HOOKS.with(|hooks| hooks.locked.add(Rc::new(handler)));
        Disposable::create(move || {
            let _ = HOOKS.try_with(|hooks| hooks.locked.remove(token));
        })
    }
    pub(crate) fn notify_invalidated(
        layout: &Ref<FerroObject>,
        trigger: LinedFlowLayoutInvalidationTrigger,
    ) {
        let handlers = HOOKS.with(|hooks| hooks.invalidated.snapshot());
        let args = LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs {
            invalidation_trigger: trigger,
        };
        for (_, handler) in handlers.iter() {
            handler(layout, &args);
        }
    }
    pub(crate) fn notify_item_locked(layout: &Ref<FerroObject>, item_index: i32, line_index: i32) {
        let handlers = HOOKS.with(|hooks| hooks.locked.snapshot());
        let args = LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs {
            item_index,
            line_index,
        };
        for (_, handler) in handlers.iter() {
            handler(layout, &args);
        }
    }
    pub fn get_layout_forced_index_based_layout_orientation(
        layout: &Ref<FerroObject>,
    ) -> IndexBasedLayoutOrientation {
        layout
            .cast::<Layout>()
            .map_or(IndexBasedLayoutOrientation::None, |layout| {
                layout.forced_index_orientation().unwrap_or_default()
            })
    }
    pub fn set_layout_forced_index_based_layout_orientation(
        layout: &Ref<FerroObject>,
        orientation: IndexBasedLayoutOrientation,
    ) {
        if let Some(layout) = layout.cast::<Layout>() {
            layout.set_forced_index_orientation(Some(orientation));
        }
    }
    pub fn reset_layout_forced_index_based_layout_orientation(layout: &Ref<FerroObject>) {
        if let Some(layout) = layout.cast::<Layout>() {
            layout.set_forced_index_orientation(None);
        }
    }
    pub fn layout_invalidate_measure(layout: &Ref<FerroObject>, relayout: bool) {
        if relayout {
            if let Some(layout) = layout.cast::<LinedFlowLayout>() {
                layout.invalidate_layout();
                return;
            }
        }
        if let Some(layout) = layout.cast::<Layout>() {
            layout.invalidate_measure();
        }
    }
    pub fn get_layout_log_item_index(layout: &Ref<FerroObject>) -> i32 {
        layout
            .cast::<Layout>()
            .map_or(-1, |layout| layout.log_item_index())
    }
    /// Selects the realized item whose completed Repeater arrangement is
    /// reported in the native `Layout` log area. A negative index disables it.
    pub fn set_layout_log_item_index(layout: &Ref<FerroObject>, index: i32) {
        if let Some(layout) = layout.cast::<Layout>() {
            layout.set_log_item_index(index);
        }
    }
    pub fn get_layout_anchor_index(layout: &Ref<FerroObject>) -> i32 {
        layout
            .cast::<Layout>()
            .map_or(-1, |layout| layout.layout_anchor_info().0)
    }
    pub fn get_layout_anchor_offset(layout: &Ref<FerroObject>) -> f64 {
        layout
            .cast::<Layout>()
            .map_or(-1.0, |layout| layout.layout_anchor_info().1)
    }
    pub fn get_layout_first_realized_item_index(layout: &Ref<FerroObject>) -> i32 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(-1, |layout| layout.realized_range().0)
    }
    pub fn get_layout_last_realized_item_index(layout: &Ref<FerroObject>) -> i32 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(-1, |layout| layout.realized_range().1)
    }
    pub fn get_lined_flow_layout_first_frozen_item_index(layout: &Ref<FerroObject>) -> i32 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(-1, |layout| layout.frozen_range().0)
    }
    pub fn get_lined_flow_layout_last_frozen_item_index(layout: &Ref<FerroObject>) -> i32 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(-1, |layout| layout.frozen_range().1)
    }
    pub fn get_lined_flow_layout_average_item_aspect_ratio(layout: &Ref<FerroObject>) -> f64 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(0.0, |layout| layout.diagnostic_average_aspect_ratio())
    }
    pub fn get_lined_flow_layout_raw_average_items_per_line(layout: &Ref<FerroObject>) -> f64 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(0.0, |layout| layout.average_items_per_line().0)
    }
    pub fn get_lined_flow_layout_snapped_average_items_per_line(layout: &Ref<FerroObject>) -> f64 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(0.0, |layout| layout.average_items_per_line().1)
    }
    pub fn get_lined_flow_layout_forced_average_items_per_line_divider(
        layout: &Ref<FerroObject>,
    ) -> f64 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(0.0, |layout| layout.forced_average_divider())
    }
    pub fn set_lined_flow_layout_forced_average_items_per_line_divider(
        layout: &Ref<FerroObject>,
        value: f64,
    ) {
        if let Some(layout) = layout.cast::<LinedFlowLayout>() {
            layout.set_forced_average_divider(value);
        }
    }
    pub fn get_lined_flow_layout_forced_average_item_aspect_ratio(
        layout: &Ref<FerroObject>,
    ) -> f64 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(0.0, |layout| layout.forced_aspect_ratio())
    }
    pub fn set_lined_flow_layout_forced_average_item_aspect_ratio(
        layout: &Ref<FerroObject>,
        value: f64,
    ) {
        if let Some(layout) = layout.cast::<LinedFlowLayout>() {
            layout.set_forced_aspect_ratio(value);
        }
    }
    pub fn get_lined_flow_layout_forced_wrap_multiplier(layout: &Ref<FerroObject>) -> f64 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(0.0, |layout| layout.forced_wrap_multiplier())
    }
    pub fn set_lined_flow_layout_forced_wrap_multiplier(layout: &Ref<FerroObject>, value: f64) {
        if let Some(layout) = layout.cast::<LinedFlowLayout>() {
            layout.set_forced_wrap_multiplier(value);
        }
    }
    pub fn get_lined_flow_layout_is_fast_path_supported(layout: &Ref<FerroObject>) -> bool {
        layout
            .cast::<LinedFlowLayout>()
            .is_some_and(|layout| layout.is_fast_path_supported())
    }
    pub fn set_lined_flow_layout_is_fast_path_supported(layout: &Ref<FerroObject>, value: bool) {
        if let Some(layout) = layout.cast::<LinedFlowLayout>() {
            layout.set_fast_path_supported(value);
        }
    }
    pub fn get_lined_flow_layout_line_index(layout: &Ref<FerroObject>, index: i32) -> i32 {
        layout
            .cast::<LinedFlowLayout>()
            .map_or(-1, |layout| layout.diagnostic_line_index(index))
    }
    pub fn clear_lined_flow_layout_item_aspect_ratios(layout: &Ref<FerroObject>) {
        if let Some(layout) = layout.cast::<LinedFlowLayout>() {
            layout.clear_item_aspect_ratios();
        }
    }
    pub fn unlock_lined_flow_layout_items(layout: &Ref<FerroObject>) {
        if let Some(layout) = layout.cast::<LinedFlowLayout>() {
            layout.unlock_items();
        }
    }
}
