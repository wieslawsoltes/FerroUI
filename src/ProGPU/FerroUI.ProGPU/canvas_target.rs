//! The canvas of a web page on `wasm32-unknown-emscripten`: the device is the one the page
//! requested before the module started, and the engine draws into the texture of the canvas.
//!
//! The browser shows the texture when the task that drew it returns, so there is no present
//! call. A canvas texture holds nothing of the frame before, so every frame draws the whole
//! scene, and a frame of another size than the canvas has by then is not shown.

use crate::device::{ProGpuEngine, TargetFormat};
use crate::drawing_context::ProGpuDrawingContext;
use crate::gpu::Gpu;
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties, RenderTargetError,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::{PixelSize, RenderTargetCorruptedError, RenderTargetNotReadyError};
use ferroui_controls::platform::IWebGpuCanvasSurface;
use progpu_native_sys::browser::*;
use progpu_native_sys::progpu::{PROGPU_NATIVE_TEXTURE_FORMAT_BGRA8_UNORM, PROGPU_NATIVE_TEXTURE_FORMAT_RGBA8_UNORM};
use progpu_native_sys::webgpu::WGPUTextureView;
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

/// The WebGPU canvas a render surface is, when it is one.
pub(crate) fn canvas_surface(surface: &dyn IPlatformRenderSurface) -> Option<Rc<dyn IWebGpuCanvasSurface>> {
    let kind = surface.try_get_surface_kind(TypeId::of::<dyn IWebGpuCanvasSurface>())?;
    kind.downcast_ref::<Rc<dyn IWebGpuCanvasSurface>>().cloned()
}

/// Whether a scene of `scene` pixels is the one for a canvas of `canvas` pixels, up to the pixel
/// of rounding between the logical size of the scene and the pixels of the canvas.
fn scene_fits_canvas(scene: PixelSize, canvas: PixelSize) -> bool {
    (scene.width - canvas.width).abs() <= 1 && (scene.height - canvas.height).abs() <= 1
}

/// The swap chain of the canvas, destroyed with the target.
struct SwapChain(*mut progpu_native_browser_canvas);

impl Drop for SwapChain {
    fn drop(&mut self) {
        // SAFETY: the canvas was created by `CanvasRenderTarget::new` and is destroyed once.
        unsafe { progpu_native_browser_canvas_destroy(self.0) };
    }
}

/// The render target of a canvas of the page.
pub(crate) struct CanvasRenderTarget {
    gpu: Rc<Gpu>,
    engine: Rc<ProGpuEngine>,
    surface: Rc<dyn IWebGpuCanvasSurface>,
    swap_chain: Rc<RefCell<Option<SwapChain>>>,
    scene: Rc<RefCell<Option<SceneBuilder>>>,
    disposed: Cell<bool>,
}

impl CanvasRenderTarget {
    /// The target of the first surface that is a WebGPU canvas, if one is and the engine takes it.
    pub(crate) fn try_new(gpu: &Rc<Gpu>, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Option<Self> {
        let surface = surfaces.iter().find_map(|surface| canvas_surface(&**surface))?;
        let (format, native_format) = if surface.prefers_rgba8() {
            (TargetFormat::Rgba8, PROGPU_NATIVE_TEXTURE_FORMAT_RGBA8_UNORM)
        } else {
            (TargetFormat::Bgra8, PROGPU_NATIVE_TEXTURE_FORMAT_BGRA8_UNORM)
        };
        let engine = gpu.engine(format)?;
        let selector = CString::new(surface.selector()).ok()?;
        let mut canvas = ptr::null_mut();
        // SAFETY: the device is alive; the selector is borrowed for the call and the slot written.
        let created = unsafe {
            progpu_native_browser_canvas_create(gpu.device().device() as usize, selector.as_ptr(), native_format as u32, &mut canvas)
        };
        if created == 0 {
            notes::failure("the WebGPU of the page made no surface for the canvas");
            return None;
        }
        Some(Self {
            gpu: gpu.clone(),
            engine,
            surface,
            swap_chain: Rc::new(RefCell::new(Some(SwapChain(canvas)))),
            scene: Rc::new(RefCell::new(None)),
            disposed: Cell::new(false),
        })
    }
}

impl IRenderTarget for CanvasRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The compositor draws straight into the frame, without a layer of its own.
        RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
    }

    fn create_drawing_context(&self, scene_info: &RenderTargetSceneInfo) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        match self.try_create_drawing_context(scene_info) {
            Ok(context) => context,
            Err(error) => {
                notes::failure(&format!("a frame of a canvas was not begun: {error}"));
                (Box::new(ProGpuDrawingContext::new(SceneBuilder::new(), None)), RenderTargetDrawingContextProperties::default())
            }
        }
    }

    fn try_create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        if self.disposed.get() {
            return Err(RenderTargetCorruptedError::new_with_message("The render target of the canvas was disposed.").into());
        }
        let size = self.surface.frame_size();
        if size.width <= 0 || size.height <= 0 || !scene_fits_canvas(scene_info.size, size) {
            return Err(RenderTargetNotReadyError::new().into());
        }
        let scene = self.scene.borrow_mut().take().unwrap_or_default();
        let (engine, surface, swap_chain, slot) =
            (self.engine.clone(), self.surface.clone(), self.swap_chain.clone(), self.scene.clone());
        let context = ProGpuDrawingContext::with_gpu(
            scene,
            Some(self.gpu.clone()),
            Some(Box::new(move |mut recording| {
                if let Some(canvas) = swap_chain.borrow().as_ref().map(|swap_chain| swap_chain.0) {
                    if scene_fits_canvas(size, surface.frame_size()) {
                        let (width, height) = (size.width as u32, size.height as u32);
                        let mut view = 0usize;
                        // SAFETY: the canvas is alive while the slot holds it; the view is written.
                        if unsafe { progpu_native_browser_canvas_begin_frame(canvas, width, height, &mut view) } != 0 {
                            let drawn = recording.render(&engine, view as WGPUTextureView, width, height, false);
                            // SAFETY: a begun frame of this canvas, which the engine is done with.
                            unsafe { progpu_native_browser_canvas_end_frame(canvas) };
                            match drawn {
                                Ok(()) => surface.frame_presented(size),
                                Err(error) => notes::failure(&format!("a frame of a canvas was not drawn: {error}")),
                            }
                        }
                    }
                }
                crate::perf::window_frame();
                *slot.borrow_mut() = Some(recording.scene);
            })),
        );
        Ok((Box::new(context), RenderTargetDrawingContextProperties { previous_frame_is_retained: false }))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        if self.disposed.get() {
            PlatformRenderTargetState::DISPOSED
        } else {
            PlatformRenderTargetState::READY
        }
    }

    fn dispose(&self) {
        self.disposed.set(true);
        *self.scene.borrow_mut() = None;
        *self.swap_chain.borrow_mut() = None;
    }
}
