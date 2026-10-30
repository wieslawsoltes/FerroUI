// Copyright The Android Open Source Project (reference behavior).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Material range selection with two independent keyboard and accessibility targets.
//!
//! Geometry and horizontal interaction follow AndroidX Material3 `Slider.kt` and
//! `SliderTokens.kt`, revision e4d933c98df7baa7ea2b83eea6e086915961e0ff. The vertical
//! orientation is a FerroUI extension using the same geometry and range contracts.

use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::{BoxShadows, DrawingContext, FlowDirection, IBrush};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Point, Rect, Ref, RoundedRect, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl, RangeBaseAutomationPeer, RangeBaseAutomationPeerImpl,
};
use ferroui_controls::automation::provider::IRangeValueProvider;
use ferroui_controls::automation::{AutomationProperties, ElementNotEnabledException};
use ferroui_controls::primitives::{
    RangeBase, RangeBaseValueChangedEventArgs, TemplateAppliedEventArgs, TemplatedControl,
    TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const HANDLE_WIDTH: f64 = 4.0;
const TRACK_HEIGHT: f64 = 16.0;
const TRACK_GAP: f64 = 6.0;
const STOP_SIZE: f64 = 4.0;
const TARGET_SIZE: f64 = 48.0;
const TOUCH_SLOP: f64 = 8.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterialRangeThumb {
    Start,
    End,
}

struct Drag {
    pointer: Rc<dyn IPointer>,
    origin: Point,
    thumb: MaterialRangeThumb,
    dragging: bool,
    coincident: bool,
}

/// A bounded range selected by two focusable Material slider handles.
#[repr(C)]
pub struct MaterialRangeSlider {
    base: TemplatedControl,
    layout: RefCell<Option<Ref<MaterialRangeSliderLayout>>>,
    start_thumb: RefCell<Option<Ref<MaterialRangeSliderThumb>>>,
    end_thumb: RefCell<Option<Ref<MaterialRangeSliderThumb>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    synchronizing: Cell<bool>,
    drag: RefCell<Option<Drag>>,
    edited: Cell<bool>,
}
ferro_class!(MaterialRangeSlider: TemplatedControl);
ferro_class_info!(MaterialRangeSlider {
    new: MaterialRangeSlider::new
});
ferro_impl_classes!(MaterialRangeSlider: StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for MaterialRangeSlider {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let p = change.property();
        if p == Self::minimum_property().as_property()
            || p == Self::maximum_property().as_property()
            || p == Self::range_start_property().as_property()
            || p == Self::range_end_property().as_property()
            || p == Self::steps_property().as_property()
            || p == Self::orientation_property().as_property()
            || p == Self::is_direction_reversed_property().as_property()
            || p == Visual::flow_direction_property().as_property()
            || p == Self::start_label_property().as_property()
            || p == Self::end_label_property().as_property()
        {
            this.synchronize();
        }
        if p == Self::orientation_property().as_property() {
            this.pseudo_classes()
                .set(":vertical", this.orientation() == Orientation::Vertical);
            this.pseudo_classes()
                .set(":horizontal", this.orientation() == Orientation::Horizontal);
        }
        if p == InputElement::is_effectively_enabled_property().as_property()
            || p == Self::is_read_only_property().as_property()
        {
            if !this.is_effectively_enabled() || this.is_read_only() {
                this.cancel_drag();
            }
        }
        if let Some(layout) = this.layout.borrow().as_ref() {
            layout.invalidate_visual();
        }
    }
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.pseudo_classes().set(":horizontal", true);
    }
}

