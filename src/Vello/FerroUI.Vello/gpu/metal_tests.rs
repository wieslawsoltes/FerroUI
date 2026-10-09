//! The path of a window without a window: a Metal device and a command
//! queue as a platform hands them out, and a render target whose sessions
//! give a texture of that device that is not on screen, as the session of
//! a top-level gives the texture of a drawable. Frames are drawn through
//! the render interface, the backend context, the render target and the
//! drawing context, and the texture is read back.
//!
//! Skipped with a message on a machine without a Metal device.

use super::*;
use crate::{PlatformRenderInterface, VelloOptions};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color, Colors};
use ferroui_base::platform::surfaces::IPlatformRenderSurfaceRenderTarget;
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformRenderInterface};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{Matrix, PixelSize, Rect, RoundedRect};
use ferroui_metal::IMetalPlatformSurfaceRenderingSession;
use objc2_metal::{MTLCreateSystemDefaultDevice, MTLStorageMode, MTLTextureDescriptor, MTLTextureUsage};
use std::any::{Any, TypeId};
use std::cell::Cell;

/// One test of this file at a time: each makes its device the one scenes
/// are drawn into memory with.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A Metal device and its queue, as the platform's graphics context.
struct TestMetalDevice {
    weak_self: std::rc::Weak<TestMetalDevice>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
}

impl TestMetalDevice {
    fn new(test: &str) -> Option<Rc<Self>> {
        let Some(device) = MTLCreateSystemDefaultDevice() else {
            println!("skipped: no adapter ({test}): the system has no Metal device");
            return None;
        };
        let queue = device.newCommandQueue()?;
        Some(Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), device, queue }))
    }
}

struct Nothing;

impl IDisposable for Nothing {
    fn dispose(&self) {}
}

impl IOptionalFeatureProvider for TestMetalDevice {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IMetalDevice>() {
            let this: Rc<dyn IMetalDevice> = self.weak_self.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for TestMetalDevice {
    fn is_lost(&self) -> bool {
        false
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        Rc::new(Nothing)
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalDevice for TestMetalDevice {
    fn device(&self) -> *mut c_void {
        Retained::as_ptr(&self.device).cast_mut().cast()
    }

    fn command_queue(&self) -> *mut c_void {
        Retained::as_ptr(&self.queue).cast_mut().cast()
    }
}

/// A render target whose sessions render to a texture that is not on
/// screen, of the size and scaling the test sets: a new texture when the
/// size changed, as a layer gives drawables of its new size.
struct TestRenderTarget {
    device: Rc<TestMetalDevice>,
    size: Cell<PixelSize>,
    scaling: Cell<f64>,
    texture: RefCell<Option<Retained<ProtocolObject<dyn MTLTexture>>>>,
    sessions_begun: Cell<u32>,
    sessions_presented: Rc<Cell<u32>>,
}

impl TestRenderTarget {
    fn new(device: Rc<TestMetalDevice>, size: PixelSize, scaling: f64) -> Rc<Self> {
        Rc::new(Self {
            device,
            size: Cell::new(size),
            scaling: Cell::new(scaling),
            texture: RefCell::new(None),
            sessions_begun: Cell::new(0),
            sessions_presented: Rc::new(Cell::new(0)),
        })
    }

    fn texture(&self) -> Retained<ProtocolObject<dyn MTLTexture>> {
        let size = self.size.get();
        let mut texture = self.texture.borrow_mut();

        let fits = texture
            .as_ref()
            .is_some_and(|texture| texture.width() == size.width as usize && texture.height() == size.height as usize);
        if !fits {
            // SAFETY: a descriptor of a texture of two dimensions in a
            // format and of a size that Metal has.
            let descriptor = unsafe {
                MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                    MTLPixelFormat::BGRA8Unorm,
                    size.width as usize,
                    size.height as usize,
                    false,
                )
            };
            descriptor.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
            descriptor.setStorageMode(MTLStorageMode::Private);
            *texture = self.device.device.newTextureWithDescriptor(&descriptor);
        }

        texture.clone().unwrap_or_else(|| panic!("The Metal device made no texture"))
    }
}

impl IPlatformRenderSurfaceRenderTarget for TestRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for TestRenderTarget {
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        self.sessions_begun.set(self.sessions_begun.get() + 1);
        Rc::new(TestSession {
            texture: self.texture(),
            size: self.size.get(),
            scaling: self.scaling.get(),
            presented: self.sessions_presented.clone(),
        })
    }

    fn dispose(&self) {
        self.texture.borrow_mut().take();
    }
}

struct TestSession {
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    size: PixelSize,
    scaling: f64,
    presented: Rc<Cell<u32>>,
}

impl IMetalPlatformSurfaceRenderingSession for TestSession {
    fn texture(&self) -> *mut c_void {
        Retained::as_ptr(&self.texture).cast_mut().cast()
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        self.presented.set(self.presented.get() + 1);
    }
}

/// The backend with a window drawn in one mode, on the test device.
struct Window {
    gpu: Rc<VelloMetalGpu>,
    platform_target: Rc<TestRenderTarget>,
    render_target: Rc<dyn IRenderTarget>,
}

impl Window {
    fn new(device: &Rc<TestMetalDevice>, mode: VelloRenderingMode, size: PixelSize, scaling: f64) -> Self {
        // The context, as the compositor makes it from the graphics context
        // of the platform.
        let interface = PlatformRenderInterface::new(VelloOptions::with_rendering_mode(mode));
        let graphics_context: Rc<dyn IPlatformGraphicsContext> = device.clone();
        let context = interface.create_backend_context(Some(graphics_context));
        assert!(!context.is_lost());

        // The render target, as the context makes it for a Metal surface;
        // the surface of a top-level needs a top-level, so the target of
        // the platform is given directly.
        let gpu = VelloMetalGpu::new(device.clone()).unwrap();
        let platform_target = TestRenderTarget::new(device.clone(), size, scaling);
        let render_target: Rc<dyn IRenderTarget> =
            Rc::new(VelloMetalRenderTarget::new(gpu.clone(), platform_target.clone(), vec![mode]));

        Self { gpu, platform_target, render_target }
    }

