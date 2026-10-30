// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::{
    ElementManager, ElementRealizationOptions, OrientationBasedMeasures, ScrollOrientation,
    VirtualizingLayoutContext,
};
use ferroui_controls::{Control, ItemsChangedEventArgs, TopLevel};
use ferroui_base::{Point, Rect, Ref, Size, WeakRef};
use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLayoutAnchorInfo {
    pub index: i32,
    pub offset: f64,
}
impl Default for FlowLayoutAnchorInfo {
    fn default() -> Self {
        Self {
            index: -1,
            offset: f64::NAN,
        }
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FlowLayoutLineAlignment {
    #[default]
    Start = 0,
    Center = 1,
    End = 2,
    SpaceAround = 3,
    SpaceBetween = 4,
    SpaceEvenly = 5,
}

/// Callback information for one end of the realized range.
#[derive(Clone)]
pub struct FlowLayoutRealizedElement {
    pub element: Option<Ref<Control>>,
    pub index: i32,
    pub bounds: Rect,
}
impl Default for FlowLayoutRealizedElement {
    fn default() -> Self {
        Self {
            element: None,
            index: -1,
            bounds: Rect::default(),
        }
    }
}

/// The policy hooks used by Stack, Flow and UniformGrid layouts.
pub trait IFlowLayoutAlgorithmDelegates {
    fn algorithm_get_measure_size(
        &self,
        index: i32,
        available_size: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> Size;
    fn algorithm_get_provisional_arrange_size(
        &self,
        index: i32,
        measure_size: Size,
        desired_size: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> Size;
    fn algorithm_should_break_line(&self, index: i32, remaining_space: f64) -> bool;
    fn algorithm_get_anchor_for_realization_rect(
        &self,
        available_size: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo;
    fn algorithm_get_anchor_for_target_element(
        &self,
        index: i32,
        available_size: Size,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> FlowLayoutAnchorInfo;
    fn algorithm_get_extent(
        &self,
        available_size: Size,
        context: &Ref<VirtualizingLayoutContext>,
        first: FlowLayoutRealizedElement,
        last: FlowLayoutRealizedElement,
    ) -> Rect;
    fn algorithm_on_element_measured(
        &self,
        element: &Ref<Control>,
        index: i32,
        available_size: Size,
        measure_size: Size,
        desired_size: Size,
        provisional_arrange_size: Size,
        context: &Ref<VirtualizingLayoutContext>,
    );
    fn algorithm_on_line_arranged(
        &self,
        start_index: i32,
        count: i32,
        line_size: f64,
        context: &Ref<VirtualizingLayoutContext>,
    );
    fn algorithm_on_layout_round_factor_changed(&self, context: &Ref<VirtualizingLayoutContext>);
    fn algorithm_set_anchor_info(&self, index: i32, offset: f64);
}

#[derive(Clone, Copy)]
pub struct FlowLayoutMeasureOptions {
    pub is_wrapping: bool,
    pub min_item_spacing: f64,
    pub line_spacing: f64,
    pub max_items_per_line: u32,
    pub orientation: ScrollOrientation,
    pub is_virtualization_enabled: bool,
}

/// The bidirectional realization and line-arrangement engine shared by the
/// three WinUI flow-based layouts. Callbacks are borrowed for each operation
/// instead of stored as an unowned C++ pointer.
pub struct FlowLayoutAlgorithm {
    pub element_manager: ElementManager,
    context: RefCell<Option<WeakRef<VirtualizingLayoutContext>>>,
    last_available_size: Cell<Size>,
    last_item_spacing: Cell<f64>,
    layout_round_factor: Cell<f64>,
    collection_change_pending: Cell<bool>,
    last_extent: Cell<Rect>,
    first_inside: Cell<i32>,
    last_inside: Cell<i32>,
    scroll_same_as_flow: Cell<bool>,
    orientation: Cell<ScrollOrientation>,
    // Deviation (winui-repeater/DEVIATIONS.md, WR-33): column-based placement and whole-row
    // justification for UniformGrid.
    // UniformGrid can retain only part of a row at either realization edge.
    // Its source-index columns still refer to the entire logical row.
    uniform_grid: Cell<Option<(u32, f64)>>,
}
impl Default for FlowLayoutAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}
impl FlowLayoutAlgorithm {
    pub fn new() -> Self {
        Self {
            element_manager: ElementManager::new(),
            context: RefCell::new(None),
            last_available_size: Cell::new(Size::default()),
            last_item_spacing: Cell::new(0.0),
            layout_round_factor: Cell::new(0.0),
            collection_change_pending: Cell::new(false),
            last_extent: Cell::new(Rect::default()),
            first_inside: Cell::new(-1),
            last_inside: Cell::new(-1),
            scroll_same_as_flow: Cell::new(false),
            orientation: Cell::new(ScrollOrientation::Vertical),
            uniform_grid: Cell::new(None),
        }
    }
    pub fn initialize_for_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        *self.context.borrow_mut() = Some(context.downgrade());
        self.element_manager.set_context(context);
    }
    pub fn uninitialize_for_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        self.element_manager.clear_realized_range();
        context.set_layout_state(None);
        self.context.borrow_mut().take();
    }
    fn context(&self) -> Ref<VirtualizingLayoutContext> {
        self.context
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .expect("Flow layout must be initialized before measuring")
    }
    fn axis(&self) -> OrientationBasedMeasures {
        OrientationBasedMeasures::new(self.orientation.get())
    }
    pub fn last_extent(&self) -> Rect {
        self.last_extent.get()
    }
    pub(crate) fn set_uniform_grid(&self, items_per_line: u32, minor_size: f64) {
        self.uniform_grid
            .set(Some((items_per_line.max(1), minor_size)));
    }
    fn grid_minor_position(&self, index: i32, spacing: f64) -> Option<f64> {
        self.uniform_grid.get().map(|(items_per_line, minor_size)| {
            (index as u32 % items_per_line) as f64 * (minor_size + spacing)
        })
    }
    pub fn get_element_if_realized(&self, index: i32) -> Option<Ref<Control>> {
        self.element_manager.get_element_if_realized(index)
    }
    pub fn on_items_source_changed(&self, args: &ItemsChangedEventArgs<'_>) {
        self.element_manager.data_source_changed(args);
        self.collection_change_pending.set(true);
    }
    pub fn measure(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        options: FlowLayoutMeasureOptions,
        callbacks: &dyn IFlowLayoutAlgorithmDelegates,
    ) -> Size {
        self.orientation.set(options.orientation);
        self.scroll_same_as_flow
            .set(self.axis().minor(available) == f64::INFINITY);
        let suggested = context.recommended_anchor_index();
        if self.element_manager.is_index_valid_in_data(suggested)
            && !self.element_manager.is_data_index_realized(suggested)
        {
            self.make_anchor(context, suggested, available, callbacks);
        }
        // Nonvirtualizing bounds also need allocation when realization is disabled.
        if options.is_virtualization_enabled || !context.is_virtualizing() {
            self.element_manager.on_begin_measure(options.orientation);
        }
        let anchor = self.get_anchor_index(available, options, callbacks);
        self.generate(true, anchor, available, options, callbacks);
        self.generate(false, anchor, available, options, callbacks);
        if options.is_wrapping
            && self.element_manager.realized_element_count() > 0
            && self.element_manager.data_index_from_range_index(0) == 0
            && self
                .axis()
                .minor_start(self.element_manager.layout_bounds_for_range_index(0))
                != 0.0
        {
            let bounds = self.element_manager.layout_bounds_for_range_index(0);
            self.element_manager
                .set_layout_bounds_for_range_index(0, self.axis().with_minor_start(bounds, 0.0));
            self.generate(true, 0, available, options, callbacks);
        }
        self.raise_line_arranged(callbacks);
        self.collection_change_pending.set(false);
        let mut extent = self.estimate_extent(available, callbacks);
        let scale = self.layout_round_factor.get();
        if scale > 0.0 {
            let round = |value: f64| (value * scale).round() / scale;
            extent = Rect::new(
                round(extent.x),
                round(extent.y),
                round(extent.width),
                round(extent.height),
            );
        }
        self.last_extent.set(extent);
        if context.is_virtualizing() {
            context.set_layout_origin(Point::new(extent.x, extent.y));
        }
        extent.size()
    }
    pub fn arrange(
        &self,
        final_size: Size,
        is_wrapping: bool,
        alignment: FlowLayoutLineAlignment,
    ) -> Size {
        let count = self.element_manager.realized_element_count();
        let axis = self.axis();
        if count > 0 {
            let mut first = 0;
            let mut previous = self.element_manager.layout_bounds_for_range_index(0);
            let mut line_major = axis.major_size(previous);
            for index in 1..=count {
                let current = (index < count)
                    .then(|| self.element_manager.layout_bounds_for_range_index(index));
                if current.is_none_or(|rect| axis.major_start(rect) != axis.major_start(previous)) {
                    let start =
                        axis.minor_start(self.element_manager.layout_bounds_for_range_index(first));
                    let end = axis.minor(final_size) - axis.minor_end(previous);
                    self.perform_line_alignment(
                        first,
                        index - first,
                        start,
                        end,
                        line_major,
                        alignment,
                        is_wrapping,
                        final_size,
                    );
                    first = index;
                    line_major = 0.0;
                }
                if let Some(current) = current {
                    line_major = line_major.max(axis.major_size(current));
                    previous = current;
                }
            }
        }
        Size::new(
            final_size.width.max(self.last_extent.get().width),
            final_size.height.max(self.last_extent.get().height),
        )
    }
    fn make_anchor(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        index: i32,
        available: Size,
        callbacks: &dyn IFlowLayoutAlgorithmDelegates,
    ) {
        self.element_manager.clear_realized_range();
        let anchor = callbacks.algorithm_get_anchor_for_target_element(index, available, context);
        if anchor.index < 0 {
            return;
        }
        for data_index in anchor.index..=index {
            let element = context.get_or_create_element_at_with_options(
                data_index,
                ElementRealizationOptions::FORCE_CREATE
                    | ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE,
            );
            element.measure(callbacks.algorithm_get_measure_size(data_index, available, context));
            self.element_manager.add(element, data_index);
        }
    }
    pub fn measure_element(
        &self,
        element: &Ref<Control>,
        index: i32,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        callbacks: &dyn IFlowLayoutAlgorithmDelegates,
    ) -> Size {
        if element.use_layout_rounding() {
            if let Some(root) = element
                .visual_root()
                .and_then(|root| root.cast::<TopLevel>())
            {
                let factor = root.render_scaling();
                if self.layout_round_factor.replace(factor) != factor {
                    callbacks.algorithm_on_layout_round_factor_changed(context);
                }
            } else {
                self.layout_round_factor.set(0.0);
            }
        } else {
            self.layout_round_factor.set(0.0);
        }
        let measure = callbacks.algorithm_get_measure_size(index, available, context);
        element.measure(measure);
        let desired = element.desired_size();
        let arranged =
            callbacks.algorithm_get_provisional_arrange_size(index, measure, desired, context);
        callbacks.algorithm_on_element_measured(
            element, index, available, measure, desired, arranged, context,
        );
        arranged
    }
    fn get_anchor_index(
        &self,
        available: Size,
        options: FlowLayoutMeasureOptions,
        callbacks: &dyn IFlowLayoutAlgorithmDelegates,
    ) -> i32 {
        let context = self.context();
        let axis = self.axis();
        let mut position = Point::default();
        let anchor;
        if !context.is_virtualizing() || !options.is_virtualization_enabled {
            anchor = if context.item_count() > 0 { 0 } else { -1 };
        } else {
            let connected = self.element_manager.is_window_connected(
                context.realization_rect(),
                self.orientation.get(),
                self.scroll_same_as_flow.get(),
            );
            let reevaluate = options.is_wrapping
                && (axis.minor(self.last_available_size.get()) != axis.minor(available)
                    || self.last_item_spacing.get() != options.min_item_spacing
                    || self.collection_change_pending.get());
            let suggested = context.recommended_anchor_index();
            if suggested >= 0 && self.element_manager.is_data_index_realized(suggested) {
                anchor = callbacks
                    .algorithm_get_anchor_for_target_element(suggested, available, &context)
                    .index;
                if self.element_manager.is_data_index_realized(anchor) {
                    let bounds = self.element_manager.layout_bounds_for_data_index(anchor);
                    position = if reevaluate {
                        axis.point(0.0, axis.major_start(bounds))
                    } else {
                        Point::new(bounds.x, bounds.y)
                    };
                } else if anchor >= 0 {
                    let first = self.element_manager.data_index_from_range_index(0);
                    for index in (anchor..first).rev() {
                        self.element_manager.ensure_element_realized(false, index);
                    }
                    position = axis.point(
                        0.0,
                        axis.major_start(
                            self.element_manager.layout_bounds_for_data_index(suggested),
                        ),
                    );
                }
            } else if reevaluate || !connected {
                let info = callbacks.algorithm_get_anchor_for_realization_rect(available, &context);
                anchor = info.index;
                position = axis.point(0.0, info.offset);
            } else {
                anchor = self.element_manager.data_index_from_range_index(0);
                if self.element_manager.is_layout_bounds_set(0) {
                    let bounds = self.element_manager.layout_bounds_for_range_index(0);
                    position = Point::new(bounds.x, bounds.y);
                }
            }
        }
        self.first_inside.set(anchor);
        self.last_inside.set(anchor);
        if self.element_manager.is_index_valid_in_data(anchor) {
            // UniformGrid intentionally returns the target itself rather than
            // the first item in its row: only one offscreen edge element needs
            // to survive recycling. Restore that target's actual column when
            // it is newly made or its source index changes after insertion.
            if let Some(minor) = self.grid_minor_position(anchor, options.min_item_spacing) {
                position = axis.point(minor, axis.major_point(position));
            }
            if !self.element_manager.is_data_index_realized(anchor) {
                self.element_manager.clear_realized_range();
                let element = context.get_or_create_element_at_with_options(
                    anchor,
                    ElementRealizationOptions::FORCE_CREATE
                        | ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE,
                );
                self.element_manager.add(element, anchor);
            }
            let element = self.element_manager.get_realized_element(anchor);
            let size = self.measure_element(&element, anchor, available, &context, callbacks);
            self.element_manager.set_layout_bounds_for_data_index(
                anchor,
                Rect::new(position.x, position.y, size.width, size.height),
            );
        } else {
            self.element_manager.clear_realized_range();
        }
        self.last_available_size.set(available);
        self.last_item_spacing.set(options.min_item_spacing);
        callbacks.algorithm_set_anchor_info(anchor, axis.major_point(position));
        anchor
    }
    fn generate(
        &self,
        forward: bool,
        anchor: i32,
        available: Size,
        options: FlowLayoutMeasureOptions,
        callbacks: &dyn IFlowLayoutAlgorithmDelegates,
    ) {
        if anchor < 0 {
            return;
        }
        let context = self.context();
        let axis = self.axis();
        let step = if forward { 1 } else { -1 };
        let mut previous_index = anchor;
        let mut current_index = anchor + step;
        let anchor_bounds = self.element_manager.layout_bounds_for_data_index(anchor);
        let mut line_offset = axis.major_start(anchor_bounds);
        let mut line_major = axis.major_size(anchor_bounds);
        let mut count_in_line = 1;
        let mut reposition = false;
        let maximum = options.max_items_per_line.max(1);
        if maximum != u32::MAX {
            let in_line = anchor as u32 % maximum;
            count_in_line = if forward {
                in_line + 1
            } else {
                maximum - in_line
            };
        }
        while self.element_manager.is_index_valid_in_data(current_index)
            && (!options.is_virtualization_enabled
                || self.should_continue_filling(previous_index, forward))
        {
            self.element_manager
                .ensure_element_realized(forward, current_index);
            let element = self.element_manager.get_realized_element(current_index);
            let size =
                self.measure_element(&element, current_index, available, &context, callbacks);
            let previous = self
                .element_manager
                .layout_bounds_for_data_index(previous_index);
            let mut bounds = Rect::from_size(size);
            if forward {
                let remaining = axis.minor(available)
                    - axis.minor_end(previous)
                    - options.min_item_spacing
                    - axis.minor(size);
                if count_in_line >= maximum
                    || callbacks.algorithm_should_break_line(current_index, remaining)
                {
                    bounds = axis.with_major_start(
                        bounds,
                        axis.major_start(previous) + line_major + options.line_spacing,
                    );
                    if reposition {
                        for i in 0..count_in_line {
                            let index = current_index - 1 - i as i32;
                            if self.element_manager.is_data_index_realized(index) {
                                let rect = self.element_manager.layout_bounds_for_data_index(index);
                                self.element_manager.set_layout_bounds_for_data_index(
                                    index,
                                    axis.with_major_size(rect, line_major),
                                );
                            }
                        }
                    }
                    line_major = axis.major_size(bounds);
                    line_offset = axis.major_start(bounds);
                    reposition = false;
                    count_in_line = 1;
                } else {
                    bounds = axis.with_minor_start(
                        bounds,
                        axis.minor_end(previous) + options.min_item_spacing,
                    );
                    bounds = axis.with_major_start(bounds, line_offset);
                    line_major = line_major.max(axis.major_size(bounds));
                    reposition = axis.major_size(previous) != axis.major_size(bounds);
                    count_in_line += 1;
                }
            } else {
                let remaining =
                    axis.minor_start(previous) - axis.minor(size) - options.min_item_spacing;
                if count_in_line >= maximum
                    || callbacks.algorithm_should_break_line(current_index, remaining)
                {
                    if options.is_wrapping {
                        let minor = if axis.minor(available).is_finite() {
                            axis.minor(available)
                        } else {
                            axis.minor_size(self.last_extent.get())
                        };
                        bounds = axis.with_minor_start(bounds, minor - axis.minor(size));
                    }
                    bounds = axis.with_major_start(
                        bounds,
                        line_offset - axis.major(size) - options.line_spacing,
                    );
                    if reposition {
                        let previous_line_index = current_index + count_in_line as i32 + 1;
                        if self
                            .element_manager
                            .is_data_index_realized(previous_line_index)
                        {
                            let previous_line_offset = axis.major_start(
                                self.element_manager
                                    .layout_bounds_for_data_index(previous_line_index),
                            );
                            for i in 0..count_in_line {
                                let index = current_index + 1 + i as i32;
                                if index != anchor
                                    && self.element_manager.is_data_index_realized(index)
                                {
                                    let rect =
                                        self.element_manager.layout_bounds_for_data_index(index);
                                    let rect = axis.with_major_start(
                                        rect,
                                        previous_line_offset - line_major - options.line_spacing,
                                    );
                                    self.element_manager.set_layout_bounds_for_data_index(
                                        index,
                                        axis.with_major_size(rect, line_major),
                                    );
                                }
                            }
                        }
                    }
                    line_major = axis.major_size(bounds);
                    line_offset = axis.major_start(bounds);
                    reposition = false;
                    count_in_line = 1;
                } else {
                    bounds = axis.with_minor_start(
                        bounds,
                        axis.minor_start(previous) - axis.minor(size) - options.min_item_spacing,
                    );
                    bounds = axis.with_major_start(bounds, line_offset);
                    line_major = line_major.max(axis.major_size(bounds));
                    reposition = axis.major_size(previous) != axis.major_size(bounds);
                    count_in_line += 1;
                }
            }
            if let Some(minor) = self.grid_minor_position(current_index, options.min_item_spacing) {
                bounds = axis.with_minor_start(bounds, minor);
            }
            self.element_manager
                .set_layout_bounds_for_data_index(current_index, bounds);
            previous_index = current_index;
            current_index += step;
        }
        let count = context.item_count();
        if forward {
            self.last_inside.set(if previous_index == count - 1 {
                count - 1
            } else {
                (previous_index - 1).max(0)
            });
        } else {
            self.first_inside.set(if previous_index == 0 {
                0
            } else {
                (previous_index + 1).min(count - 1)
            });
        }
        self.element_manager
            .discard_elements_outside_window(forward, current_index);
    }
    fn should_continue_filling(&self, index: i32, forward: bool) -> bool {
        let context = self.context();
        if !context.is_virtualizing() {
            return true;
        }
        let axis = self.axis();
        let window = context.realization_rect();
        let bounds = self.element_manager.layout_bounds_for_data_index(index);
        if forward {
            axis.major_start(bounds) < axis.major_end(window)
                && axis.minor_start(bounds) < axis.minor_end(window)
        } else {
            axis.major_end(bounds) > axis.major_start(window)
                && axis.minor_end(bounds) > axis.minor_start(window)
        }
    }
    fn estimate_extent(
        &self,
        available: Size,
        callbacks: &dyn IFlowLayoutAlgorithmDelegates,
    ) -> Rect {
        let count = self.element_manager.realized_element_count();
        let at = |index: i32| FlowLayoutRealizedElement {
            element: Some(self.element_manager.get_at(index)),
            index: self.element_manager.data_index_from_range_index(index),
            bounds: self.element_manager.layout_bounds_for_range_index(index),
        };
        let (first, last) = if count > 0 {
            (at(0), at(count - 1))
        } else {
            (
                FlowLayoutRealizedElement::default(),
                FlowLayoutRealizedElement::default(),
            )
        };
        callbacks.algorithm_get_extent(available, &self.context(), first, last)
    }
    fn raise_line_arranged(&self, callbacks: &dyn IFlowLayoutAlgorithmDelegates) {
        let context = self.context();
        let window = context.realization_rect();
        if window.width == 0.0 && window.height == 0.0 {
            return;
        }
        let first = self.first_inside.get();
        let last = self.last_inside.get();
        if first < 0 || last < first || !self.element_manager.is_data_index_realized(first) {
            return;
        }
        let axis = self.axis();
        let mut previous = self.element_manager.layout_bounds_for_data_index(first);
        let mut start = first;
        let mut major = axis.major_size(previous);
        for index in first + 1..=last + 1 {
            let current =
                (index <= last).then(|| self.element_manager.layout_bounds_for_data_index(index));
            if current.is_none_or(|rect| axis.major_start(rect) != axis.major_start(previous)) {
                callbacks.algorithm_on_line_arranged(start, index - start, major, &context);
                start = index;
                major = 0.0;
            }
            if let Some(current) = current {
                major = major.max(axis.major_size(current));
                previous = current;
            }
        }
    }
    fn perform_line_alignment(
        &self,
        first: i32,
        count: i32,
        space_start: f64,
        space_end: f64,
        major: f64,
        alignment: FlowLayoutLineAlignment,
        wrapping: bool,
        final_size: Size,
    ) {
        let axis = self.axis();
        let extent = self.last_extent.get();
        let uniform_grid = self
            .uniform_grid
            .get()
            .map(|metrics| (metrics, self.context().item_count() as u32));
        for index in first..first + count {
            let mut bounds = axis.with_major_size(
                self.element_manager.layout_bounds_for_range_index(index),
                major,
            );
            let (alignment_index, alignment_count, start, end) =
                if let Some(((items_per_line, minor_size), item_count)) = uniform_grid {
                    let data_index = self.element_manager.data_index_from_range_index(index) as u32;
                    let column = data_index % items_per_line;
                    let row_start = data_index - column;
                    let row_count = (item_count - row_start).min(items_per_line);
                    let spacing = self.last_item_spacing.get();
                    let width = row_count as f64 * minor_size
                        + row_count.saturating_sub(1) as f64 * spacing;
                    bounds = axis.with_minor_start(bounds, column as f64 * (minor_size + spacing));
                    // Missing offscreen siblings are logical members of this
                    // line; their columns and justification cannot be folded
                    // into the remaining realized children.
                    (column, row_count, 0.0, axis.minor(final_size) - width)
                } else {
                    ((index - first) as u32, count as u32, space_start, space_end)
                };
            if !self.scroll_same_as_flow.get() && (start != 0.0 || end != 0.0) {
                let total = start + end;
                let i = alignment_index as f64;
                let count = alignment_count;
                let shift = match alignment {
                    FlowLayoutLineAlignment::Start => -start,
                    FlowLayoutLineAlignment::End => end,
                    FlowLayoutLineAlignment::Center => -start + total / 2.0,
                    FlowLayoutLineAlignment::SpaceAround => {
                        -start + total / (count as f64 * 2.0) * (i * 2.0 + 1.0)
                    }
                    FlowLayoutLineAlignment::SpaceBetween => {
                        -start
                            + if count > 1 {
                                total / (count - 1) as f64 * i
                            } else {
                                0.0
                            }
                    }
                    FlowLayoutLineAlignment::SpaceEvenly => {
                        -start + total / (count + 1) as f64 * (i + 1.0)
                    }
                };
                bounds = axis.with_minor_start(bounds, axis.minor_start(bounds) + shift);
            }
            bounds = Rect::new(
                bounds.x - extent.x,
                bounds.y - extent.y,
                bounds.width,
                bounds.height,
            );
            if !wrapping {
                bounds = axis
                    .with_minor_size(bounds, axis.minor_size(bounds).max(axis.minor(final_size)));
            }
            let element = self.element_manager.get_at(index);
            element.arrange(bounds);
        }
    }
}
