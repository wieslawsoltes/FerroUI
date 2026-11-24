//! SVG export through the Skia implementation of FerroUI's drawing contracts.
//!
//! Record a frame or named, isolated layers once with [`SvgDocument`], then export normal,
//! wireframe, x-ray, cutaway or exploded views without repeating layout and text shaping.
//! The default preserves shaped text as paths and rasterizes an entire affected layer when
//! Skia's SVG writer cannot faithfully represent its operations. [`SvgFidelity::VectorOnly`]
//! instead returns an error. Every fallback is reported in [`SvgExportReport`].
//!
//! This is an export target, not a window-system backend. Geometry, fonts and images must
//! come from the Skia services used by the application. See the crate README and
//! `docs/porting/svg-backend.md` for the feature matrix and immediate-renderer boundary.

mod context;
mod document;
mod error;
mod options;
mod output;
mod platform;
mod render;
mod render_target;
mod surface;
mod xml;

pub use context::{RecordingAudit, SvgDrawingContext};
pub use document::{SvgDocument, SvgLayer, SvgLayerId, SvgLayerOptions};
pub use error::SvgError;
pub use options::{
    SvgCutaway, SvgCutawayMode, SvgExplodedView, SvgExportOptions, SvgFidelity, SvgInspection,
    SvgLayerFilter, SvgTextMode, SvgUnits, SvgView,
};
pub use output::{SvgDiagnostic, SvgExportReport, SvgOutput};
pub use platform::{SvgApplicationExtensions, SvgPlatform, SvgPlatformRenderInterface};
pub use render_target::SvgRenderTarget;
pub use surface::{SvgSurface, SvgSurfaceFrame};

#[cfg(test)]
mod cache_tests;
#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;
