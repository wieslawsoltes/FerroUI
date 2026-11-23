//! Apple system text/drawing backend for macOS 13+ and iOS 16+, with direct Metal GPU rendering.
#![cfg(any(target_os = "macos", target_os = "ios"))]
mod bitmap;
#[cfg(feature = "runtime-shaders")]
mod custom_shaders;
#[cfg(feature = "metal")]
mod external_objects;
mod writeable_bitmap;
#[cfg(feature = "metal")]
pub use external_objects::SharedMetalImage;
#[cfg(feature = "metal")]
mod metal;
#[cfg(feature = "metal")]
pub use metal::statistics as metal_statistics;
#[cfg(feature = "metal")]
pub use metal::submission_count as metal_submission_count;
mod drawing;
mod ffi;
mod fonts;
pub use fonts::{FontManager as QuartzFontManager, TextShaper as QuartzTextShaper};
mod geometry;
mod path_measure;
mod png_compression;
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

#[cfg(not(feature = "metal"))]
use ferroui_base::platform::surfaces::*;
#[cfg(not(feature = "metal"))]
use std::any::{Any, TypeId};

/// Whether the OS provides the public path APIs used by Quartz.
pub fn is_quartz_supported() -> bool {
    unsafe { ffi::q_supported() != 0 }
}

