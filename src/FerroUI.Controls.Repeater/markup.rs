// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

//! Native runtime and compiled-markup projection of the WinUI layout APIs.

use super::*;
use ferroui_controls::Control;
use ferroui_base::metadata::{MarkupDelegate, MarkupType, MarkupTyped, into_markup_value};
use ferroui_base::{
    BoxedValue, Point, Rect, Ref, Size, ferro_class_info, ferro_markup_enum, ferro_markup_type,
};

ferro_markup_enum!(IndexBasedLayoutOrientation { None, TopToBottom, LeftToRight }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(FlowLayoutLineAlignment { Start, Center, End, SpaceAround, SpaceBetween, SpaceEvenly }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(UniformGridLayoutItemsJustification { Start, Center, End, SpaceAround, SpaceBetween, SpaceEvenly }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(UniformGridLayoutItemsStretch { None, Fill, Uniform }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(LinedFlowLayoutItemsJustification { Start, Center, End, SpaceAround, SpaceBetween, SpaceEvenly }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(LinedFlowLayoutItemsStretch { None, Fill }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(flags ElementRealizationOptions {
    None = ElementRealizationOptions::NONE,
    ForceCreate = ElementRealizationOptions::FORCE_CREATE,
    SuppressAutoRecycle = ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE,
}, { namespace: "FerroUI.Controls" });

ferro_class_info!(Layout { markup: {
    namespace: "FerroUI.Controls",
    properties: [IndexBasedLayoutOrientation: IndexBasedLayoutOrientation { get: Layout::index_based_layout_orientation }],
    methods: [
        fn InitializeForContext(Ref<LayoutContext>) => |this: &Ref<Layout>, context: Ref<LayoutContext>| this.initialize_for_context(&context),
        fn UninitializeForContext(Ref<LayoutContext>) => |this: &Ref<Layout>, context: Ref<LayoutContext>| this.uninitialize_for_context(&context),
        fn Measure(Ref<LayoutContext>, Size) -> Size => |this: &Ref<Layout>, context: Ref<LayoutContext>, size: Size| this.measure(&context, size),
        fn Arrange(Ref<LayoutContext>, Size) -> Size => |this: &Ref<Layout>, context: Ref<LayoutContext>, size: Size| this.arrange(&context, size),
    ],
    events: [
        MeasureInvalidated(Option<Ref<Layout>>, Option<BoxedValue>) => |this: &Ref<Layout>, handler: MarkupDelegate| {
            let sender = this.downgrade();
            this.measure_invalidated(move || { handler.invoke(&[into_markup_value(sender.upgrade()), None]); })
        },
        ArrangeInvalidated(Option<Ref<Layout>>, Option<BoxedValue>) => |this: &Ref<Layout>, handler: MarkupDelegate| {
            let sender = this.downgrade();
            this.arrange_invalidated(move || { handler.invoke(&[into_markup_value(sender.upgrade()), None]); })
        },
    ],
} });
ferro_class_info!(LayoutContext { markup: {
    namespace: "FerroUI.Controls",
    properties: [LayoutState: Option<BoxedValue> { get: LayoutContext::layout_state, set: LayoutContext::set_layout_state }],
} });
ferro_class_info!(VirtualizingLayoutContext { markup: {
    namespace: "FerroUI.Controls",
    properties: [
        ItemCount: i32 { get: VirtualizingLayoutContext::item_count },
        RealizationRect: Rect { get: VirtualizingLayoutContext::realization_rect },
        RecommendedAnchorIndex: i32 { get: VirtualizingLayoutContext::recommended_anchor_index },
        LayoutOrigin: Point { get: VirtualizingLayoutContext::layout_origin, set: VirtualizingLayoutContext::set_layout_origin },
    ],
    methods: [
        fn GetItemAt(i32) -> Option<BoxedValue> => VirtualizingLayoutContext::get_item_at,
        fn GetOrCreateElementAt(i32) -> Ref<Control> => VirtualizingLayoutContext::get_or_create_element_at,
        fn GetOrCreateElementAt(i32, ElementRealizationOptions) -> Ref<Control> => VirtualizingLayoutContext::get_or_create_element_at_with_options,
        fn RecycleElement(Ref<Control>) => |this: &Ref<VirtualizingLayoutContext>, element: Ref<Control>| this.recycle_element(&element),
    ],
} });
ferro_class_info!(NonVirtualizingLayoutContext { markup: {
    namespace: "FerroUI.Controls",
    properties: [Children: Vec<Ref<Control>> { get: NonVirtualizingLayoutContext::children }],
} });
ferro_class_info!(LinedFlowLayout { markup: {
    namespace: "FerroUI.Controls",
    properties: [
        RequestedRangeStartIndex: i32 { get: LinedFlowLayout::requested_range_start_index },
        RequestedRangeLength: i32 { get: LinedFlowLayout::requested_range_length },
    ],
    methods: [
        fn InvalidateItemsInfo() => LinedFlowLayout::invalidate_items_info,
        try fn LockItemToLine(i32) -> i32 => LinedFlowLayout::lock_item_to_line,
    ],
    events: [
        ItemsInfoRequested(Option<Ref<LinedFlowLayout>>, LinedFlowLayoutItemsInfoRequestedEventArgs) => |this: &Ref<LinedFlowLayout>, handler: MarkupDelegate| {
            let sender = this.downgrade();
            this.items_info_requested(move |args| { handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]); })
        },
        ItemsUnlocked(Option<Ref<LinedFlowLayout>>, Option<BoxedValue>) => |this: &Ref<LinedFlowLayout>, handler: MarkupDelegate| {
            let sender = this.downgrade();
            this.items_unlocked(move || { handler.invoke(&[into_markup_value(sender.upgrade()), None]); })
        },
    ],
} });

ferro_markup_type!(class LinedFlowLayoutItemsInfoRequestedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [LinedFlowLayoutItemsInfoRequestedEventArgs],
    properties: [
        ItemsRangeStartIndex: i32 { get: LinedFlowLayoutItemsInfoRequestedEventArgs::items_range_start_index, try_set: LinedFlowLayoutItemsInfoRequestedEventArgs::set_items_range_start_index },
        ItemsRangeRequestedLength: i32 { get: LinedFlowLayoutItemsInfoRequestedEventArgs::items_range_requested_length },
        MinWidth: f64 { get: LinedFlowLayoutItemsInfoRequestedEventArgs::min_width, set: LinedFlowLayoutItemsInfoRequestedEventArgs::set_min_width },
        MaxWidth: f64 { get: LinedFlowLayoutItemsInfoRequestedEventArgs::max_width, set: LinedFlowLayoutItemsInfoRequestedEventArgs::set_max_width },
    ],
    methods: [
        try fn SetDesiredAspectRatios(Vec<f64>) => |this: &LinedFlowLayoutItemsInfoRequestedEventArgs, values: Vec<f64>| this.set_desired_aspect_ratios(&values),
        try fn SetMinWidths(Vec<f64>) => |this: &LinedFlowLayoutItemsInfoRequestedEventArgs, values: Vec<f64>| this.set_min_widths(&values),
        try fn SetMaxWidths(Vec<f64>) => |this: &LinedFlowLayoutItemsInfoRequestedEventArgs, values: Vec<f64>| this.set_max_widths(&values),
    ],
});
ferro_markup_type!(struct ItemsRepeaterElementPreparedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ItemsRepeaterElementPreparedEventArgs],
    properties: [
        Element: Ref<Control> { get: |args: &ItemsRepeaterElementPreparedEventArgs| args.element.clone() },
        Index: i32 { get: |args: &ItemsRepeaterElementPreparedEventArgs| args.index },
    ],
});
ferro_markup_type!(struct ItemsRepeaterElementClearingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ItemsRepeaterElementClearingEventArgs],
    properties: [Element: Ref<Control> { get: |args: &ItemsRepeaterElementClearingEventArgs| args.element.clone() }],
});
ferro_markup_type!(struct ItemsRepeaterElementIndexChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ItemsRepeaterElementIndexChangedEventArgs],
    properties: [
        Element: Ref<Control> { get: |args: &ItemsRepeaterElementIndexChangedEventArgs| args.element.clone() },
        OldIndex: i32 { get: |args: &ItemsRepeaterElementIndexChangedEventArgs| args.old_index },
        NewIndex: i32 { get: |args: &ItemsRepeaterElementIndexChangedEventArgs| args.new_index },
    ],
});

const TYPES: &[&MarkupType] = &[
    <IndexBasedLayoutOrientation as MarkupTyped>::MARKUP,
    <FlowLayoutLineAlignment as MarkupTyped>::MARKUP,
    <UniformGridLayoutItemsJustification as MarkupTyped>::MARKUP,
    <UniformGridLayoutItemsStretch as MarkupTyped>::MARKUP,
    <LinedFlowLayoutItemsJustification as MarkupTyped>::MARKUP,
    <LinedFlowLayoutItemsStretch as MarkupTyped>::MARKUP,
    <ElementRealizationOptions as MarkupTyped>::MARKUP,
    <LinedFlowLayoutItemsInfoRequestedEventArgs as MarkupTyped>::MARKUP,
    <ItemsRepeaterElementPreparedEventArgs as MarkupTyped>::MARKUP,
    <ItemsRepeaterElementClearingEventArgs as MarkupTyped>::MARKUP,
    <ItemsRepeaterElementIndexChangedEventArgs as MarkupTyped>::MARKUP,
];
pub(crate) fn register_types() {
    MarkupType::register_all(TYPES);
}
