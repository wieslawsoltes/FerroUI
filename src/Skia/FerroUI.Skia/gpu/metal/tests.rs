//! Offscreen rendering with a real Metal device through Graphite.
//!
//! The tests talk to Metal through the Objective-C runtime directly; they
//! are skipped (with a note on stderr) on machines without a Metal device.

use crate::gpu::ISkiaGrContext;
use crate::metal::{
    IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession,
};
use crate::{
    DrawingContextImpl, ImmutableBitmap, PlatformRenderInterface, SurfaceRenderTarget, SurfaceRenderTargetCreateInfo,
    WriteableBitmapImpl,
};
use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};
use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableImageBrush, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, BitmapInterpolationMode};
use ferroui_base::media::{
    AcrylicBackgroundSource, AlignmentX, AlignmentY, BoxShadows, Color, Colors, EdgeMode, GradientSpreadMethod,
    IExperimentalAcrylicMaterial, IImmutableBrush, RenderOptions, Stretch, TileMode,
};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IDrawingContextWithAcrylicLikeSupport,
    IOptionalFeatureProvider, IPlatformGraphicsContext, IPlatformRenderInterface, IPlatformRenderInterfaceContext,
    IReadableBitmapImpl, PixelFormat, RenderTargetSceneInfo,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{PixelSize, Rect, RelativePoint, RelativeUnit, RoundedRect, Vector};
use skia_safe::gpu::graphite::mtl::backend_textures;
use skia_safe::gpu::graphite::surfaces;
use skia_safe::ColorType;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ffi::{c_char, c_void};
use std::rc::{Rc, Weak};

type Id = *mut c_void;
type Sel = *mut c_void;

#[link(name = "Metal", kind = "framework")]
extern "C" {
    fn MTLCreateSystemDefaultDevice() -> Id;
}

#[link(name = "objc")]
extern "C" {
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_getClass(name: *const c_char) -> Id;
    fn objc_msgSend();
}

fn sel(name: &'static [u8]) -> Sel {
    assert!(name.ends_with(b"\0"));
    // SAFETY: `name` is a NUL-terminated string.
    unsafe { sel_registerName(name.as_ptr() as *const c_char) }
}

/// `[object selector]` returning an object.
///
/// # Safety
/// `object` must be a valid object responding to the zero-argument selector.
unsafe fn send(object: Id, selector: Sel) -> Id {
    // SAFETY: `objc_msgSend` is called through a pointer of the method's
    // actual C signature, as the Objective-C ABI requires.
    let f: unsafe extern "C" fn(Id, Sel) -> Id = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    unsafe { f(object, selector) }
}

/// Creates a BGRA8 render target texture.
///
/// # Safety
/// `device` must be a valid `id<MTLDevice>`.
unsafe fn create_texture(device: Id, width: usize, height: usize) -> Id {
    const MTL_PIXEL_FORMAT_BGRA8_UNORM: usize = 80;
    const MTL_TEXTURE_USAGE_SHADER_READ_AND_RENDER_TARGET: usize = 0x1 | 0x4;

    // SAFETY: the selectors exist on the receivers with exactly these
    // signatures (NSUInteger is `usize`, BOOL is `bool` on arm64 and `i8`
    // compatible on x86_64 for the values used).
    unsafe {
        let class = objc_getClass(b"MTLTextureDescriptor\0".as_ptr() as *const c_char);
        let make: unsafe extern "C" fn(Id, Sel, usize, usize, usize, bool) -> Id =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let descriptor = make(
            class,
            sel(b"texture2DDescriptorWithPixelFormat:width:height:mipmapped:\0"),
            MTL_PIXEL_FORMAT_BGRA8_UNORM,
            width,
            height,
            false,
        );

        let set_usage: unsafe extern "C" fn(Id, Sel, usize) = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        set_usage(descriptor, sel(b"setUsage:\0"), MTL_TEXTURE_USAGE_SHADER_READ_AND_RENDER_TARGET);

        let new_texture: unsafe extern "C" fn(Id, Sel, Id) -> Id =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        new_texture(device, sel(b"newTextureWithDescriptor:\0"), descriptor)
    }
}

