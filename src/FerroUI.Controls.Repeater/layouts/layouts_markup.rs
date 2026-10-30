// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

//! Markup projection of the custom FlowLayout policy and layout diagnostics.

// The imports of the markup of the test hooks, which is compiled with the feature `testing`.
#![cfg_attr(not(any(test, feature = "testing")), allow(unused_imports))]
use super::{
    FlowLayout, FlowLayoutAnchorInfo, FlowLayoutRealizedElement, IndexBasedLayoutOrientation,
    LinedFlowLayoutInvalidationTrigger, VirtualizingLayoutContext,
};
#[cfg(any(test, feature = "testing"))]
use super::{
    LayoutsTestHooks, LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs,
    LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs,
};
use ferroui_controls::Control;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class_info, ferro_markup_enum, ferro_markup_type, FerroObject, Rect, Ref, Size,
};
use std::rc::Rc;

ferro_markup_type!(struct FlowLayoutAnchorInfo {
    namespace: "FerroUI.Controls",
    handles: [FlowLayoutAnchorInfo],
    constructors: [(i32, f64) => |index: i32, offset: f64| FlowLayoutAnchorInfo { index, offset }],
    properties: [
        Index: i32 { get: |info: &FlowLayoutAnchorInfo| info.index },
        Offset: f64 { get: |info: &FlowLayoutAnchorInfo| info.offset },
    ],
});

ferro_class_info!(FlowLayout { markup: {
    namespace: "FerroUI.Controls",
    methods: [
        fn GetMeasureSize(i32, Size) -> Size => FlowLayout::get_measure_size,
        fn GetProvisionalArrangeSize(i32, Size, Size) -> Size => FlowLayout::get_provisional_arrange_size,
        fn ShouldBreakLine(i32, f64) -> bool => FlowLayout::should_break_line,
        fn GetAnchorForRealizationRect(Size, Ref<VirtualizingLayoutContext>) -> FlowLayoutAnchorInfo =>
            |layout: &Ref<FlowLayout>, available: Size, context: Ref<VirtualizingLayoutContext>| layout.get_anchor_for_realization_rect(available, &context),
        fn GetAnchorForTargetElement(i32, Size, Ref<VirtualizingLayoutContext>) -> FlowLayoutAnchorInfo =>
            |layout: &Ref<FlowLayout>, index: i32, available: Size, context: Ref<VirtualizingLayoutContext>| layout.get_anchor_for_target_element(index, available, &context),
        fn GetExtent(Size, Ref<VirtualizingLayoutContext>, Option<Ref<Control>>, i32, Rect, Option<Ref<Control>>, i32, Rect) -> Rect =>
            |layout: &Ref<FlowLayout>, available: Size, context: Ref<VirtualizingLayoutContext>, first: Option<Ref<Control>>, first_index: i32, first_bounds: Rect, last: Option<Ref<Control>>, last_index: i32, last_bounds: Rect| {
                layout.get_extent(available, &context,
                    FlowLayoutRealizedElement { element: first, index: first_index, bounds: first_bounds },
                    FlowLayoutRealizedElement { element: last, index: last_index, bounds: last_bounds })
            },
        fn OnElementMeasured(Ref<Control>, i32, Size, Size, Size, Size, Ref<VirtualizingLayoutContext>) =>
            |layout: &Ref<FlowLayout>, element: Ref<Control>, index: i32, available: Size, measure: Size, desired: Size, arranged: Size, context: Ref<VirtualizingLayoutContext>| {
                layout.on_element_measured(&element, index, available, measure, desired, arranged, &context)
            },
        fn OnLineArranged(i32, i32, f64, Ref<VirtualizingLayoutContext>) =>
            |layout: &Ref<FlowLayout>, start: i32, count: i32, size: f64, context: Ref<VirtualizingLayoutContext>| layout.on_line_arranged(start, count, size, &context),
    ],
} });

ferro_markup_enum!(LinedFlowLayoutInvalidationTrigger {
    InvalidateLayoutCall, SnappedAverageItemsPerLineChange, ItemDesiredWidthChange,
}, { namespace: "FerroUI.Controls.Repeater" });