    /// Draws a frame: white, and a blue rectangle in logical units.
    fn draw_frame(&self, frame: u32) {
        let (size, scaling) = (self.platform_target.size.get(), self.platform_target.scaling.get());
        let scene_info = RenderTargetSceneInfo::new(size, scaling, CompositionTransparencyLevel::None);
        let (mut context, _) = self.render_target.create_drawing_context(&scene_info);

        context.clear(Colors::WHITE);
        context.set_transform(Matrix::create_scale(scaling, scaling));
        context.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Color::from_argb(255, 0, 0, 200))),
            None,
            RoundedRect::from_rect(Rect::new(10.0 + f64::from(frame), 10.0, 20.0, 20.0)),
            &BoxShadows::default(),
        );
        context.dispose();
    }

    /// The pixels of the texture the frames were drawn to, as RGBA.
    fn read_back(&self) -> (PixelSize, Vec<u8>) {
        let size = self.platform_target.size.get();
        let texture = self.platform_target.texture();
        let wrapped = self.gpu.wrap_texture(Retained::as_ptr(&texture).cast_mut().cast()).unwrap();
        assert_eq!(wgpu::TextureFormat::Bgra8Unorm, wrapped.format());

        let mut pixels = vec![0u8; size.width as usize * size.height as usize * 4];
        assert!(self.gpu.device.read_texture(&wrapped, &mut pixels));
        pixels.chunks_exact_mut(4).for_each(|pixel| pixel.swap(0, 2));
        (size, pixels)
    }
}

fn pixel(size: PixelSize, pixels: &[u8], x: i32, y: i32) -> [u8; 4] {
    let index = (y * size.width + x) as usize * 4;
    [pixels[index], pixels[index + 1], pixels[index + 2], pixels[index + 3]]
}

fn window_modes(device: &VelloWgpuDevice) -> Vec<VelloRenderingMode> {
    let mut modes = vec![VelloRenderingMode::Cpu];
    if cfg!(feature = "hybrid") {
        modes.push(VelloRenderingMode::Hybrid);
    }
    if cfg!(feature = "gpu") && device.supports_compute() {
        modes.push(VelloRenderingMode::Gpu);
    }
    modes
}

