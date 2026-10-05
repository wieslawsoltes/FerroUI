//! ferroui-themes-fluent
//!
//! The Fluent theme: the control themes of the control set, the colour
//! palettes and the resources they use, in a light and a dark variant and
//! in a normal and a compact density.
//!
//! The theme is markup: `FluentTheme.xaml` (the document of the
//! [`FluentTheme`] class), `Accents/*.xaml` (palette, resources and
//! brushes), `DensityStyles/Compact.xaml`, `Strings/InvariantResources.xaml`,
//! `Controls/FluentControls.xaml` (the list of the control themes) and one
//! document per control under `Controls/`. The documents are embedded in
//! the crate as assets of the assembly `FerroUI.Themes.Fluent` and
//! addressable as `ferres://FerroUI.Themes.Fluent/<path>`.
//!
//! ```ignore
//! application.styles().add(FluentTheme::new().as_style());
//! ```

pub mod accents;
mod assets;
mod color_palette_resources;
mod color_palette_resources_collection;
mod fluent_theme;
mod register_types;

pub use assets::{excluded_documents, ExcludedDocument};
pub use color_palette_resources::ColorPaletteResources;
pub use color_palette_resources_collection::ColorPaletteResourcesCollection;
pub use fluent_theme::{DensityStyle, FluentTheme};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
