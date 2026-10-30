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
use std::cell::Cell;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UniformGridLayoutItemsJustification {
    #[default]
    Start = 0,
    Center = 1,
    End = 2,
    SpaceAround = 3,
    SpaceBetween = 4,
    SpaceEvenly = 5,
}
impl From<UniformGridLayoutItemsJustification> for FlowLayoutLineAlignment {
    fn from(value: UniformGridLayoutItemsJustification) -> Self {
        match value {
            UniformGridLayoutItemsJustification::Start => Self::Start,
            UniformGridLayoutItemsJustification::Center => Self::Center,
            UniformGridLayoutItemsJustification::End => Self::End,
            UniformGridLayoutItemsJustification::SpaceAround => Self::SpaceAround,
            UniformGridLayoutItemsJustification::SpaceBetween => Self::SpaceBetween,
            UniformGridLayoutItemsJustification::SpaceEvenly => Self::SpaceEvenly,
        }
    }
}
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UniformGridLayoutItemsStretch {
    #[default]
    None = 0,
    Fill = 1,
    Uniform = 2,
}

#[derive(Clone, Copy)]
pub(crate) struct GridLayoutParameters {
    pub item_width: f64,
    pub item_height: f64,
    pub orientation: Orientation,
    pub row_spacing: f64,
    pub column_spacing: f64,
    pub stretch: UniformGridLayoutItemsStretch,
    pub maximum: u32,
}

