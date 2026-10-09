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
//! The theme documents are compiled by the build of the crate (`build.rs`):
//! [`register_types`] registers the loader table of the compiled markup,
//! which answers a load of a document of the assembly
//! `FerroUI.Controls.ColorPicker` by its URI, and the build of an application
//! whose markup is compiled links an include of the styles to their compiled
//! markup. A compiled document is not an asset of the assembly, as upstream's
//! compiler removes a compiled resource; with the feature `document-assets`
//! the documents are embedded as assets too, which is what lets a document
//! loaded at run time include them (`build.rs` says why). The crate does not
//! link the run-time XAML loader.

// The compiled markup names the types of the crate by the name of the crate, as the markup
// of any other crate does.
extern crate self as ferroui_controls_color_picker;

// `compiled_xaml` (the compiled markup of the theme documents) and `compiled_markup` (the
// loader table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

mod alpha_component_position;
#[cfg(any(test, feature = "document-assets"))]
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
