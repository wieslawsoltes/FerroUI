//! Ported from `Composition/CompositionAnimationHelpers.cs`.
//!
//! Deviation (DEVIATIONS.md, Behaviors): the nested `ScalarKeyFrame` and `Vector3KeyFrame`
//! structs of the reference's helper class are types of this module; Rust has no nested types.

use crate::argument_out_of_range::throw_if_less_than;
use ferroui_base::animation::{PlaybackDirection, TimeSpan};
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::rendering::composition::animations::{
    AnimationIterationBehavior, ICompositionAnimation, KeyFrameAnimation,
};
use ferroui_base::rendering::composition::{CompositionVisual, ICompositionObjectAnimations};
use ferroui_controls::Control;
use std::ops::Deref;
use std::time::Duration;

/// A key frame of a scalar composition animation: the normalized progress and the value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScalarKeyFrame {
    progress: f32,
    value: f32,
}

impl ScalarKeyFrame {
    pub(crate) const fn new(progress: f32, value: f32) -> Self {
        Self { progress, value }
    }

    pub(crate) fn progress(&self) -> f32 {
        self.progress
    }

    pub(crate) fn value(&self) -> f32 {
        self.value
    }
}

/// A key frame of a vector composition animation: the normalized progress and the value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Vector3KeyFrame {
    progress: f32,
    value: Vector3,
}

impl Vector3KeyFrame {
    pub(crate) const fn new(progress: f32, value: Vector3) -> Self {
        Self { progress, value }
    }

    pub(crate) fn progress(&self) -> f32 {
        self.progress
    }

    pub(crate) fn value(&self) -> Vector3 {
        self.value
    }
}

/// The helpers the composition animation catalogs share.
pub(crate) struct CompositionAnimationHelpers;

impl CompositionAnimationHelpers {
    /// Converts an angle in degrees to radians, in single precision as the reference does.
    pub(crate) fn degrees_to_radians(degrees: f32) -> f32 {
        (std::f64::consts::PI / 180.0) as f32 * degrees
    }

    /// Centers the composition visual of `element` on its bounds (on the size of the visual when
    /// the element has no bounds yet).
    pub(crate) fn ensure_center_point(element: &Control, composition_visual: &CompositionVisual) {
        let bounds = element.bounds();
        if bounds.width > 0.0 && bounds.height > 0.0 {
            composition_visual
                .set_center_point(Vector3::new((bounds.width * 0.5) as f32, (bounds.height * 0.5) as f32, 0.0).into());
        } else {
            let size = composition_visual.size();
            let half_x = (size.x * 0.5) as f32;
            let half_y = (size.y * 0.5) as f32;
            composition_visual.set_center_point(Vector3::new(half_x, half_y, 0.0).into());
        }
    }

    /// Sets the center point of the composition visual of `element` at the normalized `anchor`
    /// of its bounds (of the size of the visual when the element has no bounds yet).
    pub(crate) fn set_normalized_center_point(element: &Control, composition_visual: &CompositionVisual, anchor: Vector2) {
        let bounds = element.bounds();
        if bounds.width > 0.0 && bounds.height > 0.0 {
            composition_visual.set_center_point(
                Vector3::new(
                    (bounds.width * anchor.x as f64) as f32,
                    (bounds.height * anchor.y as f64) as f32,
                    0.0,
                )
                .into(),
            );
        } else {
            let size = composition_visual.size();
            composition_visual.set_center_point(
                Vector3::new((size.x * anchor.x as f64) as f32, (size.y * anchor.y as f64) as f32, 0.0).into(),
            );
        }
    }

    /// Starts a vector key-frame animation of the named property (`Offset` or `Scale`); with a
    /// zero duration, sets the value of the last key frame instead.
    ///
    /// # Panics
    ///
    /// When `duration` is negative, or, for a zero duration, when the property is not `Offset`
    /// or `Scale` (the exceptions of the reference).
    #[track_caller]
    pub(crate) fn start_vector3_animation(
        visual: &CompositionVisual,
        property_name: &str,
        duration: TimeSpan,
        key_frames: &[Vector3KeyFrame],
    ) {
        let Some(last) = key_frames.last() else {
            return;
        };

        Self::validate_duration(duration);
        if duration == TimeSpan::ZERO {
            Self::set_vector3_value(visual, property_name, last.value());
            return;
        }

        let animation = visual.compositor().create_vector3_key_frame_animation();
        for key_frame in key_frames {
            animation.insert_key_frame(key_frame.progress(), key_frame.value());
        }

        Self::configure_and_start_animation(visual, property_name, duration, &*animation);
    }

