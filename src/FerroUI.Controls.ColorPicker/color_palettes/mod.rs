//! The color palettes (`ColorPalettes/` of the upstream project, namespace
//! `FerroUI.Controls`).

mod flat_color_palette;
mod flat_half_color_palette;
mod fluent_color_palette;
mod i_color_palette;
mod material_color_palette;
mod material_half_color_palette;
mod sixteen_color_palette;

pub use flat_color_palette::{FlatColor, FlatColorPalette};
pub use flat_half_color_palette::FlatHalfColorPalette;
pub use fluent_color_palette::FluentColorPalette;
pub use i_color_palette::IColorPalette;
pub use material_color_palette::{MaterialColor, MaterialColorPalette};
pub use material_half_color_palette::MaterialHalfColorPalette;
pub use sixteen_color_palette::SixteenColorPalette;