/// A Metal device and command queue owned by the test.
struct TestMetalDevice {
    weak_self: Weak<TestMetalDevice>,
    device: Id,
    queue: Id,
}

impl TestMetalDevice {
    fn try_new() -> Option<Rc<Self>> {
        // SAFETY: plain framework call; returns nil when there is no device.
        let device = unsafe { MTLCreateSystemDefaultDevice() };
        if device.is_null() {
            return None;
        }

        // SAFETY: `device` is a valid `id<MTLDevice>`.
        let queue = unsafe { send(device, sel(b"newCommandQueue\0")) };
        if queue.is_null() {
            // SAFETY: balances the +1 of the creation.
            unsafe { send(device, sel(b"release\0")) };
            return None;
        }

        Some(Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), device, queue }))
    }
}

impl Drop for TestMetalDevice {
    fn drop(&mut self) {
        // SAFETY: balances the +1 references taken in `try_new`.
        unsafe {
            send(self.queue, sel(b"release\0"));
            send(self.device, sel(b"release\0"));
        }
    }
}

impl IOptionalFeatureProvider for TestMetalDevice {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IMetalDevice>() {
            let device: Rc<dyn IMetalDevice> = self.weak_self.upgrade()?;
            return Some(Rc::new(device));
        }
        None
    }
}

impl IPlatformGraphicsContext for TestMetalDevice {
    fn is_lost(&self) -> bool {
        false
    }
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalDevice for TestMetalDevice {
    fn device(&self) -> *mut c_void {
        self.device
    }
    fn command_queue(&self) -> *mut c_void {
        self.queue
    }
}

fn red() -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(Colors::RED)
}

fn aliased() -> RenderOptions {
    RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() }
}

fn pixel(bitmap: &ImmutableBitmap, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let image = bitmap.image();
    let pixmap = image.peek_pixels().expect("the bitmap is a raster image");
    let color = pixmap.get_color((x, y));
    (color.r(), color.g(), color.b(), color.a())
}

#[test]
fn graphite_offscreen_render_target_draws_and_reads_back() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };

    let render_interface = PlatformRenderInterface::default();
    let context = render_interface.create_backend_context(Some(device.clone()));
    assert!(!context.is_lost());

    let layer = context.create_offscreen_render_target(PixelSize::new(64, 64), Vector::new(1.0, 1.0), true);
    let mut drawing_context = layer.create_drawing_context();
    drawing_context.clear(Colors::WHITE);
    drawing_context.push_render_options(aliased());
    drawing_context.draw_rectangle(
        Some(&red()),
        None,
        RoundedRect::from_rect(Rect::new(8.0, 8.0, 16.0, 16.0)),
        &BoxShadows::default(),
    );
    drawing_context.pop_render_options();

    let gradient = ImmutableLinearGradientBrush::new(
        &[ImmutableGradientStop::new(0.0, Colors::BLUE), ImmutableGradientStop::new(1.0, Colors::LIME)],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative)),
        None,
    );
    drawing_context.draw_rectangle(
        Some(&gradient),
        None,
        RoundedRect::from_radius(Rect::new(0.0, 32.0, 64.0, 32.0), 8.0),
        &BoxShadows::default(),
    );

    // A layer of the GPU context, blitted back.
    let inner = drawing_context.create_layer(PixelSize::new(64, 64));
    let mut inner_context = inner.create_drawing_context();
    inner_context.push_render_options(aliased());
    inner_context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::BLACK)),
        None,
        RoundedRect::from_rect(Rect::new(40.0, 8.0, 16.0, 16.0)),
        &BoxShadows::default(),
    );
    inner_context.pop_render_options();
    inner_context.dispose();
    inner.blit(&mut *drawing_context);
    inner.dispose();

    drawing_context.dispose();

    let surface_render_target = layer.as_any().downcast_ref::<SurfaceRenderTarget>().unwrap();
    assert!(surface_render_target.has_render_context_affinity());

    let snapshot = surface_render_target.create_non_affined_snapshot();
    let snapshot = snapshot.as_any().downcast_ref::<ImmutableBitmap>().unwrap();
    assert_eq!(PixelSize::new(64, 64), snapshot.pixel_size());

    assert_eq!((255, 255, 255, 255), pixel(snapshot, 2, 2));
    assert_eq!((255, 0, 0, 255), pixel(snapshot, 16, 16));
    assert_eq!((255, 255, 255, 255), pixel(snapshot, 30, 16));
    assert_eq!((0, 0, 0, 255), pixel(snapshot, 48, 16));

    let (r, g, b, _) = pixel(snapshot, 1, 48);
    assert!(b > 235 && g < 20 && r < 20, "gradient start: {:?}", pixel(snapshot, 1, 48));
    let (r, g, b, _) = pixel(snapshot, 62, 48);
    assert!(g > 235 && b < 20 && r < 20, "gradient end: {:?}", pixel(snapshot, 62, 48));
    // The rounded corner of the gradient rectangle shows the background.
    assert_eq!((255, 255, 255, 255), pixel(snapshot, 0, 63));

    // The same contents can be encoded.
    let mut encoded = Vec::new();
    surface_render_target.save(&mut encoded, &BitmapEncoderOptions::Png(Default::default())).unwrap();
    let decoded = ImmutableBitmap::from_stream(&mut encoded.as_slice()).unwrap();
    assert_eq!((255, 0, 0, 255), pixel(&decoded, 16, 16));

    layer.dispose();
    context.dispose();
}

