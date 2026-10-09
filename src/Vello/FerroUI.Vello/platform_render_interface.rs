use crate::combined_geometry_impl::CombinedGeometryImpl;
use crate::drawing_context_impl::not_built;
use crate::ellipse_geometry_impl::EllipseGeometryImpl;
use crate::geometry_group_impl::GeometryGroupImpl;
use crate::geometry_impl::{FillPath, VelloPath};
use crate::glyph_run_impl::GlyphRunImpl;
use crate::helpers::pixel_format_helper;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::line_geometry_impl::LineGeometryImpl;
use crate::rectangle_geometry_impl::RectangleGeometryImpl;
use crate::render_target_bitmap_impl::RenderTargetBitmapImpl;
use crate::stream_geometry_impl::StreamGeometryImpl;
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
    IPlatformRenderInterfaceContext, IPlatformRenderInterfaceRegion, IRenderTargetBitmapImpl, IStreamGeometryImpl,
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

impl IPlatformRenderInterface for PlatformRenderInterface {
    fn create_ellipse_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        EllipseGeometryImpl::new(rect)
    }

    fn create_line_geometry(&self, p1: Point, p2: Point) -> Arc<dyn IGeometryImpl> {
        LineGeometryImpl::new(p1, p2)
    }

    fn create_rectangle_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        RectangleGeometryImpl::new(rect)
    }

    fn create_stream_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
        StreamGeometryImpl::new()
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
        let glyph_typeface = VelloTypeface::try_get(&**glyph_run.glyph_typeface().platform_typeface())
            .unwrap_or_else(|| panic!("PlatformImpl can't be null."));

        let font_rendering_em_size = glyph_run.font_rendering_em_size();

        let mut path = BezPath::new();

        let baseline_origin = glyph_run.baseline_origin();
        let (mut current_x, current_y) = (baseline_origin.x, baseline_origin.y);

        let glyph_infos = glyph_run.glyph_infos();

        // As the Skia backend builds it: the outline of every glyph without
        // hinting at its pen position on the baseline.
        for glyph_info in glyph_infos.borrow().iter() {
            if let Some(glyph_path) = glyph_typeface.face().glyph_path(glyph_info.glyph_index, font_rendering_em_size) {
                if !glyph_path.elements().is_empty() {
                    path.extend(kurbo::Affine::translate((current_x, current_y)) * glyph_path);
                }
            }

            current_x += glyph_info.glyph_advance;
        }

        StreamGeometryImpl::from_paths(VelloPath::new(path, FillRule::NonZero), FillPath::SameAsStroke, None)
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
        match graphics_api_context {
            None => Rc::new(VelloContext::new(self.rendering_modes.clone())),
            // A scene is drawn into the surface of a window by the hybrid
            // or the GPU mode, on a device made from the graphics context.
            Some(_) => not_built("with a platform graphics context", "stages 7 and 8"),
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
