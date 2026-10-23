//! The Metal device of a platform as the device of the GPU modes, and the
//! surface of a top-level as their render target.
//!
//! The platform hands out a Metal device with its command queue and, for
//! each frame, a session with the texture of the drawable of the layer
//! (`ferroui_metal`); it presents the drawable itself when the session is
//! disposed, with a command buffer of its queue. The GPU modes therefore
//! draw with a `wgpu` device that is made **over that Metal device**
//! (`Adapter::create_device_from_hal`), into the texture of each session
//! wrapped as a texture of the device (`Device::create_texture_from_hal`),
//! and not with a surface of `wgpu` over the layer:
//!
//! * the layer is not part of the contract: its size, its drawables and
//!   its presentation (with a transaction while the window is resized) are
//!   the platform's, and a surface of `wgpu` would configure the layer and
//!   present on its own;
//! * the device belongs to the graphics context the compositor created on
//!   the thread that renders, like the Graphite context of the Skia
//!   backend, and ends with it.
//!
//! **The device draws with a command queue of its own, not with the queue
//! of the platform.** A command queue of Metal holds a number of command
//! buffers that are not complete (64 unless the queue was made with
//! another number, as the platform makes its queue), and the one who asks
//! for one more waits until one completes. `wgpu` makes a command buffer
//! for every render pass of a command encoder and commits none of them
//! before the encoder is submitted, and the hybrid renderer needs a pass
//! and more for every layer inside another: on the queue of the platform a
//! frame with two dozen nested layers or opacity masks waited for a command
//! buffer that could not complete because it was not committed, forever
//! and with the compositor lock held (the test
//! `a_frame_of_many_render_passes_is_drawn`; `wgpu` gives the queues it
//! makes itself 4096, for this reason). The queue of the device is made
//! with that number.
//!
//! Two queues are not ordered against each other. The frame is presented
//! by the platform on its queue when the session is disposed; before that
//! the render target orders the queue of the platform after its own
//! ([`VelloMetalGpu::order_platform_after_device`]): its queue signals a
//! shared event behind the frame, and a command buffer of the platform's
//! queue that waits for the event is committed ahead of the presentation.
//! The GPU waits, the processor does not. Without a shared event (or a
//! queue of the platform) the render target waits until the device has
//! **scheduled** what was committed to its own queue (`waitUntilScheduled`
//! on an empty command buffer behind the frame), which is what the platform
//! itself waits for before it presents a frame of the UI thread.
//!
//! This is the one file of the crate with `unsafe`: the raw handles of the
//! contract become objects of `wgpu`.

use crate::gpu::vello_wgpu_device::{block_on, device_descriptor};
use crate::gpu::window_frame::WindowFrames;
use crate::gpu::{with_autorelease_pool, IVelloGpu, VelloWgpuDevice, VelloWgpuDeviceError};
use crate::helpers::pixel_format_helper::scene_size;
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IPlatformGraphicsContext, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::PixelSize;
use ferroui_metal::{try_get_metal_surface, IMetalDevice, IMetalPlatformSurfaceRenderTarget};
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLCommandBuffer, MTLCommandQueue, MTLDevice, MTLEvent, MTLPixelFormat, MTLSharedEvent, MTLTexture, MTLTextureType,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;
use wgpu::hal;

fn unavailable(reason: impl Into<String>) -> VelloWgpuDeviceError {
    VelloWgpuDeviceError { reason: reason.into() }
}

/// The command buffers the queue of the device holds that are not
/// complete: what `wgpu` gives the queues it makes itself (`wgpu-hal`,
/// `metal/adapter.rs`, `MAX_COMMAND_BUFFERS`), and from which it refuses to
/// make another.
const COMMAND_BUFFERS: usize = 4096;

/// The GPU of the Vello backend on a Metal device of the platform.
pub struct VelloMetalGpu {
    metal: Rc<dyn IMetalDevice>,
    device: Arc<VelloWgpuDevice>,
    /// The command queue the device draws with: a queue of the Metal device
    /// of the platform that is the backend's own (see the module).
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    /// The command queue of the platform, which presents the frames.
    platform_queue: Option<Retained<ProtocolObject<dyn MTLCommandQueue>>>,
    /// The event the queue of the device signals and the queue of the
    /// platform waits for, and the value it was signaled with last.
    event: Option<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
    signaled: Cell<u64>,
}

