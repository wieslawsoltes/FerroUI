//! A Metal device and a surface that is not on screen, as a platform hands
//! out the device of its windows and the surface of a top-level.
//!
//! For what has to draw the frames of a window without a window: the
//! benchmarks of an application (a compositor created with
//! [`OffscreenMetalGraphics`] renders a top-level whose surface is an
//! [`OffscreenMetalSurface`] exactly as it renders one of the macOS
//! platform: a context of the backend over the Metal device, a render
//! target of the surface, a session a frame) and the tests of the path of a
//! window. The frame that was presented last can be read back.
//!
//! The objects are those of the Metal contracts (`ferroui_metal`), which no
//! render backend owns: the Skia backend draws to the same surface with the
//! same device.
//!
//! `unsafe`, as in `gpu/metal.rs`: the descriptor of a texture and the
//! copy of its pixels.

use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use ferroui_metal::{
    IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession,
};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLOrigin, MTLPixelFormat, MTLRegion,
    MTLSize, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};
use std::any::{Any, TypeId};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// The graphics of a platform whose contexts are Metal devices: the
/// default device of the system with a command queue of its own for each
/// context.
#[derive(Default)]
pub struct OffscreenMetalGraphics;

impl OffscreenMetalGraphics {
    /// The graphics, or `None` on a machine without a Metal device.
    pub fn try_new() -> Option<Arc<Self>> {
        MTLCreateSystemDefaultDevice().map(|_| Arc::new(Self))
    }
}

impl IPlatformGraphics for OffscreenMetalGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        OffscreenMetalDevice::try_new().unwrap_or_else(|| panic!("The system has no Metal device"))
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("The offscreen Metal graphics have no shared context")
    }
}

/// A Metal device and a command queue of it, as the graphics context of a
/// platform.
pub struct OffscreenMetalDevice {
    weak_self: Weak<OffscreenMetalDevice>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
}

impl OffscreenMetalDevice {
    /// The default device of the system with a new command queue, or
    /// `None` on a machine without a Metal device.
    pub fn try_new() -> Option<Rc<Self>> {
        let device = MTLCreateSystemDefaultDevice()?;
        let queue = device.newCommandQueue()?;
        Some(Rc::new_cyclic(|weak_self| Self { weak_self: weak_self.clone(), device, queue }))
    }
}

struct Nothing;

impl IDisposable for Nothing {
    fn dispose(&self) {}
}

impl IOptionalFeatureProvider for OffscreenMetalDevice {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IMetalDevice>() {
            let this: Rc<dyn IMetalDevice> = self.weak_self.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for OffscreenMetalDevice {
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

impl IMetalDevice for OffscreenMetalDevice {
    fn device(&self) -> *mut c_void {
        Retained::as_ptr(&self.device).cast_mut().cast()
    }

    fn command_queue(&self) -> *mut c_void {
        Retained::as_ptr(&self.queue).cast_mut().cast()
    }
}

/// What a surface does when a frame is presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffscreenFramePresentation {
    /// Nothing: the frame is complete when its commands are committed, as
    /// the frame of a window is.
    Committed,
    /// It waits until the device has drawn the frame: the time of a frame
    /// then holds the work of the device.
    Drawn,
    /// It waits for the device and copies the pixels of the frame
    /// ([`OffscreenMetalSurface::last_frame`]).
    Kept,
}

/// What a surface shares with its render targets and sessions, on whatever
/// thread they render.
struct SurfaceState {
    size: Mutex<(PixelSize, f64)>,
    /// The sessions that were disposed: the frames that were presented.
    frames: AtomicU32,
    /// The pixels of the frame that was presented last, as premultiplied
    /// BGRA rows without padding, when the surface keeps them.
    last_frame: Mutex<Option<(PixelSize, Vec<u8>)>>,
    presentation: OffscreenFramePresentation,
    /// Waits until a renderer that draws with a command queue of its own
    /// has drawn what it submitted.
    renderer_wait: Option<Arc<dyn Fn() + Send + Sync>>,
}

/// A surface of a top-level that is rendered to with Metal and is not on
/// screen: every frame is drawn into a texture of the device, of the size
/// the surface has when the frame starts.
pub struct OffscreenMetalSurface {
    state: Arc<SurfaceState>,
}

impl OffscreenMetalSurface {
    /// Creates a surface of a size in pixels and a scaling.
    ///
    /// A surface that waits for the device waits for the command queue of
    /// the graphics context: for what a backend draws with that queue (the
    /// Skia backend). A backend that draws with a queue of its own (the
    /// Vello backend, see `gpu/metal.rs`) is waited for with
    /// `renderer_wait`, before the queue of the context.
    pub fn new(
        size: PixelSize,
        scaling: f64,
        presentation: OffscreenFramePresentation,
        renderer_wait: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(SurfaceState {
                size: Mutex::new((size, scaling)),
                frames: AtomicU32::new(0),
                last_frame: Mutex::new(None),
                presentation,
                renderer_wait,
            }),
        })
    }

    /// Changes the size and the scaling of the frames that start from now.
    pub fn resize(&self, size: PixelSize, scaling: f64) {
        *self.state.size.lock().unwrap_or_else(|e| e.into_inner()) = (size, scaling);
    }

    /// The frames that were presented.
    pub fn frames(&self) -> u32 {
        self.state.frames.load(Ordering::SeqCst)
    }

