use crate::combined_geometry_impl::CombinedGeometryImpl;
use crate::ellipse_geometry_impl::EllipseGeometryImpl;
use crate::geometry_group_impl::GeometryGroupImpl;
use crate::gpu::ISkiaGpu;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::line_geometry_impl::LineGeometryImpl;
use crate::metal::IMetalDevice;
use crate::rectangle_geometry_impl::RectangleGeometryImpl;
use crate::render_target_bitmap_impl::RenderTargetBitmapImpl;
use crate::skia_backend_context::SkiaContext;
use crate::skia_region_impl::SkiaRegionImpl;
use crate::skia_sharp_extensions::to_pixel_format;
use crate::stream_geometry_impl::StreamGeometryImpl;
use crate::writeable_bitmap_impl::WriteableBitmapImpl;
use ferroui_base::media::imaging::BitmapInterpolationMode;
use ferroui_base::media::{FillRule, GeometryCombineMode};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IGeometryImpl, IOptionalFeatureProvider, IPlatformGraphicsContext,
    IPlatformRenderInterface, IPlatformRenderInterfaceContext, IPlatformRenderInterfaceRegion,
    IRenderTargetBitmapImpl, IStreamGeometryImpl, IWriteableBitmapImpl, PixelFormat, PixelFormats,
};
use ferroui_base::{PixelSize, Point, Rect, Vector};
use crate::geometry_impl::FillPath;
use crate::glyph_run_impl::GlyphRunImpl;
use crate::skia_typeface::SkiaTypeface;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{GlyphRun, GlyphTypeface};
use ferroui_base::platform::IGlyphRunImpl;
use ferroui_opengl::IGlContext;
use skia_safe::{ColorType, FontHinting, PathBuilder};
use std::fs::File;
use std::io::{self, Read};
use std::rc::Rc;

/// Skia platform render interface.
pub struct PlatformRenderInterface {
    max_resource_bytes: Option<i64>,
    use_stencil_buffers: Option<bool>,
    default_pixel_format: PixelFormat,
}

impl PlatformRenderInterface {
    /// Creates the render interface.
    ///
    /// `max_resource_bytes` limits the GPU memory Skia caches and
    /// `use_stencil_buffers` opts into stencil-based path rendering; both
    /// only matter for GPU contexts.
    pub fn new(max_resource_bytes: Option<i64>, use_stencil_buffers: Option<bool>) -> Self {
        Self { max_resource_bytes, use_stencil_buffers, default_pixel_format: to_pixel_format(ColorType::N32) }
    }

    /// Creates the Skia GPU for a platform graphics context.
    ///
    /// # Panics
    /// Panics when the graphics context is of a kind the backend cannot
    /// render with.
    fn create_gpu(&self, graphics_context: &Rc<dyn IPlatformGraphicsContext>) -> Rc<dyn ISkiaGpu> {
        let features: &dyn IOptionalFeatureProvider = &**graphics_context;

        if let Some(skia_gpu) = features.try_get::<dyn ISkiaGpu>() {
            return skia_gpu;
        }

        if let Some(gl) = features.try_get::<dyn IGlContext>() {
            return Self::create_gl_gpu(gl, self.max_resource_bytes, self.use_stencil_buffers);
        }

        if let Some(metal) = features.try_get::<dyn IMetalDevice>() {
            return Self::create_metal_gpu(metal, self.max_resource_bytes, self.use_stencil_buffers);
        }

        panic!("Graphics context of type is not supported");
    }

    #[cfg(ferro_skia_ganesh_gl)]
    fn create_gl_gpu(
        gl: Rc<dyn IGlContext>,
        max_resource_bytes: Option<i64>,
        use_stencil_buffers: Option<bool>,
    ) -> Rc<dyn ISkiaGpu> {
        crate::gpu::open_gl::GlSkiaGpu::new(gl, max_resource_bytes, use_stencil_buffers)
    }

    #[cfg(not(ferro_skia_ganesh_gl))]
    fn create_gl_gpu(
        _gl: Rc<dyn IGlContext>,
        _max_resource_bytes: Option<i64>,
        _use_stencil_buffers: Option<bool>,
    ) -> Rc<dyn ISkiaGpu> {
        panic!("Graphics context of type is not supported");
    }

    #[cfg(target_vendor = "apple")]
    fn create_metal_gpu(
        metal: Rc<dyn IMetalDevice>,
        max_resource_bytes: Option<i64>,
        use_stencil_buffers: Option<bool>,
    ) -> Rc<dyn ISkiaGpu> {
        crate::gpu::metal::SkiaMetalGpu::new(metal, max_resource_bytes, use_stencil_buffers)
    }

    #[cfg(not(target_vendor = "apple"))]
    fn create_metal_gpu(
        _metal: Rc<dyn IMetalDevice>,
        _max_resource_bytes: Option<i64>,
        _use_stencil_buffers: Option<bool>,
    ) -> Rc<dyn ISkiaGpu> {
        panic!("Graphics context of type is not supported");
    }
}

impl Default for PlatformRenderInterface {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl IPlatformRenderInterface for PlatformRenderInterface {
    fn create_ellipse_geometry(&self, rect: Rect) -> Rc<dyn IGeometryImpl> {
        EllipseGeometryImpl::new(rect)
    }

    fn create_line_geometry(&self, p1: Point, p2: Point) -> Rc<dyn IGeometryImpl> {
        LineGeometryImpl::new(p1, p2)
    }

    fn create_rectangle_geometry(&self, rect: Rect) -> Rc<dyn IGeometryImpl> {
        RectangleGeometryImpl::new(rect)
    }

    fn create_stream_geometry(&self) -> Rc<dyn IStreamGeometryImpl> {
        StreamGeometryImpl::new()
    }

