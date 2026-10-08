use super::surfaces::IPlatformRenderSurface;
use super::{
    AlphaFormat, IBitmapImpl, IDrawingContextLayerImpl, IGeometryImpl, IOptionalFeatureProvider,
    IPlatformGraphicsContext, IPlatformRenderInterfaceRegion, IRenderTarget, IRenderTargetBitmapImpl,
    IStreamGeometryImpl, IWriteableBitmapImpl, PixelFormat,
};
use crate::media::imaging::BitmapInterpolationMode;
use crate::media::{FillRule, GeometryCombineMode};
use crate::{PixelSize, Point, Rect, Vector};
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;

/// Defines the main platform-specific interface for the rendering subsystem.
///
/// This is the contract a rendering backend implements; it is registered in
/// the service locator and everything that needs platform rendering objects
/// goes through it.
pub trait IPlatformRenderInterface: 'static {
    /// Creates an ellipse geometry implementation.
    fn create_ellipse_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl>;

    /// Creates a line geometry implementation.
    fn create_line_geometry(&self, p1: Point, p2: Point) -> Arc<dyn IGeometryImpl>;

    /// Creates a rectangle geometry implementation.
    fn create_rectangle_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl>;

    /// Creates a stream geometry implementation.
    fn create_stream_geometry(&self) -> Arc<dyn IStreamGeometryImpl>;

    /// Creates a geometry group implementation.
    fn create_geometry_group(&self, fill_rule: FillRule, children: &[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl>;

    /// Creates a geometry that combines two geometries.
    fn create_combined_geometry(
        &self,
        combine_mode: GeometryCombineMode,
        g1: Arc<dyn IGeometryImpl>,
        g2: Arc<dyn IGeometryImpl>,
    ) -> Arc<dyn IGeometryImpl>;

    /// Builds a glyph run geometry.
    fn build_glyph_run_geometry(&self, glyph_run: &crate::media::GlyphRun) -> Arc<dyn IGeometryImpl>;

    /// Creates a platform implementation of a glyph run.
    ///
    /// * `glyph_typeface` — the glyph typeface.
    /// * `font_rendering_em_size` — the font rendering em size.
    /// * `glyph_infos` — the list of glyphs.
    /// * `baseline_origin` — the baseline origin of the run. Can be zero.
    fn create_glyph_run(
        &self,
        glyph_typeface: &Rc<crate::media::GlyphTypeface>,
        font_rendering_em_size: f64,
        glyph_infos: &[crate::media::text_formatting::GlyphInfo],
        baseline_origin: Point,
    ) -> Rc<dyn super::IGlyphRunImpl>;

    /// Creates a render target bitmap.
    ///
    /// `dpi` is the DPI of the bitmap; 96 means one pixel per logical unit.
    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> Rc<dyn IRenderTargetBitmapImpl>;

    /// Creates a writeable bitmap.
    fn create_writeable_bitmap(
        &self,
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha_format: AlphaFormat,
    ) -> Rc<dyn IWriteableBitmapImpl>;

    /// Loads a bitmap from a file.
    fn load_bitmap_from_file(&self, file_name: &str) -> std::io::Result<Rc<dyn IBitmapImpl>>;

    /// Loads a bitmap from a stream.
    fn load_bitmap(&self, stream: &mut dyn Read) -> std::io::Result<Rc<dyn IBitmapImpl>>;

    /// Loads a writeable bitmap from a stream, decoded to the given width.
    fn load_writeable_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>>;

    /// Loads a writeable bitmap from a stream, decoded to the given height.
    fn load_writeable_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>>;

    /// Loads a writeable bitmap from a file.
    fn load_writeable_bitmap_from_file(&self, file_name: &str) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>>;

    /// Loads a writeable bitmap from a stream.
    fn load_writeable_bitmap(&self, stream: &mut dyn Read) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>>;

    /// Loads a bitmap from a stream, decoded to the given width.
    fn load_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IBitmapImpl>>;

    /// Loads a bitmap from a stream, decoded to the given height.
    fn load_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IBitmapImpl>>;

    /// Creates a resized copy of a bitmap.
    fn resize_bitmap(
        &self,
        bitmap_impl: &dyn IBitmapImpl,
        destination_size: PixelSize,
        interpolation_mode: BitmapInterpolationMode,
    ) -> Rc<dyn IBitmapImpl>;

    /// Loads a bitmap from raw pixel data.
    ///
    /// `data` holds `stride * size.height` bytes in the given formats.
    fn load_bitmap_from_pixels(
        &self,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
        size: PixelSize,
        dpi: Vector,
        stride: i32,
    ) -> Rc<dyn IBitmapImpl>;

    /// Creates a backend-specific render context from a platform graphics
    /// context (`None` selects software rendering).
    fn create_backend_context(
        &self,
        graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext>;

    /// Whether the backend supports individual corner radii on rounded
    /// rectangles.
    fn supports_individual_round_rects(&self) -> bool;

    /// The default alpha format of the backend.
    fn default_alpha_format(&self) -> AlphaFormat;

    /// The default pixel format of the backend.
    fn default_pixel_format(&self) -> PixelFormat;

    /// Whether bitmaps can be created in the given pixel format.
    fn is_supported_bitmap_pixel_format(&self, format: PixelFormat) -> bool;

    /// Whether the backend supports regions.
    fn supports_regions(&self) -> bool;

    /// Creates an empty region.
    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion>;
}

/// A render backend bound to a graphics context (or to software rendering).
pub trait IPlatformRenderInterfaceContext: IOptionalFeatureProvider {
    /// Creates a render target for one of the given surfaces.
    fn create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget>;

    /// Creates an offscreen render target.
    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        scaling: Vector,
        enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl>;

    /// Whether the context is lost and must be recreated.
    fn is_lost(&self) -> bool;

    /// The largest offscreen render target the context can create, if
    /// limited.
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize>;

    /// The features of the context that are exposed to user code, by type.
    fn public_features(&self) -> std::collections::HashMap<std::any::TypeId, Rc<dyn std::any::Any>> {
        std::collections::HashMap::new()
    }

    /// Whether a render target can be created for the surfaces right now.
    fn is_ready_to_create_render_target(&self, _surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> bool {
        true
    }

    /// Releases the context.
    fn dispose(&self);
}
