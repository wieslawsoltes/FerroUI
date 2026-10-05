use crate::media::{Stretch, StretchDirection};
use crate::utilities::MathUtilities;
use crate::{Size, Vector};

/// Provides extension methods for media types.
pub struct MediaExtensions;

impl MediaExtensions {
    /// Calculates scaling based on a [`Stretch`] value.
    ///
    /// * `stretch` - The stretch mode.
    /// * `destination_size` - The size of the destination viewport.
    /// * `source_size` - The size of the source.
    /// * `stretch_direction` - The stretch direction (upstream default:
    ///   [`StretchDirection::Both`]).
    ///
    /// Returns a vector with the X and Y scaling factors.
    pub fn calculate_scaling(
        stretch: Stretch,
        destination_size: Size,
        source_size: Size,
        stretch_direction: StretchDirection,
    ) -> Vector {
        let mut scale_x = 1.0;
        let mut scale_y = 1.0;

        let is_constrained_width = destination_size.width != f64::INFINITY;
        let is_constrained_height = destination_size.height != f64::INFINITY;

        if (stretch == Stretch::Uniform || stretch == Stretch::UniformToFill || stretch == Stretch::Fill)
            && (is_constrained_width || is_constrained_height)
        {
            // Compute scaling factors for both axes
            scale_x =
                if MathUtilities::is_zero(source_size.width) { 0.0 } else { destination_size.width / source_size.width };
            scale_y = if MathUtilities::is_zero(source_size.height) {
                0.0
            } else {
                destination_size.height / source_size.height
            };

            if !is_constrained_width {
                scale_x = scale_y;
            } else if !is_constrained_height {
                scale_y = scale_x;
            } else {
                // If not preserving aspect ratio, then just apply transform to fit
                match stretch {
                    Stretch::Uniform => {
                        // Find minimum scale that we use for both axes
                        let minscale = if scale_x < scale_y { scale_x } else { scale_y };
                        scale_x = minscale;
                        scale_y = minscale;
                    }
                    Stretch::UniformToFill => {
                        // Find maximum scale that we use for both axes
                        let maxscale = if scale_x > scale_y { scale_x } else { scale_y };
                        scale_x = maxscale;
                        scale_y = maxscale;
                    }
                    Stretch::Fill => {
                        // We already computed the fill scale factors above, so just use them
                    }
                    Stretch::None => {}
                }
            }

            // Apply stretch direction by bounding scales.
            // In the uniform case, scale_x=scale_y, so this sort of clamping will maintain aspect ratio
            // In the uniform fill case, we have the same result too.
            // In the fill case, note that we change aspect ratio, but that is okay
            match stretch_direction {
                StretchDirection::UpOnly => {
                    if scale_x < 1.0 {
                        scale_x = 1.0;
                    }
                    if scale_y < 1.0 {
                        scale_y = 1.0;
                    }
                }
                StretchDirection::DownOnly => {
                    if scale_x > 1.0 {
                        scale_x = 1.0;
                    }
                    if scale_y > 1.0 {
                        scale_y = 1.0;
                    }
                }
                StretchDirection::Both => {}
            }
        }

        Vector::new(scale_x, scale_y)
    }

    /// Calculates a scaled size based on a [`Stretch`] value.
    ///
    /// * `stretch` - The stretch mode.
    /// * `destination_size` - The size of the destination viewport.
    /// * `source_size` - The size of the source.
    /// * `stretch_direction` - The stretch direction (upstream default:
    ///   [`StretchDirection::Both`]).
    ///
    /// Returns the size of the stretched source.
    pub fn calculate_size(
        stretch: Stretch,
        destination_size: Size,
        source_size: Size,
        stretch_direction: StretchDirection,
    ) -> Size {
        source_size * Self::calculate_scaling(stretch, destination_size, source_size, stretch_direction)
    }
}

