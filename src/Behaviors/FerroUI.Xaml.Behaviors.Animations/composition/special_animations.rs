//! Ported from `Composition/SpecialAnimations.cs`.

use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_controls::Control;

/// Provides special animation helpers that do not fit other categories.
pub struct SpecialAnimations;

impl SpecialAnimations {
    /// Applies an animate.css style `flip` animation approximation using composition animations,
    /// over `milliseconds`.
    pub fn set_flip(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::apply_flip, true, None);
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

    fn apply_flip(_element: &Control, visual: &CompositionVisual, duration: TimeSpan) {
        let rotation_frames = [
            ScalarKeyFrame::new(0.0, CompositionAnimationHelpers::degrees_to_radians(-360.0)),
            ScalarKeyFrame::new(0.4, CompositionAnimationHelpers::degrees_to_radians(-190.0)),
            ScalarKeyFrame::new(0.5, CompositionAnimationHelpers::degrees_to_radians(-170.0)),
            ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(-60.0)),
            ScalarKeyFrame::new(0.8, CompositionAnimationHelpers::degrees_to_radians(-30.0)),
            ScalarKeyFrame::new(1.0, 0.0),
        ];

        let scale_frames = [
            Vector3KeyFrame::new(0.0, Vector3::new(1.0, 1.0, 1.0)),
            Vector3KeyFrame::new(0.4, Vector3::new(1.0, 1.0, 1.0)),
            Vector3KeyFrame::new(0.5, Vector3::new(1.1, 1.1, 1.0)),
            Vector3KeyFrame::new(0.6, Vector3::new(1.1, 1.1, 1.0)),
            Vector3KeyFrame::new(0.8, Vector3::new(1.05, 1.05, 1.0)),
            Vector3KeyFrame::new(1.0, Vector3::new(1.0, 1.0, 1.0)),
        ];

        CompositionAnimationHelpers::start_scalar_animation(visual, "RotationAngle", duration, &rotation_frames);
        CompositionAnimationHelpers::start_vector3_animation(visual, "Scale", duration, &scale_frames);
    }
}