impl VisualImpl for MaterialRangeSlider {
    fn bypass_flow_direction_policies(_: &Self) -> bool {
        true
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.cancel_drag();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl TemplatedControlImpl for MaterialRangeSlider {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.cancel_drag();
        for subscription in this.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        for thumb in [&this.start_thumb, &this.end_thumb] {
            if let Some(thumb) = thumb.borrow_mut().take() {
                *thumb.owner.borrow_mut() = None;
            }
        }
        if let Some(layout) = this.layout.borrow_mut().take() {
            *layout.owner.borrow_mut() = None;
        }
        Self::parent_on_apply_template(this, e);
        *this.layout.borrow_mut() = e
            .name_scope()
            .find_as::<MaterialRangeSliderLayout>("PART_Layout");
        *this.start_thumb.borrow_mut() = e
            .name_scope()
            .find_as::<MaterialRangeSliderThumb>("PART_StartThumb");
        *this.end_thumb.borrow_mut() = e
            .name_scope()
            .find_as::<MaterialRangeSliderThumb>("PART_EndThumb");
        let weak = this.to_ref().downgrade();
        if let Some(layout) = this.layout.borrow().as_ref() {
            *layout.owner.borrow_mut() = Some(weak.clone());
        }
        for (slot, role) in [
            (&this.start_thumb, MaterialRangeThumb::Start),
            (&this.end_thumb, MaterialRangeThumb::End),
        ] {
            if let Some(thumb) = slot.borrow().as_ref() {
                *thumb.owner.borrow_mut() = Some(weak.clone());
                thumb.role.set(role);
                let weak = weak.clone();
                let subscription = thumb.add_disposable_handler_with(
                    RangeBase::value_changed_event(),
                    move |_, e: &RangeBaseValueChangedEventArgs| {
                        if let Some(owner) = weak.upgrade() {
                            if !owner.synchronizing.get() {
                                owner.set_thumb_value(role, e.new_value());
                            }
                        }
                    },
                    RoutingStrategies::BUBBLE,
                    false,
                );
                this.subscriptions.borrow_mut().push(subscription);
            }
        }
        this.synchronize();
    }
}

impl InputElementImpl for MaterialRangeSlider {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if !this.is_effectively_enabled() || this.is_read_only() || this.drag.borrow().is_some() {
            return;
        }
        if e.pointer().type_() == PointerType::Mouse
            && !e
                .get_current_point(Some(this))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        let Some(layout) = this.layout.borrow().clone() else {
            return;
        };
        let position = e.get_position(Some(&layout));
        let geometry = this.geometry(layout.bounds().size());
        let logical = geometry.fraction_at(position);
        let start = this.fraction(this.range_start());
        let end = this.fraction(this.range_end());
        let first_distance = (start - logical).abs();
        let second_distance = (end - logical).abs();
        let thumb = if first_distance < second_distance
            || (first_distance == second_distance && start > logical)
        {
            MaterialRangeThumb::Start
        } else {
            MaterialRangeThumb::End
        };
        let dragging = e.pointer().type_() == PointerType::Mouse;
        *this.drag.borrow_mut() = Some(Drag {
            pointer: e.pointer().clone(),
            origin: position,
            thumb,
            dragging,
            coincident: (geometry.start - geometry.end).abs() <= TOUCH_SLOP,
        });
        this.press_thumb(thumb, true);
        if let Some(thumb) = this.thumb(thumb) {
            thumb.focus();
        }
        if dragging {
            e.pointer().capture(Some(&this.to_ref().upcast()));
            e.prevent_gesture_recognition();
            this.set_thumb_value(thumb, this.value_at_fraction(logical));
            e.set_handled(true);
        }
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let Some(layout) = this.layout.borrow().clone() else {
            return;
        };
        let position = e.get_position(Some(&layout));
        let geometry = this.geometry(layout.bounds().size());
        let mut cancel = false;
        let mut acquired = false;
        let thumb;
        {
            let mut drag = this.drag.borrow_mut();
            let Some(drag) = drag.as_mut() else {
                return;
            };
            if drag.pointer.id() != e.pointer().id() {
                return;
            }
            if !drag.dragging {
                let delta = position - drag.origin;
                let (primary, cross) = if this.orientation() == Orientation::Horizontal {
                    (delta.x, delta.y)
                } else {
                    (delta.y, delta.x)
                };
                if cross.abs() > TOUCH_SLOP && cross.abs() > primary.abs() {
                    cancel = true;
                } else if primary.abs() > TOUCH_SLOP {
                    if drag.coincident {
                        let direction = if geometry.reversed { -primary } else { primary };
                        drag.thumb = if direction < 0.0 {
                            MaterialRangeThumb::Start
                        } else {
                            MaterialRangeThumb::End
                        };
                    }
                    drag.dragging = true;
                    acquired = true;
                } else {
                    return;
                }
            }
            thumb = drag.thumb;
        }
        if cancel {
            this.cancel_drag();
            return;
        }
        if acquired {
            this.press_thumb(
                MaterialRangeThumb::Start,
                thumb == MaterialRangeThumb::Start,
            );
            this.press_thumb(MaterialRangeThumb::End, thumb == MaterialRangeThumb::End);
            e.pointer().capture(Some(&this.to_ref().upcast()));
            e.prevent_gesture_recognition();
        }
        this.set_thumb_value(
            thumb,
            this.value_at_fraction(geometry.fraction_at(position)),
        );
        e.set_handled(true);
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        let role = this
            .drag
            .borrow()
            .as_ref()
            .filter(|drag| drag.pointer.id() == e.pointer().id())
            .map(|drag| drag.thumb);
        let Some(role) = role else {
            return;
        };
        if let Some(layout) = this.layout.borrow().clone() {
            let geometry = this.geometry(layout.bounds().size());
            this.set_thumb_value(
                role,
                this.value_at_fraction(geometry.fraction_at(e.get_position(Some(&layout)))),
            );
        }
        this.cancel_drag();
        e.set_handled(true);
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        if this
            .drag
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer.id() == e.pointer().id())
        {
            this.cancel_drag();
        }
        Self::parent_on_pointer_capture_lost(this, e);
    }
}

ferro_properties! { impl MaterialRangeSlider {
    pub fn minimum_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("Minimum", StyledPropertyOptions::new(0.0).validate(|v: &f64| v.is_finite())) }
    pub fn maximum_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("Maximum", StyledPropertyOptions::new(100.0).validate(|v: &f64| v.is_finite()).coerce(|o, v| v.max(o.get_value(Self::minimum_property())))) }
    pub fn range_start_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("RangeStart", StyledPropertyOptions::new(0.0).default_binding_mode(BindingMode::TwoWay).validate(|v: &f64| v.is_finite()).coerce(Self::coerce_start)) }
    pub fn range_end_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("RangeEnd", StyledPropertyOptions::new(100.0).default_binding_mode(BindingMode::TwoWay).validate(|v: &f64| v.is_finite()).coerce(Self::coerce_end)) }
    pub fn steps_property() -> StyledProperty<i32> { FerroProperty::register_with::<Self, _>("Steps", StyledPropertyOptions::new(0).validate(|v: &i32| *v >= 0)) }
    pub fn orientation_property() -> StyledProperty<Orientation> { FerroProperty::register::<Self, _>("Orientation", Orientation::Horizontal) }
    pub fn is_direction_reversed_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsDirectionReversed", false) }
    pub fn is_read_only_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsReadOnly", false) }
    pub fn start_label_property() -> StyledProperty<String> { FerroProperty::register::<Self, _>("StartLabel", "Range start".to_owned()) }
    pub fn end_label_property() -> StyledProperty<String> { FerroProperty::register::<Self, _>("EndLabel", "Range end".to_owned()) }
    pub fn active_tick_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self, _>("ActiveTickBrush", None) }
    pub fn inactive_tick_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self, _>("InactiveTickBrush", None) }
}}