#[test]
fn the_device_of_the_platform_becomes_the_device_of_the_modes() {
    let Some(device) = TestMetalDevice::new("the_device_of_the_platform_becomes_the_device_of_the_modes") else {
        return;
    };
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let gpu = VelloMetalGpu::new(device.clone()).unwrap();

    let info = gpu.device().adapter_info();
    println!("adapter of the platform device: {} ({:?})", info.name, info.backend);
    assert_eq!(wgpu::Backend::Metal, info.backend);
    assert!(!gpu.device().is_lost());

    // The device draws: the scenes that are drawn into memory by a GPU mode
    // are drawn on the device of the platform from now on.
    assert!(Arc::ptr_eq(gpu.device(), &VelloWgpuDevice::shared().unwrap()));
    let texture = gpu.device().create_rgba_texture(2, 2, wgpu::TextureUsages::COPY_DST);
    let mut pixels = vec![0u8; 16];
    assert!(gpu.device().read_texture(&texture, &mut pixels));

    // A graphics context that is no Metal device is refused.
    assert!(gpu.platform_graphics_context().as_any().is::<TestMetalDevice>());
}

#[test]
fn frames_are_drawn_into_the_texture_of_each_session_in_every_mode() {
    let Some(device) = TestMetalDevice::new("frames_are_drawn_into_the_texture_of_each_session_in_every_mode") else {
        return;
    };
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let probe = VelloMetalGpu::new(device.clone()).unwrap();

    for mode in window_modes(probe.device()) {
        let window = Window::new(&device, mode, PixelSize::new(200, 120), 2.0);

        for frame in 0..3 {
            window.draw_frame(frame);
        }
        assert_eq!(3, window.platform_target.sessions_begun.get(), "{mode:?}");
        assert_eq!(3, window.platform_target.sessions_presented.get(), "{mode:?}: every session is disposed, which presents");

        // The last frame: the rectangle at 12, 10 of 20 by 20 logical units
        // is at 24, 20 of 40 by 40 pixels at a scaling of two.
        let (size, pixels) = window.read_back();
        assert_eq!([0, 0, 200, 255], pixel(size, &pixels, 44, 40), "{mode:?}");
        assert_eq!([0, 0, 200, 255], pixel(size, &pixels, 25, 21), "{mode:?}");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 22, 40), "{mode:?}: a frame replaces the frame before");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 66, 40), "{mode:?}");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 199, 119), "{mode:?}");
        println!("{mode:?}: 3 frames of 200x120 pixels at a scaling of 2 drawn into the texture of the session and read back");
    }
}

#[test]
fn a_resized_surface_and_a_changed_scaling_are_followed_frame_by_frame() {
    let Some(device) = TestMetalDevice::new("a_resized_surface_and_a_changed_scaling_are_followed_frame_by_frame") else {
        return;
    };
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let probe = VelloMetalGpu::new(device.clone()).unwrap();

    for mode in window_modes(probe.device()) {
        let window = Window::new(&device, mode, PixelSize::new(64, 64), 1.0);
        window.draw_frame(0);
        let (size, pixels) = window.read_back();
        assert_eq!([0, 0, 200, 255], pixel(size, &pixels, 20, 20), "{mode:?}");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 40, 40), "{mode:?}");

        // Larger, at another scaling: the next session has a new texture.
        window.platform_target.size.set(PixelSize::new(301, 157));
        window.platform_target.scaling.set(1.5);
        window.draw_frame(0);
        let (size, pixels) = window.read_back();
        assert_eq!(PixelSize::new(301, 157), size);
        // 10 to 30 logical units are 15 to 45 pixels.
        assert_eq!([0, 0, 200, 255], pixel(size, &pixels, 40, 40), "{mode:?}");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 50, 40), "{mode:?}");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 300, 156), "{mode:?}");

        // Smaller again.
        window.platform_target.size.set(PixelSize::new(32, 48));
        window.platform_target.scaling.set(1.0);
        window.draw_frame(0);
        let (size, pixels) = window.read_back();
        assert_eq!(PixelSize::new(32, 48), size);
        assert_eq!([0, 0, 200, 255], pixel(size, &pixels, 20, 20), "{mode:?}");
        assert_eq!([255, 255, 255, 255], pixel(size, &pixels, 5, 40), "{mode:?}");

        window.render_target.dispose();
        assert_eq!(PlatformRenderTargetState::DISPOSED, window.render_target.platform_render_target_state());
    }
}

