//! ferroui-controls-flip-view
//!
//! `FlipView` of WinUI on the `Carousel` of FerroUI: one item at a time, with the previous and
//! next buttons that show while the pointer moves over the control. The control theme is part
//! of this crate: an application that uses the control includes it, and one that does not links
//! none of this crate:
//!
//! ```xml
//! <StyleInclude Source="ferres://FerroUI.Controls.FlipView/Themes/WinUI/WinUI.xaml" />
//! ```

// The compiled markup names the types of the crate by the name of the crate.
extern crate self as ferroui_controls_flip_view;

// `compiled_xaml` (the compiled markup of the theme documents) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(any(test, feature = "document-assets"))]
mod assets;
mod flip_view;
mod register_types;
mod rust_paths;

pub use flip_view::FlipView;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
