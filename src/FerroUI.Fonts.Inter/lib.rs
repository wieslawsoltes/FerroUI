//! The Inter font family, embedded.
//!
//! A platform without system fonts (the browser) has no default font; an
//! application adds this collection with
//! [`AppBuilder::with_inter_font`](ferroui_controls::AppBuilder) and names
//! the family as `fonts:Inter#Inter`.

mod app_builder_extension;
mod assets;
mod inter_font_collection;

pub use app_builder_extension::AppBuilderExtension;
pub use inter_font_collection::InterFontCollection;

/// The name of the assembly the fonts are assets of.
pub const ASSEMBLY_NAME: &str = "FerroUI.Fonts.Inter";