#[test]
fn a_lost_device_is_reported_by_the_context_and_the_render_target() {
    let Some(device) = TestMetalDevice::new("a_lost_device_is_reported_by_the_context_and_the_render_target") else {
        return;
    };
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let gpu = VelloMetalGpu::new(device.clone()).unwrap();
    let context = crate::VelloContext::with_gpu(gpu.clone(), vec![VelloRenderingMode::Cpu]);
    let platform_target = TestRenderTarget::new(device.clone(), PixelSize::new(8, 8), 1.0);
    let render_target = VelloMetalRenderTarget::new(gpu.clone(), platform_target, vec![VelloRenderingMode::Cpu]);

    use ferroui_base::platform::IPlatformRenderInterfaceContext;
    assert!(!context.is_lost());
    assert_eq!(PlatformRenderTargetState::READY, render_target.platform_render_target_state());

    // What the callback of wgpu does when the device is lost.
    gpu.device().mark_lost();
    assert!(context.is_lost());
    assert!(render_target.platform_render_target_state().is_corrupted);

    // Scenes that are drawn into memory do not stay on a lost device.
    let shared = VelloWgpuDevice::shared().unwrap();
    assert!(!Arc::ptr_eq(gpu.device(), &shared));
    assert!(!shared.is_lost());
}

/// The frames of [`a_frame_of_many_render_passes_is_drawn`]: what is nested
/// or repeated in each, `count` times.
#[derive(Clone, Copy, Debug)]
enum ManyPasses {
    /// Layers one after the other, each with a rectangle.
    LayersInARow,
    /// Layers inside each other, a rectangle in each.
    LayersInsideEachOther,
    /// Opacity masks one after the other: a layer and a mask layer each.
    MasksInARow,
    /// Opacity masks inside each other.
    MasksInsideEachOther,
}

fn draw_many_passes(context: &mut dyn IDrawingContextImpl, kind: ManyPasses, count: usize) {
    let brush = |index: usize| {
        ImmutableSolidColorBrush::new(Color::from_argb(200, (index * 37 % 255) as u8, 90, (index * 11 % 255) as u8))
    };
    let mask = ImmutableSolidColorBrush::new(Color::from_argb(128, 0, 0, 0));
    let bounds = Rect::new(0.0, 0.0, 256.0, 256.0);
    let rect = |index: usize| {
        RoundedRect::from_rect(Rect::new((index % 14) as f64 * 15.0, (index / 14 % 14) as f64 * 15.0, 24.0, 24.0))
    };

    context.clear(Colors::WHITE);
    match kind {
        ManyPasses::LayersInARow => {
            for index in 0..count {
                context.push_layer(bounds);
                context.draw_rectangle(Some(&brush(index)), None, rect(index), &BoxShadows::default());
                context.pop_layer();
            }
        }
        ManyPasses::LayersInsideEachOther => {
            for index in 0..count {
                context.push_layer(bounds);
                context.draw_rectangle(Some(&brush(index)), None, rect(index), &BoxShadows::default());
            }
            for _ in 0..count {
                context.pop_layer();
            }
        }
        ManyPasses::MasksInARow => {
            for index in 0..count {
                context.push_opacity_mask(&mask, bounds);
                context.draw_rectangle(Some(&brush(index)), None, rect(index), &BoxShadows::default());
                context.pop_opacity_mask();
            }
        }
        ManyPasses::MasksInsideEachOther => {
            for index in 0..count {
                context.push_opacity_mask(&mask, bounds);
                context.draw_rectangle(Some(&brush(index)), None, rect(index), &BoxShadows::default());
            }
            for _ in 0..count {
                context.pop_opacity_mask();
            }
        }
    }
}

