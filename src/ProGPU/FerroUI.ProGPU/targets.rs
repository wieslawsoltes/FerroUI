//! The render targets of the backend: the Metal surface of a window (the engine draws into the
//! texture of the drawable the platform presents), the framebuffer of a surface (the frame is
//! rendered on the device and read back) and the target of a context that cannot draw.

use crate::bitmap::{is_supported, write_frame};
use crate::device::ProGpuEngine;
use crate::drawing_context::ProGpuDrawingContext;
use crate::gpu::{Gpu, GpuTexture};
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::platform::surfaces::IFramebufferRenderTarget;
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties, RenderTargetError,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::RenderTargetNotReadyError;
use std::cell::RefCell;
use std::rc::Rc;

/// The target of a context without a device, or of surfaces the backend does not draw to: it is
/// never ready, and what is drawn into its contexts is drawn as nothing.
pub(crate) struct UnavailableRenderTarget;

impl IRenderTarget for UnavailableRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    fn create_drawing_context(&self, _scene_info: &RenderTargetSceneInfo) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        (Box::new(ProGpuDrawingContext::new(SceneBuilder::new(), None)), RenderTargetDrawingContextProperties::default())
    }

    fn try_create_drawing_context(
        &self,
        _scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        Err(RenderTargetNotReadyError::new_with_message("The ProGPU backend has no device or no surface it draws to.").into())
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::NOT_READY_TRY_LATER
    }

    fn dispose(&self) {}
}

/// The frame of a framebuffer target: the texture the scenes of the target are rendered into,
/// kept from frame to frame.
struct Frame {
    scene: Option<SceneBuilder>,
    texture: Option<Rc<GpuTexture>>,
}

/// The target of a framebuffer surface. The frame is kept in a texture of the device: a frame
/// draws over it (the compositor redraws what changed), and the framebuffer of the surface gets
/// the pixels read back.
pub(crate) struct FramebufferRenderTarget {
    gpu: Rc<Gpu>,
    target: RefCell<Option<Rc<dyn IFramebufferRenderTarget>>>,
    frame: Rc<RefCell<Frame>>,
}

impl FramebufferRenderTarget {
    pub(crate) fn new(gpu: Rc<Gpu>, target: Rc<dyn IFramebufferRenderTarget>) -> Self {
        Self { gpu, target: RefCell::new(Some(target)), frame: Rc::new(RefCell::new(Frame { scene: None, texture: None })) }
    }
}

impl IRenderTarget for FramebufferRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
    }

    fn create_drawing_context(&self, scene_info: &RenderTargetSceneInfo) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let Some(target) = self.target.borrow().clone() else {
            return (Box::new(ProGpuDrawingContext::new(SceneBuilder::new(), None)), RenderTargetDrawingContextProperties::default());
        };
        // The framebuffer is the size of the surface at this moment: the frame is made for it.
        let (locked, _) = target.lock(scene_info);
        let size = locked.size();
        let (width, height) = (size.width.max(1) as u32, size.height.max(1) as u32);

        let (scene, texture, retained) = {
            let mut frame = self.frame.borrow_mut();
            let scene = frame.scene.take().unwrap_or_default();
            let kept = frame.texture.take().filter(|texture| texture.width() == width && texture.height() == height);
            let retained = kept.is_some();
            let texture = kept.or_else(|| self.gpu.offscreen().map(|_| GpuTexture::new(self.gpu.device(), width, height)));
            (scene, texture, retained)
        };

        let (gpu, frame) = (self.gpu.clone(), self.frame.clone());
        let context = ProGpuDrawingContext::with_gpu(
            scene,
            Some(self.gpu.clone()),
            Some(Box::new(move |mut recording| {
                let mut drawn = false;
                if let (Some(engine), Some(texture)) = (gpu.offscreen(), &texture) {
                    match recording.render(engine, texture.view(), width, height, retained) {
                        Ok(()) => {
                            texture.changed();
                            drawn = true;
                        }
                        Err(error) => notes::failure(&format!("a frame was not drawn: {error}")),
                    }
                }
                if let (true, Some(texture)) = (drawn, &texture) {
                    let (format, alpha_format, row_bytes) = (locked.format(), locked.alpha_format(), locked.row_bytes().max(0) as usize);
                    if size.width > 0 && size.height > 0 && is_supported(format) {
                        let pixels = texture.read();
                        locked.with_data(&mut |data| {
                            write_frame(&pixels, size.width as usize, size.height as usize, data, row_bytes, format, alpha_format);
                        });
                    } else if !is_supported(format) {
                        notes::note_once("framebuffer-format", "the ProGPU backend writes framebuffers of RGBA8888 and BGRA8888 only");
                    }
                }
                locked.dispose();
                let mut frame = frame.borrow_mut();
                frame.scene = Some(recording.scene);
                // A frame that failed is not built upon.
                frame.texture = if drawn { texture } else { None };
            })),
        );
        (Box::new(context), RenderTargetDrawingContextProperties { previous_frame_is_retained: retained })
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        match &*self.target.borrow() {
            Some(target) => target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        *self.frame.borrow_mut() = Frame { scene: None, texture: None };
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) use metal::MetalRenderTarget;

