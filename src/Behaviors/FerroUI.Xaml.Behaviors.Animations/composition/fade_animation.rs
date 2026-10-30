//! Ported from `Composition/FadeAnimation.cs`.

use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::Visual;
use ferroui_controls::Control;

/// Provides fade animation helpers using composition animations.
pub struct FadeAnimation;

impl FadeAnimation {
    /// Applies a fade-in animation to the element when loaded, over `milliseconds`.
    pub fn set_fade_in(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, 0.0, 1.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a fade-out animation to the element when loaded, over `milliseconds`.
    pub fn set_fade_out(element: &Control, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, 1.0, 0.0, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies a custom fade animation to the element when loaded: from `from_opacity` to
    /// `to_opacity` (0.0 to 1.0), over `milliseconds`.
    pub fn set_custom_fade(element: &Control, from_opacity: f64, to_opacity: f64, milliseconds: f64) {
        element.loaded(move |sender, _| {
            if let Some(element) = sender.downcast_ref::<Visual>() {
                Self::apply(element, from_opacity as f32, to_opacity as f32, TimeSpan::from_milliseconds(milliseconds));
            }
        });
    }

    /// Applies an opacity animation to the visual.
    fn apply(visual: &Visual, from_opacity: f32, to_opacity: f32, duration: TimeSpan) {
        let Some(composition_visual) = ElementComposition::get_element_visual(visual) else {
            return;
        };

        CompositionAnimationHelpers::start_scalar_animation(
            &composition_visual,
            "Opacity",
            duration,
            &[ScalarKeyFrame::new(0.0, from_opacity), ScalarKeyFrame::new(1.0, to_opacity)],
        );
    }
}
