use crate::{Matrix, Point, Rect};
use std::fmt;

/// Holds information about the bounds of a control, together with a
/// transform and a clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformedBounds {
    /// The control's bounds.
    pub bounds: Rect,
    /// The clip rectangle, in global coordinates.
    pub clip: Rect,
    /// The transform that maps the control's bounds to global coordinates.
    pub transform: Matrix,
}

impl TransformedBounds {
    /// Creates transformed bounds.
    pub const fn new(bounds: Rect, clip: Rect, transform: Matrix) -> Self {
        Self { bounds, clip, transform }
    }

    /// Whether the transformed bounds contain a point given in global
    /// coordinates.
    pub fn contains(&self, point: Point) -> bool {
        if self.transform.has_inverse() {
            let tr_point = point * self.transform.invert();
            self.bounds.contains(tr_point)
        } else {
            self.bounds.contains(point)
        }
    }
}

impl fmt::Display for TransformedBounds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bounds: {} Clip: {} Transform {}", self.bounds, self.clip, self.transform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_uses_inverse_transform() {
        let bounds = TransformedBounds::new(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Matrix::create_translation(50.0, 50.0),
        );

        assert!(bounds.contains(Point::new(55.0, 55.0)));
        assert!(!bounds.contains(Point::new(5.0, 5.0)));
    }
}