    /// The size and the pixels of the frame that was presented last, as
    /// premultiplied BGRA, four bytes a pixel without padding; `None`
    /// before the first frame and for a surface that keeps no frames.
    pub fn last_frame(&self) -> Option<(PixelSize, Vec<u8>)> {
        self.state.last_frame.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

// The surface is shared between the thread of the top-level and the thread
// that renders: what it holds is behind locks and atomics.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OffscreenMetalSurface>();
};

impl IPlatformRenderSurface for OffscreenMetalSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            let this: Rc<dyn IMetalPlatformSurface> = Rc::new(OffscreenMetalSurfaceView { state: self.state.clone() });
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The surface as the thread that renders sees it.
struct OffscreenMetalSurfaceView {
    state: Arc<SurfaceState>,
}

impl IPlatformRenderSurface for OffscreenMetalSurfaceView {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalPlatformSurface for OffscreenMetalSurfaceView {
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        // SAFETY: the contract hands out a valid `id<MTLDevice>` and
        // `id<MTLCommandQueue>` that live as long as the platform device;
        // `retain` takes references of its own.
        let (mtl_device, mtl_queue) = unsafe {
            (
                Retained::<ProtocolObject<dyn MTLDevice>>::retain(device.device().cast()),
                Retained::<ProtocolObject<dyn MTLCommandQueue>>::retain(device.command_queue().cast()),
            )
        };
        let (Some(mtl_device), Some(mtl_queue)) = (mtl_device, mtl_queue) else {
            panic!("The graphics context has no Metal device and command queue");
        };

        Rc::new(OffscreenMetalRenderTarget {
            state: self.state.clone(),
            device: mtl_device,
            queue: mtl_queue,
            texture: std::cell::RefCell::new(None),
        })
    }
}

struct OffscreenMetalRenderTarget {
    state: Arc<SurfaceState>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    texture: std::cell::RefCell<Option<Retained<ProtocolObject<dyn MTLTexture>>>>,
}

impl OffscreenMetalRenderTarget {
    fn texture(&self, size: PixelSize) -> Retained<ProtocolObject<dyn MTLTexture>> {
        let mut texture = self.texture.borrow_mut();
        let (width, height) = (size.width.max(1) as usize, size.height.max(1) as usize);

        let fits = texture.as_ref().is_some_and(|texture| texture.width() == width && texture.height() == height);
        if !fits {
            // SAFETY: a descriptor of a texture of two dimensions in a
            // format and of a size that Metal has.
            let descriptor = unsafe {
                MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                    MTLPixelFormat::BGRA8Unorm,
                    width,
                    height,
                    false,
                )
            };
            // What the texture of a drawable is, and readable for the
            // frames a surface keeps.
            descriptor.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
            descriptor.setStorageMode(match self.state.presentation {
                OffscreenFramePresentation::Kept => MTLStorageMode::Shared,
                _ => MTLStorageMode::Private,
            });
            *texture = self.device.newTextureWithDescriptor(&descriptor);
        }

        texture.clone().unwrap_or_else(|| panic!("The Metal device made no texture"))
    }
}

impl IPlatformRenderSurfaceRenderTarget for OffscreenMetalRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for OffscreenMetalRenderTarget {
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        let (size, scaling) = *self.state.size.lock().unwrap_or_else(|e| e.into_inner());

        Rc::new(OffscreenMetalSession {
            state: self.state.clone(),
            queue: self.queue.clone(),
            texture: self.texture(size),
            size,
            scaling,
        })
    }

    fn dispose(&self) {
        self.texture.borrow_mut().take();
    }
}

struct OffscreenMetalSession {
    state: Arc<SurfaceState>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    size: PixelSize,
    scaling: f64,
}

impl OffscreenMetalSession {
    /// Waits until the device has run what was committed to the queue
    /// before now.
    fn wait_for_device(&self) -> Option<()> {
        if let Some(renderer_wait) = &self.state.renderer_wait {
            renderer_wait();
        }

        // The queue runs its command buffers in the order they were
        // committed: when an empty one is complete, the frame is drawn.
        let command_buffer = self.queue.commandBuffer()?;
        command_buffer.commit();
        command_buffer.waitUntilCompleted();
        Some(())
    }

    /// The pixels of the texture, which the device has drawn.
    fn read_frame(&self) -> Option<Vec<u8>> {
        let (width, height) = (self.texture.width(), self.texture.height());
        let mut pixels = vec![0u8; width * height * 4];
        let region = MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width, height, depth: 1 } };
        let bytes = NonNull::new(pixels.as_mut_ptr().cast::<c_void>())?;

        // SAFETY: the texture is of two dimensions, in a format of four
        // bytes a pixel, in shared storage (the surface keeps frames), and
        // the region is the whole of its only level; `pixels` holds
        // `height` rows of `width * 4` bytes, which is what is written.
        unsafe { self.texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(bytes, width * 4, region, 0) };

        Some(pixels)
    }
}

impl IMetalPlatformSurfaceRenderingSession for OffscreenMetalSession {
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
        // Objects of Metal that are autoreleased here end here: the thread
        // that renders has no pool of its own.
        objc2::rc::autoreleasepool(|_| {
            if self.state.presentation != OffscreenFramePresentation::Committed && self.wait_for_device().is_some() {
                if self.state.presentation == OffscreenFramePresentation::Kept {
                    if let Some(pixels) = self.read_frame() {
                        let size = PixelSize::new(self.texture.width() as i32, self.texture.height() as i32);
                        *self.state.last_frame.lock().unwrap_or_else(|e| e.into_inner()) = Some((size, pixels));
                    }
                }
            }
        });
        self.state.frames.fetch_add(1, Ordering::SeqCst);
    }
}