impl MaterialRangeSlider {
    ferro_routed_event!(
        pub fn value_change_finished_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Self, _>("ValueChangeFinished", RoutingStrategies::BUBBLE)
        }
    );
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            layout: RefCell::new(None),
            start_thumb: RefCell::new(None),
            end_thumb: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            synchronizing: Cell::new(false),
            drag: RefCell::new(None),
            edited: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }
    pub fn set_minimum(&self, v: f64) {
        self.set_value(Self::minimum_property(), v);
    }
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }
    pub fn set_maximum(&self, v: f64) {
        self.set_value(Self::maximum_property(), v);
    }
    pub fn range_start(&self) -> f64 {
        self.get_value(Self::range_start_property())
    }
    pub fn set_range_start(&self, v: f64) {
        self.set_value(Self::range_start_property(), v);
    }
    pub fn range_end(&self) -> f64 {
        self.get_value(Self::range_end_property())
    }
    pub fn set_range_end(&self, v: f64) {
        self.set_value(Self::range_end_property(), v);
    }
    /// Number of evenly spaced interior stops; zero permits continuous pointer selection.
    pub fn steps(&self) -> i32 {
        self.get_value(Self::steps_property())
    }
    pub fn set_steps(&self, v: i32) {
        self.set_value(Self::steps_property(), v);
    }
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, v: Orientation) {
        self.set_value(Self::orientation_property(), v);
    }
    pub fn is_direction_reversed(&self) -> bool {
        self.get_value(Self::is_direction_reversed_property())
    }
    pub fn set_is_direction_reversed(&self, v: bool) {
        self.set_value(Self::is_direction_reversed_property(), v);
    }
    pub fn is_read_only(&self) -> bool {
        self.get_value(Self::is_read_only_property())
    }
    pub fn set_is_read_only(&self, v: bool) {
        self.set_value(Self::is_read_only_property(), v);
    }
    pub fn start_label(&self) -> String {
        self.get_value(Self::start_label_property())
    }
    pub fn set_start_label(&self, v: impl Into<String>) {
        self.set_value(Self::start_label_property(), v.into());
    }
    pub fn end_label(&self) -> String {
        self.get_value(Self::end_label_property())
    }
    pub fn set_end_label(&self, v: impl Into<String>) {
        self.set_value(Self::end_label_property(), v.into());
    }
    pub fn active_tick_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::active_tick_brush_property())
    }
    pub fn set_active_tick_brush(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::active_tick_brush_property(), v);
    }
    pub fn inactive_tick_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::inactive_tick_brush_property())
    }
    pub fn set_inactive_tick_brush(&self, v: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::inactive_tick_brush_property(), v);
    }

    fn coerce_start(o: &FerroObject, value: f64) -> f64 {
        let minimum = o.get_value(Self::minimum_property());
        let maximum = o.get_value(Self::maximum_property()).max(minimum);
        snap(value, minimum, maximum, o.get_value(Self::steps_property())).clamp(
            minimum,
            o.get_value(Self::range_end_property())
                .clamp(minimum, maximum),
        )
    }
    fn coerce_end(o: &FerroObject, value: f64) -> f64 {
        let minimum = o.get_value(Self::minimum_property());
        let maximum = o.get_value(Self::maximum_property()).max(minimum);
        snap(value, minimum, maximum, o.get_value(Self::steps_property())).clamp(
            o.get_value(Self::range_start_property())
                .clamp(minimum, maximum),
            maximum,
        )
    }
    fn fraction(&self, value: f64) -> f64 {
        if self.maximum() <= self.minimum() {
            0.0
        } else {
            ((value - self.minimum()) / (self.maximum() - self.minimum())).clamp(0.0, 1.0)
        }
    }
    fn value_at_fraction(&self, fraction: f64) -> f64 {
        self.minimum() + fraction * (self.maximum() - self.minimum())
    }
    fn reversed(&self) -> bool {
        match self.orientation() {
            Orientation::Horizontal => {
                self.is_direction_reversed() ^ (self.flow_direction() == FlowDirection::RightToLeft)
            }
            Orientation::Vertical => !self.is_direction_reversed(),
        }
    }
    fn geometry(&self, size: Size) -> Geometry {
        Geometry::new(
            size,
            self.orientation(),
            self.reversed(),
            self.fraction(self.range_start()),
            self.fraction(self.range_end()),
            self.steps() > 0,
            self.thumb(MaterialRangeThumb::Start)
                .map_or(HANDLE_WIDTH, |thumb| thumb.handle_thickness()),
            self.thumb(MaterialRangeThumb::End)
                .map_or(HANDLE_WIDTH, |thumb| thumb.handle_thickness()),
        )
    }
    pub fn thumb(&self, role: MaterialRangeThumb) -> Option<Ref<MaterialRangeSliderThumb>> {
        match role {
            MaterialRangeThumb::Start => self.start_thumb.borrow().clone(),
            MaterialRangeThumb::End => self.end_thumb.borrow().clone(),
        }
    }
    fn synchronize(&self) {
        if self.synchronizing.replace(true) {
            return;
        }
        self.coerce_value(Self::maximum_property().as_property());
        self.coerce_value(Self::range_end_property().as_property());
        self.coerce_value(Self::range_start_property().as_property());
        let increment = (self.maximum() - self.minimum())
            / if self.steps() > 0 {
                self.steps() as f64 + 1.0
            } else {
                100.0
            };
        let page = (if self.steps() > 0 {
            self.steps() as f64 + 1.0
        } else {
            100.0
        } / 10.0)
            .floor()
            .clamp(1.0, 10.0)
            * increment;
        for role in [MaterialRangeThumb::Start, MaterialRangeThumb::End] {
            if let Some(thumb) = self.thumb(role) {
                // Reset the full range before narrowing it, avoiding stale coercion when both
                // endpoints move to the other side of their old interval.
                thumb.set_minimum(self.minimum());
                thumb.set_maximum(self.maximum());
                match role {
                    MaterialRangeThumb::Start => {
                        thumb.set_maximum(self.range_end());
                        thumb.set_range_value(self.range_start());
                    }
                    MaterialRangeThumb::End => {
                        thumb.set_minimum(self.range_start());
                        thumb.set_range_value(self.range_end());
                    }
                }
                thumb.set_small_change(increment);
                thumb.set_large_change(page);
                thumb.set_orientation(self.orientation());
                AutomationProperties::set_name(
                    &thumb,
                    Some(&match role {
                        MaterialRangeThumb::Start => self.start_label(),
                        MaterialRangeThumb::End => self.end_label(),
                    }),
                );
            }
        }
        self.synchronizing.set(false);
        if let Some(layout) = self.layout.borrow().as_ref() {
            layout.invalidate_measure();
            layout.invalidate_arrange();
            layout.invalidate_visual();
        }
    }
    fn set_thumb_value(&self, role: MaterialRangeThumb, value: f64) {
        if !value.is_finite() || !self.is_effectively_enabled() || self.is_read_only() {
            return;
        }
        let before = (self.range_start(), self.range_end());
        let value = snap(value, self.minimum(), self.maximum(), self.steps());
        // Store the accepted interactive value, not an out-of-range desired value that
        // dependency-property coercion could restore when the other handle later moves.
        let value = match role {
            MaterialRangeThumb::Start => value.clamp(self.minimum(), self.range_end()),
            MaterialRangeThumb::End => value.clamp(self.range_start(), self.maximum()),
        };
        match role {
            MaterialRangeThumb::Start => {
                self.set_current_value(Self::range_start_property(), value)
            }
            MaterialRangeThumb::End => self.set_current_value(Self::range_end_property(), value),
        }
        if before != (self.range_start(), self.range_end()) {
            self.edited.set(true);
        }
        // A child RangeBase may have been set to an unsnapped value by automation.
        self.synchronize();
    }
    fn press_thumb(&self, role: MaterialRangeThumb, pressed: bool) {
        if let Some(thumb) = self.thumb(role) {
            thumb.pseudo_classes().set(":pressed", pressed);
        }
    }
    fn cancel_drag(&self) {
        let drag = self.drag.borrow_mut().take();
        self.press_thumb(MaterialRangeThumb::Start, false);
        self.press_thumb(MaterialRangeThumb::End, false);
        if let Some(drag) = drag {
            if drag
                .pointer
                .captured()
                .is_some_and(|element| element == self.to_ref().upcast::<InputElement>())
            {
                drag.pointer.capture(None);
            }
            self.finish_edit();
        }
    }
    fn finish_edit(&self) {
        if self.edited.replace(false) {
            self.raise_event(&RoutedEventArgs::with_event(
                Self::value_change_finished_event(),
            ));
        }
    }
    fn handle_key(&self, role: MaterialRangeThumb, key: Key) -> bool {
        if !self.is_effectively_enabled() || self.is_read_only() {
            return false;
        }
        let Some(thumb) = self.thumb(role) else {
            return false;
        };
        let increment = thumb.small_change();
        let directional = match (self.orientation(), key) {
            (Orientation::Horizontal, Key::Left) | (Orientation::Vertical, Key::Up) => {
                Some(if self.reversed() {
                    increment
                } else {
                    -increment
                })
            }
            (Orientation::Horizontal, Key::Right) | (Orientation::Vertical, Key::Down) => {
                Some(if self.reversed() {
                    -increment
                } else {
                    increment
                })
            }
            _ => None,
        };
        let value = if let Some(delta) = directional {
            thumb.value() + delta
        } else {
            match key {
                Key::Home => thumb.minimum(),
                Key::End => thumb.maximum(),
                Key::PageUp => thumb.value() + thumb.large_change(),
                Key::PageDown => thumb.value() - thumb.large_change(),
                _ => return false,
            }
        };
        self.set_thumb_value(role, value);
        true
    }
}

