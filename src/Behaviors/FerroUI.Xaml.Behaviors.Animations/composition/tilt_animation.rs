//! Ported from `Composition/TiltAnimation.cs`.

use ferroui_base::numerics::{Quaternion, Vector3};
use ferroui_base::rendering::composition::{ElementComposition, ICompositionObjectAnimations};
use ferroui_base::{Point, Size};
use ferroui_controls::Control;
use std::time::Duration;

/// Calculates and applies composition-based pointer tilt animations.
pub struct TiltAnimation;

impl TiltAnimation {
    /// Calculates a tilt orientation for a pointer position (relative to the target) inside a
    /// target size; `tilt_strength` is the maximum tilt angle in degrees. Returns the calculated
    /// orientation, or identity for an empty size or central position.
    pub fn calculate_orientation(size: Size, pointer_position: Point, tilt_strength: f64) -> Quaternion {
        Self::try_calculate_orientation(size, pointer_position, tilt_strength).unwrap_or(Quaternion::IDENTITY)
    }

    /// Tries to calculate a non-neutral tilt orientation for a pointer position inside a target
    /// size. Returns the calculated orientation, or `None` for an empty size or central position.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the reference returns `bool` with the orientation in
    /// an `out` parameter (the identity when it returns `false`); here the orientation is the
    /// `Some` of the result.
    pub fn try_calculate_orientation(size: Size, pointer_position: Point, tilt_strength: f64) -> Option<Quaternion> {
        if size.width <= 0.0 || size.height <= 0.0 {
            return None;
        }

        let center_x = size.width / 2.0;
        let center_y = size.height / 2.0;
        let x_difference = (pointer_position.x - center_x) / center_x;
        let y_difference = (pointer_position.y - center_y) / center_y;
        let axis = Vector3::new((-y_difference) as f32, x_difference as f32, 0.0);
        if axis.length_squared() <= 0.001 {
            return None;
        }

        let axis = Vector3::normalize(axis);
        let distance = ((x_difference * x_difference) + (y_difference * y_difference)).sqrt().min(1.0);
        let angle = (distance * tilt_strength * (std::f64::consts::PI / 180.0)) as f32;
        Some(Quaternion::create_from_axis_angle(axis, angle))
    }

    /// Applies a tilt animation for a pointer position (relative to the target);
    /// `tilt_strength` is the maximum tilt angle in degrees. Returns `true` when a composition
    /// visual was animated; otherwise, `false`.
    pub fn apply(target: Option<&Control>, pointer_position: Point, tilt_strength: f64) -> bool {
        let Some(target) = target else {
            return false;
        };
        if target.bounds().width <= 0.0 || target.bounds().height <= 0.0 {
            return false;
        }

        let orientation = Self::calculate_orientation(target.bounds().size(), pointer_position, tilt_strength);
        Self::animate_to(Some(target), orientation, Duration::from_millis(50))
    }

    /// Animates a control back to its neutral orientation. Returns `true` when a composition
    /// visual was animated; otherwise, `false`.
    pub fn reset(target: Option<&Control>) -> bool {
        Self::animate_to(target, Quaternion::IDENTITY, Duration::from_millis(400))
    }

    /// Updates a control's composition center point from its current bounds. Returns `true` when
    /// a composition visual was updated; otherwise, `false`.
    pub fn update_center_point(target: Option<&Control>) -> bool {
        let Some(target) = target else {
            return false;
        };
        let Some(visual) = ElementComposition::get_element_visual(target) else {
            return false;
        };

        visual.set_center_point(
            Vector3::new(target.bounds().width as f32 / 2.0, target.bounds().height as f32 / 2.0, 0.0).into(),
        );
        true
    }

    fn animate_to(target: Option<&Control>, orientation: Quaternion, duration: Duration) -> bool {
        let Some(visual) = target.and_then(|target| ElementComposition::get_element_visual(target)) else {
            return false;
        };

        let animation = visual.compositor().create_quaternion_key_frame_animation();
        animation.insert_key_frame(1.0, orientation);
        animation.set_duration(duration);
        visual.start_animation("Orientation", &*animation);
        true
    }
}