    /// Starts an animation of the offset of the composition visual of `element`, whose key
    /// frames are relative to the layout offset of the element; with a zero duration, sets the
    /// offset of the last key frame instead.
    ///
    /// # Panics
    ///
    /// When `duration` is negative (the `ArgumentOutOfRangeException` of the reference).
    #[track_caller]
    pub(crate) fn start_offset_animation(
        element: &Control,
        visual: &CompositionVisual,
        duration: TimeSpan,
        key_frames: &[Vector3KeyFrame],
    ) {
        let Some(last) = key_frames.last() else {
            return;
        };

        let layout_offset = Self::get_layout_offset(element);
        Self::validate_duration(duration);
        if duration == TimeSpan::ZERO {
            visual.set_offset((layout_offset + last.value()).into());
            return;
        }

        let animation = visual.compositor().create_vector3_key_frame_animation();
        for key_frame in key_frames {
            animation.insert_key_frame(key_frame.progress(), layout_offset + key_frame.value());
        }

        Self::configure_and_start_animation(visual, "Offset", duration, &*animation);
    }

    /// The layout offset of `element`: the position of its bounds.
    pub(crate) fn get_layout_offset(element: &Control) -> Vector3 {
        let bounds = element.bounds();
        Vector3::new(bounds.x as f32, bounds.y as f32, 0.0)
    }

    /// The layout offset of `element` moved by `relative_offset`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the `GetLayoutOffset(Control, Vector3)` overload of
    /// the reference; Rust has no overloads.
    pub(crate) fn get_layout_offset_with(element: &Control, relative_offset: Vector3) -> Vector3 {
        Self::get_layout_offset(element) + relative_offset
    }

    /// Starts a scalar key-frame animation of the named property (`Opacity` or
    /// `RotationAngle`); with a zero duration, sets the value of the last key frame instead.
    ///
    /// # Panics
    ///
    /// When `duration` is negative, or, for a zero duration, when the property is not `Opacity`
    /// or `RotationAngle` (the exceptions of the reference).
    #[track_caller]
    pub(crate) fn start_scalar_animation(
        visual: &CompositionVisual,
        property_name: &str,
        duration: TimeSpan,
        key_frames: &[ScalarKeyFrame],
    ) {
        let Some(last) = key_frames.last() else {
            return;
        };

        Self::validate_duration(duration);
        if duration == TimeSpan::ZERO {
            Self::set_scalar_value(visual, property_name, last.value());
            return;
        }

        let animation = visual.compositor().create_scalar_key_frame_animation();
        for key_frame in key_frames {
            animation.insert_key_frame(key_frame.progress(), key_frame.value());
        }

        Self::configure_and_start_animation(visual, property_name, duration, &*animation);
    }

    #[track_caller]
    fn set_vector3_value(visual: &CompositionVisual, property_name: &str, value: Vector3) {
        match property_name {
            "Offset" => visual.set_offset(value.into()),
            "Scale" => visual.set_scale(value.into()),
            _ => panic!(
                "Unsupported Vector3 composition property '{property_name}'. (Parameter 'propertyName')"
            ),
        }
    }

    #[track_caller]
    fn set_scalar_value(visual: &CompositionVisual, property_name: &str, value: f32) {
        match property_name {
            "Opacity" => visual.set_opacity(value),
            "RotationAngle" => visual.set_rotation_angle(value),
            _ => panic!(
                "Unsupported scalar composition property '{property_name}'. (Parameter 'propertyName')"
            ),
        }
    }

    #[track_caller]
    fn validate_duration(duration: TimeSpan) {
        throw_if_less_than(duration, TimeSpan::ZERO, "duration");
    }

    /// Configures a key-frame animation to play once, forwards, over `duration`, and starts it
    /// on the named property.
    ///
    /// The reference takes any composition animation and configures it when it is a key-frame
    /// animation; every animation it is given is one, which the type of the parameter states
    /// here.
    fn configure_and_start_animation<A>(visual: &CompositionVisual, property_name: &str, duration: TimeSpan, animation: &A)
    where
        A: ICompositionAnimation + Deref<Target = KeyFrameAnimation>,
    {
        let key_frame_animation: &KeyFrameAnimation = animation;
        key_frame_animation.set_direction(PlaybackDirection::Normal);
        key_frame_animation.set_duration(to_duration(duration));
        key_frame_animation.set_iteration_behavior(AnimationIterationBehavior::Count);
        key_frame_animation.set_iteration_count(1);

        visual.start_animation(property_name, animation);
    }
}

/// The duration of a composition animation for an animation time that is not negative.
pub(crate) fn to_duration(duration: TimeSpan) -> Duration {
    duration.to_duration().unwrap_or_default()
}
