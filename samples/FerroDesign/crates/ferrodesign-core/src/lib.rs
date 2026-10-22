//! Renderer-independent vector-design document and editing engine.
//! Logical positions are parent-relative; rotation is about each node's center.
mod editor;
mod effects_edit;
mod flatten;
mod components;
mod corners;
mod compositing;
mod geometry;
mod grid_tracks;
mod image_edit;
mod image_info;
mod layout;
mod layout_guides;
mod layout_edit;
mod layout_drag;
mod model;
mod prototype;
mod paragraphs;
mod polygon_corners;
mod seed;
mod selection;
mod spatial;
mod style_edit;
mod stroke_profile;
mod stroke_edit;
mod stroke_hit;
mod svg;
mod svg_import;
mod text_edit;
mod typography;
mod variable_edit;
mod vector_edit;
pub use editor::*;
pub use effects_edit::*;
pub use flatten::*;
pub use components::*;
pub use corners::*;
pub use compositing::*;
pub use geometry::*;
pub use grid_tracks::*;
pub use image_edit::*;
pub use image_info::*;
pub use layout::*;
pub use layout_guides::*;
pub use layout_drag::*;
pub use model::*;
pub use prototype::*;
pub use paragraphs::{TextParagraph, ParagraphStylePatch, text_paragraph_ranges, validate_text_paragraphs};
pub use polygon_corners::*;
pub use seed::*;
pub use selection::*;
pub use spatial::*;
pub use style_edit::*;
pub use stroke_profile::*;
pub use svg::*;
pub use svg_import::*;
pub use text_edit::*;
pub use typography::*;
pub use variable_edit::*;
pub use vector_edit::*;
#[cfg(test)]
mod svg_import_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vector_edit_tests;
#[cfg(test)]
mod angular_gradient_tests;
#[cfg(test)]
mod text_preview_tests;