/// A Metal surface that renders to a texture owned by the test.
///
/// A surface is shared between the threads: the texture is kept as the
/// value of its handle, and the count of presentations is atomic.
#[derive(Clone)]
struct TextureSurface {
    texture: usize,
    size: PixelSize,
    presented: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl IPlatformRenderSurface for TextureSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            // The view is a handle of the calling thread to the same surface.
            let surface: Rc<dyn IMetalPlatformSurface> = Rc::new(self.clone());
            return Some(Rc::new(surface));
        }
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalPlatformSurface for TextureSurface {
    fn create_metal_render_target(&self, _device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        Rc::new(TextureRenderTarget { texture: self.texture as Id, size: self.size, presented: self.presented.clone() })
    }
}

struct TextureRenderTarget {
    texture: Id,
    size: PixelSize,
    presented: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl IPlatformRenderSurfaceRenderTarget for TextureRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for TextureRenderTarget {
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        Rc::new(TextureSession { texture: self.texture, size: self.size, presented: self.presented.clone() })
    }
    fn dispose(&self) {}
}

struct TextureSession {
    texture: Id,
    size: PixelSize,
    presented: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl IMetalPlatformSurfaceRenderingSession for TextureSession {
    fn texture(&self) -> *mut c_void {
        self.texture
    }
    fn size(&self) -> PixelSize {
        self.size
    }
    fn scaling(&self) -> f64 {
        2.0
    }
    fn is_y_flipped(&self) -> bool {
        false
    }
    fn dispose(&self) {
        self.presented.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
fn graphite_renders_to_a_metal_surface_texture() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };

    let size = PixelSize::new(32, 32);
    // SAFETY: the device handle is valid for the lifetime of `device`.
    let texture = unsafe { create_texture(device.device, 32, 32) };
    assert!(!texture.is_null());

    let presented = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let surface: std::sync::Arc<dyn IPlatformRenderSurface> =
        std::sync::Arc::new(TextureSurface { texture: texture as usize, size, presented: presented.clone() });

    let gpu = super::SkiaMetalGpu::new(device.clone(), None, None);
    let gr_context = gpu.gr_context();
    let context = crate::SkiaContext::new(Some(gpu));

    assert!(context.is_ready_to_create_render_target(std::slice::from_ref(&surface)));
    let render_target = context.create_render_target(std::slice::from_ref(&surface));

    let scene_info = RenderTargetSceneInfo::new(size, 2.0, CompositionTransparencyLevel::None);
    let (mut drawing_context, _) = render_target.create_drawing_context(&scene_info);
    drawing_context.clear(Colors::BLUE);
    drawing_context.push_render_options(aliased());
    drawing_context.draw_rectangle(
        Some(&red()),
        None,
        RoundedRect::from_rect(Rect::new(0.0, 0.0, 16.0, 16.0)),
        &BoxShadows::default(),
    );
    drawing_context.pop_render_options();
    assert_eq!(0, presented.load(std::sync::atomic::Ordering::SeqCst));
    drawing_context.dispose();

    // Disposing the drawing context flushes to the GPU and ends the session.
    assert_eq!(1, presented.load(std::sync::atomic::Ordering::SeqCst));

    // Read the texture back through a second surface over it.
    // SAFETY: `texture` is alive until it is released below.
    let backend_texture = unsafe { backend_textures::make_metal((32, 32), texture) };
    let mut readback = gr_context
        .with_recorder(|recorder| {
            surfaces::wrap_backend_texture(recorder, &backend_texture, ColorType::BGRA8888, None, None)
        })
        .expect("the texture can be wrapped");
    let image = gr_context.snapshot_to_raster(&mut readback).expect("the texture can be read back");
    let snapshot = ImmutableBitmap::from_image(image.clone(), None);
    let pixmap = image.peek_pixels().unwrap();
    let color = |x: i32, y: i32| pixmap.get_color((x, y));

    assert_eq!(skia_safe::Color::RED, color(8, 8));
    assert_eq!(skia_safe::Color::BLUE, color(24, 8));
    assert_eq!(skia_safe::Color::BLUE, color(8, 24));
    assert_eq!(PixelSize::new(32, 32), snapshot.pixel_size());

    drop(readback);
    render_target.dispose();
    context.dispose();
    drop(gr_context);

    // SAFETY: balances the +1 of `newTextureWithDescriptor:`; Skia no longer
    // references the texture.
    unsafe { send(texture, sel(b"release\0")) };
}

// ---------------------------------------------------------------------------
// Raster images through Graphite
// ---------------------------------------------------------------------------

fn no_interpolation() -> RenderOptions {
    RenderOptions {
        edge_mode: EdgeMode::Aliased,
        bitmap_interpolation_mode: BitmapInterpolationMode::None,
        ..RenderOptions::default()
    }
}

/// A 4x4 bitmap: the left half red, the right half blue.
fn two_tone_bitmap() -> std::sync::Arc<ferroui_base::platform::SharedBitmapImpl> {
    let mut data = Vec::new();
    for _ in 0..4 {
        for x in 0..4 {
            // BGRA, premultiplied.
            data.extend_from_slice(if x < 2 { &[0u8, 0, 255, 255] } else { &[255u8, 0, 0, 255] });
        }
    }

    std::sync::Arc::new(
        ImmutableBitmap::from_pixels(
            PixelSize::new(4, 4),
            Vector::new(96.0, 96.0),
            16,
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
            &data,
        )
        .unwrap(),
    )
}

/// Fills a 4x4 writeable bitmap with a BGRA color.
fn fill_writeable_bitmap(bitmap: &WriteableBitmapImpl, bgra: [u8; 4]) {
    let framebuffer = bitmap.lock();
    framebuffer.with_data(&mut |pixels| {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&bgra);
        }
    });
    framebuffer.dispose();
}

