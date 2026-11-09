//! The render interface of the backend: what the base library makes its geometries, bitmaps,
//! glyph runs and backend contexts with.

use crate::bitmap::{self, unsupported_codec, ProGpuBitmap};
use crate::context::ProGpuContext;
use crate::geometry::{path_data_of, PathBuilder, PathData, ProGpuGeometry, ProGpuStreamGeometryBuilder};
use crate::glyph_run::ProGpuGlyphRun;
use crate::notes;
use crate::region::ProGpuRegion;
use ferroui_base::media::imaging::BitmapInterpolationMode;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{FillRule, GeometryCombineMode, GlyphRun, GlyphTypeface, RenderOptions};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IGeometryImpl, IGlyphRunImpl, IPlatformGraphicsContext, IPlatformRenderInterface,
    IPlatformRenderInterfaceContext, IPlatformRenderInterfaceRegion, IRenderTargetBitmapImpl, IStreamGeometryBuilder,
    IWriteableBitmapImpl, PixelFormat, SharedBitmapImpl,
};
use ferroui_base::{PixelSize, Point, Rect, Vector};
use std::io::{self, Read};
use std::rc::Rc;
use std::sync::Arc;

/// The render interface of the ProGPU backend.
///
/// Geometries are the neutral geometry module of ProGPU; frames are scenes of its engine on a
/// WebGPU device. Text (step P4 of docs/porting/progpu-backend.md) and the image codecs (step
/// P5) are not there yet: glyph runs are measured and drawn as nothing, and loading or saving
/// an image file reports `Unsupported`; each is noted in the log once.
#[derive(Default)]
pub struct ProGpuPlatformRenderInterface;

impl ProGpuPlatformRenderInterface {
    pub fn new() -> Self {
        ProGpuPlatformRenderInterface
    }
}

/// An empty geometry: what stands for a geometry the backend cannot make.
fn empty_geometry() -> Arc<dyn IGeometryImpl> {
    ProGpuGeometry::new(PathBuilder::default().finish())
}

