//! Ported from `Composition/SlidingAnimation.cs`.

use super::composition_animation_helpers::{CompositionAnimationHelpers, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_controls::Control;

/// Slides an element into its place when it is loaded, from outside one of its edges.
pub struct SlidingAnimation;

impl SlidingAnimation {
    /// Slides the element in from its width to the left, over `milliseconds`, when it is loaded.
    pub fn set_left(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Control>() {
                Self::apply(element, -element.bounds().width, 0.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Slides the element in from twice its width to the right, over `milliseconds`, when it is
    /// loaded.
    pub fn set_right(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Control>() {
                Self::apply(element, 2.0 * element.bounds().width, 0.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Slides the element in from its height above, over `milliseconds`, when it is loaded.
    pub fn set_top(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Control>() {
                Self::apply(element, 0.0, -element.bounds().height, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Slides the element in from twice its height below, over `milliseconds`, when it is
    /// loaded.
    pub fn set_bottom(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Control>() {
                Self::apply(element, 0.0, 2.0 * element.bounds().height, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Animates the offset of the visual from (`offset_x`, `offset_y`) relative to its layout
    /// offset back to the layout offset.
    fn apply(visual: &Control, offset_x: f64, offset_y: f64, duration: TimeSpan) {
        let Some(composition_visual) = ElementComposition::get_element_visual(visual) else {
            return;
        };

        CompositionAnimationHelpers::start_offset_animation(
            visual,
            &composition_visual,
            duration,
            &[
                Vector3KeyFrame::new(0.0, Vector3::new(offset_x as f32, offset_y as f32, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ],
        );
    }
}
