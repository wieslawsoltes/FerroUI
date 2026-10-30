//! Ported from `Composition/OrbitAnimation.cs`.

use ferroui_base::numerics::{Quaternion, Vector3};
use ferroui_base::rendering::composition::{ElementComposition, ICompositionObjectAnimations};
use ferroui_base::Vector;
use ferroui_controls::Control;
use std::cell::Cell;
use std::time::Duration;

/// Maintains and applies a composition orientation for an orbit-style pointer interaction.
///
/// Deviation (DEVIATIONS.md, Behaviors): the orientation (a property with a private setter in
/// the reference) is kept in a cell, so that `rotate` and `reset` take `&self` as the methods of
/// the reference's class do.
#[derive(Debug)]
pub struct OrbitAnimation {
    orientation: Cell<Quaternion>,
}

impl Default for OrbitAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitAnimation {
    /// Creates an orbit animation at the identity orientation.
    pub fn new() -> Self {
        Self { orientation: Cell::new(Quaternion::IDENTITY) }
    }

    /// Gets the current orientation.
    pub fn orientation(&self) -> Quaternion {
        self.orientation.get()
    }

    /// Calculates the next normalized orientation from the current `orientation` and the pointer
    /// movement since the previous update; `sensitivity` is the rotation sensitivity.
    pub fn calculate_orientation(orientation: Quaternion, pointer_delta: Vector, sensitivity: f64) -> Quaternion {
        let rotation_around_x = (pointer_delta.y * sensitivity * 0.01) as f32;
        let rotation_around_y = (pointer_delta.x * sensitivity * 0.01) as f32;
        let rotation_x = Quaternion::create_from_axis_angle(Vector3::UNIT_X, -rotation_around_x);
        let rotation_y = Quaternion::create_from_axis_angle(Vector3::UNIT_Y, rotation_around_y);
        Quaternion::normalize(orientation * rotation_x * rotation_y)
    }

    /// Updates the orientation and applies it to a control's composition visual. Returns `true`
    /// when a composition visual was animated; otherwise, `false`.
    pub fn rotate(&self, target: Option<&Control>, pointer_delta: Vector, sensitivity: f64) -> bool {
        let Some(visual) = target.and_then(|target| ElementComposition::get_element_visual(target)) else {
            return false;
        };

        self.orientation.set(Self::calculate_orientation(self.orientation.get(), pointer_delta, sensitivity));
        let animation = visual.compositor().create_quaternion_key_frame_animation();
        animation.insert_key_frame(1.0, self.orientation.get());
        animation.set_duration(Duration::from_millis(1));
        visual.start_animation("Orientation", &*animation);
        true
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

    /// Resets the maintained orientation to the identity quaternion.
    pub fn reset(&self) {
        self.orientation.set(Quaternion::IDENTITY);
    }
}