#[cfg(target_os = "macos")]
mod metal {
    use super::*;
    use crate::device::TargetFormat;
    use ferroui_base::RenderTargetCorruptedError;
    use ferroui_metal::IMetalPlatformSurfaceRenderTarget;
    use progpu_native_sys::webgpu::{WGPUTextureFormat_BGRA8Unorm, WGPUTextureUsage_RenderAttachment};

    /// The target of the Metal surface of a window. The platform owns the drawables and presents
    /// them (so its resize stays cooperative): a frame begins a session of the surface, the
    /// engine renders the scene into the texture of the drawable, which is the size of the
    /// surface at that moment, and ending the session presents it. The device is on the Metal
    /// device and the command queue of the platform, so the presentation is ordered after the
    /// frame by the queue.
    ///
    /// A drawable holds nothing of the frame before and the target keeps no copy: every frame
    /// draws the whole scene.
    pub(crate) struct MetalRenderTarget {
        gpu: Rc<Gpu>,
        engine: Rc<ProGpuEngine>,
        target: RefCell<Option<Rc<dyn IMetalPlatformSurfaceRenderTarget>>>,
        scene: Rc<RefCell<Option<SceneBuilder>>>,
    }

    impl MetalRenderTarget {
        /// The target, or `None` when the engine refuses the device for the format of drawables.
        pub(crate) fn new(gpu: Rc<Gpu>, target: Rc<dyn IMetalPlatformSurfaceRenderTarget>) -> Option<Self> {
            let engine = gpu.engine(TargetFormat::Bgra8)?;
            Some(Self { gpu, engine, target: RefCell::new(Some(target)), scene: Rc::new(RefCell::new(None)) })
        }
    }

    impl IRenderTarget for MetalRenderTarget {
        fn properties(&self) -> RenderTargetProperties {
            // The compositor draws straight into the drawable, without a layer of its own.
            RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
        }

        fn create_drawing_context(&self, scene_info: &RenderTargetSceneInfo) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
            match self.try_create_drawing_context(scene_info) {
                Ok(context) => context,
                Err(error) => {
                    notes::failure(&format!("a frame of a window was not begun: {error}"));
                    (Box::new(ProGpuDrawingContext::new(SceneBuilder::new(), None)), RenderTargetDrawingContextProperties::default())
                }
            }
        }

        fn try_create_drawing_context(
            &self,
            _scene_info: &RenderTargetSceneInfo,
        ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
            let Some(target) = self.target.borrow().clone() else {
                return Err(RenderTargetCorruptedError::new_with_message("The render target of the window was disposed.").into());
            };
            let state = target.state();
            if !state.is_ready {
                return Err(RenderTargetNotReadyError::new().into());
            }

            // The session is the size of the surface at this moment: a window that was resized
            // gives a texture of its new size, and the frame is rendered for the texture.
            let session = target.begin_rendering();
            if session.is_y_flipped() {
                session.dispose();
                return Err(RenderTargetCorruptedError::new_with_message(
                    "The ProGPU backend does not draw to a Metal surface whose origin is its bottom-left corner.",
                )
                .into());
            }
            let size = session.size();
            let (width, height) = (size.width.max(1) as u32, size.height.max(1) as u32);
            let raw_texture = session.texture();
            if raw_texture.is_null() || size.width <= 0 || size.height <= 0 {
                session.dispose();
                return Err(RenderTargetNotReadyError::new_with_message("The Metal surface has no drawable.").into());
            }

            let scene = self.scene.borrow_mut().take().unwrap_or_default();
            let (gpu, engine, slot) = (self.gpu.clone(), self.engine.clone(), self.scene.clone());
            let context = ProGpuDrawingContext::with_gpu(
                scene,
                Some(self.gpu.clone()),
                Some(Box::new(move |mut recording| {
                    // SAFETY: the session keeps its texture, a texture of the Metal device the
                    // device is on, alive until it is disposed, which is after the wrapper is
                    // released; drawables are `BGRA8Unorm` render attachments of that size.
                    let wrapped = unsafe {
                        gpu.device().wrap_metal_texture(raw_texture, width, height, WGPUTextureFormat_BGRA8Unorm, WGPUTextureUsage_RenderAttachment)
                    };
                    match wrapped {
                        Some((texture, view)) => {
                            if let Err(error) = recording.render(&engine, view, width, height, false) {
                                notes::failure(&format!("a frame of a window was not drawn: {error}"));
                            }
                            // The reference of the wrapper is given back before the drawable is
                            // presented.
                            gpu.device().release_texture(texture, view);
                        }
                        None => notes::failure("the texture of a drawable could not be drawn to"),
                    }
                    // Ending the session presents the frame.
                    session.dispose();
                    crate::perf::window_frame();
                    *slot.borrow_mut() = Some(recording.scene);
                })),
            );
            Ok((Box::new(context), RenderTargetDrawingContextProperties { previous_frame_is_retained: false }))
        }

        fn platform_render_target_state(&self) -> PlatformRenderTargetState {
            match &*self.target.borrow() {
                Some(target) => target.state(),
                None => PlatformRenderTargetState::DISPOSED,
            }
        }

        fn dispose(&self) {
            *self.scene.borrow_mut() = None;
            if let Some(target) = self.target.borrow_mut().take() {
                target.dispose();
            }
        }
    }
}
