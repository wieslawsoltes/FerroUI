//! Interactive effects (#455, E7; not from upstream; docs/porting/custom-effects.md, section
//! 3.5): the input trait a mutable custom effect class implements to receive the pointer input of
//! the element it is the effect of, and the context of that element it receives it with.
//!
//! An interactive effect class is constructed with
//! [`Effect::construct_interactive`](super::Effect::construct_interactive). When it is set as the
//! `Effect` of an [`InputElement`] (the host), the host routes its pointer events to the effect
//! (`input/input_element_effects.rs`): one class handler of the changes of `Visual.Effect`
//! attaches the effect to the host when it is interactive and detaches it when the effect is
//! replaced or cleared, and the attachment holds the guards of the handlers it adds to the host's
//! pointer events and of its subscription to the host's bounds. The effect changes its own state
//! and raises [`invalidated`](super::Effect::invalidated), so that the visual takes a new snapshot:
//! what crosses to the render thread is still a value (a pointer position is a `Point` or two
//! `Float` parameters, a press a `Bool`), and the program of the effect, compiled once, is given
//! the new values as uniforms.
//!
//! The members follow Effector's `ISkiaInputEffectHandler`, and [`EffectHostContext`] its
//! `SkiaEffectHostContext`.

use super::Effect;
use crate::animation::SignedDuration;
use crate::input::{
    InputElement, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerWheelEventArgs,
};
use crate::media::MediaContext;
use crate::{Point, Rect, Size, Thickness, Upcast, Visual};

/// The pointer input of an interactive effect (Effector's `ISkiaInputEffectHandler`): implemented by
/// an effect class constructed with
/// [`Effect::construct_interactive`](super::Effect::construct_interactive).
///
/// Every member has an empty default, as the members of Effector's `SkiaInteractiveEffectBase`
/// have, so an effect implements what it uses.
///
/// # Routing
///
/// The host routes its pointer events to the effect from handlers it adds to the direct and the
/// bubble routes of each event (`Interactive::DEFAULT_ROUTES`), so that the effect sees what happens
/// over the host and over its descendants:
///
/// - after the handlers of the tunnel route, the class handlers of the host (its own `on_pointer_*`
///   members) and the handlers the descendants of the host have on the bubble route, and before the
///   ancestors of the host see the event on the bubble route; among the handlers of the host itself,
///   in the order they were added;
/// - an event that is already handled is not delivered, unless the effect asks for handled events
///   ([`handled_events_too`](Self::handled_events_too)): the effect of a button, whose own handler
///   handles the press, asks for them;
/// - a pointer the host or the effect captured is routed to the host wherever it is, so the effect
///   sees its moves and its release outside the bounds of the host; when the capture of the host or
///   of one of its descendants is lost (the release of an implicit capture, or another element
///   capturing the pointer), the effect receives
///   [`on_pointer_capture_lost`](Self::on_pointer_capture_lost).
///
/// The handlers of the effect may change the effect of the host: the effect then is detached
/// ([`on_detached`](Self::on_detached)) and receives nothing more from it.
pub trait IEffectInputHandler {
    /// Whether the effect receives events that are already handled (default `false`).
    fn handled_events_too(&self) -> bool {
        false
    }

