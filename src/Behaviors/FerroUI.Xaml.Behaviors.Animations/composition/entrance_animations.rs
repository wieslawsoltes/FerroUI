//! Ported from `Composition/EntranceAnimations.cs`.

use super::composition_animation_definition::CompositionAnimationDefinition;
use super::composition_animation_helpers::{CompositionAnimationHelpers, ScalarKeyFrame, Vector3KeyFrame};
use ferroui_base::animation::TimeSpan;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_controls::Control;

/// Provides entrance animation helpers inspired by common CSS transitions (e.g. animate.css,
/// Framer Motion).
pub struct EntranceAnimations;

impl EntranceAnimations {
    /// Applies the animate.css backInDown animation.
    pub fn set_back_in_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_in_down, false);
    }

    /// Applies the animate.css backInLeft animation.
    pub fn set_back_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_in_left, false);
    }

    /// Applies the animate.css backInRight animation.
    pub fn set_back_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_in_right, false);
    }

    /// Applies the animate.css backInUp animation.
    pub fn set_back_in_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_back_in_up, false);
    }

    /// Applies the animate.css bounceIn animation.
    pub fn set_bounce_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_in, true);
    }

    /// Applies the animate.css bounceInDown animation.
    pub fn set_bounce_in_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_in_down, false);
    }

    /// Applies the animate.css bounceInLeft animation.
    pub fn set_bounce_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_in_left, false);
    }

    /// Applies the animate.css bounceInRight animation.
    pub fn set_bounce_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_in_right, false);
    }

    /// Applies the animate.css bounceInUp animation.
    pub fn set_bounce_in_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_bounce_in_up, false);
    }

    /// Applies a simple fade-in animation.
    pub fn set_fade_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_fade_in, false);
    }

    /// Applies the animate.css fadeInDown animation.
    pub fn set_fade_in_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(0.0, -24.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInDownBig animation.
    pub fn set_fade_in_down_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(0.0, -240.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInLeft animation.
    pub fn set_fade_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(-24.0, 0.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInLeftBig animation.
    pub fn set_fade_in_left_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(-240.0, 0.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInRight animation.
    pub fn set_fade_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(24.0, 0.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInRightBig animation.
    pub fn set_fade_in_right_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(240.0, 0.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInUp animation.
    pub fn set_fade_in_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(0.0, 24.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInUpBig animation.
    pub fn set_fade_in_up_big(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(0.0, 240.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInTopLeft animation.
    pub fn set_fade_in_top_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(-24.0, -24.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInTopRight animation.
    pub fn set_fade_in_top_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(24.0, -24.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInBottomLeft animation.
    pub fn set_fade_in_bottom_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(-24.0, 24.0, 0.0)), false);
    }

    /// Applies the animate.css fadeInBottomRight animation.
    pub fn set_fade_in_bottom_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_fade_in_offset(Vector3::new(24.0, 24.0, 0.0)), false);
    }

    /// Applies the animate.css zoomIn animation. (Compatibility alias for fade-in zoom variant.)
    pub fn set_fade_in_zoom(element: &Control, milliseconds: f64) {
        Self::set_zoom_in(element, milliseconds);
    }

    /// Applies the animate.css flipInX animation.
    pub fn set_flip_in_x(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_flip_in_x, true);
    }

    /// Applies the animate.css flipInY animation.
    pub fn set_flip_in_y(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_flip_in_y, true);
    }

    /// Applies the animate.css lightSpeedInLeft animation.
    pub fn set_light_speed_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_light_speed_in(-1), false);
    }

    /// Applies the animate.css lightSpeedInRight animation.
    pub fn set_light_speed_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_light_speed_in(1), false);
    }

    /// Applies the animate.css rotateIn animation.
    pub fn set_rotate_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_in(-90.0, Vector2::new(0.5, 0.5)), true);
    }

    /// Applies the animate.css rotateInDownLeft animation.
    pub fn set_rotate_in_down_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_in(-45.0, Vector2::new(0.0, 1.0)), true);
    }

    /// Applies the animate.css rotateInDownRight animation.
    pub fn set_rotate_in_down_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_in(45.0, Vector2::new(1.0, 1.0)), true);
    }

    /// Applies the animate.css rotateInUpLeft animation.
    pub fn set_rotate_in_up_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_in(-45.0, Vector2::new(0.0, 0.0)), true);
    }

    /// Applies the animate.css rotateInUpRight animation.
    pub fn set_rotate_in_up_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_rotate_in(45.0, Vector2::new(1.0, 0.0)), true);
    }

    /// Applies the animate.css slideInDown animation.
    pub fn set_slide_in_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(0.0, -240.0, 0.0)), false);
    }

    /// Applies the animate.css slideInLeft animation.
    pub fn set_slide_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(-240.0, 0.0, 0.0)), false);
    }

    /// Applies the animate.css slideInRight animation.
    pub fn set_slide_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(240.0, 0.0, 0.0)), false);
    }

    /// Applies the animate.css slideInUp animation.
    pub fn set_slide_in_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_slide_in(Vector3::new(0.0, 240.0, 0.0)), false);
    }

    /// Applies the animate.css zoomIn animation.
    pub fn set_zoom_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_zoom_in, true);
    }

    /// Applies the animate.css zoomInDown animation.
    pub fn set_zoom_in_down(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_in_directional(Vector3::new(0.0, -240.0, 0.0)), true);
    }

    /// Applies the animate.css zoomInLeft animation.
    pub fn set_zoom_in_left(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_in_directional(Vector3::new(-240.0, 0.0, 0.0)), true);
    }

    /// Applies the animate.css zoomInRight animation.
    pub fn set_zoom_in_right(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_in_directional(Vector3::new(240.0, 0.0, 0.0)), true);
    }

    /// Applies the animate.css zoomInUp animation.
    pub fn set_zoom_in_up(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, || Self::create_zoom_in_directional(Vector3::new(0.0, 240.0, 0.0)), true);
    }

    /// Applies the animate.css jackInTheBox animation.
    pub fn set_jack_in_the_box(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_jack_in_the_box, true);
    }

    /// Applies the animate.css rollIn animation.
    pub fn set_roll_in(element: &Control, milliseconds: f64) {
        Self::run(element, milliseconds, Self::create_roll_in, false);
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

    fn create_back_in_down() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.7),
                ScalarKeyFrame::new(0.8, 0.7),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.0, -240.0, 0.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            initial_offset: Some(Vector3::new(0.0, -240.0, 0.0)),
            initial_scale: Some(Vector3::new(0.7, 0.7, 1.0)),
            initial_opacity: Some(0.7),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_back_in_left() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.7),
                ScalarKeyFrame::new(0.8, 0.7),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(-240.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            initial_offset: Some(Vector3::new(-240.0, 0.0, 0.0)),
            initial_scale: Some(Vector3::new(0.7, 0.7, 1.0)),
            initial_opacity: Some(0.7),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_back_in_right() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.7),
                ScalarKeyFrame::new(0.8, 0.7),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(240.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            initial_offset: Some(Vector3::new(240.0, 0.0, 0.0)),
            initial_scale: Some(Vector3::new(0.7, 0.7, 1.0)),
            initial_opacity: Some(0.7),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_back_in_up() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.7, 0.7, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.7),
                ScalarKeyFrame::new(0.8, 0.7),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.0, 240.0, 0.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            initial_offset: Some(Vector3::new(0.0, 240.0, 0.0)),
            initial_scale: Some(Vector3::new(0.7, 0.7, 1.0)),
            initial_opacity: Some(0.7),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_bounce_in() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.3, 0.3, 1.0)),
                Vector3KeyFrame::new(0.2, Vector3::new(1.1, 1.1, 1.0)),
                Vector3KeyFrame::new(0.4, Vector3::new(0.9, 0.9, 1.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(1.03, 1.03, 1.0)),
                Vector3KeyFrame::new(0.8, Vector3::new(0.97, 0.97, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_scale: Some(Vector3::new(0.3, 0.3, 1.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_bounce_in_down() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.0, -300.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(0.0, 25.0, 0.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(0.0, -10.0, 0.0)),
                Vector3KeyFrame::new(0.9, Vector3::new(0.0, 5.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(Vector3::new(0.0, -300.0, 0.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_bounce_in_left() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(-300.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(25.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(-10.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.9, Vector3::new(5.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(Vector3::new(-300.0, 0.0, 0.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_bounce_in_right() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(300.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(-25.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(10.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.9, Vector3::new(-5.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(Vector3::new(300.0, 0.0, 0.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_bounce_in_up() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.0, 300.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(0.0, -25.0, 0.0)),
                Vector3KeyFrame::new(0.75, Vector3::new(0.0, 10.0, 0.0)),
                Vector3KeyFrame::new(0.9, Vector3::new(0.0, -5.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(Vector3::new(0.0, 300.0, 0.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_fade_in() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 0.0), ScalarKeyFrame::new(1.0, 1.0)]),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_fade_in_offset(start_offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, start_offset),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 0.0), ScalarKeyFrame::new(1.0, 1.0)]),
            initial_offset: Some(start_offset),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_flip_in_x() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, CompositionAnimationHelpers::degrees_to_radians(90.0)),
                ScalarKeyFrame::new(0.4, CompositionAnimationHelpers::degrees_to_radians(-20.0)),
                ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(10.0)),
                ScalarKeyFrame::new(0.8, CompositionAnimationHelpers::degrees_to_radians(-5.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_rotation: Some(CompositionAnimationHelpers::degrees_to_radians(90.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_flip_in_y() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, CompositionAnimationHelpers::degrees_to_radians(90.0)),
                ScalarKeyFrame::new(0.4, CompositionAnimationHelpers::degrees_to_radians(-20.0)),
                ScalarKeyFrame::new(0.6, CompositionAnimationHelpers::degrees_to_radians(10.0)),
                ScalarKeyFrame::new(0.8, CompositionAnimationHelpers::degrees_to_radians(-5.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_rotation: Some(CompositionAnimationHelpers::degrees_to_radians(90.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_light_speed_in(direction: i32) -> CompositionAnimationDefinition {
        let direction = direction as f32;
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(direction * -200.0, 0.0, 0.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(direction * 30.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, CompositionAnimationHelpers::degrees_to_radians(direction * -30.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(Vector3::new(direction * -200.0, 0.0, 0.0)),
            initial_rotation: Some(CompositionAnimationHelpers::degrees_to_radians(direction * -30.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }

    fn create_rotate_in(start_angle: f32, anchor: Vector2) -> CompositionAnimationDefinition {
        let start_radians = CompositionAnimationHelpers::degrees_to_radians(start_angle);
        CompositionAnimationDefinition {
            rotation_frames: Some(vec![ScalarKeyFrame::new(0.0, start_radians), ScalarKeyFrame::new(1.0, 0.0)]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 0.0), ScalarKeyFrame::new(1.0, 1.0)]),
            anchor: Some(anchor),
            initial_rotation: Some(start_radians),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_slide_in(start_offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, start_offset),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            initial_offset: Some(start_offset),
            ..Default::default()
        }
    }

    fn create_zoom_in() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.3, 0.3, 1.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(1.05, 1.05, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.5, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_scale: Some(Vector3::new(0.3, 0.3, 1.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_zoom_in_directional(start_offset: Vector3) -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, start_offset),
                Vector3KeyFrame::new(0.6, start_offset * 0.2),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.3, 0.3, 1.0)),
                Vector3KeyFrame::new(0.6, Vector3::new(1.05, 1.05, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.6, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            initial_offset: Some(start_offset),
            initial_scale: Some(Vector3::new(0.3, 0.3, 1.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_jack_in_the_box() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            scale_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(0.1, 0.1, 1.0)),
                Vector3KeyFrame::new(0.5, Vector3::new(0.8, 0.8, 1.0)),
                Vector3KeyFrame::new(1.0, Vector3::ONE),
            ]),
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, CompositionAnimationHelpers::degrees_to_radians(30.0)),
                ScalarKeyFrame::new(0.5, CompositionAnimationHelpers::degrees_to_radians(-10.0)),
                ScalarKeyFrame::new(0.7, CompositionAnimationHelpers::degrees_to_radians(3.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            opacity_frames: Some(vec![
                ScalarKeyFrame::new(0.0, 0.0),
                ScalarKeyFrame::new(0.7, 1.0),
                ScalarKeyFrame::new(1.0, 1.0),
            ]),
            anchor: Some(Vector2::new(0.5, 1.0)),
            initial_scale: Some(Vector3::new(0.1, 0.1, 1.0)),
            initial_rotation: Some(CompositionAnimationHelpers::degrees_to_radians(30.0)),
            initial_opacity: Some(0.0),
            ensure_center_point: true,
            ..Default::default()
        }
    }

    fn create_roll_in() -> CompositionAnimationDefinition {
        CompositionAnimationDefinition {
            offset_frames: Some(vec![
                Vector3KeyFrame::new(0.0, Vector3::new(-240.0, 0.0, 0.0)),
                Vector3KeyFrame::new(1.0, Vector3::ZERO),
            ]),
            rotation_frames: Some(vec![
                ScalarKeyFrame::new(0.0, CompositionAnimationHelpers::degrees_to_radians(-120.0)),
                ScalarKeyFrame::new(1.0, 0.0),
            ]),
            opacity_frames: Some(vec![ScalarKeyFrame::new(0.0, 0.0), ScalarKeyFrame::new(1.0, 1.0)]),
            initial_offset: Some(Vector3::new(-240.0, 0.0, 0.0)),
            initial_rotation: Some(CompositionAnimationHelpers::degrees_to_radians(-120.0)),
            initial_opacity: Some(0.0),
            ..Default::default()
        }
    }
}
