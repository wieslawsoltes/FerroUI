//! Ported from `Composition/ExitAnimations.cs`.

use super::composition_animation_definition::CompositionAnimationDefinition;
use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_controls::Control;

/// Provides exit animation helpers inspired by animate.css.
///
/// Every `set_*` function animates `element` with its exit effect over `milliseconds`, the
/// animation duration in milliseconds, when the element is loaded.
pub struct ExitAnimations;

impl ExitAnimations {
    /// Animates `element` with the BackOutDown exit effect over the specified duration.
    pub fn set_back_out_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_out_down, true);
    }

    /// Animates `element` with the BackOutLeft exit effect over the specified duration.
    pub fn set_back_out_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_out_left, true);
    }

    /// Animates `element` with the BackOutRight exit effect over the specified duration.
    pub fn set_back_out_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_out_right, true);
    }

    /// Animates `element` with the BackOutUp exit effect over the specified duration.
    pub fn set_back_out_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_out_up, true);
    }

    /// Animates `element` with the BounceOut exit effect over the specified duration.
    pub fn set_bounce_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_out, true);
    }

    /// Animates `element` with the BounceOutDown exit effect over the specified duration.
    pub fn set_bounce_out_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_out_down, false);
    }

    /// Animates `element` with the BounceOutLeft exit effect over the specified duration.
    pub fn set_bounce_out_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_out_left, false);
    }

    /// Animates `element` with the BounceOutRight exit effect over the specified duration.
    pub fn set_bounce_out_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_out_right, false);
    }

    /// Animates `element` with the BounceOutUp exit effect over the specified duration.
    pub fn set_bounce_out_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_out_up, false);
    }

    /// Animates `element` with the FadeOut exit effect over the specified duration.
    pub fn set_fade_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_fade_out, false);
    }

    /// Animates `element` with the FadeOutDown exit effect over the specified duration.
    pub fn set_fade_out_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(0.0, 24.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutDownBig exit effect over the specified duration.
    pub fn set_fade_out_down_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(0.0, 240.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutLeft exit effect over the specified duration.
    pub fn set_fade_out_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(-24.0, 0.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutLeftBig exit effect over the specified duration.
    pub fn set_fade_out_left_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(-240.0, 0.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutRight exit effect over the specified duration.
    pub fn set_fade_out_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(24.0, 0.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutRightBig exit effect over the specified duration.
    pub fn set_fade_out_right_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(240.0, 0.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutUp exit effect over the specified duration.
    pub fn set_fade_out_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(0.0, -24.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutUpBig exit effect over the specified duration.
    pub fn set_fade_out_up_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(0.0, -240.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutTopLeft exit effect over the specified duration.
    pub fn set_fade_out_top_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(-24.0, -24.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutTopRight exit effect over the specified duration.
    pub fn set_fade_out_top_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(24.0, -24.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutBottomLeft exit effect over the specified duration.
    pub fn set_fade_out_bottom_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(-24.0, 24.0, 0.0)), false);
    }

    /// Animates `element` with the FadeOutBottomRight exit effect over the specified duration.
    pub fn set_fade_out_bottom_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_out_offset(Vector3::new(24.0, 24.0, 0.0)), false);
    }

    /// Animates `element` with the FlipOutX exit effect over the specified duration.
    pub fn set_flip_out_x(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_flip_out_x, true);
    }

    /// Animates `element` with the FlipOutY exit effect over the specified duration.
    pub fn set_flip_out_y(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_flip_out_y, true);
    }

    /// Animates `element` with the LightSpeedOutLeft exit effect over the specified duration.
    pub fn set_light_speed_out_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_light_speed_out(-1), false);
    }

    /// Animates `element` with the LightSpeedOutRight exit effect over the specified duration.
    pub fn set_light_speed_out_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_light_speed_out(1), false);
    }

    /// Animates `element` with the RotateOut exit effect over the specified duration.
    pub fn set_rotate_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_out(90.0, Vector2::new(0.5, 0.5)), true);
    }

    /// Animates `element` with the RotateOutDownLeft exit effect over the specified duration.
    pub fn set_rotate_out_down_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_out(45.0, Vector2::new(0.0, 1.0)), true);
    }

    /// Animates `element` with the RotateOutDownRight exit effect over the specified duration.
    pub fn set_rotate_out_down_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_out(-45.0, Vector2::new(1.0, 1.0)), true);
    }

    /// Animates `element` with the RotateOutUpLeft exit effect over the specified duration.
    pub fn set_rotate_out_up_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_out(-45.0, Vector2::new(0.0, 0.0)), true);
    }

    /// Animates `element` with the RotateOutUpRight exit effect over the specified duration.
    pub fn set_rotate_out_up_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_out(45.0, Vector2::new(1.0, 0.0)), true);
    }

    /// Animates `element` with the SlideOutDown exit effect over the specified duration.
    pub fn set_slide_out_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_out(Vector3::new(0.0, 240.0, 0.0)), false);
    }

    /// Animates `element` with the SlideOutLeft exit effect over the specified duration.
    pub fn set_slide_out_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_out(Vector3::new(-240.0, 0.0, 0.0)), false);
    }

    /// Animates `element` with the SlideOutRight exit effect over the specified duration.
    pub fn set_slide_out_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_out(Vector3::new(240.0, 0.0, 0.0)), false);
    }

    /// Animates `element` with the SlideOutUp exit effect over the specified duration.
    pub fn set_slide_out_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_out(Vector3::new(0.0, -240.0, 0.0)), false);
    }

    /// Animates `element` with the ZoomOut exit effect over the specified duration.
    pub fn set_zoom_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_zoom_out, true);
    }

    /// Animates `element` with the ZoomOutDown exit effect over the specified duration.
    pub fn set_zoom_out_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_out_directional(Vector3::new(0.0, 240.0, 0.0)), true);
    }

    /// Animates `element` with the ZoomOutLeft exit effect over the specified duration.
    pub fn set_zoom_out_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_out_directional(Vector3::new(-240.0, 0.0, 0.0)), true);
    }

    /// Animates `element` with the ZoomOutRight exit effect over the specified duration.
    pub fn set_zoom_out_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_out_directional(Vector3::new(240.0, 0.0, 0.0)), true);
    }

    /// Animates `element` with the ZoomOutUp exit effect over the specified duration.
    pub fn set_zoom_out_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_out_directional(Vector3::new(0.0, -240.0, 0.0)), true);
    }

    /// Animates `element` with the Hinge exit effect over the specified duration.
    pub fn set_hinge(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_hinge, true);
    }

    /// Animates `element` with the RollOut exit effect over the specified duration.
    pub fn set_roll_out(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_roll_out, false);
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

    fn create_back_out_down() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.2, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.7, 0.7, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.2, 0.7),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::new(0.0, 240.0, 0.0)),
            ]),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_back_out_left() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.2, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.7, 0.7, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.2, 0.7),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::new(-240.0, 0.0, 0.0)),
            ]),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_back_out_right() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.2, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.7, 0.7, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.2, 0.7),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::new(240.0, 0.0, 0.0)),
            ]),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_back_out_up() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.2, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.7, 0.7, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.2, 0.7),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::new(0.0, -240.0, 0.0)),
            ]),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_bounce_out() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.25, Vector3::new(0.9, 0.9, 1.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.3, 0.3, 1.0)),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_bounce_out_down() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::new(0.0, -20.0, 0.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(0.0, 10.0, 0.0)),
                Vector3KeyFrame::new(0.45, Vector3::new(0.0, 10.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.0, 300.0, 0.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.45, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            ..Default::default()
        }
    }

    fn create_bounce_out_left() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::new(20.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(-10.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.45, Vector3::new(-10.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(-300.0, 0.0, 0.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.45, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            ..Default::default()
        }
    }

    fn create_bounce_out_right() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::new(-20.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(10.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.45, Vector3::new(10.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(300.0, 0.0, 0.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.45, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            ..Default::default()
        }
    }

    fn create_bounce_out_up() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.2, Vector3::new(0.0, 20.0, 0.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(0.0, -10.0, 0.0)),
                Vector3KeyFrame::new(0.45, Vector3::new(0.0, -10.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.0, -300.0, 0.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.45, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            ..Default::default()
        }
    }

    fn create_fade_out() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_fade_out_offset(target_offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, target_offset),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_flip_out_x() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.3, CompositionAnimationHelpers::degrees_to_radians(20.0)),
                ScalarKeyFrame::new(1.0, CompositionAnimationHelpers::degrees_to_radians(-90.0)),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            ensure_center_point: true,
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_flip_out_y() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.3, CompositionAnimationHelpers::degrees_to_radians(-20.0)),
                ScalarKeyFrame::new(1.0, CompositionAnimationHelpers::degrees_to_radians(90.0)),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            ensure_center_point: true,
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_light_speed_out(direction: i32) -> CompositionAnimationDefinition {
        let direction = direction as f32;
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::new(direction * 200.0, 0.0, 0.0)),
            ]),
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(1.0, CompositionAnimationHelpers::degrees_to_radians(direction * 45.0)),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_rotate_out(end_angle: f32, anchor: Vector2) -> CompositionAnimationDefinition {
        let end_radians = CompositionAnimationHelpers::degrees_to_radians(end_angle);
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![ScalarKeyFrame::new(0.0, 0.0), ScalarKeyFrame::new(1.0, end_radians)]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            anchor: Some(anchor),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_slide_out(target_offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, target_offset),
            ]),
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_zoom_out() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.5, Vector3::new(0.3, 0.3, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.1, 0.1, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.5, 0.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            ensure_center_point: true,
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_zoom_out_directional(target_offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.4, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, target_offset),
            ]),
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ONE),
                Vector3KeyFrame::new(0.4, Vector3::new(0.4, 0.4, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.1, 0.1, 1.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.4, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            ensure_center_point: true,
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }

    fn create_hinge() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.2, CompositionAnimationHelpers::degrees_to_radians(80.0)),
                ScalarKeyFrame::new(0.4, CompositionAnimationHelpers::degrees_to_radians(60.0)),
                ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(80.0)),
                ScalarKeyFrame::new(0.8, CompositionAnimationHelpers::degrees_to_radians(60.0)),
                ScalarKeyFrame::new(1.0, CompositionAnimationHelpers::degrees_to_radians(80.0)),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(0.8, Vector3::new(0.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::new(0.0, 300.0, 0.0)),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 1.0),
                ScalarKeyFrame::new(0.8, 1.0),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            anchor: Some(Vector2::new(0.0, 0.0)),
            initial_opacity: Some(1.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_roll_out() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::ZERO),
                Vector3KeyFrame::new(1.0, Vector3::new(240.0, 0.0, 0.0)),
            ]),
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(1.0, CompositionAnimationHelpers::degrees_to_radians(120.0)),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 1.0), ScalarKeyFrame::new(1.0, 0.0)]),
            initial_opacity: Some(1.0),
            ..Default::default()
        }
    }
}