/// A Graphite context on the Metal device of the test, and an offscreen
/// render target of it.
struct GraphiteTarget {
    gr_context: Rc<crate::gpu::graphite::GraphiteGrContext>,
    context: crate::SkiaContext,
    layer: Rc<dyn IDrawingContextLayerImpl>,
}

impl GraphiteTarget {
    fn new(device: &Rc<TestMetalDevice>) -> Self {
        let gpu = super::SkiaMetalGpu::new(device.clone(), None, None);
        let gr_context = gpu.gr_context();
        let context = crate::SkiaContext::new(Some(gpu));
        let layer = context.create_offscreen_render_target(PixelSize::new(64, 64), Vector::new(1.0, 1.0), true);
        Self { gr_context, context, layer }
    }

    /// Draws onto the render target, without interpolation and aliased.
    fn draw(&self, draw: impl FnOnce(&mut dyn IDrawingContextImpl)) {
        let mut drawing_context = self.layer.create_drawing_context();
        drawing_context.push_render_options(no_interpolation());
        draw(&mut *drawing_context);
        drawing_context.pop_render_options();
        drawing_context.dispose();
    }

    /// Reads the render target back from the GPU.
    fn snapshot(&self) -> std::sync::Arc<ferroui_base::platform::SharedBitmapImpl> {
        self.layer.as_any().downcast_ref::<SurfaceRenderTarget>().unwrap().create_non_affined_snapshot()
    }

