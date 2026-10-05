use crate::media::{Color, IGradientStop};
use crate::utilities::MathUtilities;

/// Describes the location and color of a transition point in a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImmutableGradientStop {
    offset: f64,
    color: Color,
}

impl ImmutableGradientStop {
    /// Creates a gradient stop. The offset is clamped to `[0, 1]`.
    pub fn new(offset: f64, color: Color) -> Self {
        let mut offset = offset;
        if MathUtilities::is_zero(offset) {
            offset = 0.0;
        }

        Self { offset: offset.clamp(0.0, 1.0), color }
    }

    /// The gradient stop offset.
    #[inline]
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// The gradient stop color.
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }
}

impl IGradientStop for ImmutableGradientStop {
    #[inline]
    fn color(&self) -> Color {
        self.color
    }

    #[inline]
    fn offset(&self) -> f64 {
        self.offset
    }

    fn is_immutable_gradient_stop(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Colors;

    #[test]
    fn offset_is_clamped() {
        assert_eq!(0.0, ImmutableGradientStop::new(-1.0, Colors::RED).offset());
        assert_eq!(1.0, ImmutableGradientStop::new(2.0, Colors::RED).offset());
        assert_eq!(0.5, ImmutableGradientStop::new(0.5, Colors::RED).offset());
        assert!(ImmutableGradientStop::new(-0.0, Colors::RED).offset().is_sign_positive());
        assert!(ImmutableGradientStop::new(f64::NAN, Colors::RED).offset().is_nan());
    }
}
