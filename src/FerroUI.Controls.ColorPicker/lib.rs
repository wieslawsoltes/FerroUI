//! ferroui-controls-color-picker
//!
//! The colour picker library: the drop-down [`ColorPicker`], the
//! [`ColorView`] it presents, the primitives they are built from
//! ([`ColorSpectrum`](primitives::ColorSpectrum),
//! [`ColorSlider`](primitives::ColorSlider),
//! [`ColorPreviewer`](primitives::ColorPreviewer)), the colour palettes, the
//! value converters of their control themes and the automation peer of the
//! spectrum.
//!
//! The control themes are not part of the base themes: an application
//! includes the styles of the library for its theme, as upstream:
//!
//! ```xml
//! <StyleInclude Source="ferres://FerroUI.Controls.ColorPicker/Themes/Fluent/Fluent.xaml" />
//! ```
//!
//! The theme documents are embedded as assets of the assembly
//! `FerroUI.Controls.ColorPicker`, registered by [`register_types`].

mod alpha_component_position;
mod assets;
pub mod automation;
mod color_changed_event_args;
mod color_component;
mod color_model;
mod color_palettes;
mod color_picker;
mod color_previewer;
mod color_slider;
mod color_spectrum;
mod color_view;
pub mod converters;
mod helpers;
mod hsv_component;
mod markup_types;
mod register_types;
mod rgb_component;
mod rust_paths;

pub use alpha_component_position::AlphaComponentPosition;
pub use color_changed_event_args::ColorChangedEventArgs;
pub use color_component::ColorComponent;
pub use color_model::ColorModel;
pub use color_palettes::{
    FlatColor, FlatColorPalette, FlatHalfColorPalette, FluentColorPalette, IColorPalette, MaterialColor,
    MaterialColorPalette, MaterialHalfColorPalette, SixteenColorPalette,
};
pub use color_picker::ColorPicker;
pub use color_spectrum::{ColorSpectrumComponents, ColorSpectrumShape};
pub use color_view::{ColorView, ColorViewImpl, ColorViewTab};
pub use hsv_component::HsvComponent;
pub use register_types::{register_types, ASSEMBLY};
pub use rgb_component::RgbComponent;

/// The primitives of the colour picker (namespace `FerroUI.Controls.Primitives`).
pub mod primitives {
    pub use crate::color_previewer::{ColorPreviewer, ColorPreviewerImpl};
    pub use crate::color_slider::{ColorSlider, ColorSliderImpl};
    pub use crate::color_spectrum::ColorSpectrum;
    pub use crate::helpers::ColorHelper;

    /// The converters of the primitives (namespace
    /// `FerroUI.Controls.Primitives.Converters`).
    pub mod converters {
        pub use crate::converters::accent_color_converter::AccentColorConverter;
        pub use crate::converters::contrast_brush_converter::ContrastBrushConverter;
    }
}

#[cfg(test)]
mod tests;