    fn pixel(&self, x: i32, y: i32) -> (u8, u8, u8, u8) {
        let snapshot = self.snapshot();
        pixel(snapshot.as_any().downcast_ref::<ImmutableBitmap>().unwrap(), x, y)
    }

    fn dispose(self) {
        self.layer.dispose();
        self.context.dispose();
    }
}

const WHITE: (u8, u8, u8, u8) = (255, 255, 255, 255);
const RED: (u8, u8, u8, u8) = (255, 0, 0, 255);
const BLUE: (u8, u8, u8, u8) = (0, 0, 255, 255);
const LIME: (u8, u8, u8, u8) = (0, 255, 0, 255);

#[test]
fn graphite_draws_raster_bitmaps() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);

    let immutable = two_tone_bitmap();
    let writeable = WriteableBitmapImpl::new(
        PixelSize::new(4, 4),
        Vector::new(96.0, 96.0),
        PixelFormat::BGRA8888,
        AlphaFormat::Premul,
    );
    fill_writeable_bitmap(&writeable, [0, 255, 0, 255]);

    let source = Rect::new(0.0, 0.0, 4.0, 4.0);
    assert_eq!(0, target.gr_context.uploaded_image_count());
    target.draw(|context| {
        context.clear(Colors::WHITE);
        context.draw_bitmap(&*immutable, 1.0, source, Rect::new(8.0, 8.0, 16.0, 16.0));
        context.draw_bitmap(&*writeable, 1.0, source, Rect::new(40.0, 8.0, 16.0, 16.0));
        // A part of the bitmap, and the bitmap a second time: its texture is
        // uploaded once.
        context.draw_bitmap(&*immutable, 1.0, Rect::new(2.0, 0.0, 2.0, 4.0), Rect::new(8.0, 40.0, 16.0, 16.0));
    });
    assert_eq!(2, target.gr_context.uploaded_image_count());

    assert_eq!(WHITE, target.pixel(2, 2));
    assert_eq!(RED, target.pixel(12, 16));
    assert_eq!(BLUE, target.pixel(20, 16));
    assert_eq!(WHITE, target.pixel(30, 16));
    assert_eq!(LIME, target.pixel(48, 16));
    assert_eq!(BLUE, target.pixel(12, 48));
    assert_eq!(BLUE, target.pixel(20, 48));
    assert_eq!(WHITE, target.pixel(30, 48));

    // The textures live as long as the images they were made from: reading
    // back flushed the context, and both images are still held.
    assert_eq!(2, target.gr_context.uploaded_image_count());

    // New pixels of the writeable bitmap are a new image: it is uploaded
    // when it is drawn, and the texture of the old pixels goes with the next
    // flush.
    fill_writeable_bitmap(&writeable, [255, 0, 0, 255]);
    target.draw(|context| {
        context.draw_bitmap(&*writeable, 1.0, source, Rect::new(40.0, 40.0, 16.0, 16.0));
    });
    assert_eq!(BLUE, target.pixel(48, 48));
    assert_eq!(LIME, target.pixel(48, 16));
    assert_eq!(2, target.gr_context.uploaded_image_count());

    // A disposed bitmap releases its texture with the next flush.
    immutable.dispose();
    IBitmapImpl::dispose(&*writeable);
    target.gr_context.flush();
    assert_eq!(0, target.gr_context.uploaded_image_count());

    target.dispose();
}

