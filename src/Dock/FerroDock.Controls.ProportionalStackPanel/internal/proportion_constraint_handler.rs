// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_base::layout::Orientation;
use ferroui_controls::Control;
pub(crate) struct ProportionConstraintHandler {
    orientation: Orientation,
    available_dimension: f64,
}
impl ProportionConstraintHandler {
    pub(crate) fn new(orientation: Orientation, available_dimension: f64) -> Self {
        Self {
            orientation,
            available_dimension,
        }
    }
    pub(crate) fn clamp_proportion(&self, control: &Control, proportion: f64) -> f64 {
        let (minimum, maximum) = ProportionUtils::get_size_constraints(control, self.orientation);
        let min_prop = if !minimum.is_nan() && minimum > 0.0 {
            minimum / self.available_dimension
        } else {
            0.0
        };
        let max_prop = if !maximum.is_nan() && maximum != f64::INFINITY {
            maximum / self.available_dimension
        } else {
            f64::INFINITY
        };
        math_clamp(proportion, min_prop, max_prop)
    }
}