#[cfg(any(test, feature = "testing"))]
ferro_markup_type!(struct LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs],
    properties: [
        InvalidationTrigger: LinedFlowLayoutInvalidationTrigger { get: LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs::invalidation_trigger },
    ],
});

#[cfg(any(test, feature = "testing"))]
ferro_markup_type!(struct LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs {
    namespace: "FerroUI.Controls.Repeater",
    handles: [LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs],
    properties: [
        ItemIndex: i32 { get: LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs::item_index },
        LineIndex: i32 { get: LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs::line_index },
    ],
});

#[cfg(any(test, feature = "testing"))]
ferro_markup_type!(static LayoutsTestHooks {
    namespace: "FerroUI.Controls.Repeater",
    type_info: LayoutsTestHooks,
    methods: [
        static fn GetLayoutForcedIndexBasedLayoutOrientation(Ref<FerroObject>) -> IndexBasedLayoutOrientation => |layout: Ref<FerroObject>| LayoutsTestHooks::get_layout_forced_index_based_layout_orientation(&layout),
        static fn SetLayoutForcedIndexBasedLayoutOrientation(Ref<FerroObject>, IndexBasedLayoutOrientation) => |layout: Ref<FerroObject>, orientation: IndexBasedLayoutOrientation| LayoutsTestHooks::set_layout_forced_index_based_layout_orientation(&layout, orientation),
        static fn ResetLayoutForcedIndexBasedLayoutOrientation(Ref<FerroObject>) => |layout: Ref<FerroObject>| LayoutsTestHooks::reset_layout_forced_index_based_layout_orientation(&layout),
        static fn LayoutInvalidateMeasure(Ref<FerroObject>, bool) => |layout: Ref<FerroObject>, relayout: bool| LayoutsTestHooks::layout_invalidate_measure(&layout, relayout),
        static fn GetLayoutLogItemIndex(Ref<FerroObject>) -> i32 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_layout_log_item_index(&layout),
        static fn SetLayoutLogItemIndex(Ref<FerroObject>, i32) => |layout: Ref<FerroObject>, index: i32| LayoutsTestHooks::set_layout_log_item_index(&layout, index),
        static fn GetLayoutAnchorIndex(Ref<FerroObject>) -> i32 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_layout_anchor_index(&layout),
        static fn GetLayoutAnchorOffset(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_layout_anchor_offset(&layout),
        static fn GetLayoutFirstRealizedItemIndex(Ref<FerroObject>) -> i32 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_layout_first_realized_item_index(&layout),
        static fn GetLayoutLastRealizedItemIndex(Ref<FerroObject>) -> i32 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_layout_last_realized_item_index(&layout),
        static fn GetLinedFlowLayoutFirstFrozenItemIndex(Ref<FerroObject>) -> i32 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_first_frozen_item_index(&layout),
        static fn GetLinedFlowLayoutLastFrozenItemIndex(Ref<FerroObject>) -> i32 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_last_frozen_item_index(&layout),
        static fn GetLinedFlowLayoutAverageItemAspectRatio(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_average_item_aspect_ratio(&layout),
        static fn GetLinedFlowLayoutRawAverageItemsPerLine(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_raw_average_items_per_line(&layout),
        static fn GetLinedFlowLayoutSnappedAverageItemsPerLine(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_snapped_average_items_per_line(&layout),
        static fn GetLinedFlowLayoutForcedAverageItemsPerLineDivider(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_forced_average_items_per_line_divider(&layout),
        static fn SetLinedFlowLayoutForcedAverageItemsPerLineDivider(Ref<FerroObject>, f64) => |layout: Ref<FerroObject>, value: f64| LayoutsTestHooks::set_lined_flow_layout_forced_average_items_per_line_divider(&layout, value),
        static fn GetLinedFlowLayoutForcedAverageItemAspectRatio(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_forced_average_item_aspect_ratio(&layout),
        static fn SetLinedFlowLayoutForcedAverageItemAspectRatio(Ref<FerroObject>, f64) => |layout: Ref<FerroObject>, value: f64| LayoutsTestHooks::set_lined_flow_layout_forced_average_item_aspect_ratio(&layout, value),
        static fn GetLinedFlowLayoutForcedWrapMultiplier(Ref<FerroObject>) -> f64 => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_forced_wrap_multiplier(&layout),
        static fn SetLinedFlowLayoutForcedWrapMultiplier(Ref<FerroObject>, f64) => |layout: Ref<FerroObject>, value: f64| LayoutsTestHooks::set_lined_flow_layout_forced_wrap_multiplier(&layout, value),
        static fn GetLinedFlowLayoutIsFastPathSupported(Ref<FerroObject>) -> bool => |layout: Ref<FerroObject>| LayoutsTestHooks::get_lined_flow_layout_is_fast_path_supported(&layout),
        static fn SetLinedFlowLayoutIsFastPathSupported(Ref<FerroObject>, bool) => |layout: Ref<FerroObject>, value: bool| LayoutsTestHooks::set_lined_flow_layout_is_fast_path_supported(&layout, value),
        static fn GetLinedFlowLayoutLineIndex(Ref<FerroObject>, i32) -> i32 => |layout: Ref<FerroObject>, index: i32| LayoutsTestHooks::get_lined_flow_layout_line_index(&layout, index),
        static fn ClearLinedFlowLayoutItemAspectRatios(Ref<FerroObject>) => |layout: Ref<FerroObject>| LayoutsTestHooks::clear_lined_flow_layout_item_aspect_ratios(&layout),
        static fn UnlockLinedFlowLayoutItems(Ref<FerroObject>) => |layout: Ref<FerroObject>| LayoutsTestHooks::unlock_lined_flow_layout_items(&layout),
        // FerroUI exposes global event lifetimes as disposable subscriptions;
        // its markup event descriptor models instance events. These methods
        // retain the source event names, sender, and typed event arguments.
        static fn SubscribeLayoutAnchorIndexChanged(MarkupDelegate) -> Rc<dyn IDisposable> => |handler: MarkupDelegate| {
            LayoutsTestHooks::layout_anchor_index_changed(move |sender| { handler.invoke(&[into_markup_value(sender.clone()), None]); })
        },
        static fn SubscribeLayoutAnchorOffsetChanged(MarkupDelegate) -> Rc<dyn IDisposable> => |handler: MarkupDelegate| {
            LayoutsTestHooks::layout_anchor_offset_changed(move |sender| { handler.invoke(&[into_markup_value(sender.clone()), None]); })
        },
        static fn SubscribeLinedFlowLayoutSnappedAverageItemsPerLineChanged(MarkupDelegate) -> Rc<dyn IDisposable> => |handler: MarkupDelegate| {
            LayoutsTestHooks::lined_flow_layout_snapped_average_items_per_line_changed(move |sender| { handler.invoke(&[into_markup_value(sender.clone()), None]); })
        },
        static fn SubscribeLinedFlowLayoutInvalidated(MarkupDelegate) -> Rc<dyn IDisposable> => |handler: MarkupDelegate| {
            LayoutsTestHooks::lined_flow_layout_invalidated(move |sender, args| { handler.invoke(&[into_markup_value(sender.clone()), into_markup_value(*args)]); })
        },
        static fn SubscribeLinedFlowLayoutItemLocked(MarkupDelegate) -> Rc<dyn IDisposable> => |handler: MarkupDelegate| {
            LayoutsTestHooks::lined_flow_layout_item_locked(move |sender, args| { handler.invoke(&[into_markup_value(sender.clone()), into_markup_value(*args)]); })
        },
    ],
});

const TYPES: &[&MarkupType] = &[
    <FlowLayoutAnchorInfo as MarkupTyped>::MARKUP,
    <LinedFlowLayoutInvalidationTrigger as MarkupTyped>::MARKUP,
];

/// The test hooks, registered for markup only with the feature `testing`.
#[cfg(any(test, feature = "testing"))]
const TEST_HOOK_TYPES: &[&MarkupType] = &[
    <LayoutsTestHooksLinedFlowLayoutInvalidatedEventArgs as MarkupTyped>::MARKUP,
    <LayoutsTestHooksLinedFlowLayoutItemLockedEventArgs as MarkupTyped>::MARKUP,
    <LayoutsTestHooks as MarkupTyped>::MARKUP,
];

pub(crate) fn register_types() {
    MarkupType::register_all(TYPES);
    #[cfg(any(test, feature = "testing"))]
    MarkupType::register_all(TEST_HOOK_TYPES);
}
