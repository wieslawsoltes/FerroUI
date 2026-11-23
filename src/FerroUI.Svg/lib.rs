//! SVG images for FerroUI (#1260).
//!
//! An SVG document is decoded into [`SvgDocument`], a tree of paths, paints and clips, and
//! converted by [`SvgDocument::to_image`] to a `DrawingImage` of geometry drawings. Nothing is
//! rasterised here: the image is drawn as vectors through the drawing context of the backend, so
//! it stays sharp at any scale and on the GPU path of that backend.
//!
//! The input is untrusted. The decoder
//!
//! - bounds the bytes, the elements (after `use` is expanded), the nesting, the path segments and
//!   the size of the image ([`SvgLimits`]), and fails with an [`SvgError`] rather than a panic;
//! - rejects a document type definition, so no entity is expanded;
//! - fetches nothing: a reference is followed only to an element of the same document, and
//!   `image`, `script`, `style`, `foreignObject` and every other element it does not draw are
//!   skipped.
//!
//! Drawn: `svg`, `g`, `a`, `switch`, `use`, `symbol`, `path`, `rect`, `circle`, `ellipse`, `line`,
//! `polyline`, `polygon`, linear and radial gradients, `clipPath`, and the presentation attributes
//! of those, as attributes or in a `style` attribute. Not drawn: text, embedded images, masks,
//! filters, patterns and markers.

mod color;
mod decoder;
mod document;
mod numbers;
mod path;
mod transform;

#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod tests;

pub use document::{SvgDocument, SvgError, SvgLimits};
