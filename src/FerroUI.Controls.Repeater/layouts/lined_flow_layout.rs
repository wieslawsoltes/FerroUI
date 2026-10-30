// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::lined_flow_layout_algorithm::{
    compute_fast_path, compute_regular_path, displayed_line_range, finish_line, frozen_line_range,
    line_scale, ItemMetrics, LineOptions, LinedFlowLine,
};
use super::lined_flow_layout_items_info_requested_event_args::LinedFlowItemsInfo;
use super::*;
use crate::{ItemCollectionTransitionProvider, LinedFlowLayoutItemCollectionTransitionProvider};
use ferroui_controls::{Control, ItemsChangedEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::layout::LayoutHelper;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size,
    StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinedFlowLayoutItemsJustification {
    #[default]
    Start = 0,
    Center = 1,
    End = 2,
    SpaceAround = 3,
    SpaceBetween = 4,
    SpaceEvenly = 5,
}
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinedFlowLayoutItemsStretch {
    #[default]
    None = 0,
    Fill = 1,
}

/// A horizontally flowing layout with a fixed line height, proportional item
/// widths, bounded aspect-ratio history, and stable visible lines.
///
/// Unlike Stack, Flow and UniformGrid layouts this layout is unshareable. Its
/// state belongs to the layout, as in the WinUI class.
#[repr(C)]
pub struct LinedFlowLayout {
    base: VirtualizingLayout,
    context: RefCell<Option<WeakRef<VirtualizingLayoutContext>>>,
    element_manager: ElementManager,
    item_count: Cell<i32>,
    fast_path_layout: Cell<bool>,
    fast_path_supported: Cell<bool>,
    forced_aspect_ratio: Cell<f64>,
    forced_average_divider: Cell<f64>,
    forced_wrap_multiplier: Cell<f64>,
    diagnostic_average_ratio: Cell<f64>,
    current_available_width: Cell<f64>,
    realization_shift: Cell<f64>,
    measure_metrics: RefCell<BTreeMap<i32, ItemMetrics>>,
    actual_line_height: Cell<f64>,
    average_items_per_line: Cell<(f64, f64)>,
    aspect_ratios: RefCell<LinedFlowLayoutItemAspectRatios>,
    items_info: RefCell<Rc<LinedFlowItemsInfo>>,
    lines: RefCell<Rc<Vec<LinedFlowLine>>>,
    measured_widths: RefCell<BTreeMap<i32, f64>>,
    locked_items: RefCell<BTreeMap<i32, i32>>,
    first_or_last_locked: Cell<bool>,
    items_info_requested: HandlerList<dyn Fn(&LinedFlowLayoutItemsInfoRequestedEventArgs)>,
    items_unlocked: HandlerList<dyn Fn()>,
    first_frozen_line: Cell<i32>,
    last_frozen_line: Cell<i32>,
    previous_available_width: Cell<f64>,
    max_line_width: Cell<f64>,
    force_relayout: Cell<bool>,
    measure_countdown: Cell<i32>,
    anchor_index: Cell<i32>,
    anchor_retention: Cell<i32>,
    rounding_scale: Cell<f64>,
    context_generation: Cell<u64>,
    async_invalidation_pending: Cell<bool>,
    timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    timer_tick_count: Cell<i32>,
}
ferro_class!(LinedFlowLayout: VirtualizingLayout);
ferro_class_info!(LinedFlowLayout {
    new: LinedFlowLayout::new
});
impl FerroObjectImpl for LinedFlowLayout {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_layout_id("LinedFlowLayout".to_string());
        this.set_index_based_layout_orientation(IndexBasedLayoutOrientation::LeftToRight);
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() != Self::actual_line_height_property().as_property() {
            this.invalidate_layout();
        }
    }
}
impl LayoutImpl for LinedFlowLayout {
    fn create_default_item_transition_provider(
        _this: &Self,
    ) -> Option<Ref<ItemCollectionTransitionProvider>> {
        Some(LinedFlowLayoutItemCollectionTransitionProvider::new().upcast())
    }
}
ferro_properties! { impl LinedFlowLayout {
    pub fn items_justification_property() -> StyledProperty<LinedFlowLayoutItemsJustification> { FerroProperty::register::<LinedFlowLayout, _>("ItemsJustification", LinedFlowLayoutItemsJustification::Start) }
    pub fn items_stretch_property() -> StyledProperty<LinedFlowLayoutItemsStretch> { FerroProperty::register::<LinedFlowLayout, _>("ItemsStretch", LinedFlowLayoutItemsStretch::None) }
    pub fn min_item_spacing_property() -> StyledProperty<f64> { FerroProperty::register::<LinedFlowLayout, _>("MinItemSpacing", 0.0) }
    pub fn line_spacing_property() -> StyledProperty<f64> { FerroProperty::register::<LinedFlowLayout, _>("LineSpacing", 0.0) }
    pub fn line_height_property() -> StyledProperty<f64> { FerroProperty::register::<LinedFlowLayout, _>("LineHeight", f64::NAN) }
    pub fn actual_line_height_property() -> DirectProperty<LinedFlowLayout, f64> { FerroProperty::register_direct::<LinedFlowLayout, _>("ActualLineHeight", |this| this.actual_line_height.get(), None, 0.0) }
} }
impl LinedFlowLayout {
    pub fn construct() -> Self {
        Self {
            base: VirtualizingLayout::construct(),
            context: RefCell::new(None),
            element_manager: ElementManager::new(),
            item_count: Cell::new(0),
            fast_path_layout: Cell::new(false),
            fast_path_supported: Cell::new(true),
            forced_aspect_ratio: Cell::new(0.0),
            forced_average_divider: Cell::new(0.0),
            forced_wrap_multiplier: Cell::new(0.0),
            diagnostic_average_ratio: Cell::new(0.0),
            current_available_width: Cell::new(0.0),
            realization_shift: Cell::new(0.0),
            measure_metrics: RefCell::new(BTreeMap::new()),
            actual_line_height: Cell::new(0.0),
            average_items_per_line: Cell::new((0.0, 0.0)),
            aspect_ratios: RefCell::new(LinedFlowLayoutItemAspectRatios::new()),
            items_info: RefCell::new(Rc::new(LinedFlowItemsInfo::default())),
            lines: RefCell::new(Rc::new(Vec::new())),
            measured_widths: RefCell::new(BTreeMap::new()),
            locked_items: RefCell::new(BTreeMap::new()),
            first_or_last_locked: Cell::new(false),
            items_info_requested: HandlerList::new(),
            items_unlocked: HandlerList::new(),
            first_frozen_line: Cell::new(-1),
            last_frozen_line: Cell::new(-1),
            previous_available_width: Cell::new(0.0),
            max_line_width: Cell::new(0.0),
            force_relayout: Cell::new(true),
            measure_countdown: Cell::new(5),
            anchor_index: Cell::new(-1),
            anchor_retention: Cell::new(0),
            rounding_scale: Cell::new(1.0),
            context_generation: Cell::new(0),
            async_invalidation_pending: Cell::new(false),
            timer: RefCell::new(None),
            timer_tick_count: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items_justification(&self) -> LinedFlowLayoutItemsJustification {
        self.get_value(Self::items_justification_property())
    }
    pub fn set_items_justification(&self, value: LinedFlowLayoutItemsJustification) {
        self.set_value(Self::items_justification_property(), value);
    }
    pub fn items_stretch(&self) -> LinedFlowLayoutItemsStretch {
        self.get_value(Self::items_stretch_property())
    }
    pub fn set_items_stretch(&self, value: LinedFlowLayoutItemsStretch) {
        self.set_value(Self::items_stretch_property(), value);
    }
    pub fn min_item_spacing(&self) -> f64 {
        self.get_value(Self::min_item_spacing_property())
    }
    pub fn set_min_item_spacing(&self, value: f64) {
        self.set_value(Self::min_item_spacing_property(), value);
    }
    pub fn line_spacing(&self) -> f64 {
        self.get_value(Self::line_spacing_property())
    }
    pub fn set_line_spacing(&self, value: f64) {
        self.set_value(Self::line_spacing_property(), value);
    }
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }
    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value);
    }
    pub fn actual_line_height(&self) -> f64 {
        self.actual_line_height.get()
    }
    pub fn requested_range_start_index(&self) -> i32 {
        if self.uses_fast_path_layout() {
            0
        } else {
            self.items_info.borrow().start
        }
    }
    pub fn requested_range_length(&self) -> i32 {
        if self.uses_fast_path_layout() {
            self.item_count.get()
        } else {
            self.items_info.borrow().ratios.len() as i32
        }
    }
    pub fn items_info_requested(
        &self,
        handler: impl Fn(&LinedFlowLayoutItemsInfoRequestedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.items_info_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.items_info_requested.remove(token);
            }
        })
    }
    pub fn items_unlocked(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.items_unlocked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.items_unlocked.remove(token);
            }
        })
    }
    pub fn invalidate_items_info(&self) {
        *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
        self.invalidate_layout();
    }
    pub(crate) fn invalidate_layout(&self) {
        self.force_relayout.set(true);
        LayoutsTestHooks::notify_invalidated(
            &self.to_ref().upcast(),
            LinedFlowLayoutInvalidationTrigger::InvalidateLayoutCall,
        );
        self.invalidate_measure();
    }
    pub(crate) fn diagnostic_average_aspect_ratio(&self) -> f64 {
        self.diagnostic_average_ratio.get()
    }
    pub(crate) fn forced_average_divider(&self) -> f64 {
        self.forced_average_divider.get()
    }
    pub(crate) fn set_forced_average_divider(&self, value: f64) {
        if self.forced_average_divider.replace(value) != value {
            self.invalidate_layout();
        }
    }
    pub(crate) fn forced_aspect_ratio(&self) -> f64 {
        self.forced_aspect_ratio.get()
    }
    pub(crate) fn set_forced_aspect_ratio(&self, value: f64) {
        if self.forced_aspect_ratio.replace(value) != value {
            self.invalidate_layout();
        }
    }
    pub(crate) fn forced_wrap_multiplier(&self) -> f64 {
        self.forced_wrap_multiplier.get()
    }
    pub(crate) fn set_forced_wrap_multiplier(&self, value: f64) {
        if self.forced_wrap_multiplier.replace(value) != value {
            self.invalidate_layout();
        }
    }
    pub(crate) fn is_fast_path_supported(&self) -> bool {
        self.fast_path_supported.get()
    }
    pub(crate) fn set_fast_path_supported(&self, value: bool) {
        if self.fast_path_supported.replace(value) != value {
            if self.uses_fast_path_layout() {
                self.fast_path_layout.set(false);
                *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
            }
            self.invalidate_layout();
        }
    }
    pub(crate) fn diagnostic_line_index(&self, index: i32) -> i32 {
        if index < 0 || index >= self.item_count.get() || self.average_items_per_line.get().1 == 0.0
        {
            -1
        } else {
            self.line_index(index)
        }
    }
    pub(crate) fn clear_item_aspect_ratios(&self) {
        self.aspect_ratios.borrow_mut().clear();
    }
    fn assign_average(&self, value: (f64, f64), unlock: bool) {
        let old = self.average_items_per_line.replace(value);
        if old.1 != value.1 {
            if unlock {
                self.unlock_items();
            }
            LayoutsTestHooks::notify_average_changed(&self.to_ref().upcast());
        }
    }
    fn snap_current_average(&self, old: f64, new: f64) -> (f64, f64) {
        let power = if self.forced_average_divider.get() == 0.0 {
            1.1
        } else {
            self.forced_average_divider.get()
        };
        let snap = |value| {
            if value == 0.0 {
                0.0
            } else {
                Self::snap_to_power(value, power)
            }
        };
        let old_snap = snap(old);
        let new_snap = snap(new);
        if old_snap != new_snap && (old - new).abs() <= 0.1 {
            (old, old_snap)
        } else {
            (new, new_snap)
        }
    }
    pub fn lock_item_to_line(&self, index: i32) -> Result<i32, LinedFlowLayoutError> {
        if index < 0 || index >= self.item_count.get() {
            return Err(LinedFlowLayoutError::ItemIndexOutOfBounds);
        }
        if self.average_items_per_line.get().1 == 0.0 {
            return Ok(-1);
        }
        if let Some(line) = self.locked_items.borrow().get(&index).copied() {
            return Ok(line);
        }
        // A public lock commits only a frozen line's membership. Unfrozen
        // sized lines may still rebalance, so WinUI uses the global estimate
        // for those items even when a provisional line is already known.
        let frozen = self.frozen_range();
        let line = if index == 0 {
            0
        } else if self.uses_fast_path_layout()
            || (index != self.item_count.get() - 1
                && frozen.0 >= 0
                && index >= frozen.0
                && index <= frozen.1)
        {
            self.line_index(index)
        } else {
            (index as f64 / self.average_items_per_line.get().1) as i32
        };
        if index == 0 || index == self.item_count.get() - 1 {
            self.first_or_last_locked.set(true);
        }
        let mut newly_locked = Vec::new();
        {
            let mut locks = self.locked_items.borrow_mut();
            let on_line = locks
                .iter()
                .find(|(_, value)| **value == line)
                .map(|(index, _)| *index);
            if let Some(existing) = on_line {
                let step = if existing < index { 1 } else { -1 };
                let mut between = existing + step;
                while between != index {
                    if let std::collections::btree_map::Entry::Vacant(entry) = locks.entry(between)
                    {
                        entry.insert(line);
                        newly_locked.push(between);
                    }
                    between += step;
                }
            }
            if index != 0 && index != self.item_count.get() - 1 {
                locks.insert(index, line);
            }
        }
        // Diagnostics are application callbacks; release the lock-map borrow
        // before notification so handlers may query or change layout state.
        for interior in newly_locked {
            LayoutsTestHooks::notify_item_locked(&self.to_ref().upcast(), interior, line);
        }
        LayoutsTestHooks::notify_item_locked(&self.to_ref().upcast(), index, line);
        Ok(line)
    }
    pub fn try_initialize_for_context(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
    ) -> Result<(), LinedFlowLayoutError> {
        if self.context.borrow().is_some() {
            return Err(LinedFlowLayoutError::CannotShareLayout);
        }
        *self.context.borrow_mut() = Some(context.downgrade());
        self.context_generation
            .set(self.context_generation.get().wrapping_add(1));
        self.element_manager.set_context(context);
        self.item_count.set(context.item_count());
        self.force_relayout.set(true);
        Ok(())
    }
    pub fn realized_range(&self) -> (i32, i32) {
        if self.context.borrow().is_none() {
            return (-1, -1);
        }
        let first = self.element_manager.first_realized_data_index();
        let count = self.element_manager.realized_element_count();
        if count == 0 {
            (-1, -1)
        } else {
            (first, first + count - 1)
        }
    }
    pub fn frozen_range(&self) -> (i32, i32) {
        let lines = self.lines.borrow();
        let first = lines
            .iter()
            .find(|line| line.index == self.first_frozen_line.get())
            .map_or(-1, |line| line.first);
        let last = lines
            .iter()
            .find(|line| line.index == self.last_frozen_line.get())
            .map_or(-1, |line| line.end() - 1);
        (first, last)
    }
    pub fn average_items_per_line(&self) -> (f64, f64) {
        self.average_items_per_line.get()
    }
    pub fn uses_fast_path_layout(&self) -> bool {
        self.fast_path_layout.get()
    }
    fn uses_arrange_width_info(&self) -> bool {
        self.uses_fast_path_layout() || self.items_info.borrow().start >= 0
    }
    fn line_index(&self, item: i32) -> i32 {
        if item == 0 {
            return 0;
        }
        if let Some(line) = self.locked_items.borrow().get(&item).copied() {
            return line;
        }
        if let Some(line) = self
            .lines
            .borrow()
            .iter()
            .find(|line| item >= line.first && item < line.end())
        {
            return line.index;
        }
        (item as f64 / self.average_items_per_line.get().1.max(1.0)) as i32
    }
    fn line_count(&self) -> i32 {
        if self.item_count.get() == 0 {
            0
        } else if self.uses_fast_path_layout() {
            self.lines.borrow().len() as i32
        } else {
            ((self.item_count.get() - 1) as f64 / self.average_items_per_line.get().1.max(1.0))
                as i32
                + 1
        }
    }
    pub(crate) fn unlock_items(&self) {
        let had_locks = !self.locked_items.borrow().is_empty() || self.first_or_last_locked.get();
        self.locked_items.borrow_mut().clear();
        self.first_or_last_locked.set(false);
        if had_locks {
            for (_, handler) in self.items_unlocked.snapshot().iter() {
                handler();
            }
        }
    }
    fn set_average(&self, raw: f64) -> bool {
        let old = self.average_items_per_line.get();
        let next = self.snap_current_average(old.0, raw);
        self.assign_average(next, true);
        if old.1 != next.1 {
            self.force_relayout.set(true);
            true
        } else {
            false
        }
    }
    pub fn snap_to_power(value: f64, power: f64) -> f64 {
        let ceiling = power.powf((value.ln() / power.ln()).ceil());
        let floor = ceiling / power;
        if (ceiling - value).abs() <= (floor - value).abs() {
            ceiling
        } else {
            floor
        }
    }
    pub fn snap_average_items_per_line(old: f64, new: f64) -> (f64, f64) {
        let snap = |value| {
            if value == 0.0 {
                0.0
            } else {
                Self::snap_to_power(value, 1.1)
            }
        };
        let old_snap = snap(old);
        let new_snap = snap(new);
        if old_snap != new_snap && (old - new).abs() <= 0.1 {
            (old, old_snap)
        } else {
            (new, new_snap)
        }
    }
    fn update_actual_line_height(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> bool {
        let value = if !self.line_height().is_nan() {
            self.line_height()
        } else if context.item_count() == 0 {
            0.0
        } else {
            let measured = self
                .element_manager
                .get_element_if_realized(0)
                .filter(|_| self.actual_line_height() > 0.0)
                .and_then(|element| {
                    // GetDesiredSizeForAvailableSize restores the last
                    // allocated width before returning. A natural desired
                    // width is only its fallback: metadata and stretched or
                    // shrunk rows have their own recorded allocation.
                    let previous_width = self
                        .lines
                        .borrow()
                        .first()
                        .filter(|line| line.first == 0)
                        .and_then(|line| line.widths.first().copied())
                        .or_else(|| self.measured_widths.borrow().get(&0).copied())?;
                    element.measure(available);
                    let height = element.desired_size().height;
                    element.measure(Size::new(previous_width, self.actual_line_height()));
                    Some(height)
                });
            measured.unwrap_or_else(|| {
                // With no recorded allocation, probing an existing control
                // would leave it measured with an unrelated width. ForceCreate
                // without SuppressAutoRecycle lets the host discard a separate
                // probe at the end of measure, as in the source implementation.
                let element = context.get_or_create_element_at_with_options(
                    0,
                    ElementRealizationOptions::FORCE_CREATE,
                );
                element.measure(available);
                element.desired_size().height
            })
        };
        let value = if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        };
        self.set_and_raise_cell(
            Self::actual_line_height_property(),
            &self.actual_line_height,
            value,
        )
    }
    fn update_anchor(&self, context: &Ref<VirtualizingLayoutContext>) {
        if self.uses_fast_path_layout() && context.realization_rect().height == 0.0 {
            // EnsureItemRangeFastPath returns before observing a changed
            // recommendation or decrementing its retained anchor.
            return;
        }
        let suggested = context.recommended_anchor_index();
        if suggested >= 0 && suggested < self.item_count.get() {
            self.anchor_index.set(suggested);
            self.anchor_retention.set(10);
        } else if self.anchor_index.get() >= self.item_count.get()
            || self.anchor_retention.get() == 0
        {
            self.anchor_index.set(-1);
            self.anchor_retention.set(0);
        }
    }
    fn complete_anchor_measure(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        previous: (i32, i32),
    ) {
        if self.uses_fast_path_layout() && context.realization_rect().height == 0.0 {
            // The metadata request may have selected the fast path for the
            // first time in this pass, after UpdateAnchor already ran.
            self.anchor_index.set(previous.0);
            self.anchor_retention.set(previous.1);
            return;
        }
        let suggested = context.recommended_anchor_index();
        if context.is_virtualizing()
            && self.anchor_index.get() >= 0
            && !(suggested >= 0 && suggested < self.item_count.get())
        {
            // The source centers this pass before expiring the anchor. Keep
            // it available through sizing, frozen-line computation and actual
            // realization so a countdown of ten retains all ten passes.
            let remaining = self.anchor_retention.get() - 1;
            self.anchor_retention.set(remaining);
            if remaining == 0 {
                self.anchor_index.set(-1);
            }
        }
    }
    fn realization_window(&self, context: &Ref<VirtualizingLayoutContext>) -> Rect {
        let mut rect = context.realization_rect();
        if !context.is_virtualizing() {
            return rect;
        }
        let height = self.actual_line_height();
        let stride = height + self.line_spacing();
        let round =
            |value: f64| (value * self.rounding_scale.get()).round() / self.rounding_scale.get();
        rect.y = round(rect.y + self.realization_shift.get());
        if self.uses_fast_path_layout() {
            // EnsureItemRangeFastPath realizes exactly the rectangle supplied
            // by the host. The minimum realization/sizing windows below belong
            // exclusively to the regular path, which must estimate line sizes.
            let displayed = displayed_line_range(
                rect.height,
                rect.y,
                height,
                self.line_spacing(),
                self.line_count(),
            );
            let anchor = self.anchor_index.get();
            if anchor >= 0 && rect.height > 0.0 {
                let line = self.line_index(anchor);
                if line < displayed.0 || line > displayed.1 {
                    rect.y = round(line as f64 * stride + height / 2.0 - rect.height / 2.0);
                    self.invalidate_measure_async();
                } else {
                    self.anchor_index.set(-1);
                }
            }
            return rect;
        }
        let deficit = (3.0 * context.visible_rect().height - rect.height)
            .max(12.0 * stride - rect.height)
            .max(0.0);
        let padding = deficit / 2.0 / self.measure_countdown.get().max(1) as f64;
        let near = rect.y - padding;
        let far = rect.y + rect.height + padding;
        let anchor = self.anchor_index.get();
        if anchor >= 0 {
            let anchor_near = self.line_index(anchor) as f64 * stride;
            if anchor_near - 2.0 * height - self.line_spacing() <= near
                || anchor_near + stride >= far
            {
                rect.y = round(anchor_near + height / 2.0 - rect.height / 2.0);
                self.invalidate_measure_async();
                return rect;
            }
            self.anchor_index.set(-1);
        }
        rect.y = near;
        rect.height += 2.0 * padding;
        rect
    }
    fn sized_range(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        _width: f64,
        height: f64,
    ) -> (i32, i32, i32, i32) {
        let count = self.item_count.get();
        let average = self.average_items_per_line.get().1.max(1.0);
        let total_lines = ((count - 1) as f64 / average) as i32 + 1;
        if !context.is_virtualizing() {
            return (0, count, 0, total_lines);
        }
        let stride = (height + self.line_spacing()).max(f64::EPSILON);
        let rect = self.realization_window(context);
        let total_height = total_lines as f64 * stride - self.line_spacing();
        let sizing_height = (5.0 * context.visible_rect().height).max(20.0 * stride);
        let deficit = (sizing_height - rect.height).max(0.0);
        let near = (rect.y - deficit / 2.0).max(0.0);
        let far = (rect.y + rect.height + deficit / 2.0).clamp(0.0, total_height);
        let first_line = (near / stride) as i32;
        if first_line >= total_lines {
            return (0, 0, 0, 0);
        }
        let far_lines = ((total_height - far) / stride) as i32;
        let line_count = total_lines - first_line - far_lines;
        if line_count <= 0 {
            return (0, 0, 0, 0);
        }
        let last_line = first_line + line_count - 1;
        let old_lines = self.lines.borrow().clone();
        let overlap = if self.force_relayout.get() {
            &[][..]
        } else {
            let first = old_lines.partition_point(|line| line.index < first_line);
            let end = old_lines.partition_point(|line| line.index <= last_line);
            &old_lines[first..end]
        };
        let mut sized_count = if let (Some(first), Some(last)) = (overlap.first(), overlap.last()) {
            if first.index == first_line && last.index == last_line {
                last.end() - first.first
            } else if first_line == 0 && last.index == last_line {
                last.end()
            } else if last_line + 1 < total_lines {
                let estimated = (line_count as f64 * average).round() as i32;
                if first_line == 0 {
                    estimated.max(
                        old_lines.last().map_or(0, |last| last.end())
                            - old_lines.first().map_or(0, |first| first.first),
                    )
                } else {
                    estimated
                }
            } else {
                0
            }
        } else if last_line + 1 < total_lines {
            (line_count as f64 * average).round() as i32
        } else {
            0
        };
        sized_count = sized_count.min(count);
        let estimated_first = (first_line as f64 * average).round() as i32;
        let mut first_item = if let (Some(first), Some(last)) = (overlap.first(), overlap.last()) {
            if first_line == 0 {
                0
            } else {
                let mut first_item = first.first;
                if first.index > first_line {
                    if sized_count == 0 {
                        first_item -= ((first.index - first_line) as f64 * average).round() as i32;
                    } else {
                        let new_items = sized_count - (last.end() - first.first);
                        let near_lines = first.index - first_line;
                        let far_lines = last_line - last.index;
                        if first_item < first_line + new_items || new_items < near_lines + far_lines
                        {
                            self.force_relayout.set(true);
                            first_item = estimated_first;
                        } else {
                            first_item -=
                                near_lines.max(near_lines * new_items / (near_lines + far_lines));
                        }
                    }
                } else if first_item + sized_count + far_lines > count {
                    sized_count = count - first_item - far_lines;
                }
                first_item
            }
        } else {
            estimated_first
        };
        first_item = first_item.clamp(first_line, count - line_count - far_lines);
        if last_line + 1 == total_lines {
            sized_count = count - first_item;
        }
        sized_count = sized_count
            .max(line_count)
            .min(count - first_item - far_lines);
        (first_item, first_item + sized_count, first_line, line_count)
    }
    fn raise_items_info_requested(&self, start: i32, length: i32) -> LinedFlowItemsInfo {
        if self.items_info_requested.is_empty() || length <= 0 {
            return LinedFlowItemsInfo::default();
        }
        let args = LinedFlowLayoutItemsInfoRequestedEventArgs::new(start, length);
        for (_, handler) in self.items_info_requested.snapshot().iter() {
            handler(&args);
        }
        args.complete()
    }
    fn request_items_info(&self, first: i32, end: i32) -> bool {
        if self.items_info_requested.is_empty() {
            let was_fast = self.fast_path_layout.replace(false);
            if self.items_info.borrow().start != -1 || was_fast {
                *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
                self.force_relayout.set(true);
            }
            return false;
        }
        if self.uses_fast_path_layout() && !self.force_relayout.get() {
            return true;
        }
        let old = self.items_info.borrow().clone();
        if old.contains(first) && old.contains(end - 1) {
            return self.uses_fast_path_layout();
        }
        let mut requests = Vec::new();
        if old.start < 0 || old.end() <= first || old.start >= end {
            requests.push((first, end));
        } else {
            if first < old.start {
                requests.push((first, old.start));
            }
            if end > old.end() {
                requests.push((old.end(), end));
            }
        }
        let mut received = Vec::new();
        for (start, end) in requests {
            let info = self.raise_items_info_requested(start, end - start);
            if info.ratios.is_empty() {
                // RequestItemsInfo abandons the metadata path when ANY
                // requested interval is unavailable. Missing entries are not
                // application-provided negative aspect ratios: use measured
                // controls for the complete sizing window instead.
                *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
                self.fast_path_layout.set(false);
                self.force_relayout.set(true);
                return false;
            }
            if self.fast_path_supported.get()
                && info.start == 0
                && info.ratios.len() >= self.item_count.get() as usize
            {
                *self.items_info.borrow_mut() = Rc::new(info);
                self.fast_path_layout.set(true);
                self.force_relayout.set(true);
                self.stop_timer();
                return true;
            }
            received.push(info);
        }
        self.fast_path_layout.set(false);
        if received.is_empty() {
            // A handler may stop providing metadata after invalidation.
            if !(old.contains(first) && old.contains(end - 1)) {
                *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
            }
            return false;
        }
        // Preserve the overlap and request only the newly exposed prefixes and
        // suffixes, as UpdateItemsInfo does. Negative supplied ratios stay valid
        // metadata entries and use the measured average as their fallback.
        let globals = received.last().expect("nonempty metadata requests");
        // Handlers may cover a wider interval than requested. Retain that
        // interval so the public RequestedRange properties describe the data
        // we actually depend on, and the next scroll can reuse that prefetch.
        let merged_start = received
            .iter()
            .map(|info| info.start)
            .min()
            .unwrap()
            .min(first);
        let merged_end = received
            .iter()
            .map(LinedFlowItemsInfo::end)
            .max()
            .unwrap()
            .max(end)
            .min(self.item_count.get());
        let retains_old_info = old.start >= 0 && old.start < merged_end && old.end() > merged_start;
        // An absent width array differs from an array containing -1. In
        // particular, GetMaxWidthFromItemsInfo takes the minimum of a supplied
        // per-item value and the global maximum, so inventing -1 entries would
        // suppress a valid global MaxWidth. Allocate optional arrays only when
        // a retained or newly provided interval actually supplies them.
        let has_min_widths = (retains_old_info && !old.min_widths.is_empty())
            || received.iter().any(|info| !info.min_widths.is_empty());
        let has_max_widths = (retains_old_info && !old.max_widths.is_empty())
            || received.iter().any(|info| !info.max_widths.is_empty());
        let mut merged = LinedFlowItemsInfo {
            start: merged_start,
            ratios: Vec::with_capacity((merged_end - merged_start) as usize),
            min_widths: Vec::with_capacity(if has_min_widths {
                (merged_end - merged_start) as usize
            } else {
                0
            }),
            max_widths: Vec::with_capacity(if has_max_widths {
                (merged_end - merged_start) as usize
            } else {
                0
            }),
            min_width: globals.min_width,
            max_width: globals.max_width,
        };
        for index in merged_start..merged_end {
            let source = received
                .iter()
                .rev()
                .find(|info| info.contains(index))
                .unwrap_or(&old);
            merged.ratios.push(if source.contains(index) {
                source.ratios[(index - source.start) as usize]
            } else {
                -1.0
            });
            // Global limits belong to the current request. Preserve the raw
            // per-item limit across overlap; folding in a previous global
            // minimum would keep that obsolete limit after it is lowered.
            let offset: Option<usize> = (index - source.start).try_into().ok();
            if has_min_widths {
                merged.min_widths.push(
                    offset
                        .and_then(|offset| source.min_widths.get(offset))
                        .copied()
                        .unwrap_or(-1.0),
                );
            }
            if has_max_widths {
                merged.max_widths.push(
                    offset
                        .and_then(|offset| source.max_widths.get(offset))
                        .copied()
                        .unwrap_or(-1.0),
                );
            }
        }
        *self.items_info.borrow_mut() = Rc::new(merged);
        self.stop_timer();
        false
    }
    fn update_rounding_scale(&self, element: &Control) {
        self.rounding_scale.set(if element.use_layout_rounding() {
            LayoutHelper::get_layout_scale(element)
        } else {
            1.0
        });
    }
    fn measured_item_metrics(&self, element: &Control, index: i32) -> ItemMetrics {
        // Layoutable::DesiredSize already contains the rounded margin and
        // content constraints. LinedFlow's allocation widths are outer slots,
        // so applying a bare MaxWidth again would remove the margin and clip
        // the content. Convert the constraints to that same coordinate space.
        self.update_rounding_scale(element);
        let mut margin = element.margin();
        let mut minimum = element.min_width();
        let mut maximum = element.max_width().max(minimum);
        if element.use_layout_rounding() {
            let scale = self.rounding_scale.get();
            margin = LayoutHelper::round_layout_thickness(margin, scale);
            minimum = LayoutHelper::round_layout_size_up(Size::new(minimum, 0.0), scale).width;
            maximum = LayoutHelper::round_layout_size_up(Size::new(maximum, 0.0), scale).width;
        }
        let margin_width = margin.left + margin.right;
        ItemMetrics {
            index,
            width: element.desired_size().width,
            minimum: if element.is_visible() {
                (minimum + margin_width).max(0.0)
            } else {
                0.0
            },
            maximum: if element.is_visible() {
                (maximum + margin_width).max(0.0)
            } else {
                0.0
            },
        }
    }
    fn ensure_range(&self, context: &Ref<VirtualizingLayoutContext>, first: i32, end: i32) -> bool {
        if !context.is_virtualizing() {
            self.element_manager
                .on_begin_measure(ScrollOrientation::Vertical);
            return false;
        }
        if first >= end {
            self.element_manager.clear_realized_range();
            return false;
        }
        let mut changed = false;
        let old_first = self.element_manager.first_realized_data_index();
        let old_end = old_first + self.element_manager.realized_element_count();
        if old_first == -1 || end <= old_first || first >= old_end {
            self.element_manager.clear_realized_range();
            for index in first..end {
                self.element_manager.ensure_element_realized(true, index);
            }
            changed = true;
        } else {
            for index in (first..old_first).rev() {
                self.element_manager.ensure_element_realized(false, index);
                changed = true;
            }
            for index in old_end..end {
                self.element_manager.ensure_element_realized(true, index);
                changed = true;
            }
            self.element_manager
                .discard_elements_outside_window(false, first - 1);
            self.element_manager
                .discard_elements_outside_window(true, end);
        }
        changed
    }
    fn collect_metrics(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        first: i32,
        end: i32,
        height: f64,
        metadata_only: bool,
    ) -> Vec<ItemMetrics> {
        let info = self.items_info.borrow().clone();
        if !self.uses_fast_path_layout() {
            let lines_per_viewport =
                context.visible_rect().height / (height + self.line_spacing()).max(f64::EPSILON);
            let viewport_items =
                (lines_per_viewport * self.average_items_per_line.get().1.max(1.0)).max(20.0);
            let capacity = ((viewport_items * 10.0) as usize)
                .max((end - first).max(0) as usize)
                .min(self.item_count.get().max(1) as usize);
            let mut storage = self.aspect_ratios.borrow_mut();
            // Deviation (winui-repeater/DEVIATIONS.md, WR-15): the reference index is
            // `first + (last - first) / 2`, not `(last - first) / 2`.
            storage.resize(capacity, first + (end - first - 1).max(0) / 2);
            // UpdateAspectRatiosWithItemsInfo trusts every supplied positive
            // ratio, including prefetched entries outside the sized interval.
            // Update history before replacing negative ratios with its average.
            for index in info.start.max(0)..info.end().min(self.item_count.get()) {
                let ratio = info.ratios[(index - info.start) as usize];
                if ratio <= 0.0 || !ratio.is_finite() {
                    continue;
                }
                let minimum = info.minimum(index).max(0.0);
                let maximum = if info.maximum(index) >= 0.0 {
                    info.maximum(index)
                } else {
                    f64::INFINITY
                };
                let ratio = (ratio * height).max(minimum).min(maximum) / height;
                let old = storage.get_at(index);
                let ratio = if old.weight != 0
                    && (old.aspect_ratio - ratio).abs()
                        <= 0.5 / (self.rounding_scale.get() * height)
                {
                    old.aspect_ratio
                } else {
                    ratio
                };
                storage.set_at(
                    index,
                    ItemAspectRatio {
                        aspect_ratio: ratio,
                        weight: 16,
                    },
                );
            }
        }
        let average_ratio = if self.aspect_ratios.borrow().is_empty()
            && self.average_items_per_line.get().1 > 0.0
            && height > 0.0
        {
            self.current_available_width.get() / self.average_items_per_line.get().1 / height
        } else {
            self.average_aspect_ratio()
        };
        let old_first = self.element_manager.first_realized_data_index();
        let old_end = old_first + self.element_manager.realized_element_count();
        let newly_realized = !metadata_only && self.ensure_range(context, first, end);
        let mut metrics = Vec::with_capacity((end - first).max(0) as usize);
        let mut observations = Vec::new();
        for index in first..end {
            if info.contains(index) {
                let ratio = info.ratios[(index - info.start) as usize];
                let minimum = info.minimum(index).max(0.0);
                let maximum = if info.maximum(index) >= 0.0 {
                    info.maximum(index)
                } else {
                    f64::INFINITY
                };
                let width = ((if ratio > 0.0 && ratio.is_finite() {
                    ratio
                } else {
                    average_ratio
                }) * height)
                    .max(minimum)
                    .min(maximum);
                metrics.push(ItemMetrics {
                    index,
                    width,
                    minimum,
                    maximum,
                });
            } else {
                let cached = self.measure_metrics.borrow().get(&index).copied();
                if let Some(cached) = cached {
                    metrics.push(cached);
                    continue;
                }
                let element = self.element_manager.get_realized_element(index);
                element.measure(Size::new(f64::INFINITY, height));
                let desired = element.desired_size();
                let measured = self.measured_item_metrics(&element, index);
                metrics.push(measured);
                self.measure_metrics.borrow_mut().insert(index, measured);
                if desired.height > 0.0 {
                    let old = self.aspect_ratios.borrow().get_at(index);
                    let weight = if measured.width > measured.minimum {
                        16
                    } else if old.weight == 0 || index < old_first || index >= old_end {
                        1
                    } else {
                        (old.weight + 1).min(16)
                    };
                    observations.push((index, measured.width / desired.height, weight));
                }
            }
        }
        if !self.uses_fast_path_layout() {
            let mut storage = self.aspect_ratios.borrow_mut();
            for (index, ratio, weight) in observations {
                storage.set_at(
                    index,
                    ItemAspectRatio {
                        aspect_ratio: ratio,
                        weight,
                    },
                );
            }
        }
        if newly_realized {
            self.start_timer(0);
        }
        metrics
    }
    fn average_aspect_ratio(&self) -> f64 {
        let (first, last) = self.realized_range();
        let value = self
            .aspect_ratios
            .borrow()
            .average_aspect_ratio(first, last, 16);
        if value > 0.0 {
            value
        } else {
            1.0
        }
    }
    fn evaluated_average(&self, width: f64, height: f64, has_metadata: bool) -> (f64, f64) {
        let minimum = self.measure_countdown.get() as f64 * 0.25 + 0.5;
        let mut ratio = if self.aspect_ratios.borrow().is_empty() {
            minimum
        } else {
            self.average_aspect_ratio()
        };
        if !has_metadata && self.measure_countdown.get() > 1 && ratio < minimum {
            ratio = minimum;
            self.invalidate_measure_async();
        }
        self.diagnostic_average_ratio.set(ratio);
        if self.forced_aspect_ratio.get() != 0.0 {
            ratio = self.forced_aspect_ratio.get();
        }
        let raw = ((width + self.min_item_spacing())
            / (ratio * height + self.min_item_spacing()).max(1.0))
        .max(1.0);
        self.snap_current_average(self.average_items_per_line.get().0, raw)
    }
    fn line_options(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        width: f64,
        first_line: i32,
        count: i32,
    ) -> LineOptions {
        LineOptions {
            available: width,
            spacing: self.min_item_spacing(),
            stretch: self.items_stretch() == LinedFlowLayoutItemsStretch::Fill,
            first_line,
            begin_line_vector_index: 0,
            line_count: count.max(1) as usize,
            total_item_count: self.item_count.get(),
            average_items_per_line: self.average_items_per_line.get().1.max(1.0),
            forward: true,
            wrap_multiplier: if self.forced_wrap_multiplier.get() == 0.0 {
                2.0
            } else {
                self.forced_wrap_multiplier.get()
            },
            lines_per_viewport: if context.is_virtualizing() {
                (context.visible_rect().height
                    / (self.actual_line_height() + self.line_spacing()).max(f64::EPSILON))
                    as i32
            } else {
                i32::MAX
            },
        }
    }
    fn compute_regular_lines(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        metrics: &[ItemMetrics],
        width: f64,
        first_line: i32,
        count: i32,
    ) -> Vec<LinedFlowLine> {
        if metrics.is_empty() {
            return Vec::new();
        }
        let old_lines = self.lines.borrow().clone();
        let previous_widths = self.measured_widths.borrow().clone();
        let visible = context.visible_rect();
        let stride = self.actual_line_height() + self.line_spacing();
        let scroll_offset = if self.anchor_index.get() >= 0 {
            // realization_window retained the anchor because it centered the
            // pending realization window around a disconnected target.
            (self.line_index(self.anchor_index.get()) as f64 * stride
                + self.actual_line_height() / 2.0
                - visible.height / 2.0)
                .max(0.0)
        } else {
            visible.y + self.realization_shift.get()
        };
        let (first_frozen, last_frozen) = frozen_line_range(
            if context.is_virtualizing() {
                visible.height
            } else {
                f64::INFINITY
            },
            scroll_offset,
            self.actual_line_height(),
            self.line_spacing(),
            self.line_count(),
            (first_line, first_line + count - 1),
        );
        let first_item = metrics[0].index;
        let end_item = metrics.last().expect("nonempty metrics").index + 1;
        let locks = self.locked_items.borrow().clone();
        let can_freeze = !self.force_relayout.get()
            && context.is_virtualizing()
            && visible.height > 0.0
            && self.previous_available_width.get() == width;
        let frozen = old_lines
            .iter()
            .filter(|line| {
                can_freeze
                    && line.first >= first_item
                    && line.end() <= end_item
                    && line.index >= first_line
                    && line.index < first_line + count
                    && line.index >= first_frozen
                    && line.index <= last_frozen
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut result = None;
        if let (Some(first), Some(last)) = (frozen.first(), frozen.last()) {
            let first_offset = (first.first - first_item) as usize;
            let last_offset = (last.end() - first_item) as usize;
            let before_lines = first.index - first_line;
            let after_lines = first_line + count - last.index - 1;
            let frozen_changed = metrics[first_offset..last_offset].iter().any(|item| {
                previous_widths
                    .get(&item.index)
                    .is_none_or(|old| *old != item.width)
            });
            if frozen_changed {
                LayoutsTestHooks::notify_invalidated(
                    &self.to_ref().upcast(),
                    LinedFlowLayoutInvalidationTrigger::ItemDesiredWidthChange,
                );
            }
            // Crossing a formerly sized boundary requires a complete
            // redistribution, as in ComputeFrozenItemsRange.
            let boundaries_valid = first.index == first_frozen
                && last.index == last_frozen
                && first_offset >= before_lines as usize
                && metrics.len() - last_offset >= after_lines as usize
                && (first_offset == 0) == (before_lines == 0)
                && (last_offset == metrics.len()) == (after_lines == 0);
            if !frozen_changed && boundaries_valid {
                // Frozen lines keep their previous membership and measured
                // widths. Each unfrozen block is optimized independently from
                // the frozen edge: backward above it, forward below it.
                let mut partitioned = Vec::with_capacity(count as usize);
                if before_lines > 0 {
                    let mut options = self.line_options(context, width, first_line, before_lines);
                    options.forward = false;
                    partitioned.extend(compute_regular_path(
                        &metrics[..first_offset],
                        options,
                        &locks,
                    ));
                }
                let first_after_line = last.index + 1;
                partitioned.extend(frozen);
                if after_lines > 0 {
                    let mut options =
                        self.line_options(context, width, first_after_line, after_lines);
                    options.begin_line_vector_index = first_after_line - first_line;
                    partitioned.extend(compute_regular_path(
                        &metrics[last_offset..],
                        options,
                        &locks,
                    ));
                }
                result = Some(partitioned);
            }
        }
        let mut result = result.unwrap_or_else(|| {
            compute_regular_path(
                metrics,
                self.line_options(context, width, first_line, count),
                &locks,
            )
        });
        // ComputeLineExpandFactor probes controls whose natural desired width
        // is their minimum. Some controls stay at that minimum even when given
        // more space; remove them from the remaining expansion budget.
        if self.items_info.borrow().start == -1
            && self.items_stretch() == LinedFlowLayoutItemsStretch::Fill
        {
            for line in &mut result {
                if line.end() == self.item_count.get() {
                    continue;
                }
                let start = (line.first - first_item) as usize;
                let mut line_metrics = metrics[start..start + line.widths.len()].to_vec();
                let desired = line_metrics.iter().map(|item| item.width).sum::<f64>()
                    + self.min_item_spacing() * line_metrics.len().saturating_sub(1) as f64;
                if desired <= 0.0 || desired >= width {
                    continue;
                }
                loop {
                    let factor = line_scale(&line_metrics, width, self.min_item_spacing(), true);
                    let mut found = false;
                    for item in &mut line_metrics {
                        if item.maximum <= item.minimum
                            || (factor - 1.0) * item.minimum <= 1.0 / self.rounding_scale.get()
                            || (item.minimum - item.width).abs() > 1.0 / self.rounding_scale.get()
                        {
                            continue;
                        }
                        let element = self.element_manager.get_realized_element(item.index);
                        element.measure(Size::new(item.width * factor, self.actual_line_height()));
                        let measured = element.desired_size().width;
                        element.measure(Size::new(item.width, self.actual_line_height()));
                        if (item.minimum - measured).abs() <= 1.0 / self.rounding_scale.get() {
                            item.maximum = item.minimum;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        break;
                    }
                }
                *line = finish_line(
                    &line_metrics,
                    line.index,
                    self.line_options(context, width, line.index, 1),
                    false,
                );
            }
        }
        self.first_frozen_line.set(first_frozen);
        self.last_frozen_line.set(last_frozen);
        *self.measured_widths.borrow_mut() = metrics
            .iter()
            .map(|item| (item.index, item.width))
            .collect();
        result
    }
    fn compute_fast_lines(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        width: f64,
    ) -> Rc<Vec<LinedFlowLine>> {
        let count = self.item_count.get();
        let metrics = self.collect_metrics(context, 0, count, self.actual_line_height(), true);
        let lines = Rc::new(compute_fast_path(
            &metrics,
            self.line_options(context, width, 0, count),
        ));
        let average = if lines.len() <= 1 {
            count as f64
        } else {
            (count - lines.last().expect("nonempty fast layout").widths.len() as i32) as f64
                / (lines.len() - 1) as f64
        };
        *self.lines.borrow_mut() = lines.clone();
        // The exact fast-path distribution is known, so its average is not
        // estimated or snapped. Keep only computed widths/counts after using
        // the potentially large application-owned metadata arrays.
        self.assign_average((average, average), false);
        // ExitRegularPath discards its sized/frozen windows. A fast-path
        // distribution is known for the entire source, but is not a frozen
        // window in the public diagnostic contract.
        self.first_frozen_line.set(-1);
        self.last_frozen_line.set(-1);
        *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
        self.unlock_items();
        lines
    }
    fn realize_and_measure_lines(
        &self,
        context: &Ref<VirtualizingLayoutContext>,
        lines: &Rc<Vec<LinedFlowLine>>,
        metadata: bool,
    ) {
        let height = self.actual_line_height();
        let stride = height + self.line_spacing();
        let window = self.realization_window(context);
        let anchor = self.anchor_index.get();
        let (first, last) = if self.uses_fast_path_layout() && context.is_virtualizing() {
            if window.height == 0.0 {
                // A zero-height viewport does not discard the existing range
                // in EnsureItemRangeFastPath. Remeasure its retained controls
                // if the layout allocation changed while the viewport is zero.
                let (first, last) = self.realized_range();
                (
                    (first >= 0).then_some(first),
                    (last >= 0).then_some(last + 1),
                )
            } else {
                let (first, last) = displayed_line_range(
                    window.height,
                    window.y,
                    height,
                    self.line_spacing(),
                    lines.len() as i32,
                );
                // Fast-path lines have consecutive absolute indices. Direct
                // indexing keeps scrolling independent of the source size.
                (
                    usize::try_from(first)
                        .ok()
                        .and_then(|index| lines.get(index))
                        .map(|line| line.first),
                    usize::try_from(last)
                        .ok()
                        .and_then(|index| lines.get(index))
                        .map(|line| line.end()),
                )
            }
        } else {
            let selected = |line: &LinedFlowLine| {
                !context.is_virtualizing()
                    || !metadata
                    || (line.index as f64 * stride + height >= window.y
                        && line.index as f64 * stride <= window.y + window.height)
                    || (anchor >= line.first && anchor < line.end())
            };
            (
                lines
                    .iter()
                    .find(|line| selected(line))
                    .map(|line| line.first),
                lines
                    .iter()
                    .rfind(|line| selected(line))
                    .map(|line| line.end()),
            )
        };
        if let (Some(first), Some(end)) = (first, last) {
            let newly_realized = self.ensure_range(context, first, end);
            self.update_rounding_scale(&self.element_manager.get_realized_element(first));
            let first_line = lines.partition_point(|line| line.end() <= first);
            for line in lines[first_line..]
                .iter()
                .take_while(|line| line.first < end)
            {
                // A zero-height fast-path viewport retains its old item range
                // even when a width change redistributes the lines. Measure
                // the intersections of its partial boundary lines as well.
                let begin = (first - line.first).max(0) as usize;
                let end = (end - line.first).min(line.widths.len() as i32) as usize;
                let mut x = line.widths[..begin].iter().sum::<f64>()
                    + begin as f64 * self.min_item_spacing();
                for (offset, width) in line.widths.iter().enumerate().take(end).skip(begin) {
                    let index = line.first + offset as i32;
                    let element = self.element_manager.get_realized_element(index);
                    element.measure(Size::new(*width, height));
                    self.element_manager.set_layout_bounds_for_data_index(
                        index,
                        Rect::new(x, line.index as f64 * stride, *width, height),
                    );
                    x += *width + self.min_item_spacing();
                }
            }
            if newly_realized && !metadata {
                self.start_timer(0);
            }
            if !metadata
                && self
                    .aspect_ratios
                    .borrow()
                    .has_lower_weight(first, end - 1, 16)
            {
                self.invalidate_measure_async();
            }
        } else {
            self.element_manager.clear_realized_range();
        }
    }
    fn measure_unconstrained(&self, context: &Ref<VirtualizingLayoutContext>) -> Size {
        let count = self.item_count.get();
        *self.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
        self.fast_path_layout.set(false);
        let old_first = self.element_manager.first_realized_data_index();
        let old_end = old_first + self.element_manager.realized_element_count();
        self.ensure_range(context, 0, count);
        let metrics = self.collect_metrics(context, 0, count, self.actual_line_height(), false);
        // EnsureRange precedes CollectMetrics here, so the latter cannot see
        // which controls were newly realized. Preserve MeasureUnconstrainedLine's
        // deferred sizing for new controls still reporting only their minimum
        // width while asynchronous content loads.
        if context.is_virtualizing()
            && metrics.iter().any(|item| {
                (item.index < old_first || item.index >= old_end) && item.width == item.minimum
            })
        {
            self.start_timer(0);
        }
        let widths = metrics.iter().map(|item| item.width).collect::<Vec<_>>();
        let width =
            widths.iter().sum::<f64>() + self.min_item_spacing() * count.saturating_sub(1) as f64;
        self.set_average(count as f64);
        let lines = Rc::new(vec![LinedFlowLine {
            index: 0,
            first: 0,
            widths,
            desired_width: width,
        }]);
        self.realize_and_measure_lines(context, &lines, false);
        *self.lines.borrow_mut() = lines;
        self.max_line_width.set(width);
        Size::new(width, self.actual_line_height())
    }
    fn invalidate_measure_async(&self) {
        if self.async_invalidation_pending.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        let generation = self.context_generation.get();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    if generation != this.context_generation.get() {
                        return;
                    }
                    this.async_invalidation_pending.set(false);
                    if this.context.borrow().is_some() {
                        this.invalidate_measure();
                    }
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn start_timer(&self, tick_count: i32) {
        self.stop_timer();
        if self.context.borrow().is_none() || self.uses_arrange_width_info() {
            return;
        }
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs_f64(0.1 * 1.5f64.powi(tick_count)));
        self.timer_tick_count.set(tick_count);
        let weak = self.to_ref().downgrade();
        let generation = self.context_generation.get();
        let subscription = timer.tick(move |timer| {
            if let Some(this) = weak.upgrade() {
                if generation != this.context_generation.get() {
                    return;
                }
                this.invalidate_measure_async();
                if !this.uses_arrange_width_info() && this.timer_tick_count.get() < 7 {
                    this.start_timer(this.timer_tick_count.get() + 1);
                } else {
                    this.stop_timer();
                }
            } else {
                timer.stop();
            }
        });
        timer.start();
        *self.timer.borrow_mut() = Some((timer, subscription));
    }
    fn stop_timer(&self) {
        let timer = self.timer.borrow_mut().take();
        if let Some((timer, subscription)) = timer {
            subscription.dispose();
            timer.stop();
        }
    }
}
impl VirtualizingLayoutImpl for LinedFlowLayout {
    fn initialize_for_context_core(this: &Self, context: &Ref<VirtualizingLayoutContext>) {
        if let Err(error) = this.try_initialize_for_context(context) {
            panic!("{error}");
        }
    }
    fn uninitialize_for_context_core(this: &Self, _context: &Ref<VirtualizingLayoutContext>) {
        this.element_manager.clear_realized_range();
        this.context.borrow_mut().take();
        this.context_generation
            .set(this.context_generation.get().wrapping_add(1));
        this.async_invalidation_pending.set(false);
        this.item_count.set(0);
        this.fast_path_layout.set(false);
        this.average_items_per_line.set((0.0, 0.0));
        this.aspect_ratios.borrow_mut().clear();
        this.measured_widths.borrow_mut().clear();
        this.measure_metrics.borrow_mut().clear();
        this.measure_countdown.set(5);
        this.anchor_index.set(-1);
        this.anchor_retention.set(0);
        this.first_frozen_line.set(-1);
        this.last_frozen_line.set(-1);
        this.realization_shift.set(0.0);
        this.rounding_scale.set(1.0);
        this.max_line_width.set(0.0);
        this.previous_available_width.set(0.0);
        this.stop_timer();
        this.unlock_items();
        *this.lines.borrow_mut() = Rc::new(Vec::new());
        *this.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
    }
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        if this.context.borrow().is_none() {
            return Size::default();
        }
        this.measure_metrics.borrow_mut().clear();
        this.current_available_width.set(available.width);
        this.realization_shift.set(0.0);
        if this.previous_available_width.get() != available.width {
            this.force_relayout.set(true);
        }
        if !context.is_virtualizing() {
            let new_count = context.item_count();
            if !this.uses_fast_path_layout() && this.items_info.borrow().start == -1 {
                if this.item_count.get() < new_count {
                    // Fixed-child contexts do not send collection events.
                    // Start the deferred sizing passes when a child is added.
                    this.start_timer(0);
                } else if this.item_count.get() > 0 && new_count == 0 {
                    this.stop_timer();
                }
            }
            this.item_count.set(new_count);
            this.force_relayout.set(true);
            this.unlock_items();
            this.element_manager
                .on_begin_measure(ScrollOrientation::Vertical);
        }
        if this.measure_countdown.get() > 1 {
            this.measure_countdown.set(this.measure_countdown.get() - 1);
        }
        let previous_anchor = (this.anchor_index.get(), this.anchor_retention.get());
        this.update_anchor(context);
        if this.update_actual_line_height(context, available) {
            this.force_relayout.set(true);
        }
        let height = this.actual_line_height();
        let count = context.item_count();
        if count == 0 || height <= 0.0 {
            this.element_manager.clear_realized_range();
            this.set_average(0.0);
            this.fast_path_layout.set(false);
            this.aspect_ratios.borrow_mut().clear();
            *this.lines.borrow_mut() = Rc::new(Vec::new());
            *this.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
            this.first_frozen_line.set(-1);
            this.last_frozen_line.set(-1);
            this.stop_timer();
            return Size::default();
        }
        context.set_layout_origin(Point::default());
        let width = available.width;
        if width == f64::INFINITY {
            let size = this.measure_unconstrained(context);
            this.previous_available_width.set(width);
            this.force_relayout.set(false);
            return size;
        }
        let had_no_average = this.average_items_per_line.get().1 == 0.0;
        if had_no_average {
            let average = this.evaluated_average(width, height, false);
            this.average_items_per_line.set(average);
        }
        let mut processed_averages = Vec::new();
        let (lines, metadata) = loop {
            let average = this.average_items_per_line.get();
            processed_averages.push(average.1);
            let (first, end, first_line, line_count) = this.sized_range(context, width, height);
            if this.request_items_info(first, end) {
                if had_no_average && processed_averages.len() == 1 {
                    this.average_items_per_line.set((0.0, 0.0));
                }
                let lines = if this.force_relayout.get() || this.lines.borrow().is_empty() {
                    this.compute_fast_lines(context, width)
                } else {
                    this.lines.borrow().clone()
                };
                break (lines, true);
            }
            let metadata = this.items_info.borrow().contains(first)
                && this.items_info.borrow().contains(end - 1);
            let metrics = this.collect_metrics(context, first, end, height, metadata);
            let new_average = this.evaluated_average(width, height, metadata);
            if new_average.1 != average.1 && !processed_averages.contains(&new_average.1) {
                this.assign_average(new_average, true);
                LayoutsTestHooks::notify_invalidated(
                    &this.to_ref().upcast(),
                    LinedFlowLayoutInvalidationTrigger::SnappedAverageItemsPerLineChange,
                );
                this.force_relayout.set(true);
                if context.is_virtualizing() {
                    let old_offset = context.visible_rect().y + this.realization_shift.get();
                    let new_extent = this.line_count() as f64 * (height + this.line_spacing())
                        - this.line_spacing();
                    let new_offset = (old_offset * average.1 / new_average.1)
                        .min((new_extent - context.visible_rect().height).max(0.0));
                    this.realization_shift
                        .set(this.realization_shift.get() + new_offset - old_offset);
                }
                continue;
            }
            if new_average.1 == average.1 {
                this.average_items_per_line.set(new_average);
            }
            let lines = Rc::new(
                this.compute_regular_lines(context, &metrics, width, first_line, line_count),
            );
            break (lines, metadata);
        };
        *this.lines.borrow_mut() = lines.clone();
        this.realize_and_measure_lines(context, &lines, metadata);
        this.complete_anchor_measure(context, previous_anchor);
        let mut max_line_width = 0.0f64;
        for line in lines.iter() {
            if metadata {
                max_line_width = max_line_width.max(line.desired_width);
            } else if line.index >= this.first_frozen_line.get()
                && line.index <= this.last_frozen_line.get()
            {
                let actual_width = (line.first..line.end())
                    .filter_map(|index| this.element_manager.get_element_if_realized(index))
                    .map(|element| element.desired_size().width)
                    .sum::<f64>()
                    + this.min_item_spacing() * line.widths.len().saturating_sub(1) as f64;
                max_line_width = max_line_width.max(actual_width);
            }
        }
        this.max_line_width.set(max_line_width);
        *this.lines.borrow_mut() = lines;
        this.previous_available_width.set(width);
        this.force_relayout.set(false);
        let total_lines = this.line_count();
        let extra = this.max_line_width.get() - width;
        let desired_width = width
            + if extra > 0.5 / this.rounding_scale.get() * this.average_items_per_line.get().1 {
                extra
            } else {
                0.0
            };
        Size::new(
            desired_width.max(0.0),
            (total_lines as f64 * height + (total_lines - 1).max(0) as f64 * this.line_spacing())
                .max(0.0),
        )
    }
    fn arrange_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        if this.actual_line_height() <= 0.0 || this.context.borrow().is_none() {
            return final_size;
        }
        let height = this.actual_line_height();
        let spacing = this.min_item_spacing();
        if this.previous_available_width.get().is_infinite() {
            let mut offset = if this.items_stretch() == LinedFlowLayoutItemsStretch::None {
                match this.items_justification() {
                    LinedFlowLayoutItemsJustification::SpaceEvenly => spacing,
                    LinedFlowLayoutItemsJustification::SpaceAround => spacing / 2.0,
                    _ => 0.0,
                }
            } else {
                0.0
            };
            for index in 0..context.item_count() {
                let element = this.element_manager.get_realized_element(index);
                let desired = element.desired_size();
                element.arrange(Rect::new(offset, 0.0, desired.width, desired.height));
                offset += desired.width + spacing;
            }
            return final_size;
        }
        let lines = this.lines.borrow().clone();
        let width = this.previous_available_width.get();
        let metadata = this.uses_arrange_width_info();
        let scale = this.rounding_scale.get();
        let first_realized = this.element_manager.first_realized_data_index();
        let end_realized = first_realized + this.element_manager.realized_element_count();
        // GetFirstFullyRealizedLineIndex skips a partially realized leading
        // line, but ArrangeConstrainedLines still processes a trailing prefix.
        let start = lines.partition_point(|line| line.first < first_realized);
        for line in &lines[start..] {
            if line.first >= end_realized {
                break;
            }
            // Collection updates and custom hosts can leave only a trailing
            // line's prefix realized. Keep metadata widths for its absent
            // items; without metadata, GetItemsRangeArrangeWidth sums only
            // realized DesiredSize values. Never create missing controls here.
            let begin = line.first;
            let end = line.end().min(end_realized);
            let elements = (begin..end)
                .map(|index| this.element_manager.get_realized_element(index))
                .collect::<Vec<_>>();
            let widths = line
                .widths
                .iter()
                .enumerate()
                .map(|(position, allocated)| {
                    let index = line.first + position as i32;
                    if metadata {
                        *allocated
                    } else if index >= begin && index < end {
                        elements[(index - begin) as usize].desired_size().width
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>();
            let count = widths.len();
            let line_width = widths.iter().sum::<f64>() + spacing * count.saturating_sub(1) as f64;
            let extra = width - line_width;
            let mut x = 0.0;
            let mut item_spacing = spacing;
            if this.items_stretch() == LinedFlowLayoutItemsStretch::None {
                if extra > 0.0 {
                    match this.items_justification() {
                        LinedFlowLayoutItemsJustification::Start => {}
                        LinedFlowLayoutItemsJustification::Center => x = extra / 2.0,
                        LinedFlowLayoutItemsJustification::End => x = extra,
                        LinedFlowLayoutItemsJustification::SpaceAround => {
                            x = extra / count.max(1) as f64 / 2.0;
                            item_spacing += 2.0 * x;
                        }
                        LinedFlowLayoutItemsJustification::SpaceBetween => {
                            if count > 1 {
                                item_spacing += extra / (count - 1) as f64;
                            }
                        }
                        LinedFlowLayoutItemsJustification::SpaceEvenly => {
                            x = extra / (count + 1) as f64;
                            item_spacing += x;
                        }
                    }
                } else if count > 1 && extra >= -0.5 / scale * count as f64 {
                    item_spacing += extra / (count - 1) as f64;
                }
            } else if count > 1 && extra != 0.0 {
                let rounding_delta = extra.abs() <= 0.5 / scale * count as f64;
                let constrained_expansion = extra > 0.0 && line.index + 1 != this.line_count();
                if rounding_delta || constrained_expansion {
                    item_spacing += extra / (count - 1) as f64;
                }
            }
            let y = line.index as f64 * (height + this.line_spacing());
            for (element, item_width) in elements.iter().zip(widths) {
                // FerroUI's Layoutable applies the visual-root scaling and
                // layout rounding, just as UIElement.Arrange does upstream.
                element.arrange(Rect::new(x, y, item_width, height));
                x += item_width + item_spacing;
            }
        }
        final_size
    }
    fn on_items_changed_core(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        args: &ItemsChangedEventArgs<'_>,
    ) {
        if this.context.borrow().is_none() {
            this.invalidate_measure();
            return;
        }
        this.element_manager.data_source_changed(args);
        this.item_count.set(context.item_count());
        this.force_relayout.set(true);
        *this.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo::default());
        this.measured_widths.borrow_mut().clear();
        if args.action == NotifyCollectionChangedAction::Reset {
            this.aspect_ratios.borrow_mut().clear();
            this.measure_countdown.set(5);
            this.diagnostic_average_ratio.set(0.0);
        }
        if this.item_count.get() == 0 {
            this.stop_timer();
        }
        this.unlock_items();
        this.invalidate_layout();
    }
}
impl Drop for LinedFlowLayout {
    fn drop(&mut self) {
        // The dispatcher retains scheduled timers. A weak callback avoids
        // retaining this layout, but its timer still needs explicit shutdown.
        if let Some((timer, subscription)) = self.timer.get_mut().take() {
            subscription.dispose();
            timer.stop();
        }
    }
}

#[cfg(test)]
#[path = "lined_flow_lifecycle_tests.rs"]
mod lifecycle_tests;
