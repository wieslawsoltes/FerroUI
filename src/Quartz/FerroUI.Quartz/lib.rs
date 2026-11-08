//! Experimental macOS 13+ system text/drawing backend, with optional direct Metal GPU rendering.
#![cfg(target_os = "macos")]
mod bitmap;
#[cfg(feature = "runtime-shaders")]
mod custom_shaders;
#[cfg(feature = "metal")]
mod external_objects;
#[cfg(feature = "metal")]
mod metal;
#[cfg(feature = "metal")]
pub use metal::statistics as metal_statistics;
#[cfg(feature = "metal")]
pub use metal::submission_count as metal_submission_count;
mod drawing;
mod ffi;
mod fonts;
pub use fonts::FontManager as QuartzFontManager;
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
    any::{Any, TypeId},
    io::{self, Read},
    rc::Rc,
    sync::{Arc, Mutex, PoisonError},
};

use ferroui_base::platform::surfaces::*;

/// Prints `message` once per process: the renderer degrades instead of failing.
pub(crate) fn warn_once(message: &'static str) {
    static WARNED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut warned = WARNED.lock().unwrap_or_else(PoisonError::into_inner);
    if !warned.contains(&message) {
        eprintln!("Quartz: {message}");
        warned.push(message);
    }
}

/// Whether this macOS release can run the Quartz renderer: macOS 13 or later.
pub fn is_quartz_supported() -> bool {
    // SAFETY: an availability query without arguments.
    unsafe { ffi::q_supported() != 0 }
}

pub trait QuartzApplicationExtensions {
    /// Selects Core Graphics for rendering (Metal with the `metal` feature) and Core Text for
    /// text shaping.
    ///
    /// On macOS earlier than 13 this logs once and returns the builder unchanged, so whatever
    /// rendering and text shaping subsystems were selected before stay in effect. Call
    /// [`is_quartz_supported`] first to choose another backend explicitly.
    fn use_quartz(&self) -> AppBuilder;
}
impl QuartzApplicationExtensions for AppBuilder {
    fn use_quartz(&self) -> AppBuilder {
        if !is_quartz_supported() {
            warn_once("the renderer requires macOS 13 or later and was not selected");
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
        match run.as_any().downcast_ref::<GlyphRun>() {
            Some(run) => Geometry::new(run.outlines.clone(), FillRule::NonZero),
            None => Geometry::new(BezPath::new(), FillRule::NonZero),
        }
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
        // Pixels are stored as premultiplied BGRA; `lock` exposes `f` and `a` and converts
        // back when the lock is disposed.
        Arc::new(Bitmap::with_format(s, d, f, a))
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
            // Without a usable Metal device the context still draws into framebuffer
            // surfaces, on the CPU when no GPU renderer exists at all.
            let metal = match metal::MetalContext::new(graphics) {
                Ok(metal) => Some(metal),
                Err(error) => {
                    eprintln!("Quartz: Metal rendering unavailable ({error}); using framebuffer surfaces");
                    None
                }
            };
            Rc::new(QuartzContext { metal })
        }
        #[cfg(not(feature = "metal"))]
        {
            let _ = graphics;
            Rc::new(QuartzContext {})
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
        f == PixelFormat::BGRA8888 || f == PixelFormat::RGBA8888
    }
    fn supports_regions(&self) -> bool {
        true
    }
    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        Rc::new(region::QuartzRegionImpl::new())
    }
}
/// The render context: direct Metal presentation when the platform supplied a usable Metal
/// device, framebuffer surfaces otherwise.
struct QuartzContext {
    #[cfg(feature = "metal")]
    metal: Option<metal::MetalContext>,
}
impl IOptionalFeatureProvider for QuartzContext {
    fn try_get_feature(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        #[cfg(feature = "metal")]
        if let Some(metal) = &self.metal {
            return metal.try_get_feature(kind);
        }
        let _ = kind;
        None
    }
}
fn framebuffer_ready(surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
    surfaces
        .iter()
        .any(|surface| surface.as_framebuffer_surface().is_some() && surface.is_ready())
}
impl IPlatformRenderInterfaceContext for QuartzContext {
    fn public_features(&self) -> std::collections::HashMap<TypeId, Rc<dyn Any>> {
        #[cfg(feature = "metal")]
        if let Some(metal) = &self.metal {
            return metal.public_features();
        }
        std::collections::HashMap::new()
    }
    fn is_ready_to_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> bool {
        #[cfg(feature = "metal")]
        if self.metal.as_ref().is_some_and(|metal| metal.is_ready(surfaces)) {
            return true;
        }
        framebuffer_ready(surfaces)
    }
    fn create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Rc<dyn IRenderTarget> {
        #[cfg(feature = "metal")]
        if let Some(target) = self
            .metal
            .as_ref()
            .and_then(|metal| metal.create_render_target(surfaces))
        {
            return target;
        }
        for s in surfaces {
            if let Some(f) = s.as_framebuffer_surface() {
                return Rc::new(Target(f.create_framebuffer_render_target()));
            }
        }
        warn_once("no surface to render to: select macOS Metal or Software rendering");
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
        #[cfg(feature = "metal")]
        if let Some(metal) = &self.metal {
            return metal.is_lost();
        }
        false
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        #[cfg(feature = "metal")]
        {
            // SAFETY: a query of the native renderer.
            let max = unsafe { ffi::qm_max_texture_size() };
            if max > 0 {
                let side = i32::try_from(max).unwrap_or(i32::MAX);
                return Some(PixelSize::new(side, side));
            }
        }
        Some(PixelSize::new(16384, 16384))
    }
    fn dispose(&self) {}
}
/// A framebuffer surface: the frame is drawn into a Quartz bitmap (on the GPU when a renderer
/// exists) and copied into the framebuffer when the drawing context is disposed.
struct Target(Rc<dyn IFramebufferRenderTarget>);
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
        let layout_supported = matches!(
            frame.format(),
            PixelFormat::BGRA8888 | PixelFormat::RGBA8888
        );
        if !layout_supported {
            warn_once("the framebuffer pixel format is not BGRA8888 or RGBA8888; frames are not shown");
        }
        if props.previous_frame_is_retained && layout_supported {
            frame.with_data(&mut |data| {
                match bitmap::convert(
                    data,
                    frame.row_bytes().max(0) as usize,
                    frame.size(),
                    frame.format(),
                    frame.alpha_format(),
                ) {
                    Ok(pixels) => bitmap.write(&pixels),
                    Err(_) => warn_once("the framebuffer is smaller than its size and stride"),
                }
            });
        }
        let output = bitmap.clone();
        let done = Box::new(move || {
            if layout_supported {
                let bytes = bitmap::export(&output.pixels(), frame.format(), frame.alpha_format());
                let row = output.pixel_size().width.max(0) as usize * 4;
                frame.with_data(&mut |data| {
                    let stride = frame.row_bytes().max(0) as usize;
                    if row == 0 || stride < row {
                        return;
                    }
                    for (y, source) in bytes.chunks_exact(row).enumerate() {
                        if let Some(target) = data.get_mut(y * stride..y * stride + row) {
                            target.copy_from_slice(source)
                        }
                    }
                });
            }
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
/// Stands in when none of the surfaces can be drawn to: the compositor is told the target is
/// not ready, and a direct drawing context draws nowhere.
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
        let bitmap = Bitmap::new(PixelSize::new(1, 1), Vector::new(96., 96.));
        (
            Box::new(bitmap.drawing(false, None)),
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
                "Quartz has no Metal or framebuffer surface to render to",
            ),
        ))
    }
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::NOT_READY_TRY_LATER
    }
    fn dispose(&self) {}
}
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
