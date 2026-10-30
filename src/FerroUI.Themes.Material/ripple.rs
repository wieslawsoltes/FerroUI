// Copyright 2019 The Android Open Source Project (ripple animation reference).
// Copyright 2026 FerroUI contributors (FerroUI drawing, input and lifecycle integration).
// SPDX-License-Identifier: Apache-2.0
//! The pointer and keyboard press indication used by Material control templates.
//!
//! Timing and radius formulas follow AndroidX Material3 1.5.0-beta02 and
//! `compose/material/material-ripple/.../RippleAnimation.kt` at commit
//! `e4d933c98df7baa7ea2b83eea6e086915961e0ff` (Copyright The Android Open Source
//! Project, Apache-2.0). Drawing, input subscription and lifecycle management use FerroUI.

use crate::motion::animations_enabled;
use ferroui_base::animation::{KeySplineSolver, TimeSpan};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, Key, KeyEventArgs,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{DrawingContext, IBrush, MediaContext};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property,
    instantiate, CornerRadius, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref,
    RoundedRect, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, Visual,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::{Button, Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const FADE_IN_SECONDS: f64 = 0.075;
const RADIUS_SECONDS: f64 = 0.225;
const FADE_OUT_SECONDS: f64 = 0.150;
const MAX_SIMULTANEOUS_RIPPLES: usize = 16;

/// A bounded Material ink ripple. Place it inside the root of a control template, above the
/// background/state layer and below the content. It observes the templated parent's input,
/// so it neither intercepts hit testing nor changes the control's click, capture or focus behavior.
///
/// Hover and focus state layers remain the template's responsibility. A separate pressed state
/// layer must not be painted on top of this ripple: its default peak opacity is already 0.10.
///
/// ```xml
/// <material:MaterialRipple Fill="{TemplateBinding Foreground}"
///                          CornerRadius="{TemplateBinding CornerRadius}"
///                          ReducedMotion="{DynamicResource MaterialReducedMotion}" />
/// ```
///
/// Each touch contact has an independent wave; quick taps finish expanding before fading out.
/// Frame callbacks and input subscriptions use weak references and are canceled on detachment.
#[repr(C)]
pub struct MaterialRipple {
    base: Control,
    host: RefCell<Option<HostSubscriptions>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    waves: RefCell<Vec<RippleWave>>,
    frame_pending: Cell<bool>,
    frame_generation: Cell<u64>,
    last_frame: Cell<Option<TimeSpan>>,
}

ferro_class!(MaterialRipple: Control);
ferro_class_info!(MaterialRipple {
    new: MaterialRipple::new
});
ferro_impl_classes!(MaterialRipple: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

ferro_properties! { impl MaterialRipple {
    ferro_property!(
        /// The ripple color, normally the surrounding component's content color.
        pub fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<MaterialRipple, _>("Fill", None)
        }
    );

    ferro_property!(
        /// The clipping corners; bind to the animated surface corners for shape-changing buttons.
        pub fn corner_radius_property() -> StyledProperty<CornerRadius> {
            FerroProperty::register::<MaterialRipple, _>("CornerRadius", CornerRadius::default())
        }
    );

    ferro_property!(
        /// Whether the ripple is clipped to the rounded layout bounds.
        pub fn is_bounded_property() -> StyledProperty<bool> {
            FerroProperty::register::<MaterialRipple, _>("IsBounded", true)
        }
    );

    ferro_property!(
        /// An explicit final radius. NaN computes the radius from the control's bounds.
        pub fn radius_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<MaterialRipple, _>(
                "Radius",
                StyledPropertyOptions::new(f64::NAN)
                    .validate(|value| value.is_nan() || (value.is_finite() && *value >= 0.0)),
            )
        }
    );

    ferro_property!(
        /// The Material pressed-state opacity, multiplied by the brush's own alpha.
        pub fn interaction_opacity_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<MaterialRipple, _>(
                "InteractionOpacity",
                StyledPropertyOptions::new(0.10_f64)
                    .validate(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            )
        }
    );

    ferro_property!(
        /// Replaces radial motion by a steady pressed indication. Also honors platform settings.
        pub fn reduced_motion_property() -> StyledProperty<bool> {
            FerroProperty::register::<MaterialRipple, _>("ReducedMotion", false)
        }
    );
} }

