//! Ported from `Composition/AttentionAnimations.cs`.

use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_controls::Control;

/// Provides attention-drawing animation helpers inspired by popular web animation libraries such
/// as animate.css.
pub struct AttentionAnimations;

impl AttentionAnimations {
    /// Applies a bounce animation to the element when loaded.
    pub fn set_bounce(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_bounce, false, None);
    }

    /// Applies a flash animation to the element when loaded.
    pub fn set_flash(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_flash, false, None);
    }

    /// Applies a pulse animation to the element when loaded.
    pub fn set_pulse(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_pulse, true, None);
    }

    /// Applies a rubber band animation to the element when loaded.
    pub fn set_rubber_band(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_rubber_band, true, None);
    }

    /// Applies a horizontal shake animation to the element when loaded.
    pub fn set_shake_x(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, |ctrl, visual, duration| Self::apply_shake(ctrl, visual, duration, true), false, None);
    }

    /// Applies a vertical shake animation to the element when loaded.
    pub fn set_shake_y(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, |ctrl, visual, duration| Self::apply_shake(ctrl, visual, duration, false), false, None);
    }

    /// Applies a head shake animation to the element when loaded.
    pub fn set_head_shake(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_head_shake, true, None);
    }

    /// Applies a swing animation to the element when loaded.
    pub fn set_swing(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_swing, false, Some(Vector2::new(0.5, 0.0)));
    }

    /// Applies a tada animation (scale and rotation) to the element when loaded.
    pub fn set_tada(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_tada, true, None);
    }

    /// Applies a wobble animation to the element when loaded.
    pub fn set_wobble(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_wobble, true, None);
    }

    /// Applies a jello animation to the element when loaded.
    pub fn set_jello(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_jello, true, None);
    }

    /// Applies a heart beat animation to the element when loaded.
    pub fn set_heart_beat(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_heart_beat, true, None);
    }

    fn run(
        element: &Control,
        milliseconds: f64,
        animation: fn(&Control, &CompositionVisual, TimeSpan),
        ensure_center_point: bool,
        anchor: Option<Vector2>,
    ) {
        element.loaded(move |sender, _| {
            let Some(element) = sender.downcast_ref::<Control>() else {
                return;
            };
            let Some(visual) = ElementComposition::get_element_visual(element) else {
                return;
            };

            if let Some(anchor) = anchor {
                CompositionAnimationHelpers::set_normalized_center_point(element, &visual, anchor);
            } else if ensure_center_point {
                CompositionAnimationHelpers::ensure_center_point(element, &visual);
            }

            animation(element, &visual, TimeSpan::from_milliseconds(milliseconds));
        });
    }

    fn apply_bounce(element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_offset_animation(
            element,
            visual,
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::ZERO),
                Vector3KeyFrame::new(0.4, Vector3::new(0.0, -24.0, 0.0)),
                Vector3KeyFrame::new(0.43, Vector3::new(0.0, -24.0, 0.0)),
                Vector3KeyFrame::new(0.53, Vector3::ZERO),
                Vector3KeyFrame::new(0.7, Vector3::new(0.0, -12.0, 0.0)),
                Vector3KeyFrame::new(0.8, Vector3::ZERO),
                Vector3KeyFrame::new(0.9, Vector3::new(0.0, -4.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ],
        );
    }

    fn apply_flash(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_scalar_animation(
            visual,
            "Opacity",
            duration,
            &[
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.25, 0.0),
                ScalarKeyFrame::new(0.5, 1.0),
                ScalarKeyFrame::new(0.75, 0.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ],
        );
    }

    fn apply_pulse(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_vector3_animation(
            visual,
            "Scale",
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.5, Vector3::new(1.05, 1.05, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ],
        );
    }

    fn apply_rubber_band(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_vector3_animation(
            visual,
            "Scale",
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.3, Vector3::new(1.25, 0.75, 1.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(0.75, 1.25, 1.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(1.15, 0.85, 1.0)),
                Vector3KeyFrame::new(0.65, Vector3::new(0.95, 1.05, 1.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(1.05, 0.95, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ],
        );
    }

    fn apply_shake(element: &Control, visual: &CompositionVisual, duration: TimeSpan, horizontal: bool) {
        let shake = |amount: f32| {
            if horizontal {
                Vector3::new(amount, 0.0, 0.0)
            } else {
                Vector3::new(0.0, amount, 0.0)
            }
        };
        CompositionAnimationHelpers::start_offset_animation(
            element,
            visual,
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.1, shake(-8.0)),
                Vector3KeyFrame::new(0.2, shake(8.0)),
                Vector3KeyFrame::new(0.3, shake(-6.0)),
                Vector3KeyFrame::new(0.4, shake(6.0)),
                Vector3KeyFrame::new(0.5, shake(-4.0)),
                Vector3KeyFrame::new(0.6, shake(4.0)),
                Vector3KeyFrame::new(0.7, shake(-2.0)),
                Vector3KeyFrame::new(0.8, shake(2.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ],
        );
    }

    fn apply_head_shake(element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_offset_animation(
            element,
            visual,
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.065, Vector3::new(-6.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.185, Vector3::new(5.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.315, Vector3::new(-3.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.435, Vector3::new(2.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.5, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ],
        );

        CompositionAnimationHelpers::start_scalar_animation(
            visual,
            "RotationAngle",
            duration,
            &[
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.065, CompositionAnimationHelpers::degrees_to_radians(-6.0)),
                ScalarKeyFrame::new(0.185, CompositionAnimationHelpers::degrees_to_radians(4.0)),
                ScalarKeyFrame::new(0.315, CompositionAnimationHelpers::degrees_to_radians(-3.0)),
                ScalarKeyFrame::new(0.435, CompositionAnimationHelpers::degrees_to_radians(2.0)),
                ScalarKeyFrame::new(0.5, 0.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ],
        );
    }

    fn apply_swing(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_scalar_animation(
            visual,
            "RotationAngle",
            duration,
            &[
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.2, CompositionAnimationHelpers::degrees_to_radians(15.0)),
                ScalarKeyFrame::new(0.4, CompositionAnimationHelpers::degrees_to_radians(-10.0)),
                ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(5.0)),
                ScalarKeyFrame::new(0.8, CompositionAnimationHelpers::degrees_to_radians(-5.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ],
        );
    }

    fn apply_tada(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_vector3_animation(
            visual,
            "Scale",
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.1, Vector3::new(0.9, 0.9, 1.0)),
                Vector3KeyFrame::new(0.2, Vector3::new(0.9, 0.9, 1.0)),
                Vector3KeyFrame::new(0.3, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(0.95, 0.95, 1.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(1.05, 1.05, 1.0)),
                Vector3KeyFrame::new(0.7, Vector3::new(0.95, 0.95, 1.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(1.05, 1.05, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ],
        );

        CompositionAnimationHelpers::start_scalar_animation(
            visual,
            "RotationAngle",
            duration,
            &[
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.1, CompositionAnimationHelpers::degrees_to_radians(-12.0)),
                ScalarKeyFrame::new(0.2, CompositionAnimationHelpers::degrees_to_radians(12.0)),
                ScalarKeyFrame::new(0.3, CompositionAnimationHelpers::degrees_to_radians(-10.0)),
                ScalarKeyFrame::new(0.4, CompositionAnimationHelpers::degrees_to_radians(10.0)),
                ScalarKeyFrame::new(0.5, CompositionAnimationHelpers::degrees_to_radians(-6.0)),
                ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(6.0)),
                ScalarKeyFrame::new(0.7, CompositionAnimationHelpers::degrees_to_radians(-3.0)),
                ScalarKeyFrame::new(0.8, CompositionAnimationHelpers::degrees_to_radians(3.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ],
        );
    }

    fn apply_wobble(element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_offset_animation(
            element,
            visual,
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.15, Vector3::new(-20.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.3, Vector3::new(16.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.45, Vector3::new(-12.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(8.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(-4.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ],
        );

        CompositionAnimationHelpers::start_scalar_animation(
            visual,
            "RotationAngle",
            duration,
            &[
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.15, CompositionAnimationHelpers::degrees_to_radians(-5.0)),
                ScalarKeyFrame::new(0.3, CompositionAnimationHelpers::degrees_to_radians(3.0)),
                ScalarKeyFrame::new(0.45, CompositionAnimationHelpers::degrees_to_radians(-3.0)),
                ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(2.0)),
                ScalarKeyFrame::new(0.75, CompositionAnimationHelpers::degrees_to_radians(-1.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ],
        );
    }

    fn apply_jello(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_vector3_animation(
            visual,
            "Scale",
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.22, Vector3::new(1.02, 0.98, 1.0)),
                Vector3KeyFrame::new(0.33, Vector3::new(0.98, 1.02, 1.0)),
                Vector3KeyFrame::new(0.44, Vector3::new(1.015, 0.985, 1.0)),
                Vector3KeyFrame::new(0.55, Vector3::new(0.99, 1.01, 1.0)),
                Vector3KeyFrame::new(0.66, Vector3::new(1.01, 0.99, 1.0)),
                Vector3KeyFrame::new(0.77, Vector3::new(0.995, 1.005, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ],
        );
    }

    fn apply_heart_beat(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        CompositionAnimationHelpers::start_vector3_animation(
            visual,
            "Scale",
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.14, Vector3::ONE),
                Vector3KeyFrame::new(0.28, Vector3::new(1.3, 1.3, 1.0)),
                Vector3KeyFrame::new(0.42, Vector3::ONE),
                Vector3KeyFrame::new(0.7, Vector3::new(1.3, 1.3, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ],
        );
    }
}
