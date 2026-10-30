use crate::combined_geometry_impl::CombinedGeometryImpl;
use crate::ellipse_geometry_impl::EllipseGeometryImpl;
use crate::geometry_group_impl::GeometryGroupImpl;
use crate::geometry_impl::{FillPath, VelloPath};
use crate::glyph_run_impl::GlyphRunImpl;
use crate::helpers::pixel_format_helper;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::line_geometry_impl::LineGeometryImpl;
use crate::rectangle_geometry_impl::RectangleGeometryImpl;
use crate::render_target_bitmap_impl::RenderTargetBitmapImpl;
use crate::simple_geometry_impl::SimpleGeometryImpl;
use crate::stream_geometry_builder::StreamGeometryBuilder;
use crate::vello_backend_context::VelloContext;
use crate::vello_options::{VelloOptions, VelloRenderingMode};
use crate::vello_region_impl::VelloRegionImpl;
use crate::vello_typeface::VelloTypeface;
use crate::writeable_bitmap_impl::WriteableBitmapImpl;
use ferroui_base::media::imaging::BitmapInterpolationMode;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{FillRule, GeometryCombineMode, GlyphRun, GlyphTypeface};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IGeometryImpl, IGlyphRunImpl, IPlatformGraphicsContext, IPlatformRenderInterface,
    IPlatformRenderInterfaceContext, IPlatformRenderInterfaceRegion, IRenderTargetBitmapImpl, IStreamGeometryBuilder,
    IWriteableBitmapImpl, PixelFormat,
};
use ferroui_base::{PixelSize, Point, Rect, Vector};
use kurbo::BezPath;
use std::fs::File;
use std::io::{self, Read};
use std::rc::Rc;
use std::sync::Arc;

/// Vello platform render interface.
pub struct PlatformRenderInterface {
    /// The rendering modes in the order they are tried.
    rendering_modes: Vec<VelloRenderingMode>,
}

impl PlatformRenderInterface {
    /// Creates the render interface of the given options.
    pub fn new(options: VelloOptions) -> Self {
        Self { rendering_modes: options.rendering_mode_order() }
    }

    /// The rendering modes scenes are drawn in, in the order they are tried.
    pub fn rendering_modes(&self) -> &[VelloRenderingMode] {
        &self.rendering_modes
    }
}

impl Default for PlatformRenderInterface {
    fn default() -> Self {
        Self::new(VelloOptions::default())
    }
}

fn glyph_run_geometry(glyph_run: &GlyphRun, require_complete: bool) -> Option<Arc<dyn IGeometryImpl>> {
    match VelloTypeface::try_get(&**glyph_run.glyph_typeface().platform_typeface()) {
        Some(_) => {}
        None if require_complete => return None,
        None => panic!("PlatformImpl can't be null."),
    }
    let platform = glyph_run.platform_impl();
    let run = platform.as_any().downcast_ref::<GlyphRunImpl>()?;
    let paths = if require_complete { run.try_outline_paths()? } else { run.outline_paths() };
    let mut path = BezPath::new();
    let origin = glyph_run.baseline_origin();
    let placement = kurbo::Affine::translate((origin.x, origin.y));
    // Drawing and extraction share the cached, exact f64 shaped contours.
    // GlyphRun's glyph-info, font-size and baseline setters invalidate its
    // platform implementation, keeping drawing and extraction on one snapshot.
    for glyph_path in paths.iter().flatten() {
        path.extend(glyph_path.iter().map(|element| placement * element));
    }
    Some(SimpleGeometryImpl::new(VelloPath::new(path, FillRule::NonZero), FillPath::SameAsStroke, None))
}

impl IPlatformRenderInterface for PlatformRenderInterface {
    fn supports_compositing_layers(&self) -> bool {
        true
    }