// Upstream has no dedicated tests for these helpers; the tests below are not upstream tests.
// The expected values are computed by hand from the upstream algorithm.
#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: StretchDirection = StretchDirection::Both;

    #[test]
    fn none_never_scales() {
        let scale = MediaExtensions::calculate_scaling(Stretch::None, Size::new(200.0, 50.0), Size::new(100.0, 100.0), BOTH);
        assert_eq!(Vector::new(1.0, 1.0), scale);
    }

    #[test]
    fn fill_scales_each_axis_independently() {
        let scale = MediaExtensions::calculate_scaling(Stretch::Fill, Size::new(200.0, 50.0), Size::new(100.0, 100.0), BOTH);
        assert_eq!(Vector::new(2.0, 0.5), scale);
    }

    #[test]
    fn uniform_uses_the_smaller_scale() {
        let scale =
            MediaExtensions::calculate_scaling(Stretch::Uniform, Size::new(200.0, 50.0), Size::new(100.0, 100.0), BOTH);
        assert_eq!(Vector::new(0.5, 0.5), scale);
    }

    #[test]
    fn uniform_to_fill_uses_the_larger_scale() {
        let scale = MediaExtensions::calculate_scaling(
            Stretch::UniformToFill,
            Size::new(200.0, 50.0),
            Size::new(100.0, 100.0),
            BOTH,
        );
        assert_eq!(Vector::new(2.0, 2.0), scale);
    }

    #[test]
    fn unconstrained_axis_copies_the_other_scale() {
        let source = Size::new(100.0, 50.0);
        for stretch in [Stretch::Fill, Stretch::Uniform, Stretch::UniformToFill] {
            assert_eq!(
                Vector::new(3.0, 3.0),
                MediaExtensions::calculate_scaling(stretch, Size::new(f64::INFINITY, 150.0), source, BOTH)
            );
            assert_eq!(
                Vector::new(4.0, 4.0),
                MediaExtensions::calculate_scaling(stretch, Size::new(400.0, f64::INFINITY), source, BOTH)
            );
            assert_eq!(
                Vector::new(1.0, 1.0),
                MediaExtensions::calculate_scaling(stretch, Size::INFINITY, source, BOTH)
            );
        }
    }

    #[test]
    fn zero_source_gives_zero_scale() {
        let scale = MediaExtensions::calculate_scaling(Stretch::Fill, Size::new(200.0, 50.0), Size::new(0.0, 100.0), BOTH);
        assert_eq!(Vector::new(0.0, 0.5), scale);

        let scale =
            MediaExtensions::calculate_scaling(Stretch::Uniform, Size::new(200.0, 50.0), Size::new(100.0, 0.0), BOTH);
        assert_eq!(Vector::new(0.0, 0.0), scale);
    }

    #[test]
    fn stretch_direction_bounds_the_scales() {
        let destination = Size::new(200.0, 50.0);
        let source = Size::new(100.0, 100.0);

        assert_eq!(
            Vector::new(2.0, 1.0),
            MediaExtensions::calculate_scaling(Stretch::Fill, destination, source, StretchDirection::UpOnly)
        );
        assert_eq!(
            Vector::new(1.0, 0.5),
            MediaExtensions::calculate_scaling(Stretch::Fill, destination, source, StretchDirection::DownOnly)
        );
        assert_eq!(
            Vector::new(1.0, 1.0),
            MediaExtensions::calculate_scaling(Stretch::Uniform, destination, source, StretchDirection::UpOnly)
        );
        assert_eq!(
            Vector::new(1.0, 1.0),
            MediaExtensions::calculate_scaling(Stretch::UniformToFill, destination, source, StretchDirection::DownOnly)
        );
    }

    #[test]
    fn calculate_size_scales_the_source() {
        let destination = Size::new(200.0, 50.0);
        let source = Size::new(100.0, 80.0);

        assert_eq!(source, MediaExtensions::calculate_size(Stretch::None, destination, source, BOTH));
        assert_eq!(destination, MediaExtensions::calculate_size(Stretch::Fill, destination, source, BOTH));
        assert_eq!(Size::new(62.5, 50.0), MediaExtensions::calculate_size(Stretch::Uniform, destination, source, BOTH));
        assert_eq!(
            Size::new(200.0, 160.0),
            MediaExtensions::calculate_size(Stretch::UniformToFill, destination, source, BOTH)
        );
    }
}
