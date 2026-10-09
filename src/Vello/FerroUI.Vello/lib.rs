//! The Vello rendering backend.
//!
//! Implements the render contracts of `ferroui_base::platform` on top of
//! the crates of the Vello project: geometries on `kurbo`, brushes and
//! blending on `peniko`, and the drawing context on a scene that one of the
//! renderers of the project draws ([`scene::IVelloSceneSink`]): `vello_cpu`
//! on the processor today, the hybrid and the compute renderer on the GPU
//! as the next stages.
//!
//! The backend is an addition of the port (the original has no such
//! backend): its files follow the Skia backend's, name by name, so that the
//! two stay comparable. The design, the facts about the renderers it rests
//! on and what is built are in `docs/porting/vello-backend.md`.

pub mod helpers;
pub mod scene;
pub mod vello_extensions;

mod combined_geometry_impl;
mod drawing_context_impl;
mod ellipse_geometry_impl;
mod font_manager_impl;
mod framebuffer_render_target;
mod geometry_group_impl;
mod geometry_impl;
mod glyph_run_impl;
mod i_drawable_bitmap_impl;
mod immutable_bitmap;
mod line_geometry_impl;
mod platform_render_interface;
mod rectangle_geometry_impl;
mod render_target_bitmap_impl;
mod stream_geometry_impl;
mod surface_render_target;
mod transformed_geometry_impl;
mod vello_application_extensions;
mod vello_backend_context;
mod vello_options;
mod vello_platform;
mod vello_region_impl;
mod vello_typeface;
mod writeable_bitmap_impl;

pub use combined_geometry_impl::CombinedGeometryImpl;
pub use drawing_context_impl::{CreateInfo, DrawingContextImpl};
pub use ellipse_geometry_impl::EllipseGeometryImpl;
pub use font_manager_impl::FontManagerImpl;
pub use framebuffer_render_target::FramebufferRenderTarget;
pub use geometry_group_impl::GeometryGroupImpl;
pub use geometry_impl::{try_get_geometry_impl, FillPath, GeometryImpl, GeometryImplBase, VelloPath};
pub use glyph_run_impl::GlyphRunImpl;
pub use i_drawable_bitmap_impl::{try_get_drawable_bitmap, IDrawableBitmapImpl};
pub use immutable_bitmap::ImmutableBitmap;
pub use line_geometry_impl::LineGeometryImpl;
pub use platform_render_interface::PlatformRenderInterface;
pub use rectangle_geometry_impl::RectangleGeometryImpl;
pub use render_target_bitmap_impl::RenderTargetBitmapImpl;
pub use stream_geometry_impl::StreamGeometryImpl;
pub use surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
pub use transformed_geometry_impl::TransformedGeometryImpl;
pub use vello_application_extensions::VelloApplicationExtensions;
pub use vello_backend_context::VelloContext;
pub use vello_options::{VelloOptions, VelloRenderingMode};
pub use vello_platform::VelloPlatform;
pub use vello_region_impl::VelloRegionImpl;
pub use vello_typeface::{bold_simulation_outline_width, VelloFontFace, VelloTypeface, OBLIQUE_SKEW};
pub use writeable_bitmap_impl::WriteableBitmapImpl;

#[cfg(test)]
mod effect_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod unit_tests;
