//! Ported from `Composition/FramerMotionAnimations.cs`.

use super::composition_animation_definition::CompositionAnimationDefinition;
use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_controls::Control;

/// Provides composition animations inspired by popular Framer Motion presets.
///
/// Every `set_*` function animates `element`, the control to animate, over `milliseconds`, the
/// animation duration expressed in milliseconds, when the control is loaded.
pub struct FramerMotionAnimations;

impl FramerMotionAnimations {
    /// Fades the control in using an opacity animation.
    pub fn set_fade_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_fade_in_definition, false);
    }

    /// Fades the control in while translating it upward.
    pub fn set_fade_in_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_directional(Vector3::new(0.0, 40.0, 0.0)), false);
    }

    /// Fades the control in while translating it downward.
    pub fn set_fade_in_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_directional(Vector3::new(0.0, -40.0, 0.0)), false);
    }

    /// Fades the control in while translating it from the left.
    pub fn set_fade_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_directional(Vector3::new(-40.0, 0.0, 0.0)), false);
    }

    /// Fades the control in while translating it from the right.
    pub fn set_fade_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_directional(Vector3::new(40.0, 0.0, 0.0)), false);
    }

    /// Slides the control in from the left.
    pub fn set_slide_in_from_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(-120.0, 0.0, 0.0)), false);
    }

    /// Slides the control in from the right.
    pub fn set_slide_in_from_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(120.0, 0.0, 0.0)), false);
    }

    /// Slides the control in from the top.
    pub fn set_slide_in_from_top(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(0.0, -120.0, 0.0)), false);
    }

    /// Slides the control in from the bottom.
    pub fn set_slide_in_from_bottom(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(0.0, 120.0, 0.0)), false);
    }

    /// Runs a pop-in scale animation centered on the control.
    pub fn set_pop_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_pop_in_definition, true);
    }

    /// Runs a pop-out scale animation centered on the control.
    pub fn set_pop_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_pop_out_definition, true);
    }

    /// Runs a vertical spring-in animation similar to Framer Motion's preset.
    pub fn set_spring_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_spring_in_definition, false);
    }

    /// Runs a vertical spring-out animation similar to Framer Motion's preset.
    pub fn set_spring_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_spring_out_definition, false);
    }

    /// Rotates the control into view.
    pub fn set_rotate_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_definition(-15.0, 0.0), true);
    }

    /// Rotates the control out of view.
    pub fn set_rotate_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_definition(0.0, 15.0), true);
    }

    /// Scales the control up into view.
    pub fn set_scale_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_scale_in_definition, true);
    }

    /// Scales the control down out of view.
    pub fn set_scale_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_scale_out_definition, true);
    }

    fn run(
        element: &Control,
        milliseconds: f64,
        definition_factory: fn() -> CompositionAnimationDefinition,
        ensure_center_point: bool,
    ) {
        element.loaded(move |sender, _| {
            let Some(element) = sender.downcast_ref::<Control>() else {
                return;
            };
            let Some(visual) = ElementComposition::get_element_visual(element) else {
                return;
            };

            let definition = definition_factory();

            if let Some(initial_offset) = definition.initial_offset() {
                visual.set_offset(CompositionAnimationHelpers::get_layout_offset_with(element, initial_offset).into());
            }

            if let Some(initial_scale) = definition.initial_scale() {
                visual.set_scale(initial_scale.into());
            }

            if let Some(initial_opacity) = definition.initial_opacity() {
                visual.set_opacity(initial_opacity);
            }

            if let Some(initial_rotation) = definition.initial_rotation() {
                visual.set_rotation_angle(initial_rotation);
            }

            if let Some(anchor) = definition.anchor() {
                CompositionAnimationHelpers::set_normalized_center_point(element, &visual, anchor);
            } else if definition.ensure_center_point()
                || ensure_center_point
                || definition.scale_frames().is_some_and(|frames| !frames.is_empty())
                || definition.rotation_frames().is_some_and(|frames| !frames.is_empty())
            {
                CompositionAnimationHelpers::ensure_center_point(element, &visual);
            }

            let duration = TimeSpan::from_milliseconds(milliseconds);

            if let Some(frames) = definition.offset_frames().filter(|frames| !frames.is_empty()) {
                CompositionAnimationHelpers::start_offset_animation(element, &visual, duration, frames);
            }

            if let Some(frames) = definition.scale_frames().filter(|frames| !frames.is_empty()) {
                CompositionAnimationHelpers::start_vector3_animation(&visual, "Scale", duration, frames);
            }

            if let Some(frames) = definition.opacity_frames().filter(|frames| !frames.is_empty()) {
                CompositionAnimationHelpers::start_scalar_animation(&visual, "Opacity", duration, frames);
            }

            if let Some(frames) = definition.rotation_frames().filter(|frames| !frames.is_empty()) {
                CompositionAnimationHelpers::start_scalar_animation(&visual, "RotationAngle", duration, frames);
            }
        });
    }

    fn create_fade_in_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 0.0), ScalarKeyFrame::new(1.0, 1.0)]),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_fade_in_directional(offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, offset),
                Vector3KeyFrame::new(0.7, offset * 0.2),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.7, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(offset),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_slide_in(offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, offset),
                Vector3KeyFrame::new(0.6, offset * -0.1),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            initial_offset: Some(offset),
            ..Default::default()
        }
    }

    fn create_pop_in_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.8, 0.8, 1.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(0.7, Vector3::new(0.98, 0.98, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.4, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_scale: Some(Vector3::new(0.8, 0.8, 1.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_pop_out_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.3, Vector3::new(1.05, 1.05, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.6, 0.6, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.3, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_spring_in_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.0, 120.0, 0.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(0.0, -12.0, 0.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.0, 6.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.5, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(Vector3::new(0.0, 120.0, 0.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_spring_out_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.3, Vector3::new(0.0, -12.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(0.0, 6.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.0, 140.0, 0.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_rotate_definition(start_angle: f32, end_angle: f32) -> CompositionAnimationDefinition {
        // `float.Epsilon` of the reference is the smallest positive subnormal single, not the
        // machine epsilon: an angle is "visible" only when it is exactly zero.
        let float_epsilon = f32::from_bits(1);
        let start_radians = CompositionAnimationHelpers::degrees_to_radians(start_angle);
        let mid_radians = CompositionAnimationHelpers::degrees_to_radians(end_angle * 0.6);
        let end_radians = CompositionAnimationHelpers::degrees_to_radians(end_angle);
        let start_visible = start_angle.abs() < float_epsilon;
        let end_visible = end_angle.abs() < float_epsilon;

        CompositionAnimationDefinition {
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, start_radians),
                ScalarKeyFrame::new(0.6, mid_radians),
                ScalarKeyFrame::new(1.0, end_radians),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, if start_visible { 1.0 } else { 0.0 }),
                ScalarKeyFrame::new(0.4, 1.0),
                ScalarKeyFrame::new(1.0, if end_visible { 1.0 } else { 0.0 }),
            ]),
            initial_rotation: Some(start_radians),
            initial_opacity: Some(if start_visible { 1.0 } else { 0.0 }),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_scale_in_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.95, 0.95, 1.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(1.02, 1.02, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.5, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_scale: Some(Vector3::new(0.95, 0.95, 1.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_scale_out_definition() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.4, Vector3::new(0.95, 0.95, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.8, 0.8, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.4, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }
}
