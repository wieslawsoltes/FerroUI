//! Ported from `Composition/RotateAnimation.cs`.

use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::Visual;
use ferroui_controls::Control;

/// Provides rotation animation helpers using composition animations.
pub struct RotateAnimation;

impl RotateAnimation {
    /// Applies a clockwise rotation animation to the element when loaded (0 to 360 degrees),
    /// over `milliseconds`.
    pub fn set_rotate_clockwise(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, 0.0, 360.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a counter-clockwise rotation animation to the element when loaded (0 to -360
    /// degrees), over `milliseconds`.
    pub fn set_rotate_counter_clockwise(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, 0.0, -360.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a rotate-in animation to the element when loaded (from -180 to 0 degrees), over
    /// `milliseconds`.
    pub fn set_rotate_in(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, -180.0, 0.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a rotate-out animation to the element when loaded (from 0 to 180 degrees), over
    /// `milliseconds`.
    pub fn set_rotate_out(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, 0.0, 180.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a flip animation to the element when loaded (180 degree rotation), over
    /// `milliseconds`.
    pub fn set_flip(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, 0.0, 180.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a swing animation to the element when loaded (back and forth rotation), over
    /// `milliseconds`.
    pub fn set_swing(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply_swing(element, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a custom rotation animation to the element when loaded: from `from_angle` to
    /// `to_angle` degrees, over `milliseconds`.
    pub fn set_custom_rotate(element: &Control, from_angle: f64, to_angle: f64, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, from_angle as f32, to_angle as f32, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a rotation animation to the visual, from `from_angle` to `to_angle` degrees.
    fn apply(visual: &Visual, from_angle: f32, to_angle: f32, duration: TimeSpan) {
        let Some(composition_visual) = ElementComposition::get_element_visual(visual) else {
            return;
        };

        let from_radians = CompositionAnimationHelpers::degrees_to_radians(from_angle);
        let to_radians = CompositionAnimationHelpers::degrees_to_radians(to_angle);
        CompositionAnimationHelpers::start_scalar_animation(
            &composition_visual,
            "RotationAngle",
            duration,
            &[ScalarKeyFrame::new(0.0, from_radians), ScalarKeyFrame::new(1.0, to_radians)],
        );
    }

    /// Applies a swing animation to the visual.
    fn apply_swing(visual: &Visual, duration: TimeSpan) {
        let Some(composition_visual) = ElementComposition::get_element_visual(visual) else {
            return;
        };

        CompositionAnimationHelpers::start_scalar_animation(
            &composition_visual,
            "RotationAngle",
            duration,
            &[
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.25, CompositionAnimationHelpers::degrees_to_radians(15.0)),
                ScalarKeyFrame::new(0.5, 0.0),
                ScalarKeyFrame::new(0.75, CompositionAnimationHelpers::degrees_to_radians(-15.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ],
        );
    }
}
