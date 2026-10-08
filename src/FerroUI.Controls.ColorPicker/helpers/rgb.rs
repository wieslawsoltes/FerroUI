// Portions of this source file are adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use super::hsv::Hsv;
use ferroui_base::media::{Color, HsvColor};
use ferroui_base::utilities::MathUtilities;

/// Contains and allows modification of Red, Green and Blue components.
///
/// The is a specialized struct optimized for performance and memory:
///
/// - This is not a read-only struct like [`Color`] and allows editing the fields
/// - Removes the alpha component unnecessary in core calculations
/// - Normalizes RGB components in the range of 0..1 to simplify calculations.
/// - No component bounds checks or clamping is done.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rgb {
    /// The Red component in the range from 0..1.
    pub r: f64,

    /// The Green component in the range from 0..1.
    pub g: f64,

    /// The Blue component in the range from 0..1.
    pub b: f64,
}

impl Rgb {
    /// Initializes a new instance of the [`Rgb`] struct.
    ///
    /// Each component is in the range from 0..1.
    #[allow(dead_code)] // Unused, as in the original.
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Initializes a new instance of the [`Rgb`] struct from an existing
    /// [`Color`].
    pub fn from_color(color: Color) -> Self {
        Self { r: color.r as f64 / 255.0, g: color.g as f64 / 255.0, b: color.b as f64 / 255.0 }
    }

    /// Converts this [`Rgb`] struct into a standard [`Color`].
    ///
    /// `alpha` is the Alpha component in the range from 0..1 (upstream's
    /// default is 1.0).
    pub fn to_color(&self, alpha: f64) -> Color {
        Color::from_argb(
            MathUtilities::clamp(alpha * 255.0, 0x00 as f64, 0xFF as f64) as u8,
            MathUtilities::clamp(self.r * 255.0, 0x00 as f64, 0xFF as f64) as u8,
            MathUtilities::clamp(self.g * 255.0, 0x00 as f64, 0xFF as f64) as u8,
            MathUtilities::clamp(self.b * 255.0, 0x00 as f64, 0xFF as f64) as u8,
        )
    }

    /// Returns the [`Hsv`] color model equivalent of this [`Rgb`] color.
    pub fn to_hsv(&self) -> Hsv {
        // Instantiating an HsvColor is unfortunately necessary to use existing conversions
        // Clamping must be done here as it isn't done in the conversion method (internal-use only)
        let hsv_color: HsvColor = Color::normalized_rgb_to_hsv(
            MathUtilities::clamp(self.r, 0.0, 1.0),
            MathUtilities::clamp(self.g, 0.0, 1.0),
            MathUtilities::clamp(self.b, 0.0, 1.0),
            1.0,
        );

        Hsv::from_hsv_color(hsv_color)
    }
}