fn snap(value: f64, minimum: f64, maximum: f64, steps: i32) -> f64 {
    if maximum <= minimum {
        return minimum;
    }
    let value = value.clamp(minimum, maximum);
    if steps == 0 {
        value
    } else {
        let intervals = steps as f64 + 1.0;
        minimum
            + (((value - minimum) / (maximum - minimum) * intervals).round() / intervals)
                * (maximum - minimum)
    }
}

/// A real range control used for one endpoint, with its own focus and RangeValue provider.
#[repr(C)]
pub struct MaterialRangeSliderThumb {
    base: RangeBase,
    owner: RefCell<Option<WeakRef<MaterialRangeSlider>>>,
    role: Cell<MaterialRangeThumb>,
    key_active: Cell<bool>,
}
ferro_class!(MaterialRangeSliderThumb: RangeBase);
ferro_class_info!(MaterialRangeSliderThumb {
    new: MaterialRangeSliderThumb::new
});
ferro_impl_classes!(MaterialRangeSliderThumb: StyledElementImpl, LayoutableImpl, InteractiveImpl, TemplatedControlImpl);
impl FerroObjectImpl for MaterialRangeSliderThumb {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::handle_thickness_property().as_property() {
            if let Some(owner) = this.owner() {
                if let Some(layout) = owner.layout.borrow().as_ref() {
                    layout.invalidate_arrange();
                    layout.invalidate_visual();
                }
            }
        }
    }
}
impl VisualImpl for MaterialRangeSliderThumb {
    fn bypass_flow_direction_policies(_: &Self) -> bool {
        true
    }
}
impl ControlImpl for MaterialRangeSliderThumb {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MaterialRangeSliderThumbAutomationPeer::new(this).upcast()
    }
}
impl InputElementImpl for MaterialRangeSliderThumb {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() || !e.key_modifiers.is_empty() {
            return;
        }
        if this
            .owner()
            .is_some_and(|owner| owner.handle_key(this.role.get(), e.key))
        {
            this.key_active.set(true);
            e.set_handled(true);
        }
    }
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);
        if this.key_active.replace(false) {
            if let Some(owner) = this.owner() {
                owner.finish_edit();
            }
        }
    }
}
ferro_properties! { impl MaterialRangeSliderThumb {
    pub fn handle_thickness_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("HandleThickness", StyledPropertyOptions::new(HANDLE_WIDTH).validate(|v: &f64| v.is_finite() && *v >= 0.0)) }
    pub fn orientation_property() -> StyledProperty<Orientation> { FerroProperty::register::<Self, _>("Orientation", Orientation::Horizontal) }
}}
impl MaterialRangeSliderThumb {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<Self>(true);
    }
    pub fn construct() -> Self {
        Self {
            base: RangeBase::construct(),
            owner: RefCell::new(None),
            role: Cell::new(MaterialRangeThumb::Start),
            key_active: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn owner(&self) -> Option<Ref<MaterialRangeSlider>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn handle_thickness(&self) -> f64 {
        self.get_value(Self::handle_thickness_property())
    }
    pub fn set_handle_thickness(&self, v: f64) {
        self.set_value(Self::handle_thickness_property(), v);
    }
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, v: Orientation) {
        self.set_value(Self::orientation_property(), v);
    }
}

