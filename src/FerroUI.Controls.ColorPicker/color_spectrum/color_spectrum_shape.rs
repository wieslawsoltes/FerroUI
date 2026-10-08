// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

/// Defines the shape of a [`ColorSpectrum`](crate::primitives::ColorSpectrum).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorSpectrumShape {
    /// The spectrum is in the shape of a rectangular or square box.
    /// Note that more colors are visible to the user in Box shape.
    Box = 0,

    /// The spectrum is in the shape of an ellipse or circle.
    Ring = 1,
}