impl IPlatformRenderInterface for ProGpuPlatformRenderInterface {
    fn create_ellipse_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        ProGpuGeometry::new(PathData::ellipse(rect))
    }

    fn create_line_geometry(&self, p1: Point, p2: Point) -> Arc<dyn IGeometryImpl> {
        ProGpuGeometry::new(PathData::line(p1, p2))
    }

    fn create_rectangle_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        ProGpuGeometry::new(PathData::rectangle(rect))
    }

    fn create_stream_geometry_builder(&self, source: Option<&dyn IGeometryImpl>) -> Box<dyn IStreamGeometryBuilder> {
        Box::new(ProGpuStreamGeometryBuilder::new(source))
    }

    fn create_geometry_group(&self, fill_rule: FillRule, children: &[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl> {
        let paths: Vec<Arc<PathData>> = children.iter().filter_map(|child| path_data_of(&**child)).collect();
        let paths: Vec<&PathData> = paths.iter().map(|path| &**path).collect();
        ProGpuGeometry::new(PathData::group(fill_rule, &paths))
    }

    fn create_combined_geometry(&self, combine_mode: GeometryCombineMode, g1: Arc<dyn IGeometryImpl>, g2: Arc<dyn IGeometryImpl>) -> Arc<dyn IGeometryImpl> {
        match (path_data_of(&*g1), path_data_of(&*g2)) {
            (Some(first), Some(second)) => match first.combine(&second, combine_mode) {
                Some(combined) => ProGpuGeometry::new(combined),
                None => {
                    notes::note_once("geometry-combine", "a combined geometry could not be made by the geometry module of ProGPU");
                    empty_geometry()
                }
            },
            _ => {
                notes::note_once("geometry-kind", "a geometry of another backend is not used by the ProGPU backend");
                empty_geometry()
            }
        }
    }

    fn build_glyph_run_geometry(&self, _glyph_run: &GlyphRun) -> Arc<dyn IGeometryImpl> {
        // Step P4 of the plan: the outlines of the text stack of ProGPU.
        notes::note_once("text-geometry", "the outlines of text are not made by the ProGPU backend yet");
        empty_geometry()
    }

    fn create_glyph_run(
        &self,
        glyph_typeface: &Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        glyph_infos: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Arc<dyn IGlyphRunImpl> {
        Arc::new(ProGpuGlyphRun::new(glyph_typeface, font_rendering_em_size, glyph_infos, baseline_origin))
    }

    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> Arc<dyn IRenderTargetBitmapImpl> {
        Arc::new(ProGpuBitmap::new(size, dpi, PixelFormat::RGBA8888, AlphaFormat::Premul))
    }

    fn create_writeable_bitmap(&self, size: PixelSize, dpi: Vector, format: PixelFormat, alpha_format: AlphaFormat) -> Arc<dyn IWriteableBitmapImpl> {
        Arc::new(ProGpuBitmap::new(size, dpi, format, alpha_format))
    }

    fn load_bitmap_from_file(&self, _file_name: &str) -> io::Result<Arc<SharedBitmapImpl>> {
        Err(unsupported_codec("decoding an image file"))
    }

    fn load_bitmap(&self, _stream: &mut dyn Read) -> io::Result<Arc<SharedBitmapImpl>> {
        Err(unsupported_codec("decoding an image"))
    }

    fn load_writeable_bitmap_to_width(&self, _stream: &mut dyn Read, _width: i32, _interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Err(unsupported_codec("decoding an image"))
    }

    fn load_writeable_bitmap_to_height(&self, _stream: &mut dyn Read, _height: i32, _interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Err(unsupported_codec("decoding an image"))
    }

    fn load_writeable_bitmap_from_file(&self, _file_name: &str) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Err(unsupported_codec("decoding an image file"))
    }

    fn load_writeable_bitmap(&self, _stream: &mut dyn Read) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Err(unsupported_codec("decoding an image"))
    }

    fn load_bitmap_to_width(&self, _stream: &mut dyn Read, _width: i32, _interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<SharedBitmapImpl>> {
        Err(unsupported_codec("decoding an image"))
    }

    fn load_bitmap_to_height(&self, _stream: &mut dyn Read, _height: i32, _interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<SharedBitmapImpl>> {
        Err(unsupported_codec("decoding an image"))
    }

    /// The bitmap drawn to the destination size on the device and read back.
    fn resize_bitmap(&self, bitmap_impl: &dyn IBitmapImpl, destination_size: PixelSize, interpolation_mode: BitmapInterpolationMode) -> Arc<SharedBitmapImpl> {
        let resized = ProGpuBitmap::new(destination_size, bitmap_impl.dpi(), PixelFormat::RGBA8888, AlphaFormat::Premul);
        let source_size = bitmap_impl.pixel_size();
        if source_size.width > 0 && source_size.height > 0 && destination_size.width > 0 && destination_size.height > 0 {
            let mut context = resized.create_drawing_context();
            context.push_render_options(RenderOptions { bitmap_interpolation_mode: interpolation_mode, ..RenderOptions::default() });
            context.draw_bitmap(
                bitmap_impl,
                1.0,
                Rect::new(0.0, 0.0, source_size.width as f64, source_size.height as f64),
                Rect::new(0.0, 0.0, destination_size.width as f64, destination_size.height as f64),
            );
            context.pop_render_options();
            context.dispose();
        }
        Arc::new(resized)
    }

    fn load_bitmap_from_pixels(&self, format: PixelFormat, alpha_format: AlphaFormat, data: &[u8], size: PixelSize, dpi: Vector, stride: i32) -> Arc<SharedBitmapImpl> {
        match ProGpuBitmap::from_pixels(format, alpha_format, data, size, dpi, stride) {
            Some(bitmap) => Arc::new(bitmap),
            None => {
                notes::note_once("pixel-format", "the ProGPU backend keeps pixels of RGBA8888 and BGRA8888 only: a bitmap of other pixels is transparent");
                Arc::new(ProGpuBitmap::new(size, dpi, PixelFormat::RGBA8888, alpha_format))
            }
        }
    }

    fn create_backend_context(&self, graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>) -> Rc<dyn IPlatformRenderInterfaceContext> {
        Rc::new(ProGpuContext::new(graphics_api_context))
    }

    fn supports_individual_round_rects(&self) -> bool {
        true
    }

    fn default_alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn default_pixel_format(&self) -> PixelFormat {
        PixelFormat::RGBA8888
    }

    fn is_supported_bitmap_pixel_format(&self, format: PixelFormat) -> bool {
        bitmap::is_supported(format)
    }

    fn supports_regions(&self) -> bool {
        false
    }

    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        Rc::new(ProGpuRegion::default())
    }
}