#[repr(C)]
pub struct MaterialRangeSliderThumbAutomationPeer {
    base: RangeBaseAutomationPeer,
}
ferro_class!(MaterialRangeSliderThumbAutomationPeer: RangeBaseAutomationPeer);
ferro_class_info!(MaterialRangeSliderThumbAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => MaterialRangeSliderThumbAutomationPeer::provider]
});
ferro_impl_classes!(MaterialRangeSliderThumbAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl, RangeBaseAutomationPeerImpl);
impl AutomationPeerImpl for MaterialRangeSliderThumbAutomationPeer {
    fn get_class_name_core(_: &Self) -> String {
        "MaterialRangeSliderThumb".to_owned()
    }
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Slider
    }
}
impl MaterialRangeSliderThumbAutomationPeer {
    pub fn new(owner: &MaterialRangeSliderThumb) -> Ref<Self> {
        instantiate(Self {
            base: RangeBaseAutomationPeer::construct(owner),
        })
    }
    fn provider(peer: Ref<Self>) -> Rc<dyn IRangeValueProvider> {
        Rc::new(RangeThumbProvider(peer))
    }
    fn thumb(&self) -> Ref<MaterialRangeSliderThumb> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("range thumb peer owner")
    }
}
struct RangeThumbProvider(Ref<MaterialRangeSliderThumbAutomationPeer>);
impl IRangeValueProvider for RangeThumbProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0
            .thumb()
            .owner()
            .is_none_or(|owner| owner.is_read_only())
    }
    fn minimum(&self) -> f64 {
        self.0.thumb().minimum()
    }
    fn maximum(&self) -> f64 {
        self.0.thumb().maximum()
    }
    fn value(&self) -> f64 {
        self.0.thumb().value()
    }
    fn large_change(&self) -> f64 {
        self.0.thumb().large_change()
    }
    fn small_change(&self) -> f64 {
        self.0.thumb().small_change()
    }
    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        let thumb = self.0.thumb();
        let owner = thumb.owner().ok_or_else(ElementNotEnabledException::new)?;
        if owner.is_read_only() {
            return Err(ElementNotEnabledException::with_message(
                "Range slider is read-only.",
            ));
        }
        if value.is_finite() {
            owner.set_thumb_value(thumb.role.get(), value);
            owner.finish_edit();
        }
        Ok(())
    }
}