/// What the catalog hung on (design document, section 11): a frame whose
/// scene needs more render passes than the command queue of the platform
/// holds command buffers that are not complete (64 for a queue that was
/// made without a number). `wgpu` makes a command buffer of Metal for every
/// pass and commits none of them before the last is encoded: on the queue
/// of the platform the 65th waited for one of the first 64 to complete,
/// forever, with the compositor lock held. The frames are drawn on a
/// thread of their own, and each has to be there within a time limit.
#[test]
fn a_frame_of_many_render_passes_is_drawn() {
    use std::sync::mpsc;
    use std::time::Duration;

    let test = "a_frame_of_many_render_passes_is_drawn";
    if MTLCreateSystemDefaultDevice().is_none() {
        println!("skipped: no adapter ({test}): the system has no Metal device");
        return;
    }
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let mut hung = Vec::new();
    for mode in [VelloRenderingMode::Hybrid, VelloRenderingMode::Gpu] {
        if (mode == VelloRenderingMode::Hybrid && !cfg!(feature = "hybrid"))
            || (mode == VelloRenderingMode::Gpu && !cfg!(feature = "gpu"))
        {
            continue;
        }
        for kind in [
            ManyPasses::LayersInARow,
            ManyPasses::LayersInsideEachOther,
            ManyPasses::MasksInARow,
            ManyPasses::MasksInsideEachOther,
        ] {
            // The device of the frames, for the case that they hang: it is
            // then marked lost, so that no later test draws with it.
            let shared_device: Arc<std::sync::Mutex<Option<Arc<VelloWgpuDevice>>>> = Arc::default();
            let (sender, receiver) = mpsc::channel::<(usize, [u8; 4])>();

            let published = shared_device.clone();
            std::thread::spawn(move || {
                let Some(device) = TestMetalDevice::new(test) else { return };
                let size = PixelSize::new(256, 256);
                let window = Window::new(&device, mode, size, 1.0);
                *published.lock().unwrap_or_else(|e| e.into_inner()) = Some(window.gpu.device().clone());
                if mode == VelloRenderingMode::Gpu && !window.gpu.device().supports_compute() {
                    return;
                }

                for count in [4usize, 24, 70, 150] {
                    let scene_info = RenderTargetSceneInfo::new(size, 1.0, CompositionTransparencyLevel::None);
                    let (mut context, _) = window.render_target.create_drawing_context(&scene_info);
                    draw_many_passes(&mut *context, kind, count);
                    context.dispose();

                    let (size, pixels) = window.read_back();
                    if sender.send((count, pixel(size, &pixels, 250, 250))).is_err() {
                        return;
                    }
                }
            });

            loop {
                match receiver.recv_timeout(Duration::from_secs(20)) {
                    Ok((count, corner)) => {
                        println!("{test}: {mode:?}, {kind:?}, {count}: drawn, the corner is {corner:?}");
                        // Nothing of the frame reaches the corner: the white
                        // it was cleared to. The renderer of the GPU mode is
                        // only asked to be there in time: it has a fixed
                        // amount of memory for what is blended above the
                        // fourth layer of a pixel and draws nothing of a
                        // frame that needs more, without saying so (design
                        // document, section 11.6).
                        if mode == VelloRenderingMode::Hybrid {
                            assert_eq!([255, 255, 255, 255], corner, "{mode:?}, {kind:?}, {count}");
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        println!("{test}: {mode:?}, {kind:?}: a frame was not drawn within 20 s");
                        if let Some(device) = shared_device.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                            device.mark_lost();
                        }
                        hung.push(format!("{mode:?}, {kind:?}"));
                        break;
                    }
                }
            }
        }
    }

    assert!(hung.is_empty(), "frames that were not drawn within their time: {hung:?}");
}

