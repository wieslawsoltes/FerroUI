// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source uniform tab layout, close-burst debounce and native width transitions.
use ferroui_base::{
    animation::{easings::CubicEaseOut, DoubleTransition, TimeSpan, Transitions},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{Layoutable, LayoutableImpl},
    reactive::{AnonymousObserver, IDisposable},
    threading::DispatcherTimer,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElement, StyledElementImpl,
    StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    Control, ControlImpl, Controls, DockPanel, Panel, PanelImpl, ScrollViewer, TopLevel,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const WIDTH_EPSILON: f64 = 0.1;

/// Arranges visible tabs uniformly, shrinking immediately and debouncing close bursts.
#[repr(C)]
pub struct DelayedUniformTabPanel {
    base: Panel,
    expansion_timer: Rc<DispatcherTimer>,
    layout_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    current_tab_width: Cell<f64>,
    pending_expansion_width: Cell<f64>,
    last_visible_count: Cell<i32>,
    resolved_in_measure_pass: Cell<bool>,
    is_removal_debounce_active: Cell<bool>,
}
ferro_class!(DelayedUniformTabPanel:Panel);
ferro_class_info!(DelayedUniformTabPanel {
    new: DelayedUniformTabPanel::new
});
ferro_impl_classes!(DelayedUniformTabPanel:StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl DelayedUniformTabPanel {
    /// Identifies maximum tab width, default 220.
    pub fn max_tab_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("MaxTabWidth",220.0)}
    /// Identifies minimum tab width, default 80.
    pub fn min_tab_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("MinTabWidth",80.0)}
    /// Identifies spacing between tabs, default zero.
    pub fn item_spacing_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("ItemSpacing",0.0)}
    /// Identifies the close-burst expansion delay, default one second.
    pub fn expansion_delay_property()->StyledProperty<TimeSpan>{FerroProperty::register::<Self,_>("ExpansionDelay",TimeSpan::from_seconds(1.0))}
    /// Identifies native resize animation duration, default zero.
    pub fn resize_animation_duration_property()->StyledProperty<TimeSpan>{FerroProperty::register::<Self,_>("ResizeAnimationDuration",TimeSpan::ZERO)}
    /// Identifies the source's private animated width property.
    pub fn animated_tab_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("AnimatedTabWidth",f64::NAN)}
}}
impl DelayedUniformTabPanel {
    fn static_constructor() {
        let properties = [
            Self::max_tab_width_property().as_property(),
            Self::min_tab_width_property().as_property(),
            Self::item_spacing_property().as_property(),
            Self::animated_tab_width_property().as_property(),
        ];
        Layoutable::affects_measure::<Self>(&properties);
        Layoutable::affects_arrange::<Self>(&properties);
    }
    /// Creates fields for this class or a derived control; instantiation runs its source constructor.
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            expansion_timer: DispatcherTimer::new(),
            layout_subscriptions: RefCell::new(Vec::new()),
            current_tab_width: Cell::new(f64::NAN),
            pending_expansion_width: Cell::new(f64::NAN),
            last_visible_count: Cell::new(-1),
            resolved_in_measure_pass: Cell::new(false),
            is_removal_debounce_active: Cell::new(false),
        }
    }
    /// Creates the actual source panel, timer subscription and resize transition.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets maximum tab width.
    pub fn max_tab_width(&self) -> f64 {
        self.get_value(Self::max_tab_width_property())
    }
    /// Sets maximum tab width.
    pub fn set_max_tab_width(&self, value: f64) {
        self.set_value(Self::max_tab_width_property(), value);
    }
    /// Gets minimum tab width.
    pub fn min_tab_width(&self) -> f64 {
        self.get_value(Self::min_tab_width_property())
    }
    /// Sets minimum tab width.
    pub fn set_min_tab_width(&self, value: f64) {
        self.set_value(Self::min_tab_width_property(), value);
    }
    /// Gets spacing between tabs.
    pub fn item_spacing(&self) -> f64 {
        self.get_value(Self::item_spacing_property())
    }
    /// Sets spacing between tabs.
    pub fn set_item_spacing(&self, value: f64) {
        self.set_value(Self::item_spacing_property(), value);
    }
    /// Gets expansion delay, including signed source TimeSpan values.
    pub fn expansion_delay(&self) -> TimeSpan {
        self.get_value(Self::expansion_delay_property())
    }
    /// Sets expansion delay.
    pub fn set_expansion_delay(&self, value: TimeSpan) {
        self.set_value(Self::expansion_delay_property(), value);
    }
    /// Gets resize animation duration.
    pub fn resize_animation_duration(&self) -> TimeSpan {
        self.get_value(Self::resize_animation_duration_property())
    }
    /// Sets resize animation duration and the actual native DoubleTransition.
    pub fn set_resize_animation_duration(&self, value: TimeSpan) {
        self.set_value(Self::resize_animation_duration_property(), value);
    }
    fn set_animated_tab_width(&self, value: f64) {
        self.set_value(Self::animated_tab_width_property(), value);
    }
    fn nearly_equal(left: f64, right: f64) -> bool {
        (left - right).abs() <= WIDTH_EPSILON
    }
    fn round_to_device_pixels(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return value;
        }
        let scaling = self
            .get_visual_root()
            .and_then(|root| root.cast::<TopLevel>())
            .map_or(1.0, |root| root.render_scaling());
        if scaling <= 0.0 {
            value
        } else {
            (value * scaling).round_ties_even() / scaling
        }
    }
    fn resolve_tab_width(&self, count: i32, size: Size) -> f64 {
        let target = self.compute_target_tab_width(count, size);
        let previous = self.last_visible_count.get();
        let changed = count != previous;
        let decreased = previous >= 0 && count < previous;
        self.last_visible_count.set(count);
        if !self.current_tab_width.get().is_finite() {
            self.current_tab_width.set(target);
            self.pending_expansion_width.set(f64::NAN);
            self.set_animated_tab_width(target);
            self.is_removal_debounce_active.set(false);
            return self.current_tab_width.get();
        }
        if decreased {
            self.is_removal_debounce_active.set(true);
            self.queue_expansion(target);
            return self.current_tab_width.get();
        }
        if target < self.current_tab_width.get() - WIDTH_EPSILON {
            self.stop_expansion_timer();
            self.pending_expansion_width.set(f64::NAN);
            self.is_removal_debounce_active.set(false);
            self.animate_to_tab_width(target);
            return self.current_tab_width.get();
        }
        if target > self.current_tab_width.get() + WIDTH_EPSILON {
            if changed || self.is_removal_debounce_active.get() {
                self.queue_expansion(target);
            } else {
                self.stop_expansion_timer();
                self.pending_expansion_width.set(f64::NAN);
                self.is_removal_debounce_active.set(false);
                self.animate_to_tab_width(target);
            }
        }
        self.current_tab_width.get()
    }
    fn compute_target_tab_width(&self, count: i32, size: Size) -> f64 {
        let maximum = max(0.0, self.max_tab_width());
        let minimum = max(
            max(0.0, self.min_tab_width()),
            self.get_child_minimum_tab_width(),
        );
        let spacing = max(0.0, self.item_spacing());
        let viewport = self.resolve_viewport_width(size);
        if count <= 0 {
            return maximum;
        }
        if !viewport.is_finite() {
            return max(minimum, maximum);
        }
        let available = max(0.0, viewport - spacing * f64::from((count - 1).max(0)));
        max(minimum, min(maximum, available / f64::from(count)))
    }
    fn resolve_viewport_width(&self, size: Size) -> f64 {
        if let Some(width) = self.try_get_dock_panel_available_width() {
            return width;
        }
        if let Some(scroll) = self.find_ancestor_of_type::<ScrollViewer>() {
            let width = scroll.bounds().width;
            if width.is_finite() && width > 0.0 {
                return width;
            }
        }
        if size.width.is_finite() && size.width > 0.0 {
            return size.width;
        }
        let width = self.bounds().width;
        if width.is_finite() && width > 0.0 {
            width
        } else {
            f64::NAN
        }
    }
    fn try_get_dock_panel_available_width(&self) -> Option<f64> {
        let part = self.find_ancestor_named::<Panel>("PART_Panel")?;
        let dock = part.parent()?.cast::<DockPanel>()?;
        if !dock.bounds().width.is_finite() || dock.bounds().width <= 0.0 {
            return None;
        }
        let mut fixed = 0.0;
        for child in source_children(dock.children()) {
            if !child.is_visible()
                || child.ptr_eq(&part)
                || child.name().as_deref() == Some("PART_TrailingFill")
            {
                continue;
            }
            let bounds = child.bounds().width;
            let desired = child.desired_size().width;
            fixed += max(
                if bounds.is_finite() { bounds } else { 0.0 },
                if desired.is_finite() { desired } else { 0.0 },
            );
            fixed += child.margin().left + child.margin().right;
        }
        let width = max(0.0, dock.bounds().width - fixed);
        if width > 0.0 {
            Some(width)
        } else {
            None
        }
    }
    fn find_ancestor_named<T: ferroui_base::ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        let mut current = self.get_visual_parent();
        while let Some(value) = current {
            if let Some(typed) = value.cast::<T>() {
                if value
                    .cast::<StyledElement>()
                    .is_some_and(|element| element.name().as_deref() == Some(name))
                {
                    return Some(typed);
                }
            }
            current = value.get_visual_parent();
        }
        None
    }
    fn find_ancestor_of_type<T: ferroui_base::ObjectType>(&self) -> Option<Ref<T>> {
        let mut current = self.get_visual_parent();
        while let Some(value) = current {
            if let Some(typed) = value.cast::<T>() {
                return Some(typed);
            }
            current = value.get_visual_parent();
        }
        None
    }
    fn get_child_minimum_tab_width(&self) -> f64 {
        let mut minimum = 0.0;
        for child in source_children(self.children()) {
            if child.is_visible() {
                minimum = max(minimum, child.min_width());
            }
        }
        minimum
    }
    fn attach_layout_observers(&self) {
        self.clear_layout_observers();
        if let Some(dock) = self.find_ancestor_of_type::<DockPanel>() {
            let owner = self.to_ref();
            let subscription = FerroObject::get_observable(&dock, Visual::bounds_property())
                .subscribe(Rc::new(AnonymousObserver::new(move |_: Rect| {
                    owner.invalidate_measure()
                })));
            self.layout_subscriptions.borrow_mut().push(subscription);
            for child in source_children(dock.children()) {
                let owner = self.to_ref();
                let subscription = FerroObject::get_observable(&child, Visual::bounds_property())
                    .subscribe(Rc::new(AnonymousObserver::new(move |_: Rect| {
                        owner.invalidate_measure()
                    })));
                self.layout_subscriptions.borrow_mut().push(subscription);
                let owner = self.to_ref();
                let subscription =
                    FerroObject::get_observable(&child, Visual::is_visible_property()).subscribe(
                        Rc::new(AnonymousObserver::new(move |_: bool| {
                            owner.invalidate_measure()
                        })),
                    );
                self.layout_subscriptions.borrow_mut().push(subscription);
            }
        }
    }
    fn clear_layout_observers(&self) {
        let subscriptions = self.layout_subscriptions.borrow().clone();
        for subscription in subscriptions {
            subscription.dispose();
        }
        self.layout_subscriptions.borrow_mut().clear();
    }
    fn queue_expansion(&self, target: f64) {
        if self.expansion_delay() <= TimeSpan::ZERO {
            self.stop_expansion_timer();
            self.pending_expansion_width.set(f64::NAN);
            self.is_removal_debounce_active.set(false);
            self.animate_to_tab_width(target);
            return;
        }
        self.pending_expansion_width.set(target);
        self.restart_expansion_timer();
    }
    fn restart_expansion_timer(&self) {
        self.expansion_timer.stop();
        let delay = self.expansion_delay();
        self.expansion_timer
            .set_interval(if delay <= TimeSpan::ZERO {
                std::time::Duration::ZERO
            } else {
                delay.to_duration().expect("positive source TimeSpan")
            });
        self.expansion_timer.start();
    }
    fn stop_expansion_timer(&self) {
        self.expansion_timer.stop();
    }
    fn on_expansion_timer_tick(&self) {
        self.expansion_timer.stop();
        let pending = self.pending_expansion_width.get();
        if !pending.is_finite() {
            self.is_removal_debounce_active.set(false);
            return;
        }
        self.animate_to_tab_width(pending);
        self.pending_expansion_width.set(f64::NAN);
        self.is_removal_debounce_active.set(false);
    }
    fn animate_to_tab_width(&self, target: f64) {
        if !target.is_finite() {
            return;
        }
        if !self.current_tab_width.get().is_finite()
            || Self::nearly_equal(self.current_tab_width.get(), target)
            || self.resize_animation_duration() <= TimeSpan::ZERO
        {
            self.current_tab_width.set(target);
            self.set_animated_tab_width(target);
        } else {
            self.set_animated_tab_width(target);
        }
    }
    fn update_resize_transition(&self) {
        if self.resize_animation_duration() <= TimeSpan::ZERO {
            self.set_transitions(None);
            return;
        }
        let transition = DoubleTransition::with_property(
            Self::animated_tab_width_property().as_property(),
            self.resize_animation_duration(),
        );
        transition.set_easing(CubicEaseOut::new());
        self.set_transitions(Some(Transitions::from_items([transition.into()])));
    }
}
impl FerroObjectImpl for DelayedUniformTabPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let owner = this.to_ref();
        // Preserve the source's strong Tick delegate, including its native GC gap.
        let _tick = this
            .expansion_timer
            .tick(move |_| owner.on_expansion_timer_tick());
        this.update_resize_transition();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::expansion_delay_property().as_property()
            && this.expansion_timer.is_enabled()
        {
            this.restart_expansion_timer();
        }
        if change.property() == Self::resize_animation_duration_property().as_property() {
            this.update_resize_transition();
        }
        if change.property() == Self::animated_tab_width_property().as_property() {
            let width = change.get_new_value::<f64>();
            if width.is_finite() {
                this.current_tab_width.set(width);
            }
        }
    }
}
impl VisualImpl for DelayedUniformTabPanel {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        this.attach_layout_observers();
        this.invalidate_measure();
    }
    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, event);
        this.stop_expansion_timer();
        this.clear_layout_observers();
    }
}
impl LayoutableImpl for DelayedUniformTabPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let count = source_children(this.children())
            .filter(|child| child.is_visible())
            .count() as i32;
        if count == 0 {
            this.current_tab_width.set(f64::NAN);
            this.last_visible_count.set(-1);
            this.set_animated_tab_width(f64::NAN);
            this.resolved_in_measure_pass.set(false);
            this.is_removal_debounce_active.set(false);
            this.stop_expansion_timer();
            return Size::default();
        }
        let target = this.compute_target_tab_width(count, available);
        let width = this.resolve_tab_width(count, available);
        this.resolved_in_measure_pass.set(true);
        let spacing = max(0.0, this.item_spacing());
        let mut height = 0.0;
        for child in source_children(this.children()) {
            if !child.is_visible() {
                child.measure(Size::default());
                continue;
            }
            child.measure(Size::new(
                width,
                if available.height.is_finite() {
                    available.height
                } else {
                    f64::INFINITY
                },
            ));
            height = max(height, child.desired_size().height);
        }
        let extent = if this.is_removal_debounce_active.get() {
            width
        } else if target.is_finite() {
            target
        } else {
            width
        };
        Size::new(
            this.round_to_device_pixels(
                extent * f64::from(count) + spacing * f64::from((count - 1).max(0)),
            ),
            height,
        )
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let count = source_children(this.children())
            .filter(|child| child.is_visible())
            .count() as i32;
        if count == 0 {
            this.resolved_in_measure_pass.set(false);
            return final_size;
        }
        let width =
            if this.resolved_in_measure_pass.get() && this.current_tab_width.get().is_finite() {
                this.current_tab_width.get()
            } else {
                this.resolve_tab_width(count, final_size)
            };
        this.resolved_in_measure_pass.set(false);
        let spacing = max(0.0, this.item_spacing());
        let extent = min(
            final_size.width,
            this.round_to_device_pixels(
                width * f64::from(count) + spacing * f64::from((count - 1).max(0)),
            ),
        );
        let mut x = 0.0;
        let mut index = 0;
        for child in source_children(this.children()) {
            if !child.is_visible() {
                child.arrange(Rect::default());
                continue;
            }
            let start = this.round_to_device_pixels(x);
            let end = if index == count - 1 {
                extent
            } else {
                this.round_to_device_pixels(x + width)
            };
            child.arrange(Rect::new(
                start,
                0.0,
                max(0.0, end - start),
                final_size.height,
            ));
            x += width + spacing;
            index += 1;
        }
        final_size
    }
}
// Exact System.Math comparisons, including NaN propagation and signed-zero choice.
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a == 0.0 && b == 0.0 {
        f64::from_bits(a.to_bits() & b.to_bits())
    } else {
        a.max(b)
    }
}
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a == 0.0 && b == 0.0 {
        f64::from_bits(a.to_bits() | b.to_bits())
    } else {
        a.min(b)
    }
}
#[cfg(test)]
#[path = "delayed_uniform_tab_panel_tests.rs"]
mod tests;

// A managed foreach checks for collection mutation after callbacks. Holding the
// native immutable storage makes a mutation change its identity, without keeping
// a RefCell borrow across Measure/Arrange or observable subscriptions.
struct ChildrenEnumerator {
    list: Controls,
    storage: Rc<Vec<Ref<Control>>>,
    index: usize,
}
fn source_children(list: Controls) -> ChildrenEnumerator {
    ChildrenEnumerator {
        storage: list.snapshot(),
        list,
        index: 0,
    }
}
impl Iterator for ChildrenEnumerator {
    type Item = Ref<Control>;
    fn next(&mut self) -> Option<Self::Item> {
        assert!(
            Rc::ptr_eq(&self.storage, &self.list.snapshot()),
            "Collection was modified; enumeration operation may not execute."
        );
        let item = self.storage.get(self.index).cloned();
        if item.is_some() {
            self.index += 1;
        }
        item
    }
}