ferroui_base::ferro_overrides! { impl FerroObjectImpl for MaterialRipple {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "ReducedMotion" if !animations_enabled(this.reduced_motion()) => this.apply_reduced_motion(),
            "IsEffectivelyEnabled" if !this.is_effectively_enabled() => this.cancel(),
            "IsVisible" if !this.is_visible() => this.cancel(),
            "TemplatedParent" if this.is_attached_to_visual_tree() => this.connect_host(),
            _ => {}
        }
    }
} }

impl VisualImpl for MaterialRipple {
    fn render(this: &Self, context: &mut DrawingContext) {
        let Some(fill) = this.fill() else { return };
        let bounds = Rect::from_size(this.bounds().size());
        if bounds.width <= 0.0
            || bounds.height <= 0.0
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
        {
            return;
        }
        let waves = this.waves.borrow();
        if waves.is_empty() {
            return;
        }
        let clip = if this.is_bounded() {
            context.push_rounded_clip(RoundedRect::from_corner_radius(
                bounds,
                this.corner_radius(),
            ))
        } else {
            ferroui_base::media::PushedState::NONE
        };
        for wave in waves.iter() {
            let frame = wave.sample(bounds.size(), this.is_bounded(), this.radius());
            if frame.opacity <= 0.0 || frame.radius <= 0.0 {
                continue;
            }
            let opacity = context.push_opacity(frame.opacity * this.interaction_opacity());
            context.draw_ellipse_at(Some(&fill), None, frame.center, frame.radius, frame.radius);
            context.pop(opacity);
        }
        context.pop(clip);
    }

    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        let weak = this.to_ref().downgrade();
        *this.visibility_subscription.borrow_mut() =
            Some(this.is_effectively_visible_changed(move || {
                if let Some(this) = weak.upgrade() {
                    if !this.is_effectively_visible() {
                        this.cancel();
                    }
                }
            }));
        this.connect_host();
    }

    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        this.disconnect_host();
        this.cancel();
        Self::parent_on_detached_from_visual_tree(this, event);
    }
}

impl MaterialRipple {
    fn static_constructor() {
        InputElement::is_hit_test_visible_property()
            .override_default_value::<MaterialRipple>(false);
        InputElement::focusable_property().override_default_value::<MaterialRipple>(false);
        Visual::affects_render::<MaterialRipple>(&[
            Self::fill_property().as_property(),
            Self::corner_radius_property().as_property(),
            Self::is_bounded_property().as_property(),
            Self::radius_property().as_property(),
            Self::interaction_opacity_property().as_property(),
        ]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            host: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            waves: RefCell::new(Vec::new()),
            frame_pending: Cell::new(false),
            frame_generation: Cell::new(0),
            last_frame: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn fill(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::fill_property())
    }

    pub fn set_fill(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::fill_property(), value);
    }

