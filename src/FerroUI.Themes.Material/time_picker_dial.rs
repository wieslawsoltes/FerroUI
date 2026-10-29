// Copyright 2023 The Android Open Source Project (reference behavior).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Circular time selector, preserving spring velocity, native clipping, and real
//! RadioButton accessibility nodes. AndroidX TimePicker.kt reference is documented
//! in time_picker.rs; physical dimensions are in time_picker_state.rs.

use crate::motion::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSpringState, animations_enabled,
};
use crate::time_picker::MaterialTimePicker;
use crate::time_picker_state::{
    DialGeometry, MaterialTimeDisplayMode, MaterialTimeSelection, angle_for, shortest_target,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    KeyboardDevice, KeyboardNavigation, NavigationMethod, PointerCaptureLostEventArgs,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    BrushExtensions, DrawingContext, EllipseGeometry, FlowDirection, FormattedText, Geometry,
    IBrush, IPen, PenLineCap, PenLineJoin, Typeface,
};
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Point, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::automation::peers::{
    AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    RadioButtonAutomationPeer, RadioButtonAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{
    IInvokeProvider, ISelectionItemProvider, ISelectionProvider,
};
use ferroui_controls::automation::{AutomationProperties, ElementNotEnabledException};
use ferroui_controls::primitives::{
    TemplatedControl, TemplatedControlImpl, ToggleButton, ToggleButtonImpl,
};
use ferroui_controls::{
    ButtonImpl, ButtonImplExt, ContentControlImpl, Control, ControlImpl, Panel, PanelImpl,
    RadioButton,
};
use std::cell::{Cell, RefCell};
use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Face {
    selection: MaterialTimeSelection,
    is_24_hour: bool,
}
impl Face {
    fn count(self) -> usize {
        if self.is_24_hour && self.selection == MaterialTimeSelection::Hour {
            24
        } else {
            12
        }
    }
    fn value(self, index: usize) -> i32 {
        if self.selection == MaterialTimeSelection::Minute {
            (index % 12 * 5) as i32
        } else if self.is_24_hour {
            index as i32
        } else if index == 0 {
            12
        } else {
            index as i32
        }
    }
}
struct DialDrag {
    pointer: Rc<dyn IPointer>,
    origin: Point,
    dragging: bool,
}

/// Template panel displaying the dial and twenty-four individually selectable hour/minute nodes.
#[repr(C)]
pub struct MaterialTimePickerDial {
    base: Panel,
    owner: RefCell<Option<WeakRef<MaterialTimePicker>>>,
    face: Cell<Face>,
    old_face: Cell<Option<Face>>,
    updating: Cell<bool>,
    hand: RefCell<MaterialSpringState>,
    fade: RefCell<MaterialSpringState>,
    inner: Cell<bool>,
    pending_advance: Cell<Option<f64>>,
    drag: RefCell<Option<DialDrag>>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    last_frame: Cell<Option<f64>>,
}
ferro_class!(MaterialTimePickerDial: Panel);
ferro_class_info!(MaterialTimePickerDial {
    new: MaterialTimePickerDial::new
});
ferro_impl_classes!(MaterialTimePickerDial: StyledElementImpl, InteractiveImpl, ControlImpl, PanelImpl);
impl FerroObjectImpl for MaterialTimePickerDial {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        for index in 0..24 {
            let number = MaterialTimePickerNumber::new();
            *number.dial.borrow_mut() = Some(this.to_ref().downgrade());
            number.index.set(index);
            // All values are logical siblings; the framework scopes the unnamed
            // radio group to this dial rather than sharing a process-wide name.
            this.children().add(number.upcast::<Control>());
        }
        // Tunnel before the individual RadioButtons so mouse/touch gestures share
        // one capture and tapping can auto-advance without changing UIA behavior.
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.press(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == InputElement::is_effectively_enabled_property().as_property()
            && !this.is_effectively_enabled()
        {
            this.cancel_interaction();
        }
        this.invalidate_visual();
        for child in this.children().snapshot().iter() {
            child.invalidate_visual();
        }
    }
}
impl VisualImpl for MaterialTimePickerDial {
    fn bypass_flow_direction_policies(_: &Self) -> bool {
        true
    }
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let weak = this.to_ref().downgrade();
        let sub = this.is_effectively_visible_changed(move || {
            if let Some(this) = weak.upgrade() {
                if !this.is_effectively_visible() {
                    this.cancel_interaction();
                }
                this.update_clock();
            }
        });
        if let Some(old) = this.visibility_subscription.replace(Some(sub)) {
            old.dispose();
        }
        this.synchronize();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.cancel_interaction();
        this.stop_clock();
        if let Some(sub) = this.visibility_subscription.borrow_mut().take() {
            sub.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
    fn render(this: &Self, dc: &mut DrawingContext) {
        // Panel's rectangular background is deliberately not painted: the dial is circular.
        let geometry = DialGeometry::new(this.bounds().size());
        dc.draw_ellipse_at(
            this.background().as_ref(),
            None,
            geometry.center,
            geometry.diameter / 2.0,
            geometry.diameter / 2.0,
        );
        let hand = geometry.center_at(this.hand_angle(), this.inner.get());
        let brush = this.hand_brush();
        let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::new(
            brush.as_ref().map(BrushExtensions::to_immutable),
            2.0,
            None,
            PenLineCap::Round,
            PenLineJoin::Round,
            4.0,
        ));
        dc.draw_line(&pen, geometry.center, hand);
        dc.draw_ellipse_at(
            brush.as_ref(),
            None,
            hand,
            24.0 * geometry.scale,
            24.0 * geometry.scale,
        );
        dc.draw_ellipse_at(brush.as_ref(), None, geometry.center, 4.0, 4.0);
        if let Some(old) = this.old_face.get() {
            let alpha = 1.0 - this.label_opacity();
            if alpha > 0.0 {
                if let Some(number) = this.numbers().first() {
                    let opacity = dc.push_opacity(alpha);
                    for index in 0..old.count() {
                        let bounds = geometry.number_rect(index);
                        this.draw_label(
                            dc,
                            number,
                            &old.value(index).to_string(),
                            bounds,
                            Point::new(0.0, 0.0),
                        );
                    }
                    dc.pop(opacity);
                }
            }
        }
    }
}
impl InputElementImpl for MaterialTimePickerDial {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let point = e.get_position(Some(this));
        let dragging = {
            let mut drag = this.drag.borrow_mut();
            let Some(drag) = drag.as_mut().filter(|d| d.pointer.id() == e.pointer().id()) else {
                return;
            };
            if (point.x - drag.origin.x).hypot(point.y - drag.origin.y) > 8.0 {
                drag.dragging = true;
            }
            drag.dragging
        };
        if dragging {
            this.select_pointer(point, false);
            e.prevent_gesture_recognition();
            e.set_handled(true);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        let drag = {
            let mut slot = this.drag.borrow_mut();
            if slot
                .as_ref()
                .is_none_or(|d| d.pointer.id() != e.pointer().id())
            {
                return;
            }
            slot.take().unwrap()
        };
        this.select_pointer(e.get_position(Some(this)), !drag.dragging);
        drag.pointer.capture(None);
        if let Some(owner) = this.owner() {
            if owner.selection() == MaterialTimeSelection::Hour && owner.auto_advance() {
                if drag.dragging {
                    owner.set_selection(MaterialTimeSelection::Minute);
                } else {
                    this.pending_advance.set(Some(0.1));
                }
            }
            owner.finish_edit();
        }
        this.synchronize();
        this.update_clock();
        e.set_handled(true);
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        this.cancel_interaction();
        Self::parent_on_pointer_capture_lost(this, e);
    }
}
impl LayoutableImpl for MaterialTimePickerDial {
    fn measure_override(this: &Self, available: Size) -> Size {
        for child in this.children().snapshot().iter() {
            child.measure(Size::new(48.0, 48.0));
        }
        let side = available.width.min(available.height).min(256.0);
        Size::new(side, side)
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let geometry = DialGeometry::new(size);
        for (index, child) in this.children().snapshot().iter().enumerate() {
            child.arrange(geometry.number_rect(index));
        }
        size
    }
}
ferro_properties! { impl MaterialTimePickerDial {
    pub fn hand_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self,_>("HandBrush",None) }
    pub fn number_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self,_>("NumberBrush",None) }
    pub fn selected_number_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self,_>("SelectedNumberBrush",None) }
}}
impl MaterialTimePickerDial {
    pub fn construct() -> Self {
        let scheme = MaterialMotionScheme::Expressive;
        Self {
            base: Panel::construct(),
            owner: RefCell::new(None),
            face: Cell::new(Face {
                selection: MaterialTimeSelection::Hour,
                is_24_hour: false,
            }),
            old_face: Cell::new(None),
            updating: Cell::new(false),
            hand: RefCell::new(MaterialSpringState::new(
                scheme.spring(MaterialMotionRole::DefaultSpatial),
                -FRAC_PI_2,
            )),
            fade: RefCell::new(MaterialSpringState::new(
                scheme.spring(MaterialMotionRole::DefaultEffects),
                1.0,
            )),
            inner: Cell::new(false),
            pending_advance: Cell::new(None),
            drag: RefCell::new(None),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn hand_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::hand_brush_property())
    }
    pub fn set_hand_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::hand_brush_property(), value);
    }
    pub fn number_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::number_brush_property())
    }
    pub fn set_number_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::number_brush_property(), value);
    }
    pub fn selected_number_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selected_number_brush_property())
    }
    pub fn set_selected_number_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selected_number_brush_property(), value);
    }
    pub fn hand_angle(&self) -> f64 {
        self.hand.borrow().current().value
    }
    pub fn hand_velocity(&self) -> f64 {
        self.hand.borrow().current().velocity
    }
    pub fn label_opacity(&self) -> f64 {
        self.fade.borrow().current().value.clamp(0.0, 1.0)
    }
    #[cfg(test)]
    pub(crate) fn has_clock_subscription(&self) -> bool {
        self.clock_subscription.borrow().is_some()
    }
    pub(crate) fn owner(&self) -> Option<Ref<MaterialTimePicker>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn set_owner(&self, owner: Option<WeakRef<MaterialTimePicker>>) {
        *self.owner.borrow_mut() = owner;
    }
    pub(crate) fn numbers(&self) -> Vec<Ref<MaterialTimePickerNumber>> {
        self.children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast())
            .collect()
    }
    pub(crate) fn synchronize(&self) {
        let Some(owner) = self.owner() else {
            return;
        };
        let face = Face {
            selection: owner.selection(),
            is_24_hour: owner.is_24_hour(),
        };
        let old = self.face.replace(face);
        let scheme = owner.motion_scheme();
        if old != face {
            self.pending_advance.set(None);
            self.old_face.set(Some(old));
            *self.fade.borrow_mut() =
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::DefaultEffects), 0.0);
            self.fade.borrow_mut().retarget(1.0);
        }
        self.hand
            .borrow_mut()
            .set_spec(scheme.spring(MaterialMotionRole::DefaultSpatial));
        self.fade
            .borrow_mut()
            .set_spec(scheme.spring(MaterialMotionRole::DefaultEffects));
        self.inner.set(
            face.is_24_hour && face.selection == MaterialTimeSelection::Hour && owner.hour() >= 12,
        );
        if !self.drag.borrow().as_ref().is_some_and(|d| d.dragging) {
            let target = shortest_target(
                self.hand_angle(),
                angle_for(owner.hour(), owner.minute(), owner.selection()),
            );
            self.hand.borrow_mut().retarget(target);
        }
        self.updating.set(true);
        for (index, number) in self.numbers().into_iter().enumerate() {
            number.set_is_visible(index < face.count());
            let value = face.value(index);
            let selected = index < face.count()
                && if face.selection == MaterialTimeSelection::Minute {
                    ((owner.minute() + 2) / 5) % 12 == index as i32
                } else if face.is_24_hour {
                    owner.hour() == value
                } else {
                    owner.hour() % 12 == value % 12
                };
            number.set_is_checked(Some(selected));
            KeyboardNavigation::set_is_tab_stop(&number, selected);
            let label = if face.selection == MaterialTimeSelection::Hour {
                owner.hour_label()
            } else {
                owner.minute_label()
            };
            AutomationProperties::set_name(&number, Some(&format!("{value} {label}")));
            number.invalidate_visual();
        }
        self.updating.set(false);
        if !self.is_attached_to_visual_tree()
            || owner.display_mode() != MaterialTimeDisplayMode::Dial
        {
            self.hand.borrow_mut().finish();
            self.fade.borrow_mut().finish();
            self.old_face.set(None);
            self.cancel_interaction();
        }
        self.advance_motion(0.0);
        self.update_clock();
        self.invalidate_visual();
    }
    fn press(&self, e: &PointerPressedEventArgs) {
        let Some(_owner) = self.owner().filter(|o| o.can_edit()) else {
            return;
        };
        if self.drag.borrow().is_some() {
            return;
        }
        if e.pointer().type_() == PointerType::Mouse
            && !e
                .get_current_point(Some(self))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        let point = e.get_position(Some(self));
        let g = DialGeometry::new(self.bounds().size());
        if (point.x - g.center.x).hypot(point.y - g.center.y) > g.diameter / 2.0 {
            return;
        }
        self.pending_advance.set(None);
        *self.drag.borrow_mut() = Some(DialDrag {
            pointer: e.pointer().clone(),
            origin: point,
            dragging: false,
        });
        e.pointer().capture(Some(&self.to_ref().upcast()));
        e.prevent_gesture_recognition();
        e.set_handled(true);
        // Touch clears keyboard focus, matching the Android dial's onPress path.
        if let Some(keyboard) = KeyboardDevice::instance() {
            keyboard.set_focused_element(None, NavigationMethod::Pointer, KeyModifiers::NONE);
        }
    }
    fn select_pointer(&self, position: Point, tapping: bool) {
        let Some(owner) = self.owner().filter(|o| o.can_edit()) else {
            return;
        };
        let (value, angle) = DialGeometry::new(self.bounds().size()).value_at(
            position,
            owner.selection(),
            owner.is_24_hour(),
            owner.hour() >= 12,
            tapping,
        );
        owner.select_number(value);
        if !tapping {
            let target = shortest_target(self.hand_angle(), angle);
            self.hand.borrow_mut().retarget(target);
            self.hand.borrow_mut().finish();
        }
        self.invalidate_visual();
        for number in self.numbers() {
            number.invalidate_visual();
        }
    }

    fn activate_number(&self, index: usize, semantic_click: bool) -> bool {
        let Some(owner) = self.owner().filter(|owner| {
            owner.can_edit() && owner.display_mode() == MaterialTimeDisplayMode::Dial
        }) else {
            return false;
        };
        if index >= self.face.get().count() {
            return false;
        }
        self.cancel_interaction();
        owner.select_number(self.face.get().value(index));
        // ClockText uses SnapSpec for keyboard and semantic activation. Pointer
        // taps use the animated path instead. Activation also applies to the
        // already selected number, which raises no IsChecked property change.
        self.hand.borrow_mut().finish();
        if semantic_click
            && owner.auto_advance()
            && owner.selection() == MaterialTimeSelection::Hour
        {
            if animations_enabled(owner.reduced_motion()) && self.is_attached_to_visual_tree() {
                self.pending_advance.set(Some(0.1));
            } else {
                owner.set_selection(MaterialTimeSelection::Minute);
            }
        }
        owner.finish_edit();
        self.invalidate_visual();
        for number in self.numbers() {
            number.invalidate_visual();
        }
        self.update_clock();
        true
    }
    pub(crate) fn cancel_interaction(&self) {
        self.pending_advance.set(None);
        let drag = self.drag.borrow_mut().take();
        if let Some(drag) = drag {
            if drag.pointer.captured().is_some() {
                drag.pointer.capture(None);
            }
        }
        if !self.is_attached_to_visual_tree() || !self.is_effectively_visible() {
            self.stop_clock();
        }
    }
    pub(crate) fn focus_selected(&self) {
        if let Some(number) = self
            .numbers()
            .into_iter()
            .find(|n| n.is_visible() && n.is_checked() == Some(true))
        {
            number.focus_with(NavigationMethod::Directional, KeyModifiers::empty());
        }
    }
    fn motion_pending(&self) -> bool {
        !self.hand.borrow().is_at_rest(0.001, 0.01) || !self.fade.borrow().is_at_rest(0.001, 0.01)
    }
    fn advance_motion(&self, seconds: f64) {
        let hand_was_moving = !self.hand.borrow().is_at_rest(0.001, 0.01);
        let reduced = self
            .owner()
            .is_none_or(|o| !animations_enabled(o.reduced_motion()));
        for spring in [&self.hand, &self.fade] {
            let mut state = spring.borrow_mut();
            state.set_reduced_motion(reduced);
            state.advance(seconds);
            if state.is_at_rest(0.001, 0.01) {
                state.finish();
            }
        }
        if self.fade.borrow().is_at_rest(0.001, 0.01) {
            self.old_face.set(None);
        }
        // Native tap auto-advance waits for the hand motion, then another 100ms.
        if !hand_was_moving && self.hand.borrow().is_at_rest(0.001, 0.01) {
            if let Some(remaining) = self.pending_advance.get() {
                let remaining = remaining - seconds;
                if remaining <= 0.0 {
                    self.pending_advance.set(None);
                    if let Some(owner) = self.owner().filter(|o| {
                        o.can_edit()
                            && o.auto_advance()
                            && o.selection() == MaterialTimeSelection::Hour
                    }) {
                        owner.set_selection(MaterialTimeSelection::Minute);
                    }
                } else {
                    self.pending_advance.set(Some(remaining));
                }
            }
        }
        self.invalidate_visual();
        for number in self.numbers() {
            number.invalidate_visual();
        }
    }
    fn update_clock(&self) {
        if self
            .owner()
            .is_none_or(|o| !animations_enabled(o.reduced_motion()))
        {
            self.advance_motion(0.0);
        }
        if !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || (!self.motion_pending() && self.pending_advance.get().is_none())
        {
            self.stop_clock();
            return;
        }
        if self.clock_subscription.borrow().is_some() {
            return;
        }
        let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() else {
            self.hand.borrow_mut().finish();
            self.fade.borrow_mut().finish();
            self.pending_advance.set(None);
            return;
        };
        self.last_frame.set(None);
        let weak = self.to_ref().downgrade();
        let subscription =
            clock.subscribe(Rc::new(AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    if !this.is_attached_to_visual_tree() || !this.is_effectively_visible() {
                        this.cancel_interaction();
                        this.stop_clock();
                        return;
                    }
                    let time = time.total_seconds();
                    if !time.is_finite() {
                        return;
                    }
                    let delta = this
                        .last_frame
                        .replace(Some(time))
                        .map_or(0.0, |last| (time - last).max(0.0));
                    this.advance_motion(delta);
                    if !this.motion_pending() && this.pending_advance.get().is_none() {
                        this.stop_clock();
                    }
                }
            })));
        *self.clock_subscription.borrow_mut() = Some(subscription);
    }
    fn stop_clock(&self) {
        if let Some(subscription) = self.clock_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.last_frame.set(None);
    }
    fn draw_label(
        &self,
        dc: &mut DrawingContext,
        number: &MaterialTimePickerNumber,
        text: &str,
        bounds: Rect,
        origin: Point,
    ) {
        let typeface = Typeface::with_style(
            number.font_family(),
            number.font_style(),
            number.font_weight(),
            number.font_stretch(),
        );
        let normal = FormattedText::new(
            text,
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            typeface.clone(),
            number.font_size(),
            self.number_brush(),
        );
        let selected = FormattedText::new(
            text,
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            typeface,
            number.font_size(),
            self.selected_number_brush(),
        );
        let position = Point::new(
            bounds.x + (bounds.width - normal.width()) / 2.0,
            bounds.y + (bounds.height - normal.height()) / 2.0,
        );
        dc.draw_text(&normal, position);
        let geometry = DialGeometry::new(self.bounds().size());
        let center = geometry.center_at(self.hand_angle(), self.inner.get());
        let radius = 24.0 * geometry.scale;
        let clip = EllipseGeometry::new();
        clip.set_rect(Rect::new(
            center.x - origin.x - radius,
            center.y - origin.y - radius,
            2.0 * radius,
            2.0 * radius,
        ));
        let clip_state = dc.push_geometry_clip(&clip.upcast::<Geometry>());
        dc.draw_text(&selected, position);
        dc.pop(clip_state);
    }
}
impl Drop for MaterialTimePickerDial {
    fn drop(&mut self) {
        if let Some(s) = self.clock_subscription.get_mut().take() {
            s.dispose();
        }
        if let Some(s) = self.visibility_subscription.get_mut().take() {
            s.dispose();
        }
    }
}

