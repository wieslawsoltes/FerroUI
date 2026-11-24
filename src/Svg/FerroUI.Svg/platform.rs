use crate::{SvgRenderTarget, SvgSurface};
use ferroui_base::media::imaging::BitmapInterpolationMode;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{FillRule, GeometryCombineMode, GlyphRun, GlyphTypeface};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextLayerImpl, IGeometryImpl, IGlyphRunImpl,
    IOptionalFeatureProvider, IPlatformGraphicsContext, IPlatformRenderInterface,
    IPlatformRenderInterfaceContext, IPlatformRenderInterfaceRegion, IRenderTarget,
    IRenderTargetBitmapImpl, IStreamGeometryBuilder, IWriteableBitmapImpl, PixelFormat,
    SharedBitmapImpl,
};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Point, Rect, Vector};
use ferroui_controls::AppBuilder;
use ferroui_skia::{PlatformRenderInterface, SkiaOptions, SkiaPlatform};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;

/// Skia resource factories plus native SVG surface support. Ordinary platform surfaces retain
/// the existing Skia behavior; an explicit [`SvgSurface`] selects the SVG frame capture target.
/// This keeps resource identity and optional capabilities consistent with the Skia backend.
pub struct SvgPlatformRenderInterface {
    skia: PlatformRenderInterface,
}

impl SvgPlatformRenderInterface {
    pub fn new(max_resource_bytes: Option<i64>, use_stencil_buffers: Option<bool>) -> Self {
        Self {
            skia: PlatformRenderInterface::new(max_resource_bytes, use_stencil_buffers),
        }
    }
}

impl Default for SvgPlatformRenderInterface {
    fn default() -> Self {
        Self::new(None, None)
    }
}

macro_rules! delegate {
    ($(fn $name:ident(&self $(,$arg:ident:$ty:ty)*) -> $result:ty;)+) => {$(
        fn $name(&self,$($arg:$ty),*) -> $result { self.skia.$name($($arg),*) }
    )+};
}

