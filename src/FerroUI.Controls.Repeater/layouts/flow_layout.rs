// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::*;
use ferroui_controls::{Control, ItemsChangedEventArgs};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::layout::Orientation;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref,
    Size, StyledProperty,
};

#[repr(C)]
pub struct FlowLayout {
    base: VirtualizingLayout,
}
ferro_class! {
    FlowLayout: VirtualizingLayout, virtuals FlowLayoutImpl: VirtualizingLayoutImpl {
        fn get_measure_size(this, index: i32, available_size: Size) -> Size;
        fn get_provisional_arrange_size(this, index: i32, measure_size: Size, desired_size: Size) -> Size;
        fn should_break_line(this, index: i32, remaining_space: f64) -> bool;
        fn get_anchor_for_realization_rect(this, available_size: Size, context: &Ref<VirtualizingLayoutContext>) -> FlowLayoutAnchorInfo;
        fn get_anchor_for_target_element(this, index: i32, available_size: Size, context: &Ref<VirtualizingLayoutContext>) -> FlowLayoutAnchorInfo;
        fn get_extent(this, available_size: Size, context: &Ref<VirtualizingLayoutContext>, first: FlowLayoutRealizedElement, last: FlowLayoutRealizedElement) -> Rect;
        fn on_element_measured(this, element: &Ref<Control>, index: i32, available_size: Size, measure_size: Size, desired_size: Size, provisional_arrange_size: Size, context: &Ref<VirtualizingLayoutContext>);
        fn on_line_arranged(this, start_index: i32, count_in_line: i32, line_size: f64, context: &Ref<VirtualizingLayoutContext>);
    }
}
ferro_class_info!(FlowLayout {
    new: FlowLayout::new
});
ferro_impl_classes!(FlowLayout: LayoutImpl);
impl FerroObjectImpl for FlowLayout {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_layout_id("FlowLayout".to_string());
        this.update_index_orientation();
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::orientation_property().as_property() {
            this.update_index_orientation();
        }
        this.invalidate_measure();
    }
}
ferro_properties! { impl FlowLayout {
    pub fn orientation_property() -> StyledProperty<Orientation> { FerroProperty::register::<FlowLayout, _>("Orientation", Orientation::Horizontal) }
    pub fn line_spacing_property() -> StyledProperty<f64> { FerroProperty::register::<FlowLayout, _>("LineSpacing", 0.0) }
    pub fn min_item_spacing_property() -> StyledProperty<f64> { FerroProperty::register::<FlowLayout, _>("MinItemSpacing", 0.0) }
    pub fn line_alignment_property() -> StyledProperty<FlowLayoutLineAlignment> { FerroProperty::register::<FlowLayout, _>("LineAlignment", FlowLayoutLineAlignment::Start) }
} }
impl FlowLayout {
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
    pub fn line_spacing(&self) -> f64 {
        self.get_value(Self::line_spacing_property())
    }
    pub fn set_line_spacing(&self, value: f64) {
        self.set_value(Self::line_spacing_property(), value);
    }
    pub fn min_item_spacing(&self) -> f64 {
        self.get_value(Self::min_item_spacing_property())
    }
    pub fn set_min_item_spacing(&self, value: f64) {
        self.set_value(Self::min_item_spacing_property(), value);
    }
    pub fn line_alignment(&self) -> FlowLayoutLineAlignment {
        self.get_value(Self::line_alignment_property())
    }
    pub fn set_line_alignment(&self, value: FlowLayoutLineAlignment) {
        self.set_value(Self::line_alignment_property(), value);
    }
    pub fn state(context: &Ref<VirtualizingLayoutContext>) -> Option<Ref<FlowLayoutState>> {
        context
            .layout_state()
            .and_then(|state| ValueTypes::as_object(&*state))
            .and_then(|state| state.cast::<FlowLayoutState>())
    }
    fn axis(&self) -> OrientationBasedMeasures {
        OrientationBasedMeasures::new(if self.orientation() == Orientation::Horizontal {
            ScrollOrientation::Vertical
        } else {
            ScrollOrientation::Horizontal
        })
    }
    fn update_index_orientation(&self) {
        self.set_index_based_layout_orientation(if self.orientation() == Orientation::Horizontal {
            IndexBasedLayoutOrientation::LeftToRight
        } else {
            IndexBasedLayoutOrientation::TopToBottom
        });
    }
    fn average_line_info(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        state: &Ref<FlowLayoutState>,
    ) -> (f64, f64) {
        if state.total_lines_measured() == 0 {
            let temporary = context.get_or_create_element_at_with_options(
                0,
                ElementRealizationOptions::FORCE_CREATE
                    | ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE,
            );
            let desired = state.flow_algorithm.measure_element(
                &temporary,
                0,
                available,
                context,
                &FlowDelegates(self),
            );
            context.recycle_element(&temporary);
            let axis = self.axis();
            let estimated_count = ((axis.minor(available) / axis.minor(desired)) as i32).max(1);
            state.on_line_arranged(0, estimated_count, axis.major(desired), context);
            state.set_special_element_desired_size(desired);
        }
        let lines = state.total_lines_measured().max(1) as f64;
        (
            (state.total_line_size() / lines).round(),
            (state.total_items_per_line() / lines).max(1.0),
        )
    }
}
impl VirtualizingLayoutImpl for FlowLayout {
    fn initialize_for_context_core(_this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        let state = Self::state(context).unwrap_or_else(|| {
            assert!(
                context.layout_state().is_none(),
                "LayoutState must derive from FlowLayoutState"
            );
            FlowLayoutState::new()
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
        state.flow_algorithm.measure(
            available,
            context,
            FlowLayoutMeasureOptions {
                is_wrapping: true,
                min_item_spacing: this.min_item_spacing(),
                line_spacing: this.line_spacing(),
                max_items_per_line: u32::MAX,
                orientation: this.axis().orientation,
                is_virtualization_enabled: true,
            },
            &FlowDelegates(this),
        )
    }
    fn arrange_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        Self::state(context).map_or(Size::default(), |state| {
            state
                .flow_algorithm
                .arrange(final_size, true, this.line_alignment())
        })
    }
    fn on_items_changed_core(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        args: &ItemsChangedEventArgs<'_>,
    ) {
        if let Some(state) = Self::state(context) {
            state.flow_algorithm.on_items_source_changed(args);
        }
        this.invalidate_measure();
    }
}
impl FlowLayoutImpl for FlowLayout {
    fn get_measure_size(_this: &Self, _index: i32, available: Size) -> Size {
        available
    }
    fn get_provisional_arrange_size(
        _this: &Self,
        _index: i32,
        _measure: Size,
        desired: Size,
    ) -> Size {
        desired
    }
    fn should_break_line(_this: &Self, _index: i32, remaining: f64) -> bool {
        remaining < 0.0
    }
    fn get_anchor_for_realization_rect(
        this: &Self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        let mut info = FlowLayoutAnchorInfo::default();
        let count = context.item_count();
        if count > 0 {
            let state = Self::state(context).expect("Flow layout state must be initialized");
            let axis = this.axis();
            let extent = state.flow_algorithm.last_extent();
            let rect = context.realization_rect();
            let (line, items) = this.average_line_info(available, context, &state);
            let stride = line + this.line_spacing();
            let major = if axis.major_size(extent) == 0.0 {
                count as f64 / items * stride
            } else {
                axis.major_size(extent)
            };
            if axis.major_size(rect) > 0.0
                && axis.major_end(rect) >= axis.major_start(extent)
                && axis.major_start(rect) <= axis.major_start(extent) + major
            {
                let line_index = if stride == 0.0 {
                    0
                } else {
                    ((axis.major_start(rect) - axis.major_start(extent)) / stride) as i32
                }
                .max(0);
                info.index = (line_index as f64 * items) as i32;
                info.index = info.index.clamp(0, count - 1);
                info.offset = line_index as f64 * stride + axis.major_start(extent);
            }
        }
        info
    }
    fn get_anchor_for_target_element(
        this: &Self,
        index: i32,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        if index < 0 || index >= context.item_count() {
            return FlowLayoutAnchorInfo::default();
        }
        let state = Self::state(context).expect("Flow layout state must be initialized");
        let (line, items) = this.average_line_info(available, context, &state);
        FlowLayoutAnchorInfo {
            index,
            offset: (index as f64 / items).trunc() * (line + this.line_spacing())
                + this.axis().major_start(state.flow_algorithm.last_extent()),
        }
    }
    fn get_extent(
        this: &Self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        first: FlowLayoutRealizedElement,
        last: FlowLayoutRealizedElement,
    ) -> Rect {
        let count = context.item_count();
        if count == 0 {
            return Rect::default();
        }
        let state = Self::state(context).expect("Flow layout state must be initialized");
        let axis = this.axis();
        let minor = axis.minor(available);
        let (line, items) = this.average_line_info(available, context, &state);
        let stride = line + this.line_spacing();
        if first.element.is_some() {
            let before = (first.index as f64 / items).trunc();
            let start = axis.major_start(first.bounds) - before * stride;
            let remaining_lines = ((count - last.index - 1) as f64 / items).trunc();
            let major = axis.major_end(last.bounds) - start + remaining_lines * stride;
            axis.rect(
                0.0,
                start,
                if minor.is_finite() {
                    minor
                } else {
                    axis.minor_end(last.bounds).max(0.0)
                },
                major.max(0.0),
            )
        } else if minor.is_finite() {
            axis.rect(
                0.0,
                0.0,
                minor,
                ((count as f64 / items).ceil() * stride - this.line_spacing()).max(0.0),
            )
        } else {
            axis.rect(
                0.0,
                0.0,
                ((axis.minor(state.special_element_desired_size()) + this.min_item_spacing())
                    * count as f64
                    - this.min_item_spacing())
                .max(0.0),
                (stride - this.line_spacing()).max(0.0),
            )
        }
    }
    fn on_element_measured(
        _this: &Self,
        _element: &Ref<Control>,
        _index: i32,
        _available: Size,
        _measure: Size,
        _desired: Size,
        _arranged: Size,
        _context: &Ref<VirtualizingLayoutContext>,
    ) {
    }
    fn on_line_arranged(
        _this: &Self,
        start: i32,
        count: i32,
        size: f64,
        context: &Ref<VirtualizingLayoutContext>,
    ) {
        if let Some(state) = Self::state(context) {
            state.on_line_arranged(start, count, size, context);
        }
    }
}
struct FlowDelegates<'a>(&'a FlowLayout);
impl IFlowLayoutAlgorithmDelegates for FlowDelegates<'_> {
    fn algorithm_set_anchor_info(&self, index: i32, offset: f64) {
        self.0.set_layout_anchor_info(index, offset);
    }
    fn algorithm_get_measure_size(
        &self,
        index: i32,
        available: Size,
        _context: &Ref<VirtualizingLayoutContext>,
    ) -> Size {
        self.0.get_measure_size(index, available)
    }
    fn algorithm_get_provisional_arrange_size(
        &self,
        index: i32,
        measure: Size,
        desired: Size,
        _context: &Ref<VirtualizingLayoutContext>,
    ) -> Size {
        self.0.get_provisional_arrange_size(index, measure, desired)
    }
    fn algorithm_should_break_line(&self, index: i32, remaining: f64) -> bool {
        self.0.should_break_line(index, remaining)
    }
    fn algorithm_get_anchor_for_realization_rect(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        self.0.get_anchor_for_realization_rect(available, context)
    }
    fn algorithm_get_anchor_for_target_element(
        &self,
        index: i32,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        self.0
            .get_anchor_for_target_element(index, available, context)
    }
    fn algorithm_get_extent(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        first: FlowLayoutRealizedElement,
        last: FlowLayoutRealizedElement,
    ) -> Rect {
        self.0.get_extent(available, context, first, last)
    }
    fn algorithm_on_element_measured(
        &self,
        element: &Ref<Control>,
        index: i32,
        available: Size,
        measure: Size,
        desired: Size,
        arranged: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) {
        self.0.on_element_measured(
            element, index, available, measure, desired, arranged, context,
        );
    }
    fn algorithm_on_line_arranged(
        &self,
        start: i32,
        count: i32,
        size: f64,
        context: &Ref<VirtualizingLayoutContext>,
    ) {
        self.0.on_line_arranged(start, count, size, context);
    }
    fn algorithm_on_layout_round_factor_changed(&self, _context: &Ref<VirtualizingLayoutContext>) {}
}
