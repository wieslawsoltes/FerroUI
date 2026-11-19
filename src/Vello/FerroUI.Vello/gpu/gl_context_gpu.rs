//! The GPU modes on the context of OpenGL of the platform.
//!
//! A platform whose graphics are OpenGL (GLX or EGL on X11, EGL on Wayland,
//! ANGLE on Direct3D 11 or WGL on Windows) renders its compositor with a
//! context of its own and hands its windows out as surfaces of that context.
//! Where the machine has no Vulkan or Direct3D 12 device that is hardware
//! (`window_surface.rs`), the device of the GPU modes is made over that very
//! context: `wgpu` on OpenGL with an adapter of an existing context
//! (`wgpu-hal`, `gles::Adapter::new_external`), which uses the context that
//! is current and makes no context of its own. The hybrid mode draws on it
//! (the GPU mode needs compute shaders, which OpenGL ES 3.0 lacks), and the
//! frame of a window, a texture of the context, is copied into the
//! framebuffer of the session of the surface, which presents it.
//!
//! The context has to be current whenever the device is used: the compositor
//! renders, and disposes render targets and layers, with the context of the
//! platform current (`ensure_current` of the render interface; a session of
//! a surface makes it current with the surface). The device is therefore
//! not made the device other threads draw into memory with
//! ([`VelloWgpuDevice::prefer`]), and is used by the thread that renders
//! only.
//!
//! This file has `unsafe` for the adapter of an existing context and the raw
//! texture of the frame.

use crate::gl_window::{blit_texture, gl_render_target, has_gl_surface};
use crate::gpu::i_vello_gpu::window_rendering_mode;
use crate::gpu::vello_wgpu_device::{block_on, device_descriptor};
use crate::gpu::window_frame::WindowFrames;
use crate::gpu::{IVelloGpu, VelloWgpuDevice, VelloWgpuDeviceError};
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IPlatformGraphicsContext, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::PixelSize;
use ferroui_opengl::gl_consts::GL_FRAMEBUFFER_BINDING;
use ferroui_opengl::surfaces::IGlPlatformSurfaceRenderTarget;
use ferroui_opengl::IGlContext;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use wgpu::hal;

fn unavailable(reason: impl Into<String>) -> VelloWgpuDeviceError {
    VelloWgpuDeviceError { reason: reason.into() }
}

/// The GPU of the Vello backend on the context of OpenGL of the platform
/// (see the module).
pub struct VelloGlGpu {
    gl_context: Rc<dyn IGlContext>,
    device: Arc<VelloWgpuDevice>,
    /// The import of the frames the producers of the context hand out as its textures, when the
    /// context shares its frames (#575, `gl_external_objects.rs`).
    external_objects: Option<Rc<crate::gpu::gl_external_objects::VelloGlExternalObjectsFeature>>,
}

impl VelloGlGpu {
    /// Makes the device over the context, with the context current.
    ///
    /// Fails when `wgpu` takes no adapter of the context, when the adapter
    /// is software (llvmpipe; unless `FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER`
    /// is `1`), and when the adapter gives no device.
    pub fn new(gl_context: Rc<dyn IGlContext>) -> Result<Rc<Self>, VelloWgpuDeviceError> {
        let current = gl_context.make_current();
        let device = Self::create_device(&gl_context);
        current.dispose();
        let device = device?;
        let external_objects = gl_context.shared_frame_context_id().map(|context_id| {
            Rc::new(crate::gpu::gl_external_objects::VelloGlExternalObjectsFeature::new(
                device.clone(),
                context_id,
                gl_context.shared_frame_context_generation(),
            ))
        });
        Ok(Rc::new(Self { gl_context, device, external_objects }))
    }

    fn create_device(gl_context: &Rc<dyn IGlContext>) -> Result<Arc<VelloWgpuDevice>, VelloWgpuDeviceError> {
        let gl = gl_context.gl_interface();
        // SAFETY: the context is current (the caller made it so) and stays
        // current whenever the device is used (see the module); the loader
        // returns the entry points of the context.
        let exposed = unsafe {
            hal::gles::Adapter::new_external(|name| gl.get_proc_address(name), wgpu::GlBackendOptions::default())
        }
        .ok_or_else(|| unavailable("wgpu took no adapter of the context of OpenGL of the platform"))?;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        // SAFETY: the adapter was made over the context that is current.
        let adapter = unsafe { instance.create_adapter_from_hal(exposed) };
        let info = adapter.get_info();
        let software_allowed = std::env::var("FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER").is_ok_and(|value| value == "1");
        if info.device_type == wgpu::DeviceType::Cpu && !software_allowed {
            return Err(unavailable(format!(
                "the context of OpenGL of the platform is software ({}): the CPU mode draws instead",
                info.name
            )));
        }

        let (device, queue) = block_on(adapter.request_device(&device_descriptor(&adapter)))
            .map_err(|error| unavailable(format!("the context of OpenGL {} gave no device ({error})", info.name)))?;
        Ok(VelloWgpuDevice::new(&adapter, device, queue))
    }
}