/// Template panel drawing native track segments and positioning the two 48dp input cells.
#[repr(C)]
pub struct MaterialRangeSliderLayout {
    base: Panel,
    owner: RefCell<Option<WeakRef<MaterialRangeSlider>>>,
}
ferro_class!(MaterialRangeSliderLayout: Panel);
ferro_class_info!(MaterialRangeSliderLayout {
    new: MaterialRangeSliderLayout::new
});
ferro_impl_classes!(MaterialRangeSliderLayout: FerroObjectImpl, StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);
impl MaterialRangeSliderLayout {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            owner: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn owner(&self) -> Option<Ref<MaterialRangeSlider>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}
impl LayoutableImpl for MaterialRangeSliderLayout {
    fn measure_override(this: &Self, available: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(Size::new(TARGET_SIZE, TARGET_SIZE));
        }
        let vertical = this
            .owner()
            .is_some_and(|owner| owner.orientation() == Orientation::Vertical);
        if vertical {
            Size::new(
                TARGET_SIZE,
                if available.height.is_finite() {
                    available.height.max(TARGET_SIZE)
                } else {
                    200.0
                },
            )
        } else {
            Size::new(
                if available.width.is_finite() {
                    available.width.max(TARGET_SIZE)
                } else {
                    200.0
                },
                TARGET_SIZE,
            )
        }
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        if let Some(owner) = this.owner() {
            let geometry = owner.geometry(size);
            for (index, child) in this.children().snapshot().iter().enumerate() {
                let position = if index == 0 {
                    geometry.start
                } else {
                    geometry.end
                };
                let rect = if geometry.vertical {
                    Rect::new(
                        (size.width - TARGET_SIZE) / 2.0,
                        position - TARGET_SIZE / 2.0,
                        TARGET_SIZE,
                        TARGET_SIZE,
                    )
                } else {
                    Rect::new(
                        position - TARGET_SIZE / 2.0,
                        (size.height - TARGET_SIZE) / 2.0,
                        TARGET_SIZE,
                        TARGET_SIZE,
                    )
                };
                child.arrange(rect);
            }
        }
        size
    }
}
impl VisualImpl for MaterialRangeSliderLayout {
    fn bypass_flow_direction_policies(_: &Self) -> bool {
        true
    }
    fn render(this: &Self, dc: &mut DrawingContext) {
        Self::parent_render(this, dc);
        let Some(owner) = this.owner() else {
            return;
        };
        let g = owner.geometry(this.bounds().size());
        if g.extent <= HANDLE_WIDTH {
            return;
        }
        let half_start = owner
            .thumb(MaterialRangeThumb::Start)
            .map_or(2.0, |thumb| thumb.handle_thickness() / 2.0);
        let half_end = owner
            .thumb(MaterialRangeThumb::End)
            .map_or(2.0, |thumb| thumb.handle_thickness() / 2.0);
        let (low, high, low_gap, high_gap) = if g.start <= g.end {
            (g.start, g.end, half_start + TRACK_GAP, half_end + TRACK_GAP)
        } else {
            (g.end, g.start, half_end + TRACK_GAP, half_start + TRACK_GAP)
        };
        let active = owner.foreground();
        let inactive = owner.background();
        g.segment(dc, inactive.as_ref(), g.inset, low - low_gap, 8.0, 2.0);
        g.segment(
            dc,
            active.as_ref(),
            low + low_gap,
            high - high_gap,
            2.0,
            2.0,
        );
        g.segment(
            dc,
            inactive.as_ref(),
            high + high_gap,
            g.extent - g.far_inset,
            2.0,
            8.0,
        );
        if owner.steps() > 0 {
            let intervals = owner.steps() as usize + 1;
            // Subpixel stops collapse visually; bound work by the actual pixel footprint.
            let stride =
                ((intervals as f64 * STOP_SIZE / (g.extent - g.inset - g.far_inset)).ceil()
                    as usize)
                    .max(1);
            // The two end ticks are replaced by stop indicators in AndroidX.
            for step in (1..intervals).step_by(stride) {
                let position = g.coordinate(step as f64 / intervals as f64);
                if (position - g.start).abs() < half_start + TRACK_GAP
                    || (position - g.end).abs() < half_end + TRACK_GAP
                {
                    continue;
                }
                let brush = if position > low && position < high {
                    owner.active_tick_brush()
                } else {
                    owner.inactive_tick_brush()
                };
                g.dot(dc, brush.as_ref(), position);
            }
        }
        for position in [g.inset + 8.0, g.extent - g.far_inset - 8.0] {
            if position > 0.0
                && (position - g.start).abs() >= half_start + TRACK_GAP
                && (position - g.end).abs() >= half_end + TRACK_GAP
            {
                g.dot(dc, owner.inactive_tick_brush().as_ref(), position);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Geometry {
    extent: f64,
    cross: f64,
    inset: f64,
    far_inset: f64,
    raw_min: f64,
    raw_max: f64,
    start: f64,
    end: f64,
    vertical: bool,
    reversed: bool,
    discrete: bool,
}
impl Geometry {
    fn new(
        size: Size,
        orientation: Orientation,
        reversed: bool,
        start: f64,
        end: f64,
        discrete: bool,
        start_width: f64,
        end_width: f64,
    ) -> Self {
        let vertical = orientation == Orientation::Vertical;
        let (extent, cross) = if vertical {
            (size.height, size.width / 2.0)
        } else {
            (size.width, size.height / 2.0)
        };
        let raw_max = (extent - end_width / 2.0).max(0.0);
        let raw_min = (start_width / 2.0).min(raw_max);
        let mut result = Self {
            extent: extent.max(0.0),
            cross,
            inset: if reversed { extent - raw_max } else { raw_min },
            far_inset: if reversed { raw_min } else { extent - raw_max },
            raw_min,
            raw_max,
            start: 0.0,
            end: 0.0,
            vertical,
            reversed,
            discrete,
        };
        result.start = result.coordinate(start);
        result.end = result.coordinate(end);
        result
    }
    fn coordinate(self, fraction: f64) -> f64 {
        let fraction = fraction.clamp(0.0, 1.0);
        let span = self.raw_max - self.raw_min;
        // Native discrete interior handles follow the rounded track's tick inset. Endpoint
        // handles remain at the exact ends. Pointer conversion intentionally stays linear.
        let corner = if self.discrete && fraction > 0.0 && fraction < 1.0 {
            8.0_f64.min(span / 2.0)
        } else {
            0.0
        };
        let position = self.raw_min + corner + fraction * (span - corner * 2.0);
        if self.reversed {
            self.extent - position
        } else {
            position
        }
    }
    fn fraction_at(self, point: Point) -> f64 {
        let span = self.raw_max - self.raw_min;
        if span <= 0.0 {
            return 0.0;
        }
        let position = if self.vertical { point.y } else { point.x };
        let position = if self.reversed {
            self.extent - position
        } else {
            position
        };
        ((position - self.raw_min) / span).clamp(0.0, 1.0)
    }
    fn segment(
        self,
        dc: &mut DrawingContext,
        brush: Option<&Rc<dyn IBrush>>,
        from: f64,
        to: f64,
        first_radius: f64,
        last_radius: f64,
    ) {
        if to <= from || brush.is_none() {
            return;
        }
        let rect = if self.vertical {
            Rect::new(
                self.cross - TRACK_HEIGHT / 2.0,
                from,
                TRACK_HEIGHT,
                to - from,
            )
        } else {
            Rect::new(
                from,
                self.cross - TRACK_HEIGHT / 2.0,
                to - from,
                TRACK_HEIGHT,
            )
        };
        let first = first_radius.min((to - from) / 2.0);
        let last = last_radius.min((to - from) / 2.0);
        let rounded = if self.vertical {
            RoundedRect::from_radii(rect, first, first, last, last)
        } else {
            RoundedRect::from_radii(rect, first, last, last, first)
        };
        dc.draw_rounded_rectangle(brush, None, rounded, &BoxShadows::default());
    }
    fn dot(self, dc: &mut DrawingContext, brush: Option<&Rc<dyn IBrush>>, position: f64) {
        let point = if self.vertical {
            Point::new(self.cross, position)
        } else {
            Point::new(position, self.cross)
        };
        dc.draw_ellipse(
            brush,
            None,
            Rect::new(
                point.x - STOP_SIZE / 2.0,
                point.y - STOP_SIZE / 2.0,
                STOP_SIZE,
                STOP_SIZE,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::RawInputModifiers;
    use ferroui_base::input::{
        KeyModifiers, MouseButton, Pointer, PointerPointProperties, PointerUpdateKind,
    };
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_controls::{Border, Control, Window};

    fn widget() -> (Ref<MaterialRangeSlider>, Ref<Window>) {
        let slider = MaterialRangeSlider::new();
        slider.set_range_start(20.0);
        slider.set_range_end(80.0);
        slider.set_width(240.0);
        slider.set_height(48.0);
        slider.set_horizontal_alignment(HorizontalAlignment::Left);
        slider.set_vertical_alignment(VerticalAlignment::Top);
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(320.0);
        window.set_content(Some(Control::boxed(&slider)));
        window.show();
        window.update_layout();
        (slider, window)
    }

    fn key(thumb: &MaterialRangeSliderThumb, key: Key, down: bool) -> bool {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(if down {
            InputElement::key_down_event()
        } else {
            InputElement::key_up_event()
        }));
        thumb.raise_event(&event);
        event.handled()
    }

    fn point(slider: &MaterialRangeSlider, window: &Window, fraction: f64, cross: f64) -> Point {
        let layout = slider.layout.borrow().clone().unwrap();
        let geometry = slider.geometry(layout.bounds().size());
        let p = if geometry.vertical {
            Point::new(cross, geometry.coordinate(fraction))
        } else {
            Point::new(geometry.coordinate(fraction), cross)
        };
        layout.translate_point(p, window).unwrap()
    }

    fn press(slider: &MaterialRangeSlider, window: &Window, pointer: &Rc<Pointer>, point: Point) {
        let event = PointerPressedEventArgs::new(
            slider.to_ref(),
            pointer.clone(),
            window,
            point,
            0,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        );
        slider.raise_event(&event);
    }

    fn moved(
        slider: &MaterialRangeSlider,
        window: &Window,
        pointer: &Rc<Pointer>,
        point: Point,
    ) -> bool {
        let event = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            slider.to_ref(),
            pointer.clone(),
            Some(window),
            point,
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        );
        slider.raise_event(&event);
        event.handled()
    }

    fn release(slider: &MaterialRangeSlider, window: &Window, pointer: &Rc<Pointer>, point: Point) {
        let event = PointerReleasedEventArgs::new(
            slider.to_ref(),
            pointer.clone(),
            window,
            point,
            2,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        slider.raise_event(&event);
    }

    #[test]
    fn native_geometry_insets_and_logical_fraction_round_trip_in_both_directions() {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let size = if orientation == Orientation::Horizontal {
                Size::new(240.0, 48.0)
            } else {
                Size::new(48.0, 240.0)
            };
            for reversed in [false, true] {
                let g = Geometry::new(size, orientation, reversed, 0.2, 0.8, false, 4.0, 4.0);
                assert_eq!(2.0, g.inset);
                assert_eq!(240.0, g.extent);
                for fraction in [0.0, 0.2, 0.5, 0.8, 1.0] {
                    let p = if g.vertical {
                        Point::new(24.0, g.coordinate(fraction))
                    } else {
                        Point::new(g.coordinate(fraction), 24.0)
                    };
                    assert!((g.fraction_at(p) - fraction).abs() < 1e-12);
                }
            }
        }
    }

    #[test]
    fn discrete_steps_include_endpoints_and_nonzero_minimum() {
        assert_eq!(40.0, snap(41.0, 10.0, 110.0, 9));
        assert_eq!(50.0, snap(45.0, 10.0, 110.0, 9));
        assert_eq!(10.0, snap(-500.0, 10.0, 110.0, 9));
        assert_eq!(110.0, snap(500.0, 10.0, 110.0, 9));
        assert_eq!(41.0, snap(41.0, 10.0, 110.0, 0));
        assert_eq!(10.0, snap(500.0, 10.0, 10.0, 9));
    }

    #[test]
    fn discrete_corner_inset_is_visual_while_pointer_values_remain_linear() {
        let g = Geometry::new(
            Size::new(240.0, 48.0),
            Orientation::Horizontal,
            false,
            0.2,
            0.8,
            true,
            4.0,
            4.0,
        );
        assert_eq!(2.0, g.coordinate(0.0));
        assert_eq!(238.0, g.coordinate(1.0));
        assert_eq!(54.0, g.coordinate(0.2)); // 2dp handle half + 8dp corner + 0.2 * 220dp.
        assert!((g.fraction_at(Point::new(49.2, 24.0)) - 0.2).abs() < 1e-12);
        let pressed = Geometry::new(
            Size::new(240.0, 48.0),
            Orientation::Horizontal,
            true,
            0.0,
            1.0,
            false,
            2.0,
            4.0,
        );
        assert_eq!(239.0, pressed.start);
        assert_eq!(2.0, pressed.end);
    }

    #[test]
    fn endpoint_properties_enforce_order_and_recoerce_when_range_changes() {
        let _app = crate::tests::support::start_themed_application();
        let slider = MaterialRangeSlider::new();
        slider.set_range_start(20.0);
        slider.set_range_end(80.0);
        slider.set_range_start(90.0);
        assert_eq!(80.0, slider.range_start());
        slider.set_range_end(10.0);
        assert_eq!(80.0, slider.range_end());
        slider.set_maximum(40.0);
        assert_eq!((40.0, 40.0), (slider.range_start(), slider.range_end()));
        slider.set_minimum(50.0);
        assert_eq!(
            (50.0, 50.0, 50.0),
            (slider.maximum(), slider.range_start(), slider.range_end())
        );
    }

    #[test]
    fn compiled_template_positions_two_real_handles_and_keeps_touch_cells() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        for role in [MaterialRangeThumb::Start, MaterialRangeThumb::End] {
            let thumb = slider.thumb(role).expect("compiled thumb");
            assert_eq!(Size::new(48.0, 48.0), thumb.bounds().size());
            assert!(thumb.focusable());
            let handle = thumb
                .get_visual_descendants()
                .find(|node| node.name().as_deref() == Some("PART_Handle"))
                .unwrap()
                .cast::<Border>()
                .unwrap();
            assert_eq!(Size::new(4.0, 44.0), handle.bounds().size());
        }
        let a = slider.thumb(MaterialRangeThumb::Start).unwrap();
        let b = slider.thumb(MaterialRangeThumb::End).unwrap();
        assert!(a.bounds().x < b.bounds().x);
        slider.set_flow_direction(FlowDirection::RightToLeft);
        window.update_layout();
        assert!(a.bounds().x > b.bounds().x);
        window.close();
    }

    #[test]
    fn keyboard_steps_rtl_page_and_bounds_match_native_range_contract() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        let start = slider.thumb(MaterialRangeThumb::Start).unwrap();
        assert!(key(&start, Key::Right, true));
        assert_eq!(21.0, slider.range_start());
        assert!(
            !key(&start, Key::Up, true),
            "horizontal range slider must leave vertical arrows to its host"
        );
        slider.set_flow_direction(FlowDirection::RightToLeft);
        assert!(key(&start, Key::Right, true));
        assert_eq!(20.0, slider.range_start());
        slider.set_steps(9);
        assert!(key(&start, Key::Left, true));
        assert_eq!(30.0, slider.range_start());
        assert!(key(&start, Key::End, true));
        assert_eq!(80.0, slider.range_start());
        let end = slider.thumb(MaterialRangeThumb::End).unwrap();
        assert!(key(&end, Key::Home, true));
        assert_eq!(80.0, slider.range_end());
        assert!(key(&end, Key::PageUp, true));
        assert_eq!(90.0, slider.range_end());
        window.close();
    }

    #[test]
    fn each_thumb_has_its_own_named_bounded_range_provider_and_disabled_policy() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        slider.set_steps(9);
        slider.set_start_label("Minimum price");
        slider.set_end_label("Maximum price");
        let start = slider.thumb(MaterialRangeThumb::Start).unwrap();
        let end = slider.thumb(MaterialRangeThumb::End).unwrap();
        let start_peer = ControlAutomationPeer::create_peer_for_element(&start);
        let end_peer = ControlAutomationPeer::create_peer_for_element(&end);
        assert_eq!(
            AutomationControlType::Slider,
            start_peer.get_automation_control_type()
        );
        assert_eq!("Minimum price", start_peer.get_name());
        assert_eq!("Maximum price", end_peer.get_name());
        let first = start_peer
            .get_provider::<dyn IRangeValueProvider>()
            .unwrap();
        let second = end_peer.get_provider::<dyn IRangeValueProvider>().unwrap();
        assert_eq!(
            (0.0, 80.0, 20.0),
            (first.minimum(), first.maximum(), first.value())
        );
        assert_eq!(
            (20.0, 100.0, 80.0),
            (second.minimum(), second.maximum(), second.value())
        );
        first.set_value(47.0).unwrap();
        assert_eq!(50.0, slider.range_start());
        assert_eq!(50.0, second.minimum());
        second.set_value(30.0).unwrap();
        assert_eq!(50.0, slider.range_end());
        slider.set_is_read_only(true);
        assert!(first.is_read_only());
        assert!(first.set_value(10.0).is_err());
        slider.set_is_read_only(false);
        slider.set_is_enabled(false);
        assert!(first.set_value(10.0).is_err());
        assert_eq!(50.0, slider.range_start());
        window.close();
    }

    #[test]
    fn mouse_drag_selects_nearest_thumb_captures_and_cannot_cross() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let initial = point(&slider, &window, 0.25, 24.0);
        press(&slider, &window, &pointer, initial);
        assert_eq!(25.0, slider.range_start());
        assert!(pointer.captured().is_some());
        let end = point(&slider, &window, 0.95, 24.0);
        assert!(moved(&slider, &window, &pointer, end));
        assert_eq!(80.0, slider.range_start());
        release(&slider, &window, &pointer, end);
        assert!(pointer.captured().is_none());
        assert!(slider.drag.borrow().is_none());
        window.close();
    }

    #[test]
    fn touch_cross_axis_drag_yields_to_scroll_and_never_changes_value() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
        press(
            &slider,
            &window,
            &pointer,
            point(&slider, &window, 0.25, 24.0),
        );
        assert_eq!(20.0, slider.range_start());
        assert!(!moved(
            &slider,
            &window,
            &pointer,
            point(&slider, &window, 0.25, 50.0)
        ));
        assert!(slider.drag.borrow().is_none());
        assert!(pointer.captured().is_none());
        assert_eq!(20.0, slider.range_start());
        window.close();
    }

