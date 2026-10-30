// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::dock_point::{double_equals, InvariantDouble};
use std::fmt;

/// A rectangle in docking coordinates.
#[derive(Clone, Copy, Debug, Default)]
pub struct DockRect {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Width, retained without validation or normalization.
    pub width: f64,
    /// Height, retained without validation or normalization.
    pub height: f64,
}

impl DockRect {
    /// Initializes a rectangle with the supplied coordinates and dimensions.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl PartialEq for DockRect {
    fn eq(&self, other: &Self) -> bool {
        double_equals(self.x, other.x)
            && double_equals(self.y, other.y)
            && double_equals(self.width, other.width)
            && double_equals(self.height, other.height)
    }
}

impl fmt::Display for DockRect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}, {}, {}, {}",
            InvariantDouble(self.x),
            InvariantDouble(self.y),
            InvariantDouble(self.width),
            InvariantDouble(self.height)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_preserve_coordinates_and_mutable_value_semantics() {
        let rectangle = DockRect::new(-1.5, 2.25, -3.0, 4.0);
        assert_eq!(
            (rectangle.x, rectangle.y, rectangle.width, rectangle.height),
            (-1.5, 2.25, -3.0, 4.0)
        );
        assert_eq!(rectangle.to_string(), "-1.5, 2.25, -3, 4");
        assert_eq!(DockRect::default(), DockRect::new(0.0, 0.0, 0.0, 0.0));
        let mut copy = rectangle;
        copy.x = 5.0;
        copy.y = 6.0;
        copy.width = 7.0;
        copy.height = 8.0;
        assert_eq!(rectangle, DockRect::new(-1.5, 2.25, -3.0, 4.0));
        assert_eq!(copy, DockRect::new(5.0, 6.0, 7.0, 8.0));
    }

    #[test]
    fn value_equality_uses_double_equals_for_all_four_components() {
        let nan = f64::from_bits(0x7ff8_0000_0000_0001);
        let rectangle = DockRect::new(nan, f64::NAN, -0.0, f64::INFINITY);
        assert_eq!(rectangle, DockRect::new(f64::NAN, nan, 0.0, f64::INFINITY));
        assert_ne!(
            rectangle,
            DockRect::new(f64::NAN, nan, 0.0, f64::NEG_INFINITY)
        );
    }

    #[test]
    fn formatting_keeps_invariant_separators_and_special_values() {
        assert_eq!(
            DockRect::new(f64::NAN, f64::NEG_INFINITY, 1e-5, -0.0).to_string(),
            "NaN, -Infinity, 1E-05, -0"
        );
    }
}