impl VelloMetalGpu {
    /// Makes the `wgpu` device over the Metal device of the platform, with
    /// a command queue of its own.
    ///
    /// Fails when `wgpu` has no adapter for the device or does not accept
    /// it.
    pub fn new(metal: Rc<dyn IMetalDevice>) -> Result<Rc<Self>, VelloWgpuDeviceError> {
        let raw_device = metal.device();

        // SAFETY: the contract hands out a valid `id<MTLDevice>` (or null),
        // which lives at least as long as the platform device this call
        // borrows; `retain` takes a reference of its own, which the `wgpu`
        // device keeps until it is dropped.
        let mtl_device: Retained<ProtocolObject<dyn MTLDevice>> = unsafe { Retained::retain(raw_device.cast()) }
            .ok_or_else(|| unavailable("the platform has no Metal device"))?;
        // SAFETY: the contract hands out a valid `id<MTLCommandQueue>` of
        // the device (or null), which lives as long as the platform device;
        // `retain` takes a reference of its own.
        let platform_queue: Option<Retained<ProtocolObject<dyn MTLCommandQueue>>> =
            unsafe { Retained::retain(metal.command_queue().cast()) };
        let event = mtl_device.newSharedEvent();
        let mtl_queue: Retained<ProtocolObject<dyn MTLCommandQueue>> = mtl_device
            .newCommandQueueWithMaxCommandBufferCount(COMMAND_BUFFERS)
            .ok_or_else(|| unavailable("the Metal device of the platform made no command queue"))?;
        let own_queue = mtl_queue.clone();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        // The adapter of the device of the platform: the one whose Metal
        // device is the same device of the system.
        let registry_id = mtl_device.registryID();
        let adapter = block_on(instance.enumerate_adapters(wgpu::Backends::METAL))
            .into_iter()
            .find(|adapter| {
                // SAFETY: the adapter is only asked for the identifier of
                // its device; nothing of it is destroyed or kept.
                unsafe { adapter.as_hal::<hal::api::Metal>() }
                    .is_some_and(|adapter| adapter.raw_device().registryID() == registry_id)
            })
            .ok_or_else(|| unavailable("wgpu has no adapter for the Metal device of the platform"))?;

        let descriptor = device_descriptor(&adapter);
        // What `wgpu` itself assumes of the ticks of a timestamp when it
        // opens a device (`wgpu-hal`, `metal/adapter.rs`); the backend asks
        // for no timestamps.
        let timestamp_period = if adapter.get_info().name.starts_with("Intel") { 83.333 } else { 1.0 };

        // SAFETY: the device and the queue are the retained objects of the
        // platform. The features and limits are the ones of the descriptor
        // the device is created with below, which `device_descriptor` takes
        // from this adapter.
        let hal_device = unsafe {
            hal::metal::Device::device_from_raw(mtl_device, descriptor.required_features, &descriptor.required_limits)
        };
        // SAFETY: the queue is a queue of that device: the device made it
        // above.
        let hal_queue = unsafe { hal::metal::Queue::queue_from_raw(mtl_queue, timestamp_period) };

        // SAFETY: `hal_device` wraps the Metal device the adapter was
        // chosen for (the same registry identifier: the same device of the
        // system), and the features of the descriptor are a subset of the
        // adapter's (`device_descriptor`).
        let (device, queue) = unsafe {
            adapter.create_device_from_hal(hal::OpenDevice::<hal::api::Metal> { device: hal_device, queue: hal_queue }, &descriptor)
        }
        .map_err(|error| unavailable(format!("wgpu did not accept the Metal device of the platform ({error})")))?;

        let device = VelloWgpuDevice::new(&adapter, device, queue);
        // Layers and bitmaps of the GPU modes are drawn on this device too.
        device.prefer();

        Ok(Rc::new(Self { metal, device, queue: own_queue, platform_queue, event, signaled: Cell::new(0) }))
    }