    fn create_geometry_group(&self, fill_rule: FillRule, children: &[Rc<dyn IGeometryImpl>]) -> Rc<dyn IGeometryImpl> {
        GeometryGroupImpl::new(fill_rule, children)
    }

    fn create_combined_geometry(
        &self,
        combine_mode: GeometryCombineMode,
        g1: Rc<dyn IGeometryImpl>,
        g2: Rc<dyn IGeometryImpl>,
    ) -> Rc<dyn IGeometryImpl> {
        CombinedGeometryImpl::force_create(combine_mode, &*g1, &*g2)
    }

    fn build_glyph_run_geometry(&self, glyph_run: &GlyphRun) -> Rc<dyn IGeometryImpl> {
        let glyph_typeface = SkiaTypeface::try_get(&**glyph_run.glyph_typeface().platform_typeface())
            .unwrap_or_else(|| panic!("PlatformImpl can't be null."));

        let font_rendering_em_size = glyph_run.font_rendering_em_size() as f32;
        let mut sk_font = glyph_typeface.create_sk_font(font_rendering_em_size);

        sk_font.set_hinting(FontHinting::None);

        let mut path = PathBuilder::new();

        let baseline_origin = glyph_run.baseline_origin();
        let (mut current_x, current_y) = (baseline_origin.x, baseline_origin.y);

        let glyph_infos = glyph_run.glyph_infos();

        for glyph_info in glyph_infos.borrow().iter() {
            if let Some(glyph_path) = sk_font.get_path(glyph_info.glyph_index) {
                if !glyph_path.is_empty() {
                    path.add_path_with_offset(&glyph_path, (current_x as f32, current_y as f32), None);
                }
            }

            current_x += glyph_info.glyph_advance;
        }

        StreamGeometryImpl::from_paths(path.detach(), FillPath::SameAsStroke, None)
    }

    fn create_glyph_run(
        &self,
        glyph_typeface: &Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        glyph_infos: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Rc<dyn IGlyphRunImpl> {
        Rc::new(GlyphRunImpl::new(glyph_typeface, font_rendering_em_size, glyph_infos, baseline_origin))
    }

    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> Rc<dyn IRenderTargetBitmapImpl> {
        if size.width < 1 {
            panic!("Width can't be less than 1");
        }

        if size.height < 1 {
            panic!("Height can't be less than 1");
        }

        Rc::new(RenderTargetBitmapImpl::new(size, dpi))
    }

    fn create_writeable_bitmap(
        &self,
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha_format: AlphaFormat,
    ) -> Rc<dyn IWriteableBitmapImpl> {
        WriteableBitmapImpl::new(size, dpi, format, alpha_format)
    }

    fn load_bitmap_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IBitmapImpl>> {
        let mut stream = File::open(file_name)?;
        self.load_bitmap(&mut stream)
    }

    fn load_bitmap(&self, stream: &mut dyn Read) -> io::Result<Rc<dyn IBitmapImpl>> {
        Ok(Rc::new(ImmutableBitmap::from_stream(stream)?))
    }

    fn load_writeable_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(WriteableBitmapImpl::from_stream_to_size(stream, width, true, interpolation_mode)?)
    }

    fn load_writeable_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(WriteableBitmapImpl::from_stream_to_size(stream, height, false, interpolation_mode)?)
    }

    fn load_writeable_bitmap_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWriteableBitmapImpl>> {
        let mut stream = File::open(file_name)?;
        self.load_writeable_bitmap(&mut stream)
    }

    fn load_writeable_bitmap(&self, stream: &mut dyn Read) -> io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(WriteableBitmapImpl::from_stream(stream)?)
    }

    fn load_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Rc<dyn IBitmapImpl>> {
        Ok(Rc::new(ImmutableBitmap::from_stream_to_size(stream, width, true, interpolation_mode)?))
    }

    fn load_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Rc<dyn IBitmapImpl>> {
        Ok(Rc::new(ImmutableBitmap::from_stream_to_size(stream, height, false, interpolation_mode)?))
    }

    fn resize_bitmap(
        &self,
        bitmap_impl: &dyn IBitmapImpl,
        destination_size: PixelSize,
        interpolation_mode: BitmapInterpolationMode,
    ) -> Rc<dyn IBitmapImpl> {
        match bitmap_impl.as_any().downcast_ref::<ImmutableBitmap>() {
            Some(ibmp) => Rc::new(
                ImmutableBitmap::resized(ibmp, destination_size, interpolation_mode)
                    .unwrap_or_else(|error| panic!("Unable to resize the bitmap: {error}")),
            ),
            None => panic!("Invalid source bitmap type."),
        }
    }

    fn load_bitmap_from_pixels(
        &self,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
        size: PixelSize,
        dpi: Vector,
        stride: i32,
    ) -> Rc<dyn IBitmapImpl> {
        Rc::new(
            ImmutableBitmap::from_pixels(size, dpi, stride, format, alpha_format, data)
                .unwrap_or_else(|error| panic!("{error}")),
        )
    }

    fn create_backend_context(
        &self,
        graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        match graphics_api_context {
            None => Rc::new(SkiaContext::new(None)),
            Some(graphics_context) => Rc::new(SkiaContext::new(Some(self.create_gpu(&graphics_context)))),
        }
    }

    fn supports_individual_round_rects(&self) -> bool {
        true
    }

    fn default_alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn default_pixel_format(&self) -> PixelFormat {
        self.default_pixel_format
    }

    fn is_supported_bitmap_pixel_format(&self, format: PixelFormat) -> bool {
        format == PixelFormats::RGB565 || format == PixelFormats::BGRA8888 || format == PixelFormats::RGBA8888
    }

    fn supports_regions(&self) -> bool {
        true
    }

    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        Rc::new(SkiaRegionImpl::new())
    }
}
