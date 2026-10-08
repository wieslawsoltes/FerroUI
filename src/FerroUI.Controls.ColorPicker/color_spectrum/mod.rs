//! `ColorSpectrum/` of the upstream project: the spectrum (namespace
//! `FerroUI.Controls.Primitives`) and its enumerations (namespace
//! `FerroUI.Controls`).

#[allow(clippy::module_inception)]
mod color_spectrum;
mod color_spectrum_components;
mod color_spectrum_properties;
mod color_spectrum_shape;

pub use color_spectrum::ColorSpectrum;
pub use color_spectrum_components::ColorSpectrumComponents;
pub use color_spectrum_shape::ColorSpectrumShape;