impl IPlatformRenderInterface for SvgPlatformRenderInterface {
    delegate! {
        fn create_ellipse_geometry(&self,rect:Rect) -> Arc<dyn IGeometryImpl>;
        fn create_line_geometry(&self,p1:Point,p2:Point) -> Arc<dyn IGeometryImpl>;
        fn create_rectangle_geometry(&self,rect:Rect) -> Arc<dyn IGeometryImpl>;
        fn create_stream_geometry_builder(&self,source:Option<&dyn IGeometryImpl>) -> Box<dyn IStreamGeometryBuilder>;
        fn create_geometry_group(&self,fill_rule:FillRule,children:&[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl>;
        fn create_combined_geometry(&self,combine_mode:GeometryCombineMode,g1:Arc<dyn IGeometryImpl>,g2:Arc<dyn IGeometryImpl>) -> Arc<dyn IGeometryImpl>;
        fn build_glyph_run_geometry(&self,glyph_run:&GlyphRun) -> Arc<dyn IGeometryImpl>;
        fn try_build_glyph_run_geometry(&self,glyph_run:&GlyphRun) -> Option<Arc<dyn IGeometryImpl>>;
        fn create_glyph_run(&self,glyph_typeface:&Rc<GlyphTypeface>,font_rendering_em_size:f64,glyph_infos:&[GlyphInfo],baseline_origin:Point) -> Arc<dyn IGlyphRunImpl>;
        fn create_render_target_bitmap(&self,size:PixelSize,dpi:Vector) -> Arc<dyn IRenderTargetBitmapImpl>;
        fn create_writeable_bitmap(&self,size:PixelSize,dpi:Vector,format:PixelFormat,alpha_format:AlphaFormat) -> Arc<dyn IWriteableBitmapImpl>;
        fn load_bitmap_from_file(&self,file_name:&str) -> std::io::Result<Arc<SharedBitmapImpl>>;
        fn load_bitmap(&self,stream:&mut dyn Read) -> std::io::Result<Arc<SharedBitmapImpl>>;
        fn load_writeable_bitmap_to_width(&self,stream:&mut dyn Read,width:i32,interpolation_mode:BitmapInterpolationMode) -> std::io::Result<Arc<dyn IWriteableBitmapImpl>>;
        fn load_writeable_bitmap_to_height(&self,stream:&mut dyn Read,height:i32,interpolation_mode:BitmapInterpolationMode) -> std::io::Result<Arc<dyn IWriteableBitmapImpl>>;
        fn load_writeable_bitmap_from_file(&self,file_name:&str) -> std::io::Result<Arc<dyn IWriteableBitmapImpl>>;
        fn load_writeable_bitmap(&self,stream:&mut dyn Read) -> std::io::Result<Arc<dyn IWriteableBitmapImpl>>;
        fn load_bitmap_to_width(&self,stream:&mut dyn Read,width:i32,interpolation_mode:BitmapInterpolationMode) -> std::io::Result<Arc<SharedBitmapImpl>>;
        fn load_bitmap_to_height(&self,stream:&mut dyn Read,height:i32,interpolation_mode:BitmapInterpolationMode) -> std::io::Result<Arc<SharedBitmapImpl>>;
        fn resize_bitmap(&self,bitmap_impl:&dyn IBitmapImpl,destination_size:PixelSize,interpolation_mode:BitmapInterpolationMode) -> Arc<SharedBitmapImpl>;
        fn load_bitmap_from_pixels(&self,format:PixelFormat,alpha_format:AlphaFormat,data:&[u8],size:PixelSize,dpi:Vector,stride:i32) -> Arc<SharedBitmapImpl>;
        fn supports_individual_round_rects(&self) -> bool;
        fn supports_compositing_layers(&self) -> bool;
        fn default_alpha_format(&self) -> AlphaFormat;
        fn default_pixel_format(&self) -> PixelFormat;
        fn is_supported_bitmap_pixel_format(&self,format:PixelFormat) -> bool;
        fn supports_regions(&self) -> bool;
        fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion>;
    }

    fn create_backend_context(
        &self,
        graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        Rc::new(SvgBackendContext {
            skia: self.skia.create_backend_context(graphics_api_context),
            disposed: Cell::new(false),
        })
    }
}

struct SvgBackendContext {
    skia: Rc<dyn IPlatformRenderInterfaceContext>,
    disposed: Cell<bool>,
}

impl IOptionalFeatureProvider for SvgBackendContext {
    fn try_get_feature(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.skia.try_get_feature(kind)
    }
}

impl IPlatformRenderInterfaceContext for SvgBackendContext {
    fn poll_pending_gpu_work(&self) -> bool {
        self.skia.poll_pending_gpu_work()
    }

    fn create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        assert!(!self.disposed.get(), "SVG backend context is disposed");
        if let Some(surface) = surfaces
            .iter()
            .find_map(|surface| surface.as_any().downcast_ref::<SvgSurface>())
        {
            let surface = surface.clone();
            return Rc::new(SvgRenderTarget::with_completion(move |document| {
                surface.publish(document)
            }));
        }
        self.skia.create_render_target(surfaces)
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        scaling: Vector,
        enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        self.skia
            .create_offscreen_render_target(pixel_size, scaling, enable_text_antialiasing)
    }

    fn is_lost(&self) -> bool {
        self.disposed.get() || self.skia.is_lost()
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        self.skia.max_offscreen_render_target_pixel_size()
    }
    fn public_features(&self) -> HashMap<TypeId, Rc<dyn Any>> {
        self.skia.public_features()
    }
    fn is_ready_to_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> bool {
        !self.disposed.get()
            && (surfaces
                .iter()
                .any(|surface| surface.as_any().is::<SvgSurface>() && surface.is_ready())
                || self.skia.is_ready_to_create_render_target(surfaces))
    }
    fn dispose(&self) {
        if !self.disposed.replace(true) {
            self.skia.dispose();
        }
    }
}

/// Initializes Skia resources and registers an interface that recognizes SVG surfaces.
pub struct SvgPlatform;

impl SvgPlatform {
    pub fn initialize() {
        Self::initialize_with_options(SkiaOptions::default());
    }
    pub fn initialize_with_options(options: SkiaOptions) {
        SkiaPlatform::initialize_with_options(options);
        let render_interface: Rc<dyn IPlatformRenderInterface> =
            Rc::new(SvgPlatformRenderInterface::new(
                options.max_gpu_resource_size_bytes,
                options.use_stencil_buffers,
            ));
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(render_interface);
    }
}

/// Application-builder opt-in to Skia plus SVG export surfaces.
pub trait SvgApplicationExtensions {
    fn use_svg(&self) -> AppBuilder;
}

impl SvgApplicationExtensions for AppBuilder {
    fn use_svg(&self) -> AppBuilder {
        self.use_rendering_subsystem(
            || {
                let options = FerroLocator::current().get_service::<SkiaOptions>();
                SvgPlatform::initialize_with_options(
                    options.as_deref().copied().unwrap_or_default(),
                );
            },
            "Skia + SVG",
        )
    }
}