#[repr(C)]
pub struct UniformGridLayout {
    base: VirtualizingLayout,
    // Upstream's property defaults are zero but the unassigned sizing fields
    // are NaN, so an untouched grid measures its first item.
    layout_item_width: Cell<f64>,
    layout_item_height: Cell<f64>,
}
ferro_class!(UniformGridLayout: VirtualizingLayout);
ferro_class_info!(UniformGridLayout {
    new: UniformGridLayout::new
});
ferro_impl_classes!(UniformGridLayout: LayoutImpl);
impl FerroObjectImpl for UniformGridLayout {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_layout_id("UniformGridLayout".to_string());
        this.update_index_orientation();
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::orientation_property().as_property() {
            this.update_index_orientation();
        }
        if args.property() == Self::min_item_width_property().as_property() {
            this.layout_item_width.set(this.min_item_width());
        }
        if args.property() == Self::min_item_height_property().as_property() {
            this.layout_item_height.set(this.min_item_height());
        }
        this.invalidate_measure();
    }
}
ferro_properties! { impl UniformGridLayout {
    pub fn orientation_property() -> StyledProperty<Orientation> { FerroProperty::register::<UniformGridLayout, _>("Orientation", Orientation::Horizontal) }
    pub fn min_item_width_property() -> StyledProperty<f64> { FerroProperty::register::<UniformGridLayout, _>("MinItemWidth", 0.0) }
    pub fn min_item_height_property() -> StyledProperty<f64> { FerroProperty::register::<UniformGridLayout, _>("MinItemHeight", 0.0) }
    pub fn min_row_spacing_property() -> StyledProperty<f64> { FerroProperty::register::<UniformGridLayout, _>("MinRowSpacing", 0.0) }
    pub fn min_column_spacing_property() -> StyledProperty<f64> { FerroProperty::register::<UniformGridLayout, _>("MinColumnSpacing", 0.0) }
    pub fn items_justification_property() -> StyledProperty<UniformGridLayoutItemsJustification> { FerroProperty::register::<UniformGridLayout, _>("ItemsJustification", UniformGridLayoutItemsJustification::Start) }
    pub fn items_stretch_property() -> StyledProperty<UniformGridLayoutItemsStretch> { FerroProperty::register::<UniformGridLayout, _>("ItemsStretch", UniformGridLayoutItemsStretch::None) }
    pub fn maximum_rows_or_columns_property() -> StyledProperty<i32> { FerroProperty::register::<UniformGridLayout, _>("MaximumRowsOrColumns", -1) }
} }
impl UniformGridLayout {
    pub fn construct() -> Self {
        Self {
            base: VirtualizingLayout::construct(),
            layout_item_width: Cell::new(f64::NAN),
            layout_item_height: Cell::new(f64::NAN),
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
    pub fn min_item_width(&self) -> f64 {
        self.get_value(Self::min_item_width_property())
    }
    pub fn set_min_item_width(&self, value: f64) {
        self.set_value(Self::min_item_width_property(), value);
    }
    pub fn min_item_height(&self) -> f64 {
        self.get_value(Self::min_item_height_property())
    }
    pub fn set_min_item_height(&self, value: f64) {
        self.set_value(Self::min_item_height_property(), value);
    }
    pub fn min_row_spacing(&self) -> f64 {
        self.get_value(Self::min_row_spacing_property())
    }
    pub fn set_min_row_spacing(&self, value: f64) {
        self.set_value(Self::min_row_spacing_property(), value);
    }
    pub fn min_column_spacing(&self) -> f64 {
        self.get_value(Self::min_column_spacing_property())
    }
    pub fn set_min_column_spacing(&self, value: f64) {
        self.set_value(Self::min_column_spacing_property(), value);
    }
    pub fn items_justification(&self) -> UniformGridLayoutItemsJustification {
        self.get_value(Self::items_justification_property())
    }
    pub fn set_items_justification(&self, value: UniformGridLayoutItemsJustification) {
        self.set_value(Self::items_justification_property(), value);
    }
    pub fn items_stretch(&self) -> UniformGridLayoutItemsStretch {
        self.get_value(Self::items_stretch_property())
    }
    pub fn set_items_stretch(&self, value: UniformGridLayoutItemsStretch) {
        self.set_value(Self::items_stretch_property(), value);
    }
    pub fn maximum_rows_or_columns(&self) -> i32 {
        self.get_value(Self::maximum_rows_or_columns_property())
    }
    pub fn set_maximum_rows_or_columns(&self, value: i32) {
        self.set_value(Self::maximum_rows_or_columns_property(), value);
    }
    pub fn state(context: &Ref<VirtualizingLayoutContext>) -> Option<Ref<UniformGridLayoutState>> {
        context
            .layout_state()
            .and_then(|state| ValueTypes::as_object(&*state))
            .and_then(|state| state.cast::<UniformGridLayoutState>())
    }
    fn axis(&self) -> OrientationBasedMeasures {
        OrientationBasedMeasures::new(if self.orientation() == Orientation::Horizontal {
            ScrollOrientation::Vertical
        } else {
            ScrollOrientation::Horizontal
        })
    }
    fn maximum(&self) -> u32 {
        (self.maximum_rows_or_columns() as u32).max(1)
    }
    fn min_item_spacing(&self) -> f64 {
        if self.orientation() == Orientation::Horizontal {
            self.min_column_spacing()
        } else {
            self.min_row_spacing()
        }
    }
    fn line_spacing(&self) -> f64 {
        if self.orientation() == Orientation::Horizontal {
            self.min_row_spacing()
        } else {
            self.min_column_spacing()
        }
    }
    fn update_index_orientation(&self) {
        self.set_index_based_layout_orientation(if self.orientation() == Orientation::Horizontal {
            IndexBasedLayoutOrientation::LeftToRight
        } else {
            IndexBasedLayoutOrientation::TopToBottom
        });
    }
    fn parameters(&self) -> GridLayoutParameters {
        GridLayoutParameters {
            item_width: self.layout_item_width.get(),
            item_height: self.layout_item_height.get(),
            orientation: self.orientation(),
            row_spacing: self.min_row_spacing(),
            column_spacing: self.min_column_spacing(),
            stretch: self.items_stretch(),
            maximum: self.maximum(),
        }
    }
    fn items_per_line(&self, available: Size, state: &UniformGridLayoutState) -> u32 {
        let minor = self.axis().minor(available);
        if minor.is_finite() {
            (((minor + self.min_item_spacing())
                / (self.axis().minor(state.effective_size()) + self.min_item_spacing()))
                as u32)
                .max(1)
                .min(self.maximum())
        } else {
            self.maximum()
        }
    }
    fn major_size(&self, count: i32, per_line: u32, stride: f64) -> f64 {
        if count <= 0 {
            0.0
        } else {
            (count as u32).div_ceil(per_line) as f64 * stride - self.line_spacing()
        }
    }
    fn rect_for_index(&self, available: Size, index: i32, state: &UniformGridLayoutState) -> Rect {
        let axis = self.axis();
        let per_line = self.items_per_line(available, state);
        let line = index as u32 / per_line;
        let in_line = index as u32 % per_line;
        let extent = state.flow_algorithm.last_extent();
        let size = state.effective_size();
        axis.rect(
            in_line as f64 * (axis.minor(size) + self.min_item_spacing())
                + axis.minor_start(extent),
            line as f64 * (axis.major(size) + self.line_spacing()) + axis.major_start(extent),
            axis.minor(size),
            axis.major(size),
        )
    }
}
impl VirtualizingLayoutImpl for UniformGridLayout {
    fn initialize_for_context_core(_this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        let state = Self::state(context).unwrap_or_else(|| {
            assert!(
                context.layout_state().is_none(),
                "LayoutState must derive from UniformGridLayoutState"
            );
            UniformGridLayoutState::new()
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
        state.ensure_element_size(available, context, this.parameters());
        let items_per_line = this.items_per_line(available, &state);
        state
            .flow_algorithm
            .set_uniform_grid(items_per_line, this.axis().minor(state.effective_size()));
        state.flow_algorithm.measure(
            available,
            context,
            FlowLayoutMeasureOptions {
                is_wrapping: true,
                min_item_spacing: this.min_item_spacing(),
                line_spacing: this.line_spacing(),
                max_items_per_line: items_per_line,
                orientation: this.axis().orientation,
                is_virtualization_enabled: true,
            },
            &GridDelegates(this),
        )
    }
    fn arrange_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        let Some(state) = Self::state(context) else {
            return Size::default();
        };
        let size =
            state
                .flow_algorithm
                .arrange(final_size, true, this.items_justification().into());
        state.invalidate_element_size();
        size
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
struct GridDelegates<'a>(&'a UniformGridLayout);
impl IFlowLayoutAlgorithmDelegates for GridDelegates<'_> {
    fn algorithm_set_anchor_info(&self, index: i32, offset: f64) {
        self.0.set_layout_anchor_info(index, offset);
    }
    fn algorithm_get_measure_size(
        &self,
        _index: i32,
        _available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> Size {
        UniformGridLayout::state(context)
            .expect("Grid state must be initialized")
            .effective_size()
    }
    fn algorithm_get_provisional_arrange_size(
        &self,
        _index: i32,
        _measure: Size,
        _desired: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> Size {
        UniformGridLayout::state(context)
            .expect("Grid state must be initialized")
            .effective_size()
    }
    fn algorithm_should_break_line(&self, _index: i32, remaining: f64) -> bool {
        remaining < 0.0
    }
    fn algorithm_get_anchor_for_realization_rect(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo {
        let count = context.item_count();
        let axis = self.0.axis();
        let rect = context.realization_rect();
        if count <= 0 || axis.major_size(rect) <= 0.0 {
            return FlowLayoutAnchorInfo::default();
        }
        let state = UniformGridLayout::state(context).expect("Grid state must be initialized");
        let extent = state.flow_algorithm.last_extent();
        let per_line = self.0.items_per_line(available, &state);
        let stride = axis.major(state.effective_size()) + self.0.line_spacing();
        let major = self.0.major_size(count, per_line, stride);
        let start = axis.major_start(rect) - axis.major_start(extent);
        if start + axis.major_size(rect) < 0.0 || start > major {
            return FlowLayoutAnchorInfo::default();
        }
        let line = if stride == 0.0 {
            0
        } else {
            (start.max(0.0) / stride) as i64
        };
        let index = (line.saturating_mul(per_line as i64)).clamp(0, count as i64 - 1) as i32;
        FlowLayoutAnchorInfo {
            index,
            offset: axis.major_start(self.0.rect_for_index(available, index, &state)),
        }
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
        let state = UniformGridLayout::state(context).expect("Grid state must be initialized");
        FlowLayoutAnchorInfo {
            index,
            offset: self
                .0
                .axis()
                .major_start(self.0.rect_for_index(available, index, &state)),
        }
    }
    fn algorithm_get_extent(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        first: FlowLayoutRealizedElement,
        last: FlowLayoutRealizedElement,
    ) -> Rect {
        let count = context.item_count();
        if count == 0 {
            return Rect::default();
        }
        let state = UniformGridLayout::state(context).expect("Grid state must be initialized");
        let axis = self.0.axis();
        let available_minor = axis.minor(available);
        let minor_stride = axis.minor(state.effective_size()) + self.0.min_item_spacing();
        let major_stride = axis.major(state.effective_size()) + self.0.line_spacing();
        let per_line = if available_minor.is_finite() {
            (((available_minor + self.0.min_item_spacing()) / minor_stride) as u32).max(1)
        } else {
            count as u32
        }
        .min(self.0.maximum());
        let minor = if available_minor.is_finite()
            && self.0.items_stretch() == UniformGridLayoutItemsStretch::Fill
        {
            available_minor
        } else {
            (per_line as f64 * minor_stride - self.0.min_item_spacing()).max(0.0)
        };
        let mut start = 0.0;
        let mut major = self.0.major_size(count, per_line, major_stride);
        if first.element.is_some() {
            start = axis.major_start(first.bounds)
                - (first.index as u32 / per_line) as f64 * major_stride;
            let remainder_on_line = (count - last.index - 1)
                .min((per_line - ((last.index as u32 + 1) % per_line)) as i32);
            let remaining = count - last.index - 1 - remainder_on_line;
            let remaining_major = self.0.major_size(remaining, per_line, major_stride);
            major = axis.major_end(last.bounds) - start
                + if remaining_major > 0.0 {
                    self.0.line_spacing() + remaining_major
                } else {
                    0.0
                };
        }
        axis.rect(0.0, start, minor, major.max(0.0))
    }
    fn algorithm_on_element_measured(
        &self,
        _element: &Ref<Control>,
        _index: i32,
        _available: Size,
        _measure: Size,
        _desired: Size,
        _arranged: Size,
        _context: &Ref<VirtualizingLayoutContext>,
    ) {
    }
    fn algorithm_on_line_arranged(
        &self,
        _start: i32,
        _count: i32,
        _size: f64,
        _context: &Ref<VirtualizingLayoutContext>,
    ) {
    }
    fn algorithm_on_layout_round_factor_changed(&self, _context: &Ref<VirtualizingLayoutContext>) {}
}
