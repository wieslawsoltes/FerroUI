//! Ported from `Composition/ScaleAnimation.cs`.

use super::composition_animation_helpers::{CompositionAnimationHelpers, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::Visual;
use ferroui_controls::Control;

/// Provides scale animation helpers using composition animations.
pub struct ScaleAnimation;

impl ScaleAnimation {
    /// Applies a scale-in animation to the element when loaded (from 0 to 1), over
    /// `milliseconds`.
    pub fn set_scale_in(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, Vector3::new(0.0, 0.0, 0.0), Vector3::ONE, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a scale-out animation to the element when loaded (from 1 to 0), over
    /// `milliseconds`.
    pub fn set_scale_out(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, Vector3::ONE, Vector3::new(0.0, 0.0, 0.0), TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a zoom-in animation to the element when loaded (from 1.5 to 1), over
    /// `milliseconds`.
    pub fn set_zoom_in(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, Vector3::new(1.5, 1.5, 1.0), Vector3::ONE, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a zoom-out animation to the element when loaded (from 1 to 1.5), over
    /// `milliseconds`.
    pub fn set_zoom_out(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, Vector3::ONE, Vector3::new(1.5, 1.5, 1.0), TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a bounce animation to the element when loaded, over `milliseconds`.
    pub fn set_bounce(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply_bounce(element, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a custom scale animation to the element when loaded: from (`from_scale_x`,
    /// `from_scale_y`) to (`to_scale_x`, `to_scale_y`), over `milliseconds`.
    pub fn set_custom_scale(
        element: &Control,
        from_scale_x: f64,
        from_scale_y: f64,
        to_scale_x: f64,
        to_scale_y: f64,
        milliseconds: f64,
    ) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(
                    element,
                    Vector3::new(from_scale_x as f32, from_scale_y as f32, 1.0),
                    Vector3::new(to_scale_x as f32, to_scale_y as f32, 1.0),
                    TimeSpan::from_milliseconds(milliseconds),
                );
            }
        });
    }

    /// Applies a scale animation to the visual.
    fn apply(visual: &Visual, from_scale: Vector3, to_scale: Vector3, duration: TimeSpan) {
        let Some(composition_visual) = ElementComposition::get_element_visual(visual) else {
            return;
        };

        CompositionAnimationHelpers::start_vector3_animation(
            &composition_visual,
            "Scale",
            duration,
            &[Vector3KeyFrame::new(0.0, from_scale), Vector3KeyFrame::new(1.0, to_scale)],
        );
    }

    /// Applies a bounce animation to the visual.
    fn apply_bounce(visual: &Visual, duration: TimeSpan) {
        let Some(composition_visual) = ElementComposition::get_element_visual(visual) else {
            return;
        };

        CompositionAnimationHelpers::start_vector3_animation(
            &composition_visual,
            "Scale",
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.5, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(0.9, 0.9, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ],
        );
    }
}