#[test]
fn graphite_draws_an_in_memory_render_target() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);

    // A render target without a GPU context: its snapshot is a raster image.
    let raster = SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
        width: 8,
        height: 8,
        dpi: Vector::new(96.0, 96.0),
        format: None,
        disable_text_lcd_rendering: true,
        gr_context: None,
        gpu: None,
        session: None,
        disable_manual_fbo: true,
        use_scaled_drawing: false,
    });
    assert!(!raster.has_render_context_affinity());
    let fill = |color| {
        let mut raster_context = raster.create_drawing_context();
        raster_context.clear(color);
        raster_context.dispose();
    };
    fill(Colors::LIME);

    target.draw(|context| {
        context.clear(Colors::WHITE);
        context.draw_bitmap(&raster, 1.0, Rect::new(0.0, 0.0, 8.0, 8.0), Rect::new(16.0, 16.0, 32.0, 32.0));
    });
    assert_eq!(WHITE, target.pixel(8, 8));
    assert_eq!(LIME, target.pixel(32, 32));
    assert_eq!(1, target.gr_context.uploaded_image_count());

    // Drawing into the render target again makes a new snapshot of it.
    fill(Colors::BLUE);
    target.draw(|context| {
        context.draw_bitmap(&raster, 1.0, Rect::new(0.0, 0.0, 8.0, 8.0), Rect::new(16.0, 16.0, 32.0, 32.0));
    });
    assert_eq!(BLUE, target.pixel(32, 32));
    assert_eq!(1, target.gr_context.uploaded_image_count());

    // A layer that cannot blit is drawn as its snapshot: an in-memory layer
    // onto the canvas of the GPU context too.
    fill(Colors::RED);
    target.draw(|context| {
        context.clear(Colors::WHITE);
        raster.blit(context);
    });
    assert_eq!(RED, target.pixel(4, 4));
    assert_eq!(WHITE, target.pixel(12, 12));

    IBitmapImpl::dispose(&raster);
    target.gr_context.flush();
    assert_eq!(0, target.gr_context.uploaded_image_count());
    target.dispose();
}

#[test]
fn graphite_fills_with_an_image_brush() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);

    let bitmap = Rc::new(Bitmap::from_impl(two_tone_bitmap()));
    let brush = ImmutableImageBrush::new(
        Some(bitmap),
        AlignmentX::Center,
        AlignmentY::Center,
        None,
        1.0,
        None,
        RelativePoint::default(),
        None,
        Stretch::Fill,
        TileMode::None,
        None,
    );

    target.draw(|context| {
        context.clear(Colors::WHITE);
        // As a fill.
        context.draw_rectangle(
            Some(&brush),
            None,
            RoundedRect::from_rect(Rect::new(0.0, 0.0, 32.0, 32.0)),
            &BoxShadows::default(),
        );
        // As a stroke: the image is laid out over the stroked rectangle.
        let stroke_brush: Rc<dyn IImmutableBrush> = Rc::new(brush.clone());
        let pen = ImmutablePen::with_brush(Some(stroke_brush), 8.0);
        context.draw_rectangle(
            None,
            Some(&pen),
            RoundedRect::from_rect(Rect::new(36.0, 36.0, 24.0, 24.0)),
            &BoxShadows::default(),
        );
    });

    assert_eq!(RED, target.pixel(8, 16));
    assert_eq!(BLUE, target.pixel(24, 16));
    assert_eq!(WHITE, target.pixel(48, 16));
    assert_eq!(RED, target.pixel(36, 48));
    assert_eq!(BLUE, target.pixel(60, 48));
    assert_eq!(WHITE, target.pixel(48, 48));

    target.dispose();
}

