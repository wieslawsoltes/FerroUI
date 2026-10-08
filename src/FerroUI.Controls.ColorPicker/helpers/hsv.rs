// Portions of this source file are adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use super::rgb::Rgb;
use ferroui_base::media::{Color, HsvColor};

/// Contains and allows modification of Hue, Saturation and Value components.
///
/// The is a specialized struct optimized for performance and memory:
///
/// - This is not a read-only struct like [`HsvColor`] and allows editing the fields
/// - Removes the alpha component unnecessary in core calculations
/// - No component bounds checks or clamping is done.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Hsv {
    /// The Hue component in the range from 0..359.
    pub h: f64,

    /// The Saturation component in the range from 0..1.
    pub s: f64,

    /// The Value component in the range from 0..1.
    pub v: f64,
}

impl Hsv {
    /// Initializes a new instance of the [`Hsv`] struct.
    ///
    /// `h` is the Hue component in the range from 0..360, `s` the Saturation
    /// component in the range from 0..1 and `v` the Value component in the
    /// range from 0..1.
    pub fn new(h: f64, s: f64, v: f64) -> Self {
        Self { h, s, v }
    }

    /// Initializes a new instance of the [`Hsv`] struct from an existing
    /// [`HsvColor`].
    pub fn from_hsv_color(hsv_color: HsvColor) -> Self {
        Self { h: hsv_color.h, s: hsv_color.s, v: hsv_color.v }
    }

    /// Converts this [`Hsv`] struct into a standard [`HsvColor`].
    ///
    /// `alpha` is the Alpha component in the range from 0..1 (upstream's
    /// default is 1.0).
    pub fn to_hsv_color(&self, alpha: f64) -> HsvColor {
        // Clamping is done automatically in the constructor
        HsvColor::from_ahsv(alpha, self.h, self.s, self.v)
    }

    /// Returns the [`Rgb`] color model equivalent of this [`Hsv`] color.
    pub fn to_rgb(&self) -> Rgb {
        // Instantiating a Color is unfortunately necessary to use existing conversions
        // Clamping is done internally in the conversion method
        let color: Color = HsvColor::hsv_to_rgb(self.h, self.s, self.v, 1.0);

        Rgb::from_color(color)
    }
}
