//! Offscreen rendering with a real Metal device through Graphite.
//!
//! The tests talk to Metal through the Objective-C runtime directly; they
//! are skipped (with a note on stderr) on machines without a Metal device.

use crate::gpu::ISkiaGrContext;
use crate::metal::{
    IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession,
};
use crate::{ImmutableBitmap, PlatformRenderInterface, SurfaceRenderTarget};
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::media::{BoxShadows, Colors, EdgeMode, GradientSpreadMethod, RenderOptions};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{
    IBitmapImpl, IOptionalFeatureProvider, IPlatformGraphicsContext, IPlatformRenderInterface,
    RenderTargetSceneInfo,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{PixelSize, Rect, RelativePoint, RelativeUnit, RoundedRect, Vector};
use skia_safe::gpu::graphite::mtl::backend_textures;
use skia_safe::gpu::graphite::surfaces;
use skia_safe::ColorType;
use std::any::{Any, TypeId};
use std::cell::Cell;
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
struct TextureSurface {
    weak_self: Weak<TextureSurface>,
    texture: Id,
    size: PixelSize,
    presented: Rc<Cell<u32>>,
}

impl IPlatformRenderSurface for TextureSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            let surface: Rc<dyn IMetalPlatformSurface> = self.weak_self.upgrade()?;
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
        Rc::new(TextureRenderTarget { texture: self.texture, size: self.size, presented: self.presented.clone() })
    }
}

struct TextureRenderTarget {
    texture: Id,
    size: PixelSize,
    presented: Rc<Cell<u32>>,
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
    presented: Rc<Cell<u32>>,
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
        self.presented.set(self.presented.get() + 1);
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

    let presented = Rc::new(Cell::new(0));
    let surface: Rc<dyn IPlatformRenderSurface> = Rc::new_cyclic(|weak_self| TextureSurface {
        weak_self: weak_self.clone(),
        texture,
        size,
        presented: presented.clone(),
    });

    let gpu = super::SkiaMetalGpu::new(device.clone(), None, None);
    let gr_context = gpu.gr_context();
    let context = crate::SkiaContext::new(Some(gpu));

    use ferroui_base::platform::IPlatformRenderInterfaceContext;
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
    assert_eq!(0, presented.get());
    drawing_context.dispose();

    // Disposing the drawing context flushes to the GPU and ends the session.
    assert_eq!(1, presented.get());

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