pub trait QuartzApplicationExtensions {
    fn use_quartz(&self) -> AppBuilder;
}
impl QuartzApplicationExtensions for AppBuilder {
    fn use_quartz(&self) -> AppBuilder {
        if !is_quartz_supported() {
            eprintln!("Quartz requires macOS 13+ or iOS 16+");
            return self.clone();
        }
        self.use_rendering_subsystem(
            || {
                FerroLocator::current_mutable()
                    .bind::<dyn IPlatformRenderInterface>()
                    .to_constant(Rc::new(QuartzRenderInterface) as Rc<dyn IPlatformRenderInterface>)
                    .bind::<dyn IFontManagerImpl>()
                    .to_constant(Rc::new(fonts::FontManager::default()) as Rc<dyn IFontManagerImpl>);
            },
            if cfg!(feature = "metal") { "QuartzMetal" } else { "Quartz" },
        )
        .use_text_shaping_subsystem(
            || {
                FerroLocator::current_mutable()
                    .bind::<dyn ITextShaperImpl>()
                    .to_constant(Rc::new(fonts::TextShaper) as Rc<dyn ITextShaperImpl>);
            },
            "CoreText",
        )
    }
}
pub struct QuartzRenderInterface;
fn resized(
    stream: &mut dyn Read,
    dimension: i32,
    width: bool,
    mode: BitmapInterpolationMode,
) -> io::Result<Bitmap> {
    if dimension <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Decode dimension must be positive",
        ));
    }
    let bitmap = Bitmap::decode(stream)?;
    let s = bitmap.pixel_size();
    let size = if width {
        PixelSize::new(
            dimension,
            (s.height as f64 * dimension as f64 / s.width as f64)
                .floor()
                .max(1.) as i32,
        )
    } else {
        PixelSize::new(
            (s.width as f64 * dimension as f64 / s.height as f64)
                .floor()
                .max(1.) as i32,
            dimension,
        )
    };
    Ok(Bitmap::resized(&bitmap, size, mode))
}
impl IPlatformRenderInterface for QuartzRenderInterface {
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
            let g = child
                .as_any()
                .downcast_ref::<Geometry>()
                .expect("Quartz geometry group requires Quartz children");
            stroke.extend(g.stroke.elements().iter().copied());
            if let Some(p) = &g.fill {
                fill.extend(p.elements().iter().copied())
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
            .expect("Quartz glyph run");
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
        #[cfg(feature = "metal")]
        {
            match metal::MetalContext::new(graphics) {
                Ok(context) => Rc::new(context),
                Err(error) => {
                    eprintln!("Quartz Metal unavailable: {error}");
                    Rc::new(UnavailableContext)
                }
            }
        }
        #[cfg(not(feature = "metal"))]
        {
            let _ = graphics;
            Rc::new(QuartzContext)
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
        Rc::new(region::QuartzRegionImpl::new())
    }
}
#[cfg(not(feature = "metal"))]
struct QuartzContext;
#[cfg(not(feature = "metal"))]
impl IOptionalFeatureProvider for QuartzContext {
    fn try_get_feature(&self, _: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}
#[cfg(not(feature = "metal"))]
impl IPlatformRenderInterfaceContext for QuartzContext {
    fn is_ready_to_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> bool {
        surfaces
            .iter()
            .any(|surface| surface.as_framebuffer_surface().is_some() && surface.is_ready())
    }
    fn create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        for s in surfaces {
            if let Some(f) = s.as_framebuffer_surface() {
                return Rc::new(Target(f.create_framebuffer_render_target()));
            }
        }
        Rc::new(UnavailableTarget)
    }
    fn create_offscreen_render_target(
        &self,
        s: PixelSize,
        scaling: Vector,
        _: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(Bitmap::new(s, scaling * 96.0))
    }
    fn is_lost(&self) -> bool {
        false
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        Some(PixelSize::new(16384, 16384))
    }
    fn dispose(&self) {}
}
#[cfg(not(feature = "metal"))]
struct Target(Rc<dyn IFramebufferRenderTarget>);
#[cfg(not(feature = "metal"))]
impl IRenderTarget for Target {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: self.0.retains_frame_contents(),
            is_suitable_for_direct_rendering: true,
        }
    }
    fn create_drawing_context(
        &self,
        scene: &RenderTargetSceneInfo,
    ) -> (
        Box<dyn IDrawingContextImpl>,
        RenderTargetDrawingContextProperties,
    ) {
        let (frame, props) = self.0.lock(scene);
        let bitmap = Bitmap::new(frame.size(), frame.dpi());
        frame.with_data(&mut |data| {
            bitmap.write(&bitmap::convert(
                data,
                frame.row_bytes() as usize,
                frame.size(),
                frame.format(),
                frame.alpha_format(),
            ))
        });
        let output = bitmap.clone();
        let done = Box::new(move || {
            let mut bytes = output.pixels();
            if frame.format() == PixelFormat::RGBA8888 {
                for p in bytes.chunks_exact_mut(4) {
                    p.swap(0, 2)
                }
            }
            let row = output.pixel_size().width as usize * 4;
            frame.with_data(&mut |data| {
                let stride = frame.row_bytes() as usize;
                for (y, source) in bytes.chunks_exact(row).enumerate() {
                    data[y * stride..y * stride + row].copy_from_slice(source)
                }
            });
            frame.dispose();
        });
        (
            Box::new(bitmap.drawing(false, Some(done))),
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: props.previous_frame_is_retained,
            },
        )
    }
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        self.0.state()
    }
    fn dispose(&self) {
        self.0.dispose()
    }
}
/// GPU-only builds report unavailable surfaces instead of silently switching to CPU rendering.
#[cfg(feature = "metal")]
struct UnavailableContext;
#[cfg(feature = "metal")]
impl IOptionalFeatureProvider for UnavailableContext {
    fn try_get_feature(&self, _: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        None
    }
}
#[cfg(feature = "metal")]
impl IPlatformRenderInterfaceContext for UnavailableContext {
    fn is_ready_to_create_render_target(
        &self,
        _: &[Arc<dyn ferroui_base::platform::surfaces::IPlatformRenderSurface>],
    ) -> bool {
        false
    }
    fn create_render_target(
        &self,
        _: &[Arc<dyn ferroui_base::platform::surfaces::IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        Rc::new(UnavailableTarget)
    }
    fn create_offscreen_render_target(
        &self,
        size: PixelSize,
        scaling: Vector,
        _: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(Bitmap::new(size, scaling * 96.))
    }
    fn is_lost(&self) -> bool {
        false
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        Some(PixelSize::new(16384, 16384))
    }
    fn dispose(&self) {}
}
struct UnavailableTarget;
impl IRenderTarget for UnavailableTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }
    fn create_drawing_context(
        &self,
        _: &RenderTargetSceneInfo,
    ) -> (
        Box<dyn IDrawingContextImpl>,
        RenderTargetDrawingContextProperties,
    ) {
        (
            Box::new(Bitmap::new(PixelSize::new(1, 1), Vector::new(96., 96.)).drawing(false, None)),
            RenderTargetDrawingContextProperties::default(),
        )
    }
    fn try_create_drawing_context(
        &self,
        _: &RenderTargetSceneInfo,
    ) -> Result<
        (
            Box<dyn IDrawingContextImpl>,
            RenderTargetDrawingContextProperties,
        ),
        RenderTargetError,
    > {
        Err(RenderTargetError::NotReady(
            ferroui_base::RenderTargetNotReadyError::new_with_message(
                "Quartz has no usable render surface",
            ),
        ))
    }
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::NOT_READY_TRY_LATER
    }
    fn dispose(&self) {}
}
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;

/// Creates an immutable IOSurface-backed synthetic camera frame, filled by Metal.
/// Development-only: requires the explicit `media-test-fixtures` feature.
#[cfg(feature = "media-test-fixtures")]
pub fn create_media_test_buffer(sequence: u64) -> Option<ferroui_media_apple::PixelBuffer> {
    extern "C" {
        fn qm_fixture_buffer(sequence: u64) -> ffi::Ptr;
    }
    // SAFETY: the fixture returns an immutable +1 CVPixelBuffer after its GPU
    // producer finishes. PixelBuffer owns that reference for its entire lease.
    unsafe { ferroui_media_apple::PixelBuffer::from_owned(qm_fixture_buffer(sequence)) }
}

#[cfg(all(test, feature = "metal"))]
mod effects_tests;