#[test]
fn graphite_uploads_encoded_images_and_draws_the_acrylic_noise() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);

    // An image that is decoded on demand has a texture form too, and the
    // form of a texture is the texture itself.
    let encoded = {
        let mut bytes = Vec::new();
        two_tone_bitmap().save(&mut bytes, &BitmapEncoderOptions::Png(Default::default())).unwrap();
        skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes)).unwrap()
    };
    assert!(!encoded.is_texture_backed());
    let texture = target.gr_context.drawable_image(&encoded, false);
    assert!(texture.is_texture_backed());
    assert_ne!(encoded.unique_id(), texture.unique_id());
    assert_eq!(texture.unique_id(), target.gr_context.drawable_image(&encoded, false).unique_id());
    assert_eq!(texture.unique_id(), target.gr_context.drawable_image(&texture, false).unique_id());
    assert_eq!(texture.unique_id(), target.gr_context.drawable_image(&texture, true).unique_id());
    // The form with mipmaps is another texture, made once.
    let mipmapped = target.gr_context.drawable_image(&encoded, true);
    assert!(mipmapped.is_texture_backed() && mipmapped.has_mipmaps());
    assert!(!texture.has_mipmaps());
    assert_eq!(mipmapped.unique_id(), target.gr_context.drawable_image(&encoded, true).unique_id());
    drop(mipmapped);
    assert_eq!(1, target.gr_context.uploaded_image_count());
    drop(encoded);
    target.gr_context.flush();
    assert_eq!(0, target.gr_context.uploaded_image_count());
    drop(texture);

    struct Material;

    impl IExperimentalAcrylicMaterial for Material {
        fn background_source(&self) -> AcrylicBackgroundSource {
            AcrylicBackgroundSource::None
        }
        fn tint_color(&self) -> Color {
            Colors::BLUE
        }
        fn tint_opacity(&self) -> f64 {
            1.0
        }
        fn material_color(&self) -> Color {
            Colors::RED
        }
        fn fallback_color(&self) -> Color {
            Colors::GRAY
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    target.draw(|context| {
        context.clear(Colors::WHITE);
        let context = context.as_any_mut().downcast_mut::<DrawingContextImpl>().unwrap();
        context.draw_rectangle_with_material(&Material, RoundedRect::from_rect(Rect::new(8.0, 8.0, 48.0, 48.0)));
    });

    let snapshot = target.snapshot();
    let snapshot = snapshot.as_any().downcast_ref::<ImmutableBitmap>().unwrap();
    let (r, g, b, a) = pixel(snapshot, 32, 32);
    assert!(a == 255 && b > 240 && r < 12 && g < 12, "tinted blue: {:?}", (r, g, b, a));
    assert_eq!(WHITE, pixel(snapshot, 2, 2));
    // The noise makes the fill slightly uneven.
    let distinct: std::collections::HashSet<_> = (10..54).map(|x| pixel(snapshot, x, 32)).collect();
    assert!(distinct.len() > 1, "the noise texture is applied");

    target.dispose();
}

/// A 64x64 bitmap of vertical stripes four pixels wide, black and white,
/// placed so that the middle of every block of eight columns lies inside a
/// black stripe.
fn striped_bitmap() -> std::sync::Arc<ferroui_base::platform::SharedBitmapImpl> {
    let mut data = Vec::new();
    for _ in 0..64 {
        for x in 0..64 {
            let black = (2..6).contains(&(x % 8));
            data.extend_from_slice(if black { &[0u8, 0, 0, 255] } else { &[255u8, 255, 255, 255] });
        }
    }

    std::sync::Arc::new(
        ImmutableBitmap::from_pixels(
            PixelSize::new(64, 64),
            Vector::new(96.0, 96.0),
            256,
            PixelFormat::BGRA8888,
            AlphaFormat::Premul,
            &data,
        )
        .unwrap(),
    )
}

#[test]
fn graphite_downscales_with_mipmaps_like_the_raster_backend() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);
    let bitmap = striped_bitmap();

    let raster = SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
        width: 64,
        height: 64,
        dpi: Vector::new(96.0, 96.0),
        format: None,
        disable_text_lcd_rendering: true,
        gr_context: None,
        gpu: None,
        session: None,
        disable_manual_fbo: true,
        use_scaled_drawing: false,
    });

    // The bitmap at an eighth of its size, once per quality.
    let draw = |context: &mut dyn IDrawingContextImpl| {
        context.clear(Colors::WHITE);
        for (mode, y) in [(BitmapInterpolationMode::LowQuality, 0.0), (BitmapInterpolationMode::MediumQuality, 16.0)] {
            context.push_render_options(RenderOptions {
                edge_mode: EdgeMode::Aliased,
                bitmap_interpolation_mode: mode,
                ..RenderOptions::default()
            });
            context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 64.0, 64.0), Rect::new(0.0, y, 8.0, 8.0));
            context.pop_render_options();
        }
    };
    {
        let mut raster_context = raster.create_drawing_context();
        draw(&mut *raster_context);
        raster_context.dispose();
    }
    let mut drawing_context = target.layer.create_drawing_context();
    draw(&mut *drawing_context);
    drawing_context.dispose();
    drop(drawing_context);

    let expected = raster.snapshot_image();
    let expected = expected.peek_pixels().expect("the snapshot of an in-memory surface is a raster image");
    let expected = |x: i32, y: i32| {
        let color = expected.get_color((x, y));
        (color.r(), color.g(), color.b(), color.a())
    };
    let snapshot = target.snapshot();
    let snapshot = snapshot.as_any().downcast_ref::<ImmutableBitmap>().unwrap();

    // Without mipmaps a sample falls inside a black stripe; with them every
    // pixel is the average of a black and a white stripe.
    for x in 0..8 {
        assert!(pixel(snapshot, x, 4).0 < 16, "low quality at {x}: {:?}", pixel(snapshot, x, 4));
        let (actual, raster) = (pixel(snapshot, x, 20), expected(x, 20));
        assert!((100..=156).contains(&raster.0), "the raster backend averages at {x}: {raster:?}");
        for (a, b) in [(actual.0, raster.0), (actual.1, raster.1), (actual.2, raster.2), (actual.3, raster.3)] {
            assert!(a.abs_diff(b) <= 24, "medium quality at {x}: {actual:?}, the raster backend: {raster:?}");
        }
    }
    // One image, uploaded once; the mipmaps were made for the second draw.
    assert_eq!(1, target.gr_context.uploaded_image_count());

    IBitmapImpl::dispose(&raster);
    target.dispose();
}

