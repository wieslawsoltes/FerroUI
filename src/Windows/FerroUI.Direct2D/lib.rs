//! Windows system GPU rendering and text.
#![cfg(windows)]
mod bitmap;
mod external_objects;
mod writeable_bitmap;
pub use external_objects::{device_luid, native_device, D3D11Texture};
pub fn device_generation() -> u64 {
    unsafe { ffi::d2_device_generation() }
}
#[cfg(feature = "media-controls")]
pub use external_objects::D3D11TextureImporter;
#[cfg(feature = "runtime-shaders")]
mod custom_shaders;
mod drawing;
mod ffi;
mod fonts;
#[cfg(feature = "gpu")]
mod presentation;
pub use fonts::FontManager as Direct2DFontManager;
pub use fonts::TextShaper as DirectWriteTextShaper;
/// [surface allocations, path draws, image draws, presents, explicit readbacks,
/// image uploads, texture imports, GPU copies, errors, completed media reads,
/// compute dispatches, compute pixels].
pub fn statistics() -> [u64; 12] {
    extern "C" {
        fn d2_statistics(out: *mut u64, count: usize);
    }
    let mut counters = [0; 12];
    unsafe {
        d2_statistics(counters.as_mut_ptr(), counters.len());
    }
    counters
}
/// Optional CPU instrumentation, enabled with FERROUI_D3D_PROFILE at process start.
/// Slots: end draw, frame acquisition, snapshot, kernel, layer end, fill rule,
/// clip, glyph draw, glyph outline, glyph metrics, shaping, device check,
/// path creation, path export, shaping cache hits and shaping cache misses.
/// Final slots count glyph-ink-bounds cache hits and time/count its misses.
/// Slots 18/19 time core glyph drawing and count draws with positioned advances.
/// Times may nest and are not GPU timestamps.
#[doc(hidden)]
pub fn profiling_statistics() -> ([u64; 20], [u64; 20]) {
    extern "C" {
        fn d2_profile(nanoseconds: *mut u64, counts: *mut u64, count: usize);
    }
    let mut nanoseconds = [0; 20];
    let mut counts = [0; 20];
    unsafe { d2_profile(nanoseconds.as_mut_ptr(), counts.as_mut_ptr(), counts.len()) };
    (nanoseconds, counts)
}
mod geometry;
mod path_measure;
mod region;
use bitmap::Bitmap;
use drawing::GlyphRun;
use ferroui_base::{
    media::{
        imaging::BitmapInterpolationMode, text_formatting::GlyphInfo, FillRule,
        GeometryCombineMode, GlyphRun as MediaGlyphRun, GlyphTypeface,
    },
    platform::*,
    FerroLocator, PixelSize, Point, Rect, Vector,
};
use ferroui_controls::AppBuilder;
use geometry::{Builder, Geometry};
use kurbo::BezPath;
use std::{
    io::{self, Read},
    rc::Rc,
    sync::Arc,
};

