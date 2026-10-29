//! The Skia rendering backend.
//!
//! Implements the render contracts of `ferroui_base::platform` on top of
//! Skia: geometries, bitmaps, regions, the drawing context and the render
//! targets (software framebuffers and GPU surfaces).
//!
//! GPU rendering goes through the [`gpu::ISkiaGpu`] abstraction. The Metal
//! implementation (`gpu::metal::SkiaMetalGpu`, Apple platforms) runs on
//! Skia's Graphite backend, the OpenGL one (`gpu::open_gl::GlSkiaGpu`, the
//! browser) on Ganesh.

pub mod gpu;
pub mod helpers;
pub mod metal;

mod combined_geometry_impl;
mod drawing_context_impl;
mod ellipse_geometry_impl;
mod font_manager_impl;
mod framebuffer_render_target;
mod geometry_group_impl;
mod geometry_impl;
mod glyph_run_impl;
mod i_drawable_bitmap_impl;
mod i_skia_api_lease_feature;
mod immutable_bitmap;
mod line_geometry_impl;
mod locked_framebuffer;
mod picture_render_target;
mod platform_render_interface;
mod rectangle_geometry_impl;
mod render_target_bitmap_impl;
mod sk_cache_base;
mod sk_paint_cache;
mod sk_round_rect_cache;
mod sk_text_blob_builder_cache;
mod skia_backend_context;
mod skia_application_extensions;
mod skia_options;
mod skia_platform;
mod skia_region_impl;
mod skia_typeface;
mod simple_geometry_impl;
pub mod skia_sharp_extensions;
mod stream_geometry_builder;
mod surface_render_target;
mod text_rendering_override;
mod text_rendering_settings;
mod transformed_geometry_impl;
mod two_level_cache;
mod writeable_bitmap_impl;

pub use combined_geometry_impl::CombinedGeometryImpl;
pub use drawing_context_impl::{CanvasSource, CreateInfo, DrawingContextImpl};
pub use ellipse_geometry_impl::EllipseGeometryImpl;
pub use font_manager_impl::FontManagerImpl;
pub use framebuffer_render_target::FramebufferRenderTarget;
pub use geometry_group_impl::GeometryGroupImpl;
pub use geometry_impl::{try_get_geometry_impl, GeometryImpl, GeometryImplBase};
pub use glyph_run_impl::GlyphRunImpl;
pub use i_drawable_bitmap_impl::{try_get_drawable_bitmap, IDrawableBitmapImpl};
pub use i_skia_api_lease_feature::{ISkiaApiLease, ISkiaApiLeaseFeature, ISkiaPlatformGraphicsApiLease};
pub use immutable_bitmap::ImmutableBitmap;
pub use line_geometry_impl::LineGeometryImpl;
pub use locked_framebuffer::LockedFramebuffer;
pub use picture_render_target::PictureRenderTarget;
pub use platform_render_interface::PlatformRenderInterface;
pub use rectangle_geometry_impl::RectangleGeometryImpl;
pub use render_target_bitmap_impl::RenderTargetBitmapImpl;
pub use sk_cache_base::SkCacheBase;
pub use sk_paint_cache::SkPaintCache;
pub use sk_round_rect_cache::SkRoundRectCache;
pub use sk_text_blob_builder_cache::SkTextBlobBuilderCache;
pub use skia_backend_context::SkiaContext;
pub use skia_application_extensions::SkiaApplicationExtensions;
pub use skia_options::SkiaOptions;
pub use text_rendering_override::{
    set_text_rendering_override, startup_text_rendering_override, text_rendering_override, TextRenderingOverride,
    TEXT_OVERRIDE_ENV_PREFIX, TEXT_OVERRIDE_KEYS,
};
pub use text_rendering_settings::{TextRenderingSettings, TextRenderingSource, GREYSCALE_TEXT_SCALING};
pub use skia_platform::SkiaPlatform;
pub use skia_region_impl::SkiaRegionImpl;
pub use skia_typeface::SkiaTypeface;
pub use simple_geometry_impl::SimpleGeometryImpl;
pub use stream_geometry_builder::StreamGeometryBuilder;
pub use surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
pub use transformed_geometry_impl::TransformedGeometryImpl;
pub use two_level_cache::TwoLevelCache;
pub use writeable_bitmap_impl::WriteableBitmapImpl;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod compositing_tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod text_rendering_tests;
#[cfg(test)]
mod unit_tests;