/// Collects what is logged on the thread of the test.
#[derive(Default)]
struct LogSink {
    messages: RefCell<Vec<String>>,
}

impl ILogSink for LogSink {
    fn is_enabled(&self, _level: LogEventLevel, _area: &str) -> bool {
        true
    }

    fn log(&self, _level: LogEventLevel, _area: &str, _source: Option<&dyn Any>, message_template: &str) {
        self.messages.borrow_mut().push(message_template.to_string());
    }

    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        message_template: &str,
        _property_values: &[&dyn std::fmt::Display],
    ) {
        self.log(level, area, source, message_template);
    }
}

#[test]
fn graphite_reports_an_image_it_cannot_upload_once() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);
    let sink = Rc::new(LogSink::default());
    let previous = Logger::set_thread_sink(Some(sink.clone()));

    // Wider than any texture of the device.
    let info = skia_safe::ImageInfo::new_a8((100_000, 1));
    let too_wide = skia_safe::images::raster_from_data(&info, skia_safe::Data::new_copy(&vec![255u8; 100_000]), 100_000)
        .expect("a raster image");

    let drawn = target.gr_context.drawable_image(&too_wide, false);
    assert!(!drawn.is_texture_backed());
    assert_eq!(too_wide.unique_id(), drawn.unique_id());
    drop(drawn);
    // The failure is remembered: asking again neither uploads nor reports.
    for mipmapped in [false, true, false] {
        assert_eq!(too_wide.unique_id(), target.gr_context.drawable_image(&too_wide, mipmapped).unique_id());
    }
    assert_eq!(1, target.gr_context.uploaded_image_count());
    let messages = sink.messages.borrow().clone();
    assert_eq!(1, messages.len(), "{messages:?}");
    assert!(messages[0].starts_with("Unable to upload"), "{messages:?}");

    // The entry goes with the image, as that of an uploaded one.
    drop(too_wide);
    target.gr_context.flush();
    assert_eq!(0, target.gr_context.uploaded_image_count());

    Logger::set_thread_sink(previous);
    target.dispose();
}