pub trait Direct2DApplicationExtensions {
    fn use_direct2d(&self) -> AppBuilder;
}
impl Direct2DApplicationExtensions for AppBuilder {
    fn use_direct2d(&self) -> AppBuilder {
        self.use_rendering_subsystem(
            || {
                assert!(unsafe { ffi::d2_supported() } != 0, "Native rendering requires Windows 10 1709 or later and hardware D3D11 feature level 11.0 or later");
                FerroLocator::current_mutable()
                    .bind::<dyn IPlatformRenderInterface>()
                    .to_constant(Rc::new(Direct2DRenderInterface) as Rc<dyn IPlatformRenderInterface>)
                    .bind::<dyn IFontManagerImpl>()
                    .to_constant(Rc::new(fonts::FontManager::default()) as Rc<dyn IFontManagerImpl>);
            },
            "Direct2D",
        )
        .use_text_shaping_subsystem(
            || {
                FerroLocator::current_mutable()
                    .bind::<dyn ITextShaperImpl>()
                    .to_constant(Rc::new(fonts::TextShaper) as Rc<dyn ITextShaperImpl>);
            },
            "DirectWrite",
        )
    }
}
pub struct Direct2DRenderInterface;
fn resized(
    stream: &mut dyn Read,
    dimension: i32,
    width: bool,
    mode: BitmapInterpolationMode,
) -> io::Result<Bitmap> {
    let bitmap = Bitmap::decode(stream)?;
    let s = bitmap.pixel_size();
    let size = if width {
        PixelSize::new(
            dimension,
            (s.height as f64 * dimension as f64 / s.width as f64)
                .round()
                .max(1.) as i32,
        )
    } else {
        PixelSize::new(
            (s.width as f64 * dimension as f64 / s.height as f64)
                .round()
                .max(1.) as i32,
            dimension,
        )
    };
    Ok(Bitmap::resized(&bitmap, size, mode))
}
impl IPlatformRenderInterface for Direct2DRenderInterface {
    fn create_ellipse_geometry(&self, r: Rect) -> Arc<dyn IGeometryImpl> {
        Geometry::ellipse(r)
    }
    fn create_line_geometry(&self, a: Point, b: Point) -> Arc<dyn IGeometryImpl> {
        let mut p = BezPath::new();
        p.move_to((a.x, a.y));
        p.line_to((b.x, b.y));
        Geometry::paths(p, None, FillRule::NonZero)
    }
    fn create_rectangle_geometry(&self, r: Rect) -> Arc<dyn IGeometryImpl> {
        Geometry::rectangle(r)
    }
    fn create_stream_geometry_builder(
        &self,
        s: Option<&dyn IGeometryImpl>,
    ) -> Box<dyn IStreamGeometryBuilder> {
        Box::new(Builder::new(s))
    }
    fn create_geometry_group(
        &self,
        rule: FillRule,
        children: &[Arc<dyn IGeometryImpl>],
    ) -> Arc<dyn IGeometryImpl> {
        let mut stroke = BezPath::new();
        let mut fill = BezPath::new();
        for child in children {
            if let Some(g) = child.as_any().downcast_ref::<Geometry>() {
                stroke.extend(g.stroke.elements().iter().copied());
                if let Some(p) = &g.fill {
                    fill.extend(p.elements().iter().copied())
                }
            }
        }
        Geometry::paths(stroke, Some(fill), rule)
    }
    fn create_combined_geometry(
        &self,
        m: GeometryCombineMode,
        a: Arc<dyn IGeometryImpl>,
        b: Arc<dyn IGeometryImpl>,
    ) -> Arc<dyn IGeometryImpl> {
        Geometry::combine(&*a, &*b, m)
    }
    fn build_glyph_run_geometry(&self, g: &MediaGlyphRun) -> Arc<dyn IGeometryImpl> {
        let run = g.platform_impl();
        let run = run
            .as_any()
            .downcast_ref::<GlyphRun>()
            .expect("Direct2D glyph run");
        Geometry::new(run.outlines().clone(), FillRule::NonZero)
    }
    fn create_glyph_run(
        &self,
        f: &Rc<GlyphTypeface>,
        s: f64,
        g: &[GlyphInfo],
        p: Point,
    ) -> Arc<dyn IGlyphRunImpl> {
        Arc::new(GlyphRun::new(f, s, g, p))
    }
    fn create_render_target_bitmap(
        &self,
        s: PixelSize,
        d: Vector,
    ) -> Arc<dyn IRenderTargetBitmapImpl> {
        Arc::new(Bitmap::new(s, d))
    }
    fn create_writeable_bitmap(
        &self,
        s: PixelSize,
        d: Vector,
        f: PixelFormat,
        a: AlphaFormat,
    ) -> Arc<dyn IWriteableBitmapImpl> {
        Arc::new(writeable_bitmap::WriteableBitmap::new(s, d, f, a))
    }
    fn load_bitmap_from_file(&self, p: &str) -> io::Result<Arc<SharedBitmapImpl>> {
        self.load_bitmap(&mut std::fs::File::open(p)?)
    }
    fn load_bitmap(&self, s: &mut dyn Read) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(Bitmap::decode(s)?))
    }
    fn load_writeable_bitmap_to_width(
        &self,
        s: &mut dyn Read,
        w: i32,
        mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(resized(s, w, true, mode)?))
    }
    fn load_writeable_bitmap_to_height(
        &self,
        s: &mut dyn Read,
        h: i32,
        mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(resized(s, h, false, mode)?))
    }
    fn load_writeable_bitmap_from_file(
        &self,
        p: &str,
    ) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        self.load_writeable_bitmap(&mut std::fs::File::open(p)?)
    }
    fn load_writeable_bitmap(&self, s: &mut dyn Read) -> io::Result<Arc<dyn IWriteableBitmapImpl>> {
        Ok(Arc::new(Bitmap::decode(s)?))
    }
    fn load_bitmap_to_width(
        &self,
        s: &mut dyn Read,
        w: i32,
        mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(resized(s, w, true, mode)?))
    }
    fn load_bitmap_to_height(
        &self,
        s: &mut dyn Read,
        h: i32,
        mode: BitmapInterpolationMode,
    ) -> io::Result<Arc<SharedBitmapImpl>> {
        Ok(Arc::new(resized(s, h, false, mode)?))
    }
    fn resize_bitmap(
        &self,
        b: &dyn IBitmapImpl,
        s: PixelSize,
        mode: BitmapInterpolationMode,
    ) -> Arc<SharedBitmapImpl> {
        Arc::new(Bitmap::resized(b, s, mode))
    }
    fn load_bitmap_from_pixels(
        &self,
        f: PixelFormat,
        a: AlphaFormat,
        data: &[u8],
        s: PixelSize,
        d: Vector,
        stride: i32,
    ) -> Arc<SharedBitmapImpl> {
        Arc::new(Bitmap::from_pixels(f, a, data, s, d, stride))
    }
    fn create_backend_context(
        &self,
        graphics: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        #[cfg(feature = "gpu")]
        {
            {
                let _ = graphics;
                Rc::new(presentation::Context::new())
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            let _ = graphics;
            Rc::new(Direct2DContext)
        }
    }
    fn supports_individual_round_rects(&self) -> bool {
        true
    }
    fn supports_compositing_layers(&self) -> bool {
        true
    }
    fn default_alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }
    fn default_pixel_format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }
    fn is_supported_bitmap_pixel_format(&self, f: PixelFormat) -> bool {
        matches!(
            f,
            PixelFormat::BGRA8888 | PixelFormat::RGBA8888 | PixelFormat::RGB565
        )
    }
    fn supports_regions(&self) -> bool {
        true
    }
    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        Rc::new(region::Direct2DRegionImpl::new())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod effects_tests;