    /// Orders what the platform commits to its queue from now on (the
    /// presentation of a frame) after every command buffer that was
    /// committed to the queue of the device: the queue of the device
    /// signals the shared event with a new value, and the queue of the
    /// platform gets a command buffer that waits for that value. Neither
    /// waits on the processor. Without an event or a queue of the platform,
    /// waits until the device has scheduled what was committed instead
    /// ([`wait_until_scheduled`](Self::wait_until_scheduled)).
    pub fn order_platform_after_device(&self) {
        let (Some(event), Some(platform_queue)) = (&self.event, &self.platform_queue) else {
            self.wait_until_scheduled();
            return;
        };
        autoreleasepool(|_| {
            let value = self.signaled.get() + 1;
            let event: &ProtocolObject<dyn MTLEvent> = ProtocolObject::from_ref(&**event);
            let (Some(signal), Some(wait)) = (self.queue.commandBuffer(), platform_queue.commandBuffer()) else {
                self.wait_until_scheduled();
                return;
            };
            signal.encodeSignalEvent_value(event, value);
            signal.commit();
            wait.encodeWaitForEvent_value(event, value);
            wait.commit();
            self.signaled.set(value);
        });
    }

    /// Waits until the device has scheduled every command buffer that was
    /// committed to its queue: from then on the GPU runs them before what
    /// is scheduled later, on whatever queue, so a drawable they draw to
    /// can be presented. It does not wait for the GPU to run them.
    pub fn wait_until_scheduled(&self) {
        autoreleasepool(|_| {
            // The queue schedules its command buffers in the order they
            // were committed: when an empty one is scheduled, every one
            // before it is.
            if let Some(buffer) = self.queue.commandBuffer() {
                buffer.commit();
                buffer.waitUntilScheduled();
            }
        });
    }

    /// The texture of a session as a texture of the `wgpu` device, or why
    /// it cannot be drawn to.
    fn wrap_texture(&self, raw_texture: *mut c_void) -> Result<wgpu::Texture, String> {
        // SAFETY: the session hands out a valid `id<MTLTexture>` (or null)
        // that it keeps alive until it is disposed; `retain` takes a
        // reference of its own, which the wrapped texture releases when
        // `wgpu` drops it.
        let texture: Retained<ProtocolObject<dyn MTLTexture>> =
            unsafe { Retained::retain(raw_texture.cast()) }.ok_or("the session has no texture")?;

        let format = match texture.pixelFormat() {
            MTLPixelFormat::BGRA8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            MTLPixelFormat::RGBA8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            other => return Err(format!("the texture of the session has the pixel format {other:?}")),
        };
        let size = wgpu::Extent3d {
            width: texture.width() as u32,
            height: texture.height() as u32,
            depth_or_array_layers: 1,
        };
        // The texture of a drawable is a render target and nothing else
        // unless the layer says otherwise; a texture that is more can also
        // be read back.
        let usage = match texture.isFramebufferOnly() {
            true => wgpu::TextureUsages::RENDER_ATTACHMENT,
            false => wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        };

        // SAFETY: the object is a texture, of two dimensions with one
        // layer and one level (what a drawable has), in the format and
        // of the size it was just asked for.
        let hal_texture = unsafe {
            hal::metal::Device::texture_from_raw(
                texture,
                format,
                MTLTextureType::Type2D,
                1,
                1,
                hal::CopyExtent { width: size.width, height: size.height, depth: 1 },
                None,
            )
        };

        let descriptor = wgpu::TextureDescriptor {
            label: Some("FerroUI Vello drawable"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        };

        // SAFETY: the texture is a texture of the Metal device this `wgpu`
        // device was made over (the platform creates the drawables of the
        // render target for the device it was given), it matches the
        // descriptor (above), and a drawable is initialized memory whose
        // content the frame replaces: nothing is read from it before it is
        // cleared.
        Ok(unsafe {
            self.device.device().create_texture_from_hal::<hal::api::Metal>(
                hal_texture,
                &descriptor,
                wgpu::TextureUses::UNINITIALIZED,
            )
        })
    }
}

impl IVelloGpu for VelloMetalGpu {
    fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    fn platform_graphics_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.metal.clone()
    }

