// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::*;
use ferroui_controls::{Control, ItemsChangedEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::layout::Orientation;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    Size, StyledProperty,
};

/// A virtualizing stack with the upstream 100-entry size estimator.
#[repr(C)]
pub struct StackLayout {
    base: VirtualizingLayout,
}
ferro_class!(StackLayout: VirtualizingLayout);
ferro_class_info!(StackLayout {
    new: StackLayout::new
});
ferro_impl_classes!(StackLayout: LayoutImpl);
impl FerroObjectImpl for StackLayout {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_layout_id("StackLayout".to_string());
        this.update_index_orientation();
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::orientation_property().as_property() {
            this.update_index_orientation();
        }
        if args.property() == Self::orientation_property().as_property()
            || args.property() == Self::spacing_property().as_property()
            || args.property() == Self::is_virtualization_enabled_property().as_property()
        {
            this.invalidate_measure();
        }
    }
}
ferro_properties! { impl StackLayout {
    pub fn orientation_property() -> StyledProperty<Orientation> { FerroProperty::register::<StackLayout, _>("Orientation", Orientation::Vertical) }
    pub fn spacing_property() -> StyledProperty<f64> { FerroProperty::register::<StackLayout, _>("Spacing", 0.0) }
    pub fn is_virtualization_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<StackLayout, _>("IsVirtualizationEnabled", true) }
} }
impl StackLayout {
    pub fn construct() -> Self {
        Self {
            base: VirtualizingLayout::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value);
    }
    pub fn spacing(&self) -> f64 {
        self.get_value(Self::spacing_property())
    }
    pub fn set_spacing(&self, value: f64) {
        self.set_value(Self::spacing_property(), value);
    }
    pub fn is_virtualization_enabled(&self) -> bool {
        self.get_value(Self::is_virtualization_enabled_property())
    }
    pub fn set_is_virtualization_enabled(&self, value: bool) {
        self.set_value(Self::is_virtualization_enabled_property(), value);
    }
    fn axis(&self) -> OrientationBasedMeasures {
        OrientationBasedMeasures::new(if self.orientation() == Orientation::Vertical {
            ScrollOrientation::Vertical
        } else {
            ScrollOrientation::Horizontal
        })
    }
    fn update_index_orientation(&self) {
        self.set_index_based_layout_orientation(if self.orientation() == Orientation::Vertical {
            IndexBasedLayoutOrientation::TopToBottom
        } else {
            IndexBasedLayoutOrientation::LeftToRight
        });
    }
    pub fn state(context: &Ref<VirtualizingLayoutContext>) -> Option<Ref<StackLayoutState>> {
        context
            .layout_state()
            .and_then(|state| ValueTypes::as_object(&*state))
            .and_then(|state| state.cast::<StackLayoutState>())
    }
    fn average_element_size(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        state: &Ref<StackLayoutState>,
    ) -> f64 {
        if context.item_count() == 0 {
            return 0.0;
        }
        if state.total_elements_measured() == 0 {
            let temporary = context.get_or_create_element_at_with_options(
                0,
                ElementRealizationOptions::FORCE_CREATE
                    | ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE,
            );
            state.flow_algorithm.measure_element(
                &temporary,
                0,
                available,
                context,
                &StackDelegates(self),
            );
            context.recycle_element(&temporary);
        }
        let average = state.total_element_size() / state.total_elements_measured().max(1) as f64;
        if state.are_elements_measured_regular() {
            average
        } else {
            average.round()
        }
    }
}
impl VirtualizingLayoutImpl for StackLayout {
    fn initialize_for_context_core(_this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        let state = Self::state(context).unwrap_or_else(|| {
            assert!(
                context.layout_state().is_none(),
                "LayoutState must derive from StackLayoutState"
            );
            StackLayoutState::new()
        });
        state.initialize_for_context(context);
    }
    fn uninitialize_for_context_core(_this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        if let Some(state) = Self::state(context) {
            state.uninitialize_for_context(context);
        }
    }
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        let Some(state) = Self::state(context) else {
            return Size::default();
        };
        state.on_measure_start();
        state.flow_algorithm.measure(
            available,
            context,
            FlowLayoutMeasureOptions {
                is_wrapping: false,
                min_item_spacing: 0.0,
                line_spacing: this.spacing(),
                max_items_per_line: u32::MAX,
                orientation: this.axis().orientation,
                is_virtualization_enabled: this.is_virtualization_enabled(),
            },
            &StackDelegates(this),
        )
    }
    fn arrange_override(
        _this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        Self::state(context).map_or(Size::default(), |state| {
            state
                .flow_algorithm
                .arrange(final_size, false, FlowLayoutLineAlignment::Start)
        })
    }
    fn on_items_changed_core(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        args: &ItemsChangedEventArgs<'_>,
    ) {
        if let Some(state) = Self::state(context) {
            if args.action == NotifyCollectionChangedAction::Reset {
                state.on_element_sizes_reset();
            }
            state.flow_algorithm.on_items_source_changed(args);
        }
        this.invalidate_measure();
    }
}
struct StackDelegates<'a>(&'a StackLayout);
impl IFlowLayoutAlgorithmDelegates for StackDelegates<'_> {
    fn algorithm_set_anchor_info(&self, index: i32, offset: f64) {
        self.0.set_layout_anchor_info(index, offset);
    }
    fn algorithm_get_measure_size(
        &self,
        _index: i32,
        available: Size,
        _context: &Ref<VirtualizingLayoutContext>,
    ) -> Size {
        available
    }
    fn algorithm_get_provisional_arrange_size(
        &self,
        _index: i32,
        _measure: Size,
        desired: Size,
        _context: &Ref<VirtualizingLayoutContext>,
    ) -> Size {
        desired
    }
    fn algorithm_should_break_line(&self, _index: i32, _remaining: f64) -> bool {
        true
    }
    fn algorithm_get_anchor_for_realization_rect(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        let mut info = FlowLayoutAnchorInfo::default();
        let count = context.item_count();
        if count > 0 {
            let state =
                StackLayout::state(context).expect("Stack layout state must be initialized");
            let axis = self.0.axis();
            let rect = context.realization_rect();
            let extent = state.flow_algorithm.last_extent();
            let average =
                self.0.average_element_size(available, context, &state) + self.0.spacing();
            let offset = axis.major_start(rect) - axis.major_start(extent);
            let major = if axis.major_size(extent) == 0.0 {
                (average * count as f64 - self.0.spacing()).max(0.0)
            } else {
                axis.major_size(extent)
            };
            if axis.major_size(rect) >= 0.0
                && offset + axis.major_size(rect) >= 0.0
                && offset <= major
            {
                info.index = if average == 0.0 {
                    0
                } else {
                    (offset / average) as i32
                }
                .clamp(0, count - 1);
                info.offset = info.index as f64 * average + axis.major_start(extent);
            }
        }
        info
    }
    fn algorithm_get_anchor_for_target_element(
        &self,
        index: i32,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        if index < 0 || index >= context.item_count() {
            return FlowLayoutAnchorInfo::default();
        }
        let state = StackLayout::state(context).expect("Stack layout state must be initialized");
        FlowLayoutAnchorInfo {
            index,
            offset: index as f64
                * (self.0.average_element_size(available, context, &state) + self.0.spacing())
                + self
                    .0
                    .axis()
                    .major_start(state.flow_algorithm.last_extent()),
        }
    }
    fn algorithm_get_extent(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        first: FlowLayoutRealizedElement,
        last: FlowLayoutRealizedElement,
    ) -> Rect {
        let state = StackLayout::state(context).expect("Stack layout state must be initialized");
        let axis = self.0.axis();
        let count = context.item_count();
        let average = self.0.average_element_size(available, context, &state) + self.0.spacing();
        let mut start = 0.0;
        let mut major = (count as f64 * average - self.0.spacing()).max(0.0);
        if count > 0 && first.element.is_some() {
            start = axis.major_start(first.bounds) - first.index as f64 * average;
            major = axis.major_end(last.bounds) - start + (count - last.index - 1) as f64 * average;
        }
        axis.rect(0.0, start, state.max_arrange_bounds(), major.max(0.0))
    }
    fn algorithm_on_element_measured(
        &self,
        _element: &Ref<Control>,
        index: i32,
        _available: Size,
        _measure: Size,
        _desired: Size,
        arranged: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) {
        if let Some(state) = StackLayout::state(context) {
            let axis = self.0.axis();
            state.on_element_measured(index, axis.major(arranged), axis.minor(arranged));
        }
    }
    fn algorithm_on_line_arranged(
        &self,
        _start: i32,
        _count: i32,
        _size: f64,
        _context: &Ref<VirtualizingLayoutContext>,
    ) {
    }
    fn algorithm_on_layout_round_factor_changed(&self, context: &Ref<VirtualizingLayoutContext>) {
        if let Some(state) = StackLayout::state(context) {
            state.on_element_sizes_reset();
            self.0.invalidate_measure();
        }
    }
}