    fn create_ellipse_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        EllipseGeometryImpl::new(rect)
    }

    fn create_line_geometry(&self, p1: Point, p2: Point) -> Arc<dyn IGeometryImpl> {
        LineGeometryImpl::new(p1, p2)
    }

    fn create_rectangle_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        RectangleGeometryImpl::new(rect)
    }

    fn create_stream_geometry_builder(&self, source: Option<&dyn IGeometryImpl>) -> Box<dyn IStreamGeometryBuilder> {
        Box::new(StreamGeometryBuilder::new(source))
    }

    fn create_geometry_group(&self, fill_rule: FillRule, children: &[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl> {
        GeometryGroupImpl::new(fill_rule, children)
    }

    fn create_combined_geometry(
        &self,
        combine_mode: GeometryCombineMode,
        g1: Arc<dyn IGeometryImpl>,
        g2: Arc<dyn IGeometryImpl>,
    ) -> Arc<dyn IGeometryImpl> {
        CombinedGeometryImpl::force_create(combine_mode, &*g1, &*g2)
    }

    fn build_glyph_run_geometry(&self, glyph_run: &GlyphRun) -> Arc<dyn IGeometryImpl> {
        glyph_run_geometry(glyph_run, false).expect("Partial glyph outlines are always available")
    }

    fn try_build_glyph_run_geometry(&self, glyph_run: &GlyphRun) -> Option<Arc<dyn IGeometryImpl>> {
        glyph_run_geometry(glyph_run, true)
    }

    fn create_glyph_run(
        &self,
        glyph_typeface: &Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        glyph_infos: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Arc<dyn IGlyphRunImpl> {
        Arc::new(GlyphRunImpl::new(glyph_typeface, font_rendering_em_size, glyph_infos, baseline_origin))
    }

    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> Arc<dyn IRenderTargetBitmapImpl> {
        if size.width < 1 {
            panic!("Width can't be less than 1");
        }

        if size.height < 1 {
            panic!("Height can't be less than 1");
        }

        Arc::new(RenderTargetBitmapImpl::new(size, dpi, self.rendering_modes.clone()))
    }

    fn create_writeable_bitmap(
        &self,
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha_format: AlphaFormat,
    ) -> Arc<dyn IWriteableBitmapImpl> {
        WriteableBitmapImpl::new(size, dpi, format, alpha_format)
    }

    fn load_bitmap_from_file(&self, file_name: &str) -> io::Result<Arc<ferroui_base::platform::SharedBitmapImpl>> {
        let mut stream = File::open(file_name)?;
        self.load_bitmap(&mut stream)
    }

    fn load_bitmap(&self, stream: &mut dyn Read) -> io::Result<Arc<ferroui_base::platform::SharedBitmapImpl>> {
        Ok(Arc::new(ImmutableBitmap::from_stream(stream)?))
    }

    fn load_writeable_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(WriteableBitmapImpl::from_stream_to_size(stream, width, true, interpolation_mode)?)
    }

    fn load_writeable_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(WriteableBitmapImpl::from_stream_to_size(stream, height, false, interpolation_mode)?)
    }

    fn load_writeable_bitmap_from_file(&self, file_name: &str) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        let mut stream = File::open(file_name)?;
        self.load_writeable_bitmap(&mut stream)
    }

    fn load_writeable_bitmap(&self, stream: &mut dyn Read) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(WriteableBitmapImpl::from_stream(stream)?)
    }

    fn load_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<ferroui_base::platform::SharedBitmapImpl>> {
        Ok(Arc::new(ImmutableBitmap::from_stream_to_size(stream, width, true, interpolation_mode)?))
    }

    fn load_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        interpolation_mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<ferroui_base::platform::SharedBitmapImpl>> {
        Ok(Arc::new(ImmutableBitmap::from_stream_to_size(stream, height, false, interpolation_mode)?))
    }

    fn resize_bitmap(
        &self,
        bitmap_impl: &dyn IBitmapImpl,
        destination_size: PixelSize,
        interpolation_mode: BitmapInterpolationMode,
    ) -> Arc<ferroui_base::platform::SharedBitmapImpl> {
        match bitmap_impl.as_any().downcast_ref::<ImmutableBitmap>() {
            Some(ibmp) => Arc::new(
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
    ) -> Arc<ferroui_base::platform::SharedBitmapImpl> {
        Arc::new(
            ImmutableBitmap::from_pixels(size, dpi, stride, format, alpha_format, data)
                .unwrap_or_else(|error| panic!("{error}")),
        )
    }

    fn create_backend_context(
        &self,
        graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        // The WebGL2 context of a canvas of a web page: the hybrid mode
        // draws the canvas with the renderer of that context.
        #[cfg(feature = "hybrid-webgl")]
        if let Some(graphics_context) = &graphics_api_context {
            match crate::web_gl::VelloWebGlGpu::try_new(graphics_context) {
                Some(Ok(web_gl)) => return Rc::new(VelloContext::with_web_gl(web_gl, self.rendering_modes.clone())),
                Some(Err(error)) => panic!("The Vello backend cannot draw with the WebGL2 context of the canvas: {error}."),
                None => {}
            }
        }

        match graphics_api_context {
            None => Rc::new(VelloContext::new(self.rendering_modes.clone())),
            // A scene is drawn into the surface of a window on a device
            // made over the graphics context of the platform.
            // The windows the GPU modes do not draw, on a platform whose
            // graphics are OpenGL, are drawn in the CPU mode and presented
            // through its context.
            #[cfg(any(feature = "hybrid", feature = "gpu"))]
            Some(graphics_context) => {
                let gl_windows = crate::gl_window::VelloGlWindows::try_new(&graphics_context);
                let context = match crate::gpu::create_gpu(&graphics_context) {
                    Some(gpu) => VelloContext::with_gpu(gpu, self.rendering_modes.clone()),
                    None => VelloContext::new(self.rendering_modes.clone()),
                };
                Rc::new(context.with_gl_windows(gl_windows))
            }
            #[cfg(all(not(any(feature = "hybrid", feature = "gpu")), not(target_family = "wasm")))]
            Some(graphics_context) => Rc::new(
                VelloContext::new(self.rendering_modes.clone())
                    .with_gl_windows(crate::gl_window::VelloGlWindows::try_new(&graphics_context)),
            ),
            #[cfg(all(not(any(feature = "hybrid", feature = "gpu")), target_family = "wasm"))]
            Some(_) => panic!(
                "The Vello backend draws a canvas of a web page with the WebGL2 context of the canvas (the feature \
                 `hybrid-webgl` of ferroui-vello) or into the framebuffer of the platform"
            ),
        }
    }

    fn supports_individual_round_rects(&self) -> bool {
        true
    }

    fn default_alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn default_pixel_format(&self) -> PixelFormat {
        // What the renderers of the Vello project draw.
        PixelFormat::RGBA8888
    }

    fn is_supported_bitmap_pixel_format(&self, format: PixelFormat) -> bool {
        pixel_format_helper::is_supported(format)
    }

    fn supports_regions(&self) -> bool {
        true
    }

    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        Rc::new(VelloRegionImpl::new())
    }
}
