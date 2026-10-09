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
//! the render target waits until the device has **scheduled** what was
//! committed to its own queue ([`VelloMetalGpu::wait_until_scheduled`]:
//! an empty command buffer behind the frame, `waitUntilScheduled`), which
//! is what the platform itself waits for before it presents a frame of the
//! UI thread. The wait is for the hand-over to the GPU, not for the GPU to
//! draw.
//!
//! This is the one file of the crate with `unsafe`: the raw handles of the
//! contract become objects of `wgpu`.

use crate::drawing_context_impl::DrawingContextImpl;
use crate::gpu::i_vello_gpu::window_rendering_mode;
use crate::gpu::vello_wgpu_device::{block_on, device_descriptor};
use crate::gpu::{with_autorelease_pool, DeviceSurfaceRenderTarget, IVelloGpu, VelloWgpuDevice, VelloWgpuDeviceError};
use crate::helpers::pixel_format_helper::scene_size;
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IPlatformGraphicsContext, IRenderTarget,
    PlatformRenderTargetState, RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::PixelSize;
use ferroui_metal::{try_get_metal_surface, IMetalDevice, IMetalPlatformSurfaceRenderTarget};
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLCommandBuffer, MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLTexture, MTLTextureType};
use std::cell::RefCell;
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

        Ok(Rc::new(Self { metal, device, queue: own_queue }))
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

/// What a window holds between its frames: the surface its scenes are
/// rendered into. The texture of each drawable is a copy of it.
enum WindowFrame {
    /// A texture of the device: the hybrid and the GPU mode.
    Device(DeviceSurfaceRenderTarget),
    /// Pixels in memory: the CPU mode.
    Memory(SurfaceRenderTarget),
}

impl WindowFrame {
    fn pixel_size(&self) -> PixelSize {
        match self {
            WindowFrame::Device(surface) => surface.pixel_size(),
            WindowFrame::Memory(surface) => surface.pixel_size(),
        }
    }

    fn dpi(&self) -> ferroui_base::Vector {
        match self {
            WindowFrame::Device(surface) => surface.dpi(),
            WindowFrame::Memory(surface) => surface.dpi(),
        }
    }

    fn has_content(&self) -> bool {
        match self {
            WindowFrame::Device(surface) => surface.has_content(),
            WindowFrame::Memory(surface) => surface.has_content(),
        }
    }

    fn is_corrupted(&self) -> bool {
        match self {
            WindowFrame::Device(surface) => surface.is_corrupted(),
            WindowFrame::Memory(surface) => surface.is_corrupted(),
        }
    }

    fn create_drawing_context_with(&self, disposables: Vec<Box<dyn FnOnce()>>) -> DrawingContextImpl {
        match self {
            WindowFrame::Device(surface) => surface.create_drawing_context_with(disposables),
            WindowFrame::Memory(surface) => surface.create_drawing_context_with(disposables),
        }
    }

    /// Copies what the frame holds into the texture of a drawable.
    fn copy_to(&self, device: &VelloWgpuDevice, target: &wgpu::Texture) {
        match self {
            WindowFrame::Device(surface) => {
                if let Some(texture) = surface.texture() {
                    let premultiply = texture.alpha() == crate::gpu::VelloTextureAlpha::Straight;
                    device.copy_texture_to_texture(texture.view(), target, premultiply);
                }
            }
            WindowFrame::Memory(surface) => {
                if let Some(image) = IDrawableBitmapImpl::image(surface) {
                    device.copy_pixels_to_texture(image.data.data(), image.width, image.height, target);
                }
            }
        }
    }

    fn dispose(&self) {
        match self {
            WindowFrame::Device(surface) => IBitmapImpl::dispose(surface),
            WindowFrame::Memory(surface) => IBitmapImpl::dispose(surface),
        }
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
    rendering_modes: Vec<VelloRenderingMode>,
    /// The frame of the window, once a frame was drawn: of the size and
    /// the scaling of the last session.
    frame: RefCell<Option<Rc<WindowFrame>>>,
}

impl VelloMetalRenderTarget {
    fn new(
        gpu: Rc<VelloMetalGpu>,
        target: Rc<dyn IMetalPlatformSurfaceRenderTarget>,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        Self { gpu, target: RefCell::new(Some(target)), rendering_modes, frame: RefCell::new(None) }
    }

    /// The frame of the window for a drawable of `pixel_size` at `dpi`: the
    /// one of the frame before when it still fits, a new one otherwise (the
    /// window was resized, its scaling changed, the device was lost).
    fn frame_for(&self, pixel_size: PixelSize, dpi: ferroui_base::Vector) -> Rc<WindowFrame> {
        let mut frame = self.frame.borrow_mut();

        let fits = frame
            .as_ref()
            .is_some_and(|frame| frame.pixel_size() == pixel_size && frame.dpi() == dpi && !frame.is_corrupted());
        if !fits {
            if let Some(old) = frame.take() {
                old.dispose();
            }

            let device = &self.gpu.device;
            let mode = window_rendering_mode(device, &self.rendering_modes);
            *frame = Some(Rc::new(match DeviceSurfaceRenderTarget::is_available(mode, device) {
                true => WindowFrame::Device(DeviceSurfaceRenderTarget::new(
                    device.clone(),
                    mode,
                    pixel_size,
                    dpi,
                    self.rendering_modes.clone(),
                    false,
                )),
                false => WindowFrame::Memory(SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
                    width: pixel_size.width,
                    height: pixel_size.height,
                    dpi,
                    rendering_modes: vec![VelloRenderingMode::Cpu],
                    use_scaled_drawing: false,
                })),
            }));
        }

        frame.clone().unwrap_or_else(|| panic!("The frame of the window was just created"))
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
        let frame = self.frame_for(pixel_size, VelloPlatform::default_dpi() * scaling);
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
                // The platform presents on its own queue: the frame is on
                // its way to the GPU before it does.
                gpu.wait_until_scheduled();
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
        if let Some(frame) = self.frame.borrow_mut().take() {
            frame.dispose();
        }
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}

#[cfg(test)]
#[path = "metal_tests.rs"]
mod tests;