    /// The effect was attached to the host: it became the host's effect.
    fn on_attached(&self, _context: &EffectHostContext<'_>) {}

    /// The effect was detached from the host: the host's effect was replaced or cleared.
    fn on_detached(&self, _context: &EffectHostContext<'_>) {}

    /// The bounds, the render transform or the origin of the render transform of the host
    /// changed; also called once after [`on_attached`](Self::on_attached).
    fn on_host_bounds_changed(&self, _context: &EffectHostContext<'_>) {}

    /// The pointer entered the host.
    fn on_pointer_entered(&self, _context: &EffectHostContext<'_>, _e: &PointerEventArgs) {}

    /// The pointer left the host.
    fn on_pointer_exited(&self, _context: &EffectHostContext<'_>, _e: &PointerEventArgs) {}

    /// The pointer moved over the host, or anywhere while the host captures it.
    fn on_pointer_moved(&self, _context: &EffectHostContext<'_>, _e: &PointerEventArgs) {}

    /// The pointer was pressed over the host.
    fn on_pointer_pressed(&self, _context: &EffectHostContext<'_>, _e: &PointerPressedEventArgs) {}

    /// The pointer was released over the host, or anywhere while the host captures it.
    fn on_pointer_released(&self, _context: &EffectHostContext<'_>, _e: &PointerReleasedEventArgs) {}

    /// The host or one of its descendants lost the capture of the pointer.
    fn on_pointer_capture_lost(&self, _context: &EffectHostContext<'_>, _e: &PointerCaptureLostEventArgs) {}

    /// The wheel was turned over the host.
    fn on_pointer_wheel_changed(&self, _context: &EffectHostContext<'_>, _e: &PointerWheelEventArgs) {}
}

/// The element an interactive effect is the effect of, as the effect's input handler sees it
/// (Effector's `SkiaEffectHostContext`): its bounds, the pointer position in its coordinates,
/// normalized to its bounds and in the coordinates of the effect's program, the capture of the
/// pointer, and the invalidation of the effect.
///
/// Three coordinate spaces meet here:
///
/// - the host's own coordinates ([`get_position`](Self::get_position)), device independent pixels
///   from the top left corner of the host, through every render transform up to the root;
/// - the normalized coordinates ([`get_normalized_position`](Self::get_normalized_position)), 0 to
///   1 across the [input bounds](Self::input_bounds), the host's own size, which is what the
///   interactive effects of the library pass as uniforms: independent of the scale, they need no
///   conversion on the render thread;
/// - the coordinates of the program ([`get_effect_position`](Self::get_effect_position)): device
///   pixels from the top left corner of the area of the effect, the host's bounds inflated by the
///   padding of the effect, at [`layer_scale`](Self::layer_scale) (section 3.4 of the plan, the
///   `coord` a program is given, when the content of the host lies within its bounds).
pub struct EffectHostContext<'a> {
    effect: &'a Effect,
    host: &'a InputElement,
}

impl<'a> EffectHostContext<'a> {
    pub(crate) fn new(effect: &'a Effect, host: &'a InputElement) -> Self {
        Self { effect, host }
    }

    /// The effect.
    pub fn effect(&self) -> &'a Effect {
        self.effect
    }

    /// The host: the input element the effect is the effect of.
    pub fn host(&self) -> &'a InputElement {
        self.host
    }

    /// The host as a visual.
    pub fn visual(&self) -> &'a Visual {
        self.host.upcast()
    }

    /// The bounds of the host, in the coordinates of its parent.
    pub fn bounds(&self) -> Rect {
        self.visual().bounds()
    }

    /// The size of the host.
    pub fn size(&self) -> Size {
        self.bounds().size()
    }

    /// The bounds the pointer is normalized to: the host's own size, from its top left corner.
    pub fn input_bounds(&self) -> Rect {
        Rect::from_position_size(Point::default(), self.size())
    }

    /// The input size: the size of the [input bounds](Self::input_bounds).
    pub fn input_size(&self) -> Size {
        self.size()
    }

    /// The padding of the effect: how far its area reaches beyond the host's bounds.
    pub fn padding(&self) -> Thickness {
        self.effect.custom_snapshot().map(|snapshot| snapshot.padding()).unwrap_or_default()
    }

    /// The area of the effect in the host's coordinates: the input bounds inflated by the padding
    /// of the effect.
    pub fn effect_bounds(&self) -> Rect {
        self.input_bounds().inflate_thickness(self.padding())
    }

    /// Device pixels per device independent pixel of the host, as the render thread computes the
    /// `layer_scale` of the effect: the render scaling of the host's presentation source times
    /// the mean of the lengths of the axes of the transform from the host to the root. 1 for a
    /// host that is not in a visual tree.
    pub fn layer_scale(&self) -> f64 {
        let visual = self.visual();
        let scaling = visual.presentation_source().map_or(1.0, |source| source.render_scaling());
        let transform = visual.visual_root().and_then(|root| visual.transform_to_visual(&root));
        let axes = transform.map_or(1.0, |m| ((m.m11 * m.m11 + m.m12 * m.m12).sqrt() + (m.m21 * m.m21 + m.m22 * m.m22).sqrt()) / 2.0);
        let scale = scaling * axes;
        if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        }
    }

    /// The position of the pointer of `e` in the host's coordinates.
    pub fn get_position(&self, e: &PointerEventArgs) -> Point {
        e.get_position(Some(self.visual()))
    }

    /// The position of the pointer of `e` normalized to the input bounds: 0 to 1 from the left
    /// to the right and from the top to the bottom of the host, clamped to 0..1 when `clamp` is
    /// set (Effector's `GetNormalizedPosition`).
    pub fn get_normalized_position(&self, e: &PointerEventArgs, clamp: bool) -> Point {
        self.normalize(self.get_position(e), clamp)
    }

    /// `position`, in the host's coordinates, normalized to the input bounds; an empty side
    /// counts as one pixel.
    pub fn normalize(&self, position: Point, clamp: bool) -> Point {
        let bounds = self.input_bounds();
        let width = if bounds.width > 0.0 { bounds.width } else { 1.0 };
        let height = if bounds.height > 0.0 { bounds.height } else { 1.0 };
        let x = (position.x - bounds.x) / width;
        let y = (position.y - bounds.y) / height;
        if clamp {
            Point::new(clamp_unit(x), clamp_unit(y))
        } else {
            Point::new(x, y)
        }
    }

    /// The position of the pointer of `e` in the coordinates of the effect's program: device
    /// pixels from the top left corner of the [area of the effect](Self::effect_bounds).
    pub fn get_effect_position(&self, e: &PointerEventArgs) -> Point {
        self.to_effect_space(self.get_position(e))
    }

    /// `position`, in the host's coordinates, in the coordinates of the effect's program.
    pub fn to_effect_space(&self, position: Point) -> Point {
        let area = self.effect_bounds();
        let scale = self.layer_scale();
        Point::new((position.x - area.x) * scale, (position.y - area.y) * scale)
    }

    /// Captures the pointer of `e` to the host, so that the host and the effect receive its
    /// input wherever it is until it is released.
    pub fn capture_pointer(&self, e: &PointerEventArgs) {
        e.pointer().capture(Some(&self.host.to_ref()));
    }

    /// Releases the capture of the pointer of `e` when the host has it.
    pub fn release_pointer_capture(&self, e: &PointerEventArgs) {
        if self.has_pointer_capture(e) {
            e.pointer().capture(None);
        }
    }

    /// Whether the host captures the pointer of `e`.
    pub fn has_pointer_capture(&self, e: &PointerEventArgs) -> bool {
        e.pointer().captured().is_some_and(|captured| std::ptr::eq(&*captured, self.host))
    }

    /// Asks for a new snapshot of the effect: raises its
    /// [`invalidated`](super::Effect::invalidated) notification (Effector's `Invalidate`).
    pub fn invalidate(&self) {
        self.effect.raise_invalidated();
    }

    /// Runs `action` on the next frame of the animation clock with the frame time, while the
    /// effect is still the effect of the host: an effect that animates its uniforms (a ripple
    /// that spreads) asks for one frame at a time. Nothing runs when the effect was detached
    /// before the frame, or the host or the effect is gone.
    pub fn request_animation_frame(&self, action: impl FnOnce(&EffectHostContext<'_>, SignedDuration) + 'static) {
        let host = self.host.to_ref().downgrade();
        let effect = self.effect.to_ref().downgrade();
        MediaContext::instance().request_animation_frame(move |time| {
            let (Some(host), Some(effect)) = (host.upgrade(), effect.upgrade()) else { return };
            if !host.has_effect_input_attached(&effect) {
                return;
            }
            action(&EffectHostContext::new(&effect, &host), time);
        });
    }
}

/// `value` clamped to 0..1; not a number is 0.
fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}
