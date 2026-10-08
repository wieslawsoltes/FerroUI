// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

/// Defines the two HSV color components displayed by a
/// [`ColorSpectrum`](crate::primitives::ColorSpectrum).
///
/// Order of the color components is important and correspond with an X/Y axis in Box
/// shape or a degree/radius in Ring shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorSpectrumComponents {
    /// The Hue and Value components.
    ///
    /// In Box shape, Hue is mapped to the X-axis and Value is mapped to the Y-axis.
    /// In Ring shape, Hue is mapped to degrees and Value is mapped to radius.
    HueValue = 0,

    /// The Value and Hue components.
    ///
    /// In Box shape, Value is mapped to the X-axis and Hue is mapped to the Y-axis.
    /// In Ring shape, Value is mapped to degrees and Hue is mapped to radius.
    ValueHue = 1,

    /// The Hue and Saturation components.
    ///
    /// In Box shape, Hue is mapped to the X-axis and Saturation is mapped to the Y-axis.
    /// In Ring shape, Hue is mapped to degrees and Saturation is mapped to radius.
    HueSaturation = 2,

    /// The Saturation and Hue components.
    ///
    /// In Box shape, Saturation is mapped to the X-axis and Hue is mapped to the Y-axis.
    /// In Ring shape, Saturation is mapped to degrees and Hue is mapped to radius.
    SaturationHue = 3,

    /// The Saturation and Value components.
    ///
    /// In Box shape, Saturation is mapped to the X-axis and Value is mapped to the Y-axis.
    /// In Ring shape, Saturation is mapped to degrees and Value is mapped to radius.
    SaturationValue = 4,

    /// The Value and Saturation components.
    ///
    /// In Box shape, Value is mapped to the X-axis and Saturation is mapped to the Y-axis.
    /// In Ring shape, Value is mapped to degrees and Saturation is mapped to radius.
    ValueSaturation = 5,
}