/// The render target keeps the frame of the window: the compositor draws
/// what changed straight into it, inside its dirty rectangle, and the
/// drawable of the next session, which holds nothing, shows the whole
/// frame. A frame of another size starts over.
#[test]
fn the_frame_of_the_window_is_retained_and_a_dirty_rectangle_is_drawn_into_it() {
    let test = "the_frame_of_the_window_is_retained_and_a_dirty_rectangle_is_drawn_into_it";
    let Some(device) = TestMetalDevice::new(test) else {
        return;
    };
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let probe = VelloMetalGpu::new(device.clone()).unwrap();

    for mode in window_modes(probe.device()) {
        let size = PixelSize::new(96, 64);
        let window = Window::new(&device, mode, size, 1.0);
        let properties = window.render_target.properties();
        assert!(properties.retains_previous_frame_contents && properties.is_suitable_for_direct_rendering);

        let scene_info = RenderTargetSceneInfo::new(size, 1.0, CompositionTransparencyLevel::None);
        let rectangle = |context: &mut dyn IDrawingContextImpl, rect: Rect, color: Color| {
            context.draw_rectangle(
                Some(&ImmutableSolidColorBrush::new(color)),
                None,
                RoundedRect::from_rect(rect),
                &BoxShadows::default(),
            );
        };

        // The first frame: nothing is retained, everything is drawn, as
        // the compositor draws it (a clip of the whole, a clear).
        let (mut context, properties) = window.render_target.create_drawing_context(&scene_info);
        assert!(!properties.previous_frame_is_retained, "{mode:?}");
        context.push_clip(Rect::new(0.0, 0.0, 96.0, 64.0));
        context.clear(Colors::TRANSPARENT);
        rectangle(&mut *context, Rect::new(0.0, 0.0, 96.0, 64.0), Color::from_rgb(250, 250, 250));
        rectangle(&mut *context, Rect::new(8.0, 8.0, 30.0, 30.0), Color::from_rgb(200, 20, 20));
        context.pop_clip();
        context.dispose();

        let (size, pixels) = window.read_back();
        assert_eq!([200, 20, 20, 255], pixel(size, &pixels, 10, 10), "{mode:?}");
        assert_eq!([250, 250, 250, 255], pixel(size, &pixels, 80, 50), "{mode:?}");

        // The second frame: a dirty rectangle is cleared and drawn again.
        // The session has the same texture here; a window has another
        // drawable, which holds nothing: the texture is cleared to show
        // that the whole frame reaches it.
        let (mut context, properties) = window.render_target.create_drawing_context(&scene_info);
        assert!(properties.previous_frame_is_retained, "{mode:?}");
        context.push_clip(Rect::new(20.0, 20.0, 40.0, 30.0));
        context.clear(Colors::TRANSPARENT);
        rectangle(&mut *context, Rect::new(0.0, 0.0, 96.0, 64.0), Color::from_rgb(250, 250, 250));
        rectangle(&mut *context, Rect::new(30.0, 30.0, 10.0, 10.0), Color::from_rgb(20, 20, 200));
        context.pop_clip();
        context.dispose();

        let (size, pixels) = window.read_back();
        // Outside the dirty rectangle: the first frame.
        assert_eq!([200, 20, 20, 255], pixel(size, &pixels, 10, 10), "{mode:?}");
        assert_eq!([250, 250, 250, 255], pixel(size, &pixels, 80, 50), "{mode:?}");
        // Inside it: the red rectangle is gone where it was not drawn
        // again, and the blue one is there.
        assert_eq!([250, 250, 250, 255], pixel(size, &pixels, 25, 25), "{mode:?}");
        assert_eq!([20, 20, 200, 255], pixel(size, &pixels, 35, 35), "{mode:?}");

        // A frame that draws nothing shows the same.
        let (mut context, properties) = window.render_target.create_drawing_context(&scene_info);
        assert!(properties.previous_frame_is_retained, "{mode:?}");
        context.dispose();
        let (_, again) = window.read_back();
        assert_eq!(pixels, again, "{mode:?}");

        // Another size: nothing is retained.
        window.platform_target.size.set(PixelSize::new(50, 40));
        let scene_info = RenderTargetSceneInfo::new(PixelSize::new(50, 40), 1.0, CompositionTransparencyLevel::None);
        let (mut context, properties) = window.render_target.create_drawing_context(&scene_info);
        assert!(!properties.previous_frame_is_retained, "{mode:?}");
        context.clear(Colors::TRANSPARENT);
        rectangle(&mut *context, Rect::new(0.0, 0.0, 20.0, 20.0), Color::from_rgb(20, 200, 20));
        context.dispose();
        let (size, pixels) = window.read_back();
        assert_eq!(PixelSize::new(50, 40), size);
        assert_eq!([20, 200, 20, 255], pixel(size, &pixels, 10, 10), "{mode:?}");
        assert_eq!([0, 0, 0, 0], pixel(size, &pixels, 30, 30), "{mode:?}");

        println!("{test}: {mode:?}: drawn");
    }
}