    fn try_create_render_target(
        self: Rc<Self>,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
        rendering_modes: &[VelloRenderingMode],
    ) -> Option<Rc<dyn IRenderTarget>> {
        for surface in surfaces {
            if let Some(metal_surface) = try_get_metal_surface(&**surface) {
                let target = metal_surface.create_metal_render_target(self.metal.clone());
                return Some(Rc::new(VelloMetalRenderTarget::new(self, target, rendering_modes.to_vec())));
            }
        }

        None
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        for surface in surfaces {
            if try_get_metal_surface(&**surface).is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    fn dispose(&self) {
        // The device ends with the last scene that holds it; what the
        // renderers keep on it goes with it.
    }
}

/// The render target of a Metal surface: every frame is a session of the
/// platform, whose texture is given the frame of the window.
///
/// The frame of the window is a surface of the backend that lives from
/// frame to frame ([`WindowFrame`]): a texture of the device in the hybrid
/// and in the GPU mode, pixels in memory in the CPU mode. The render target
/// says that it retains its frame and can be rendered to directly
/// ([`RenderTargetProperties`]), so the compositor draws what changed
/// straight into it, clipped to the dirty rectangles, without a layer of
/// its own in between; the scene of a frame is composed over what the
/// surface holds, and the texture of the drawable, which holds nothing of
/// the frame before, gets a copy of the surface: one pass on the device
/// (for the CPU mode, an upload).
pub struct VelloMetalRenderTarget {
    gpu: Rc<VelloMetalGpu>,
    target: RefCell<Option<Rc<dyn IMetalPlatformSurfaceRenderTarget>>>,
    /// The frame of the window.
    frames: WindowFrames,
}

impl VelloMetalRenderTarget {
    fn new(
        gpu: Rc<VelloMetalGpu>,
        target: Rc<dyn IMetalPlatformSurfaceRenderTarget>,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        let frames = WindowFrames::new(gpu.device.clone(), rendering_modes);
        Self { gpu, target: RefCell::new(Some(target)), frames }
    }
}

impl IRenderTarget for VelloMetalRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The frame of the window is kept by the render target (a drawable
        // holds nothing of the frame before), and the compositor draws into
        // it directly.
        RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
    }

    fn create_drawing_context(
        &self,
        _scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let target =
            self.target.borrow().clone().unwrap_or_else(|| panic!("VelloMetalRenderTarget has been disposed"));

        let frame_start = crate::perf::frame_start();

        // The session is the size of the layer at this moment: a window
        // that was resized gives a larger texture, and the frame of the
        // window is made for the texture.
        let session = target.begin_rendering();
        let scaling = session.scaling();

        if session.is_y_flipped() {
            session.dispose();
            panic!("The Vello backend does not draw to a Metal surface whose origin is its bottom-left corner");
        }

        let texture = match self.gpu.wrap_texture(session.texture()) {
            Ok(texture) => texture,
            Err(error) => {
                session.dispose();
                panic!("Unable to draw to the Metal texture: {error}.");
            }
        };

        let pixel_size = PixelSize::new(texture.width() as i32, texture.height() as i32);
        let (width, height) = scene_size(pixel_size);
        let frame = self.frames.frame_for(pixel_size, VelloPlatform::default_dpi() * scaling);
        let properties = RenderTargetDrawingContextProperties { previous_frame_is_retained: frame.has_content() };

        let (gpu, copied) = (self.gpu.clone(), frame.clone());
        let (frame_width, frame_height) = (u32::from(width), u32::from(height));
        let disposables: Vec<Box<dyn FnOnce()>> = vec![
            // The scene was rendered into the frame of the window: the
            // texture of the drawable gets a copy.
            Box::new(move || {
                // Objects of Metal that are autoreleased while the copy is
                // encoded end with it: the thread that renders has no pool
                // of its own.
                with_autorelease_pool(|| {
                    copied.copy_to(&gpu.device, &texture);
                    // The reference of the wrapped texture is given back
                    // before the session presents the drawable.
                    drop(texture);
                });
                // The platform presents on its own queue: it runs the
                // presentation after the frame.
                gpu.order_platform_after_device();
            }),
            // Disposing the session presents the frame, with a command
            // buffer of the queue of the platform.
            Box::new(move || {
                {
                    let _perf = crate::perf::scope(crate::perf::Phase::Present, 0);
                    session.dispose();
                }
                crate::perf::frame_end(frame_start, "window", frame_width, frame_height);
            }),
        ];

        (Box::new(frame.create_drawing_context_with(disposables)), properties)
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        if self.gpu.device.is_lost() {
            return PlatformRenderTargetState::CORRUPTED;
        }

        match &*self.target.borrow() {
            Some(target) => target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        self.frames.dispose();
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}

#[cfg(test)]
#[path = "metal_tests.rs"]
mod tests;