    #[test]
    fn coincident_touch_handles_choose_drag_direction_and_release_commits_once() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        slider.set_range_start(50.0);
        slider.set_range_end(50.0);
        let count = Rc::new(Cell::new(0));
        let counter = count.clone();
        slider.add_handler(
            MaterialRangeSlider::value_change_finished_event(),
            move |_, _| counter.set(counter.get() + 1),
        );
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
        press(
            &slider,
            &window,
            &pointer,
            point(&slider, &window, 0.5, 24.0),
        );
        assert!(moved(
            &slider,
            &window,
            &pointer,
            point(&slider, &window, 0.3, 24.0)
        ));
        assert_eq!((30.0, 50.0), (slider.range_start(), slider.range_end()));
        release(
            &slider,
            &window,
            &pointer,
            point(&slider, &window, 0.3, 24.0),
        );
        assert_eq!(1, count.get());
        window.close();
    }

    #[test]
    fn detaching_captured_range_releases_pointer_and_pressed_state() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        press(
            &slider,
            &window,
            &pointer,
            point(&slider, &window, 0.3, 24.0),
        );
        assert!(pointer.captured().is_some());
        window.set_content(None);
        window.update_layout();
        assert!(pointer.captured().is_none());
        assert!(slider.drag.borrow().is_none());
        assert!(
            !slider
                .thumb(MaterialRangeThumb::Start)
                .unwrap()
                .classes()
                .contains(":pressed")
        );
        window.close();
    }

    #[test]
    fn vertical_extension_rotates_handle_and_keyboard_geometry() {
        let _app = crate::tests::support::start_themed_application();
        let (slider, window) = widget();
        slider.set_orientation(Orientation::Vertical);
        slider.set_width(48.0);
        slider.set_height(240.0);
        window.update_layout();
        let start = slider.thumb(MaterialRangeThumb::Start).unwrap();
        let end = slider.thumb(MaterialRangeThumb::End).unwrap();
        assert!(start.bounds().y > end.bounds().y);
        let handle = start
            .get_visual_descendants()
            .find(|node| node.name().as_deref() == Some("PART_Handle"))
            .unwrap()
            .cast::<Border>()
            .unwrap();
        assert_eq!(Size::new(44.0, 4.0), handle.bounds().size());
        assert!(key(&start, Key::Up, true));
        assert_eq!(21.0, slider.range_start());
        assert!(!key(&start, Key::Left, true));
        window.close();
    }
}