impl IVelloGpu for VelloGlGpu {
    fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    /// The context of OpenGL the device is made over, as the Skia backend on the context hands it
    /// out (the producers of the platform draw their frames with it: `NativeGlFrameProducer`
    /// asks the render interface for it), and the external objects feature of the GPU when the
    /// context shares the frames of its producers.
    fn try_get_feature(&self, feature_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        use ferroui_base::platform::IExternalObjectsRenderInterfaceContextFeature;
        if feature_type == std::any::TypeId::of::<dyn IGlContext>() {
            return Some(Rc::new(self.gl_context.clone()));
        }
        if feature_type == std::any::TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature> = self.external_objects.clone()?;
            return Some(Rc::new(feature));
        }
        None
    }

    fn platform_graphics_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.gl_context.clone()
    }

    fn try_create_render_target(
        self: Rc<Self>,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
        rendering_modes: &[VelloRenderingMode],
    ) -> Option<Rc<dyn IRenderTarget>> {
        // A window drawn in the CPU mode is presented by `gl_window.rs`.
        if window_rendering_mode(&self.device, rendering_modes) == VelloRenderingMode::Cpu {
            return None;
        }
        let target = gl_render_target(&self.gl_context, surfaces)?;
        Some(Rc::new(VelloGlGpuRenderTarget::new(self, target, rendering_modes.to_vec())))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        has_gl_surface(&self.gl_context, surfaces).unwrap_or(false)
    }

    fn dispose(&self) {
        // The device ends with the last scene that holds it.
    }
}

/// The render target of a surface of the context of the platform drawn by
/// the hybrid mode on the device over that context. It keeps the frame of
/// the window, a texture of the context ([`WindowFrames`]), says that it
/// retains it and can be rendered to directly, and copies it into the
/// framebuffer of each session before the session presents.
pub struct VelloGlGpuRenderTarget {
    gpu: Rc<VelloGlGpu>,
    target: RefCell<Option<Rc<dyn IGlPlatformSurfaceRenderTarget>>>,
    frames: WindowFrames,
    /// The framebuffer the frame is read from for the copy.
    read_framebuffer: Rc<Cell<i32>>,
}

impl VelloGlGpuRenderTarget {
    fn new(
        gpu: Rc<VelloGlGpu>,
        target: Rc<dyn IGlPlatformSurfaceRenderTarget>,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        let frames = WindowFrames::new(gpu.device.clone(), rendering_modes);
        Self { gpu, target: RefCell::new(Some(target)), frames, read_framebuffer: Rc::new(Cell::new(0)) }
    }
}

impl IRenderTarget for VelloGlGpuRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let target = self.target.borrow().clone().unwrap_or_else(|| panic!("VelloGlGpuRenderTarget has been disposed"));
        let frame_start = crate::perf::frame_start();

        // The session makes the context current with the surface until it
        // is disposed; the framebuffer it binds is the one presented, read
        // now, before the device binds its own.
        let session = target.begin_draw(scene_info);
        let gl = self.gpu.gl_context.gl_interface();
        let target_framebuffer = gl.get_integerv(GL_FRAMEBUFFER_BINDING);
        let size = session.size();
        let size = PixelSize::new(size.width.max(1), size.height.max(1));
        let frame = self.frames.frame_for(size, VelloPlatform::default_dpi() * session.scaling());
        let properties = RenderTargetDrawingContextProperties { previous_frame_is_retained: frame.has_content() };

        let (copied, read_framebuffer) = (frame.clone(), self.read_framebuffer.clone());
        let disposables: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || {
            if let Some(texture) = copied.device_texture() {
                // SAFETY: the texture is a texture of the device, made over
                // the context that is current.
                let raw = unsafe { texture.texture().as_hal::<hal::api::Gles>() }.and_then(|texture| {
                    match &texture.inner {
                        hal::gles::TextureInner::Texture { raw, .. } => Some(raw.0.get() as i32),
                        _ => None,
                    }
                });
                if let Some(raw) = raw {
                    let _perf =
                        crate::perf::scope(crate::perf::Phase::TextureCopy, size.width as u64 * size.height as u64 * 4);
                    if read_framebuffer.get() == 0 {
                        let mut name = 0;
                        // SAFETY: one name is written.
                        unsafe { gl.gen_framebuffers(1, &mut name) };
                        read_framebuffer.set(name);
                    }
                    // The rows of a texture `wgpu` drew into go up in the
                    // coordinates of OpenGL, as the rows uploaded by the
                    // CPU presenter do.
                    blit_texture(
                        &gl,
                        raw,
                        read_framebuffer.get(),
                        target_framebuffer,
                        size.width,
                        size.height,
                        session.is_y_flipped(),
                    );
                }
            }
            {
                let _perf = crate::perf::scope(crate::perf::Phase::Present, 0);
                session.dispose();
            }
            crate::perf::frame_end(frame_start, "window", size.width as u32, size.height as u32);
        })];

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
        let current = self.gpu.gl_context.make_current();
        self.frames.dispose();
        if self.read_framebuffer.get() != 0 {
            let gl = self.gpu.gl_context.gl_interface();
            // SAFETY: the name was made by this context.
            unsafe { gl.delete_framebuffers(1, &self.read_framebuffer.get()) };
            self.read_framebuffer.set(0);
        }
        current.dispose();
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}
