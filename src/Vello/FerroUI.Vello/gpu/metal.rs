//! The Metal device of a platform as the device of the GPU modes, and the
//! surface of a top-level as their render target.
//!
//! The platform hands out a Metal device with its command queue and, for
//! each frame, a session with the texture of the drawable of the layer
//! (`ferroui_metal`); it presents the drawable itself when the session is
//! disposed, with a command buffer of its queue. The GPU modes therefore
//! draw with a `wgpu` device that is made **over that Metal device and
//! that queue** (`Adapter::create_device_from_hal`), into the texture of
//! each session wrapped as a texture of the device
//! (`Device::create_texture_from_hal`), and not with a surface of `wgpu`
//! over the layer:
//!
//! * the layer is not part of the contract: its size, its drawables and
//!   its presentation (with a transaction while the window is resized) are
//!   the platform's, and a surface of `wgpu` would configure the layer and
//!   present on its own;
//! * what is drawn and the presentation are command buffers of one queue,
//!   which runs them in the order they were committed: the frame is
//!   complete when it is presented, without waiting for the GPU;
//! * the device belongs to the graphics context the compositor created on
//!   the thread that renders, like the Graphite context of the Skia
//!   backend, and ends with it.
//!
//! This is the one file of the crate with `unsafe`: the raw handles of the
//! contract become objects of `wgpu`.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::i_vello_gpu::{create_window_scene_sink, render_window_scene};
use crate::gpu::vello_wgpu_device::{block_on, device_descriptor};
use crate::gpu::{log_render_failure, IVelloGpu, VelloWgpuDevice, VelloWgpuDeviceError};
use crate::helpers::pixel_format_helper::scene_size;
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IPlatformGraphicsContext, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_metal::{try_get_metal_surface, IMetalDevice, IMetalPlatformSurfaceRenderTarget};
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLCommandQueue, MTLDevice, MTLPixelFormat, MTLTexture, MTLTextureType};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;
use wgpu::hal;

fn unavailable(reason: impl Into<String>) -> VelloWgpuDeviceError {
    VelloWgpuDeviceError { reason: reason.into() }
}

/// The GPU of the Vello backend on a Metal device of the platform.
pub struct VelloMetalGpu {
    metal: Rc<dyn IMetalDevice>,
    device: Arc<VelloWgpuDevice>,
}

impl VelloMetalGpu {
    /// Makes the `wgpu` device over the Metal device and the command queue
    /// of the platform.
    ///
    /// Fails when `wgpu` has no adapter for the device or does not accept
    /// it.
    pub fn new(metal: Rc<dyn IMetalDevice>) -> Result<Rc<Self>, VelloWgpuDeviceError> {
        let (raw_device, raw_queue) = (metal.device(), metal.command_queue());

        // SAFETY: the contract hands out a valid `id<MTLDevice>` (or null),
        // which lives at least as long as the platform device this call
        // borrows; `retain` takes a reference of its own, which the `wgpu`
        // device keeps until it is dropped.
        let mtl_device: Retained<ProtocolObject<dyn MTLDevice>> = unsafe { Retained::retain(raw_device.cast()) }
            .ok_or_else(|| unavailable("the platform has no Metal device"))?;
        // SAFETY: as above, for the `id<MTLCommandQueue>` of the contract.
        let mtl_queue: Retained<ProtocolObject<dyn MTLCommandQueue>> = unsafe { Retained::retain(raw_queue.cast()) }
            .ok_or_else(|| unavailable("the platform has no Metal command queue"))?;

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
        // SAFETY: the queue is a queue of that device (the contract: the
        // queue "used to render with it").
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

        Ok(Rc::new(Self { metal, device }))
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
                return Some(Rc::new(VelloMetalRenderTarget {
                    gpu: self,
                    target: RefCell::new(Some(target)),
                    rendering_modes: rendering_modes.to_vec(),
                }));
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
/// platform, whose texture a scene is rendered into.
pub struct VelloMetalRenderTarget {
    gpu: Rc<VelloMetalGpu>,
    target: RefCell<Option<Rc<dyn IMetalPlatformSurfaceRenderTarget>>>,
    rendering_modes: Vec<VelloRenderingMode>,
}

impl IRenderTarget for VelloMetalRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // A drawable holds nothing of the frame before.
        RenderTargetProperties::default()
    }

    fn create_drawing_context(
        &self,
        _scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let target =
            self.target.borrow().clone().unwrap_or_else(|| panic!("VelloMetalRenderTarget has been disposed"));

        // The session is the size of the layer at this moment: a window
        // that was resized gives a larger texture, and the scene is made
        // for the texture.
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

        let pixel_size = ferroui_base::PixelSize::new(texture.width() as i32, texture.height() as i32);
        let (width, height) = scene_size(pixel_size);
        let device = self.gpu.device.clone();
        let sink = create_window_scene_sink(&device, &self.rendering_modes, width, height, texture.format());

        let create_info = CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| {
                // Objects of Metal that are autoreleased while the frame is
                // encoded end with the frame: the thread that renders has
                // no pool of its own.
                autoreleasepool(|_| {
                    if let Err(error) = render_window_scene(&device, sink, &texture) {
                        log_render_failure("window", &error);
                    }
                    // The reference of the wrapped texture is given back
                    // before the session presents the drawable.
                    drop(texture);
                });
            }),
            scale_drawing_to_dpi: false,
            dpi: VelloPlatform::default_dpi() * scaling,
            rendering_modes: self.rendering_modes.clone(),
        };

        // Disposing the session presents the frame: with a command buffer
        // of the queue the frame was committed to, after it.
        let context = DrawingContextImpl::new(create_info, vec![Box::new(move || session.dispose())]);

        (Box::new(context), RenderTargetDrawingContextProperties::default())
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
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}

#[cfg(test)]
#[path = "metal_tests.rs"]
mod tests;
