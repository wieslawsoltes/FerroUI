//! The render interface of the backend: what the base library makes its geometries, bitmaps,
//! glyph runs and backend contexts with.

use crate::bitmap::{self, ProGpuBitmap};
use crate::context::ProGpuContext;
use crate::geometry::{path_data_of, PathBuilder, PathData, ProGpuGeometry, ProGpuStreamGeometryBuilder};
use crate::glyph_run::{self, ProGpuGlyphRun};
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
/// WebGPU device; text is the text stack of the engine and image files are read by its image
/// module (PNG). What the engine does not have (other image formats, saving a bitmap) reports
/// `Unsupported` and is noted in the log once.
#[derive(Default)]
pub struct ProGpuPlatformRenderInterface;

impl ProGpuPlatformRenderInterface {
    pub fn new() -> Self {
        ProGpuPlatformRenderInterface
    }
}

/// The bitmap of the image file a stream holds.
fn decode_stream(stream: &mut dyn Read) -> io::Result<ProGpuBitmap> {
    let mut file = Vec::new();
    stream.read_to_end(&mut file)?;
    ProGpuBitmap::decode(&file)
}

/// The bitmap of the image file a stream holds, scaled so that its width (`horizontal`) or its
/// height is `decode_size` and the other side keeps the proportion.
fn decode_to_size(stream: &mut dyn Read, decode_size: i32, horizontal: bool, interpolation_mode: BitmapInterpolationMode) -> io::Result<ProGpuBitmap> {
    let bitmap = decode_stream(stream)?;
    let size = bitmap.pixel_size();
    if decode_size <= 0 || size.width <= 0 || size.height <= 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "an image cannot be decoded to an empty size"));
    }
    let (along, across) = if horizontal { (size.width, size.height) } else { (size.height, size.width) };
    if along == decode_size {
        return Ok(bitmap);
    }
    let other = ((across as f64 * decode_size as f64 / along as f64).round() as i32).max(1);
    let destination = if horizontal { PixelSize::new(decode_size, other) } else { PixelSize::new(other, decode_size) };
    Ok(resized(&bitmap, destination, interpolation_mode))
}

/// `bitmap` drawn to `destination_size` on the device and read back: premultiplied `RGBA8888`.
fn resized(bitmap: &dyn IBitmapImpl, destination_size: PixelSize, interpolation_mode: BitmapInterpolationMode) -> ProGpuBitmap {
    let resized = ProGpuBitmap::new(destination_size, bitmap.dpi(), PixelFormat::RGBA8888, AlphaFormat::Premul);
    let source_size = bitmap.pixel_size();
    if source_size.width > 0 && source_size.height > 0 && destination_size.width > 0 && destination_size.height > 0 {
        let mut context = resized.create_drawing_context();
        context.push_render_options(RenderOptions { bitmap_interpolation_mode: interpolation_mode, ..RenderOptions::default() });
        context.draw_bitmap(
            bitmap,
            1.0,
            Rect::new(0.0, 0.0, source_size.width as f64, source_size.height as f64),
            Rect::new(0.0, 0.0, destination_size.width as f64, destination_size.height as f64),
        );
        context.pop_render_options();
        context.dispose();
    }
    resized
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

    fn build_glyph_run_geometry(&self, glyph_run: &GlyphRun) -> Arc<dyn IGeometryImpl> {
        let glyph_infos = glyph_run.glyph_infos();
        let glyph_infos = glyph_infos.borrow();
        match glyph_run::outline_geometry(glyph_run.glyph_typeface(), glyph_run.font_rendering_em_size(), &glyph_infos, glyph_run.baseline_origin()) {
            Some(data) => ProGpuGeometry::new(data),
            None => {
                notes::note_once("text-geometry", "the outlines of text of a typeface of another font manager are not made by the ProGPU backend");
                empty_geometry()
            }
        }
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

    fn load_bitmap_from_file(&self, file_name: &str) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(ProGpuBitmap::decode(&std::fs::read(file_name)?)?))
    }

    fn load_bitmap(&self, stream: &mut dyn Read) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(decode_stream(stream)?))
    }

    fn load_writeable_bitmap_to_width(&self, stream: &mut dyn Read, width: i32, interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(decode_to_size(stream, width, true, interpolation_mode)?))
    }

    fn load_writeable_bitmap_to_height(&self, stream: &mut dyn Read, height: i32, interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(decode_to_size(stream, height, false, interpolation_mode)?))
    }

    fn load_writeable_bitmap_from_file(&self, file_name: &str) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(ProGpuBitmap::decode(&std::fs::read(file_name)?)?))
    }

    fn load_writeable_bitmap(&self, stream: &mut dyn Read) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(decode_stream(stream)?))
    }

    fn load_bitmap_to_width(&self, stream: &mut dyn Read, width: i32, interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(decode_to_size(stream, width, true, interpolation_mode)?))
    }

    fn load_bitmap_to_height(&self, stream: &mut dyn Read, height: i32, interpolation_mode: BitmapInterpolationMode) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(decode_to_size(stream, height, false, interpolation_mode)?))
    }

    /// The bitmap drawn to the destination size on the device and read back.
    fn resize_bitmap(&self, bitmap_impl: &dyn IBitmapImpl, destination_size: PixelSize, interpolation_mode: BitmapInterpolationMode) -> Arc<SharedBitmapImpl> {
        Arc::new(resized(bitmap_impl, destination_size, interpolation_mode))
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
