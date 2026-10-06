use super::Layoutable;
use crate::{Rect, Size};

/// Provides access to layout information of a control.
pub struct LayoutInformation;

impl LayoutInformation {
    /// Gets the available size constraint passed in the previous layout pass.
    ///
    /// Returns the previous control measure constraint, if any.
    pub fn get_previous_measure_constraint(control: &Layoutable) -> Option<Size> {
        control.previous_measure()
    }

    /// Gets the control bounds used in the previous layout arrange pass.
    ///
    /// Returns the previous control arrange bounds, if any.
    pub fn get_previous_arrange_bounds(control: &Layoutable) -> Option<Rect> {
        control.previous_arrange()
    }
}