/// Each clock value retains RadioButton's selection-item automation contract.
#[repr(C)]
pub struct MaterialTimePickerNumber {
    base: RadioButton,
    dial: RefCell<Option<WeakRef<MaterialTimePickerDial>>>,
    index: Cell<usize>,
}
ferro_class!(MaterialTimePickerNumber:RadioButton);
ferro_class_info!(MaterialTimePickerNumber {
    new: MaterialTimePickerNumber::new
});
ferro_impl_classes!(MaterialTimePickerNumber:StyledElementImpl,LayoutableImpl,InteractiveImpl,TemplatedControlImpl,ContentControlImpl,ToggleButtonImpl);
impl FerroObjectImpl for MaterialTimePickerNumber {
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        if e.property() == ToggleButton::is_checked_property().as_property()
            && this.is_checked() == Some(true)
        {
            if let Some(dial) = this.dial() {
                if !dial.updating.get() {
                    if !dial.activate_number(this.index.get(), false) {
                        dial.synchronize();
                    }
                }
            }
        }
        if e.property() == TemplatedControl::font_size_property().as_property() {
            this.invalidate_visual();
        }
    }
}
impl ControlImpl for MaterialTimePickerNumber {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MaterialTimePickerNumberAutomationPeer::new(this).upcast()
    }
}
impl ButtonImpl for MaterialTimePickerNumber {
    fn on_click(this: &Self) {
        if this.activate(false) {
            Self::parent_on_click(this);
        }
    }
}
impl VisualImpl for MaterialTimePickerNumber {
    fn bypass_flow_direction_policies(_: &Self) -> bool {
        true
    }
    fn render(this: &Self, dc: &mut DrawingContext) {
        Self::parent_render(this, dc);
        if let Some(dial) = this.dial() {
            let opacity = dc.push_opacity(dial.label_opacity());
            let bounds = this.bounds();
            dial.draw_label(
                dc,
                this,
                &dial.face.get().value(this.index.get()).to_string(),
                Rect::new(0.0, 0.0, bounds.width, bounds.height),
                Point::new(bounds.x, bounds.y),
            );
            dc.pop(opacity);
        }
    }
}
impl InputElementImpl for MaterialTimePickerNumber {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if let Some(dial) = this.dial() {
            if e.key == Key::Tab {
                if let Some(owner) = dial.owner() {
                    owner.exit_dial(e.key_modifiers.contains(KeyModifiers::SHIFT));
                    e.set_handled(true);
                    return;
                }
            }
            if e.key_modifiers.is_empty() {
                let direction = match e.key {
                    Key::Left | Key::Up => -1,
                    Key::Right | Key::Down => 1,
                    _ => 0,
                };
                if direction != 0 {
                    let count = dial.face.get().count();
                    let next =
                        (this.index.get() as i32 + direction).rem_euclid(count as i32) as usize;
                    if let Some(number) = dial.numbers().get(next) {
                        number.focus_with(NavigationMethod::Directional, KeyModifiers::empty());
                    }
                    e.set_handled(true);
                    return;
                }
            }
        }
        Self::parent_on_key_down(this, e);
    }
}
impl MaterialTimePickerNumber {
    pub fn construct() -> Self {
        Self {
            base: RadioButton::construct(),
            dial: RefCell::new(None),
            index: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn dial(&self) -> Option<Ref<MaterialTimePickerDial>> {
        self.dial.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    fn activate(&self, semantic_click: bool) -> bool {
        self.dial()
            .is_some_and(|dial| dial.activate_number(self.index.get(), semantic_click))
    }
}

/// A clock number exposes selection and the distinct native semantic click.
/// Invoking may advance to minutes; selection and keyboard activation do not.
#[repr(C)]
pub struct MaterialTimePickerNumberAutomationPeer {
    base: RadioButtonAutomationPeer,
}
ferro_class!(MaterialTimePickerNumberAutomationPeer: RadioButtonAutomationPeer);
ferro_class_info!(MaterialTimePickerNumberAutomationPeer {
    interfaces: [
        Rc<dyn IInvokeProvider> => MaterialTimePickerNumberAutomationPeer::invoke_provider,
        Rc<dyn ISelectionItemProvider> => MaterialTimePickerNumberAutomationPeer::selection_provider
    ]
});
ferro_impl_classes!(MaterialTimePickerNumberAutomationPeer: FerroObjectImpl, AutomationPeerImpl, ControlAutomationPeerImpl, RadioButtonAutomationPeerImpl);
impl MaterialTimePickerNumberAutomationPeer {
    pub fn new(owner: &MaterialTimePickerNumber) -> Ref<Self> {
        instantiate(Self {
            base: RadioButtonAutomationPeer::construct(owner),
        })
    }
    fn number(&self) -> Ref<MaterialTimePickerNumber> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("clock number owner")
    }
    fn invoke_provider(peer: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(TimeNumberProvider(peer))
    }
    fn selection_provider(peer: Ref<Self>) -> Rc<dyn ISelectionItemProvider> {
        Rc::new(TimeNumberProvider(peer))
    }
    fn activate(&self, semantic_click: bool) -> Result<(), ElementNotEnabledException> {
        if self.number().activate(semantic_click) {
            Ok(())
        } else {
            Err(ElementNotEnabledException::with_message(
                "The clock number is unavailable, disabled, or read-only.",
            ))
        }
    }
}
struct TimeNumberProvider(Ref<MaterialTimePickerNumberAutomationPeer>);
impl IInvokeProvider for TimeNumberProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.activate(true)
    }
}
impl ISelectionItemProvider for TimeNumberProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.number().is_checked() == Some(true)
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        None
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.add_to_selection()
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.remove_from_selection()
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.activate(false)
    }
}