    pub fn corner_radius(&self) -> CornerRadius {
        self.get_value(Self::corner_radius_property())
    }

    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.set_value(Self::corner_radius_property(), value);
    }

    pub fn is_bounded(&self) -> bool {
        self.get_value(Self::is_bounded_property())
    }

    pub fn set_is_bounded(&self, value: bool) {
        self.set_value(Self::is_bounded_property(), value);
    }

    pub fn radius(&self) -> f64 {
        self.get_value(Self::radius_property())
    }

    pub fn set_radius(&self, value: f64) {
        self.set_value(Self::radius_property(), value);
    }

    pub fn interaction_opacity(&self) -> f64 {
        self.get_value(Self::interaction_opacity_property())
    }

    pub fn set_interaction_opacity(&self, value: f64) {
        self.set_value(Self::interaction_opacity_property(), value);
    }

    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }

    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }

    /// Starts a programmatic indication. The location is in this control's coordinate space.
    /// Template instances receive their input automatically and do not need this method.
    pub fn begin_press(&self, origin: Point) {
        self.press(PressId::Programmatic, origin);
    }

    /// Finishes the corresponding programmatic indication.
    pub fn end_press(&self) {
        self.release(PressId::Programmatic);
    }

    /// The number of held or fading press indications, useful to inspect control behavior.
    pub fn active_ripple_count(&self) -> usize {
        self.waves.borrow().len()
    }

    /// Clears every indication, for example when a control becomes disabled.
    pub fn cancel(&self) {
        self.waves.borrow_mut().clear();
        self.invalidate_pending_frame();
        self.invalidate_visual();
    }

    fn press(&self, id: PressId, origin: Point) {
        let host_enabled = self
            .host
            .borrow()
            .as_ref()
            .and_then(|subscriptions| subscriptions.host.upgrade())
            .is_none_or(|host| host.is_effectively_enabled());
        if !self.is_effectively_enabled()
            || !host_enabled
            || !self.is_effectively_visible()
            || !origin.x.is_finite()
            || !origin.y.is_finite()
        {
            return;
        }
        let mut waves = self.waves.borrow_mut();
        // Repeated keyboard downs and repeated reports of a captured pointer are one press.
        if waves
            .iter()
            .any(|wave| wave.id == id && wave.released_at.is_none())
        {
            return;
        }
        if waves.len() == MAX_SIMULTANEOUS_RIPPLES {
            let index = waves
                .iter()
                .position(|wave| wave.released_at.is_some())
                .unwrap_or(0);
            waves.remove(index);
        }
        let mut wave = RippleWave::new(id, origin);
        if !animations_enabled(self.reduced_motion()) {
            wave.elapsed = RADIUS_SECONDS;
        }
        waves.push(wave);
        drop(waves);
        self.invalidate_visual();
        self.request_frame();
    }

    fn release(&self, id: PressId) {
        let reduced = !animations_enabled(self.reduced_motion());
        let mut waves = self.waves.borrow_mut();
        if reduced {
            waves.retain(|wave| wave.id != id);
        } else {
            for wave in waves
                .iter_mut()
                .filter(|wave| wave.id == id && wave.released_at.is_none())
            {
                wave.released_at = Some(wave.elapsed);
            }
        }
        drop(waves);
        self.invalidate_visual();
        self.request_frame();
    }

    fn release_keyboard(&self) {
        self.release(PressId::Keyboard(Key::Space));
        self.release(PressId::Keyboard(Key::Enter));
    }

    fn apply_reduced_motion(&self) {
        let mut waves = self.waves.borrow_mut();
        waves.retain(|wave| wave.released_at.is_none());
        for wave in waves.iter_mut() {
            wave.elapsed = RADIUS_SECONDS;
        }
        drop(waves);
        self.invalidate_pending_frame();
        self.invalidate_visual();
    }

    fn advance(&self, seconds: f64) {
        if !animations_enabled(self.reduced_motion()) {
            self.apply_reduced_motion();
            return;
        }
        if seconds.is_nan() || seconds < 0.0 {
            return;
        }
        let mut waves = self.waves.borrow_mut();
        for wave in waves.iter_mut() {
            wave.elapsed += seconds;
        }
        waves.retain(|wave| !wave.finished());
        drop(waves);
        self.invalidate_visual();
    }

    fn invalidate_pending_frame(&self) {
        self.frame_generation
            .set(self.frame_generation.get().wrapping_add(1));
        self.frame_pending.set(false);
        self.last_frame.set(None);
    }

    fn request_frame(&self) {
        if self.frame_pending.get()
            || !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
        {
            return;
        }
        if !animations_enabled(self.reduced_motion())
            || !self.waves.borrow().iter().any(RippleWave::animating)
        {
            self.last_frame.set(None);
            return;
        }
        self.frame_pending.set(true);
        let generation = self.frame_generation.get();
        let weak = self.to_ref().downgrade();
        MediaContext::instance().request_animation_frame(move |now| {
            let Some(this) = weak.upgrade() else { return };
            if generation != this.frame_generation.get() {
                return;
            }
            this.frame_pending.set(false);
            if !this.is_attached_to_visual_tree() {
                return;
            }
            let seconds = this
                .last_frame
                .replace(Some(now))
                .map_or(0.0, |last| (now - last).total_seconds().max(0.0));
            this.advance(seconds);
            this.request_frame();
        });
    }

    fn connect_host(&self) {
        if self.host.borrow().is_some() {
            self.cancel();
        }
        self.disconnect_host();
        let host = self
            .templated_parent()
            .and_then(|parent| parent.cast::<InputElement>())
            .or_else(|| {
                self.visual_parent()
                    .and_then(|parent| parent.cast::<InputElement>())
            });
        if let Some(host) = host {
            self.attach_host(&host);
        }
    }

    fn disconnect_host(&self) {
        let old = self.host.borrow_mut().take();
        drop(old);
    }

    fn attach_host(&self, host: &Ref<InputElement>) {
        let weak = self.to_ref().downgrade();
        let pressed = host.add_handler_with(
            InputElement::pointer_pressed_event(),
            {
                let weak = weak.clone();
                move |sender, event: &PointerPressedEventArgs| {
                    let Some(this) = weak.upgrade() else { return };
                    // A nested action owns its own indication. Observe handled events from
                    // this host's template, without lighting an outer row when its trailing
                    // button or an embedded editor is the actual interaction target.
                    if let Some(source) = event.source().and_then(|source| source.cast::<Visual>())
                    {
                        let host = sender.to_ref().upcast::<Visual>();
                        let nested_action = source
                            .get_self_and_visual_ancestors()
                            .take_while(|node| *node != host)
                            .any(|node| {
                                node.cast::<InputElement>().is_some_and(|input| {
                                    input.is_effectively_enabled()
                                        && (input.focusable() || node.cast::<Button>().is_some())
                                })
                            });
                        if nested_action {
                            return;
                        }
                    }
                    if event.pointer().type_() != PointerType::Mouse
                        || event.properties().is_left_button_pressed
                    {
                        this.press(
                            PressId::Pointer(event.pointer().id()),
                            event.get_position(Some(&this)),
                        );
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let released = host.add_handler_with(
            InputElement::pointer_released_event(),
            {
                let weak = weak.clone();
                move |_, event: &PointerReleasedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.release(PressId::Pointer(event.pointer().id()));
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let capture_lost = host.add_handler_with(
            InputElement::pointer_capture_lost_event(),
            {
                let weak = weak.clone();
                move |_, event: &PointerCaptureLostEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.release(PressId::Pointer(event.pointer().id()));
                    }
                }
            },
            RoutingStrategies::DIRECT.union(RoutingStrategies::BUBBLE),
            true,
        );
        let exited = host.add_handler_with(
            InputElement::pointer_exited_event(),
            {
                let weak = weak.clone();
                move |_, event: &PointerEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.release(PressId::Pointer(event.pointer().id()));
                    }
                }
            },
            RoutingStrategies::DIRECT,
            true,
        );
        let key_down = host.add_handler_with(
            InputElement::key_down_event(),
            {
                let weak = weak.clone();
                move |sender, event: &KeyEventArgs| {
                    if matches!(event.key, Key::Space | Key::Enter)
                        && event
                            .source()
                            .is_some_and(|source| source == sender.to_ref().upcast::<FerroObject>())
                    {
                        if let Some(this) = weak.upgrade() {
                            let size = this.bounds().size();
                            this.press(
                                PressId::Keyboard(event.key),
                                Point::new(size.width * 0.5, size.height * 0.5),
                            );
                        }
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let key_up = host.add_handler_with(
            InputElement::key_up_event(),
            {
                let weak = weak.clone();
                move |_, event: &KeyEventArgs| {
                    if matches!(event.key, Key::Space | Key::Enter) {
                        if let Some(this) = weak.upgrade() {
                            this.release(PressId::Keyboard(event.key));
                        }
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let lost_focus = host.add_handler_with(
            InputElement::lost_focus_event(),
            {
                let weak = weak.clone();
                move |_, _: &FocusChangedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.release_keyboard();
                    }
                }
            },
            RoutingStrategies::DIRECT.union(RoutingStrategies::BUBBLE),
            true,
        );
        let enabled = host
            .clone()
            .upcast::<FerroObject>()
            .get_observable(InputElement::is_effectively_enabled_property())
            .subscribe_fn(move |enabled| {
                if !enabled {
                    if let Some(this) = weak.upgrade() {
                        this.cancel();
                    }
                }
            });
        *self.host.borrow_mut() = Some(HostSubscriptions {
            host: host.downgrade(),
            pressed,
            released,
            capture_lost,
            exited,
            key_down,
            key_up,
            lost_focus,
            enabled,
        });
    }
}

impl Drop for MaterialRipple {
    fn drop(&mut self) {
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

struct HostSubscriptions {
    host: WeakRef<InputElement>,
    pressed: RoutedEventHandlerToken,
    released: RoutedEventHandlerToken,
    capture_lost: RoutedEventHandlerToken,
    exited: RoutedEventHandlerToken,
    key_down: RoutedEventHandlerToken,
    key_up: RoutedEventHandlerToken,
    lost_focus: RoutedEventHandlerToken,
    enabled: Rc<dyn IDisposable>,
}

impl Drop for HostSubscriptions {
    fn drop(&mut self) {
        self.enabled.dispose();
        if let Some(host) = self.host.upgrade() {
            host.remove_handler(InputElement::pointer_pressed_event(), self.pressed);
            host.remove_handler(InputElement::pointer_released_event(), self.released);
            host.remove_handler(
                InputElement::pointer_capture_lost_event(),
                self.capture_lost,
            );
            host.remove_handler(InputElement::pointer_exited_event(), self.exited);
            host.remove_handler(InputElement::key_down_event(), self.key_down);
            host.remove_handler(InputElement::key_up_event(), self.key_up);
            host.remove_handler(InputElement::lost_focus_event(), self.lost_focus);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PressId {
    Pointer(i32),
    Keyboard(Key),
    Programmatic,
}

#[derive(Clone, Copy, Debug)]
struct RippleWave {
    id: PressId,
    origin: Point,
    elapsed: f64,
    released_at: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RippleFrame {
    center: Point,
    radius: f64,
    opacity: f64,
}

impl RippleWave {
    fn new(id: PressId, origin: Point) -> Self {
        Self {
            id,
            origin,
            elapsed: 0.0,
            released_at: None,
        }
    }

    fn sample(&self, size: Size, bounded: bool, explicit_radius: f64) -> RippleFrame {
        let progress = (self.elapsed / RADIUS_SECONDS).clamp(0.0, 1.0);
        let mut easing = KeySplineSolver::new(0.4, 0.0, 0.2, 1.0);
        let radial_progress = easing.get_spline_progress(progress);
        let target = Point::new(size.width * 0.5, size.height * 0.5);
        let origin = if bounded {
            Point::new(
                self.origin.x.clamp(0.0, size.width),
                self.origin.y.clamp(0.0, size.height),
            )
        } else {
            target
        };
        let end_radius = if explicit_radius.is_finite() {
            explicit_radius.max(0.0)
        } else {
            size.width.hypot(size.height) * 0.5 + if bounded { 10.0 } else { 0.0 }
        };
        let start_radius = (size.width.max(size.height) * 0.3).min(end_radius);
        let opacity = match self.released_at {
            Some(released) if self.elapsed >= RADIUS_SECONDS => (1.0
                - (self.elapsed - released.max(RADIUS_SECONDS)) / FADE_OUT_SECONDS)
                .clamp(0.0, 1.0),
            // Upstream immediately raises alpha on a quick tap so the tap remains visible.
            Some(_) => 1.0,
            None => (self.elapsed / FADE_IN_SECONDS).clamp(0.0, 1.0),
        };
        RippleFrame {
            center: Point::new(
                origin.x + (target.x - origin.x) * progress,
                origin.y + (target.y - origin.y) * progress,
            ),
            radius: start_radius + (end_radius - start_radius) * radial_progress,
            opacity,
        }
    }

    fn finished(&self) -> bool {
        self.released_at
            .is_some_and(|released| self.elapsed >= released.max(RADIUS_SECONDS) + FADE_OUT_SECONDS)
    }

    fn animating(&self) -> bool {
        !self.finished() && (self.elapsed < RADIUS_SECONDS || self.released_at.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::{
        KeyModifiers, Pointer, PointerPointProperties, PointerUpdateKind, RawInputModifiers,
    };

    #[test]
    fn pointer_ripple_uses_source_geometry_and_center_migration() {
        let mut wave = RippleWave::new(PressId::Pointer(1), Point::new(8.0, 4.0));
        let size = Size::new(100.0, 40.0);
        let first = wave.sample(size, true, f64::NAN);
        assert_eq!(Point::new(8.0, 4.0), first.center);
        assert_eq!(30.0, first.radius);
        assert_eq!(0.0, first.opacity);
        wave.elapsed = 0.1125;
        let middle = wave.sample(size, true, f64::NAN);
        assert_eq!(Point::new(29.0, 12.0), middle.center);
        assert!(middle.radius > 30.0 && middle.radius < 64.0);
        assert_eq!(1.0, middle.opacity);
        wave.elapsed = RADIUS_SECONDS;
        let final_frame = wave.sample(size, true, f64::NAN);
        assert_eq!(Point::new(50.0, 20.0), final_frame.center);
        assert!((final_frame.radius - (100.0_f64.hypot(40.0) / 2.0 + 10.0)).abs() < 1e-9);
        assert!(
            !wave.animating(),
            "holding a settled press must not keep requesting frames"
        );
    }

    #[test]
    fn a_quick_tap_expands_before_fading_out() {
        let mut wave = RippleWave::new(PressId::Pointer(1), Point::default());
        wave.elapsed = 0.01;
        wave.released_at = Some(0.01);
        assert_eq!(
            1.0,
            wave.sample(Size::new(80.0, 40.0), true, f64::NAN).opacity
        );
        wave.elapsed = RADIUS_SECONDS + FADE_OUT_SECONDS / 2.0;
        assert!((wave.sample(Size::new(80.0, 40.0), true, f64::NAN).opacity - 0.5).abs() < 1e-12);
        wave.elapsed = RADIUS_SECONDS + FADE_OUT_SECONDS;
        assert!(wave.finished());
    }

    #[test]
    fn unbounded_selection_ripple_stays_centered_at_its_explicit_radius() {
        let mut wave = RippleWave::new(PressId::Pointer(1), Point::new(1.0, 2.0));
        wave.elapsed = RADIUS_SECONDS;
        let frame = wave.sample(Size::new(40.0, 40.0), false, 20.0);
        assert_eq!(Point::new(20.0, 20.0), frame.center);
        assert_eq!(20.0, frame.radius);
    }

    #[test]
    fn independent_contacts_release_without_canceling_other_contacts() {
        let ripple = MaterialRipple::new();
        ripple.press(PressId::Pointer(1), Point::default());
        ripple.press(PressId::Pointer(2), Point::default());
        ripple.press(PressId::Pointer(2), Point::default());
        assert_eq!(2, ripple.active_ripple_count());
        ripple.advance(0.1);
        ripple.release(PressId::Pointer(1));
        ripple.advance(0.3);
        assert_eq!(1, ripple.active_ripple_count());
        assert_eq!(PressId::Pointer(2), ripple.waves.borrow()[0].id);
        ripple.cancel();
        assert_eq!(0, ripple.active_ripple_count());
    }

    #[test]
    fn reduced_motion_preserves_press_feedback_without_running_animation() {
        let ripple = MaterialRipple::new();
        ripple.set_reduced_motion(true);
        ripple.begin_press(Point::new(1.0, 2.0));
        assert_eq!(1, ripple.active_ripple_count());
        assert_eq!(RADIUS_SECONDS, ripple.waves.borrow()[0].elapsed);
        assert!(!ripple.waves.borrow()[0].animating());
        ripple.end_press();
        assert_eq!(0, ripple.active_ripple_count());
        assert!(!ripple.is_hit_test_visible());
        assert!(!ripple.focusable());
    }

    #[test]
    fn platform_reduced_motion_finishes_active_expansion_and_removes_released_waves() {
        use crate::motion::MotionTestSettings;
        use ferroui_controls::testing::{TestServices, UnitTestApplication};
        let settings = MotionTestSettings::new();
        let _application = UnitTestApplication::start(
            TestServices::default().with_platform_settings(settings.clone()),
        );
        let ripple = MaterialRipple::new();
        ripple.press(PressId::Pointer(1), Point::default());
        ripple.press(PressId::Pointer(2), Point::default());
        ripple.advance(0.05);
        ripple.release(PressId::Pointer(1));
        assert_eq!(2, ripple.active_ripple_count());
        settings.animations_enabled.set(false);
        ripple.advance(0.01);
        assert_eq!(1, ripple.active_ripple_count());
        assert_eq!(RADIUS_SECONDS, ripple.waves.borrow()[0].elapsed);
        assert!(!ripple.waves.borrow()[0].animating());
        ripple.release(PressId::Pointer(2));
        assert_eq!(0, ripple.active_ripple_count());
    }

    #[test]
    fn handled_parent_input_drives_ripple_and_disconnect_removes_handlers() {
        let host = Control::new();
        let ripple = MaterialRipple::new();
        ripple.attach_host(&host.clone().upcast());
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let press = || {
            let event = PointerPressedEventArgs::new(
                host.clone(),
                pointer.clone(),
                &host,
                Point::default(),
                0,
                PointerPointProperties::new(
                    RawInputModifiers::LEFT_MOUSE_BUTTON,
                    PointerUpdateKind::LeftButtonPressed,
                ),
                KeyModifiers::NONE,
                1,
            );
            event.set_handled(true);
            host.raise_event(&event);
        };
        press();
        assert_eq!(1, ripple.active_ripple_count());
        ripple.disconnect_host();
        ripple.cancel();
        press();
        assert_eq!(0, ripple.active_ripple_count());
    }

    #[test]
    fn a_nested_button_owns_its_press_indication_without_lighting_the_outer_row() {
        use ferroui_controls::Panel;
        let host = Panel::new();
        let child = Button::new();
        // Some toolbar actions intentionally avoid keyboard focus; they are still actions.
        child.set_focusable(false);
        host.children().add(child.clone());
        let outer = MaterialRipple::new();
        let inner = MaterialRipple::new();
        outer.attach_host(&host.clone().upcast());
        inner.attach_host(&child.clone().upcast());
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let event = PointerPressedEventArgs::new(
            child.clone(),
            pointer,
            &host,
            Point::default(),
            0,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        );
        child.raise_event(&event);
        assert!(child.is_pressed());
        assert_eq!(1, inner.active_ripple_count());
        assert_eq!(0, outer.active_ripple_count());
    }

    #[test]
    fn disabling_host_clears_a_held_press() {
        let host = Control::new();
        let ripple = MaterialRipple::new();
        ripple.attach_host(&host.clone().upcast());
        ripple.begin_press(Point::default());
        assert_eq!(1, ripple.active_ripple_count());
        host.set_is_enabled(false);
        assert_eq!(0, ripple.active_ripple_count());
        ripple.begin_press(Point::default());
        assert_eq!(0, ripple.active_ripple_count());
        host.set_is_enabled(true);
        ripple.begin_press(Point::default());
        assert_eq!(1, ripple.active_ripple_count());
    }

    #[test]
    fn input_subscriptions_do_not_retain_the_ripple_or_its_host() {
        let host = Control::new();
        let ripple = MaterialRipple::new();
        let weak_ripple = ripple.downgrade();
        let weak_host = host.downgrade();
        ripple.attach_host(&host.clone().upcast());
        drop(ripple);
        assert!(weak_ripple.upgrade().is_none());
        drop(host);
        assert!(weak_host.upgrade().is_none());
    }
}
