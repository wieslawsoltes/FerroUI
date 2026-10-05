//! The `glyf` (TrueType outline) table reader.

mod composite_flags;
mod composite_glyph;
mod glyf_table;
mod glyph_component;
mod glyph_decycler;
mod glyph_descriptor;
mod glyph_flag;
mod simple_glyph;

#[cfg(test)]
mod glyf_table_contour_walk_tests;
#[cfg(test)]
mod glyf_table_point_matching_tests;
#[cfg(test)]
mod glyf_table_tests;

pub use composite_flags::CompositeFlags;
pub use composite_glyph::CompositeGlyph;
pub use glyf_table::GlyfTable;
pub use glyph_component::GlyphComponent;
pub use glyph_decycler::GlyphDecycler;
pub use glyph_descriptor::GlyphDescriptor;
pub use glyph_flag::GlyphFlag;
pub use simple_glyph::SimpleGlyph;
