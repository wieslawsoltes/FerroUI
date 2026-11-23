//! The windows of the Linux and Windows platforms: a surface of the WebGPU implementation made
//! from the handle of the window, which the platform hands out as a render surface
//! (`INativePlatformHandleSurface`: an `XID` on X11, an `HWND` on Windows). The device is the
//! backend's own, on Vulkan or on Direct3D 12 (`ProGpuDevice::new`); on X11 the surface is made
//! with a connection to the X server of the backend's own, as an `XID` names the window on every
//! connection.
//!
//! A texture of the swap chain holds nothing of the frame before and the target keeps no copy,
//! so every frame draws the whole scene, as on a Metal surface.
//!
//! **Resizing.** A frame is drawn for the size of the window when it starts, and is not presented
//! if the window has another size by the time it is done: a swap chain of the old size would be
//! stretched to the window. While the sizes differ the target is not ready, the compositor skips
//! the frame and the platform renders the scene of the new size itself when it resizes.
//!
//! This file has `unsafe` for the handles of the window and the calls of the WebGPU C ABI.

use crate::device::{thread_is_ending, ProGpuEngine, TargetFormat};
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
use ferroui_controls::platform::INativePlatformHandleSurface;
use progpu_native_sys::webgpu::*;
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

/// The kind of window handle a surface is made from on this platform.
#[cfg(target_os = "linux")]
const WINDOW_HANDLE_DESCRIPTOR: &str = "XID";
#[cfg(windows)]
const WINDOW_HANDLE_DESCRIPTOR: &str = "HWND";

/// The handle of the window a render surface is, when it is one of the kind of this platform.
fn native_handle(surface: &dyn IPlatformRenderSurface) -> Option<Rc<dyn INativePlatformHandleSurface>> {
    let kind = surface.try_get_surface_kind(TypeId::of::<dyn INativePlatformHandleSurface>())?;
    let handle = kind.downcast_ref::<Rc<dyn INativePlatformHandleSurface>>()?.clone();
    (handle.handle_descriptor() == Some(WINDOW_HANDLE_DESCRIPTOR) && handle.handle() != 0).then_some(handle)
}

/// Whether one of the surfaces is a window this module draws.
pub(crate) fn has_window(surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
    surfaces.iter().any(|surface| native_handle(&**surface).is_some())
}

/// Whether a scene of `scene` pixels is the one for a window of `window` pixels: the same size, or
/// one pixel off in either direction, the rounding between the logical size of the scene and the
/// pixels of the window.
fn scene_fits_window(scene: PixelSize, window: PixelSize) -> bool {
    (scene.width - window.width).abs() <= 1 && (scene.height - window.height).abs() <= 1
}

/// What the surfaces of a context are made with: the connection to the X server on X11.
#[cfg(target_os = "linux")]
pub(crate) struct WindowDisplay {
    xlib: x11_dl::xlib::Xlib,
    display: ptr::NonNull<x11_dl::xlib::Display>,
}

#[cfg(target_os = "linux")]
impl WindowDisplay {
    /// Opens a connection to the display of the environment (`DISPLAY`), the one the X11 platform
    /// connects to, or `None` without one.
    pub(crate) fn open() -> Option<Rc<Self>> {
        let xlib = match x11_dl::xlib::Xlib::open() {
            Ok(xlib) => xlib,
            Err(error) => {
                notes::note_once("no-x11", &format!("libX11 was not loaded, windows are read back: {error}"));
                return None;
            }
        };
        // SAFETY: a null name opens the display of the environment; the connection is closed when
        // this value is dropped, after the surfaces made with it (their targets hold it).
        let display = ptr::NonNull::new(unsafe { (xlib.XOpenDisplay)(ptr::null()) });
        match display {
            Some(display) => Some(Rc::new(Self { xlib, display })),
            None => {
                notes::note_once("no-x11", "the X server refused a connection, windows are read back");
                None
            }
        }
    }

    fn source(&self, handle: &dyn INativePlatformHandleSurface) -> WGPUSurfaceSourceXlibWindow {
        WGPUSurfaceSourceXlibWindow {
            chain: WGPUChainedStruct { next: ptr::null_mut(), sType: WGPUSType_SurfaceSourceXlibWindow },
            display: self.display.as_ptr().cast(),
            window: handle.handle() as u64,
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for WindowDisplay {
    fn drop(&mut self) {
        // SAFETY: the display was opened by `open` and is closed once.
        unsafe { (self.xlib.XCloseDisplay)(self.display.as_ptr()) };
    }
}

/// What the surfaces of a context are made with: nothing on Windows.
#[cfg(windows)]
pub(crate) struct WindowDisplay;

#[cfg(windows)]
impl WindowDisplay {
    pub(crate) fn open() -> Option<Rc<Self>> {
        Some(Rc::new(Self))
    }

    fn source(&self, handle: &dyn INativePlatformHandleSurface) -> WGPUSurfaceSourceWindowsHWND {
        WGPUSurfaceSourceWindowsHWND {
            chain: WGPUChainedStruct { next: ptr::null_mut(), sType: WGPUSType_SurfaceSourceWindowsHWND },
            hinstance: ptr::null_mut(),
            hwnd: handle.handle() as *mut _,
        }
    }
}

/// What a surface is configured with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Configuration {
    format: WGPUTextureFormat,
    present_mode: WGPUPresentMode,
    alpha_mode: WGPUCompositeAlphaMode,
}

/// The format, the presentation and the alpha of a swap chain out of what a surface offers: four
/// 8 bit channels that are not sRGB (the engine writes the encoded colors), without waiting for
/// the display when it can, opaque unless the surface has no opaque mode.
fn choose_configuration(
    formats: &[WGPUTextureFormat],
    present_modes: &[WGPUPresentMode],
    alpha_modes: &[WGPUCompositeAlphaMode],
) -> Option<Configuration> {
    let format = [WGPUTextureFormat_BGRA8Unorm, WGPUTextureFormat_RGBA8Unorm]
        .into_iter()
        .find(|format| formats.contains(format))?;
    let present_mode =
        if present_modes.contains(&WGPUPresentMode_Mailbox) { WGPUPresentMode_Mailbox } else { WGPUPresentMode_Fifo };
    let alpha_mode = if alpha_modes.contains(&WGPUCompositeAlphaMode_Opaque) {
        WGPUCompositeAlphaMode_Opaque
    } else {
        alpha_modes.first().copied().unwrap_or(WGPUCompositeAlphaMode_Auto)
    };
    Some(Configuration { format, present_mode, alpha_mode })
}

/// What to do with the status of a texture asked of a swap chain.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Acquired {
    /// Draw into it.
    Texture,
    /// Draw into it, and configure the swap chain again before the next frame.
    Suboptimal,
    /// Configure the swap chain again and ask once more.
    Reconfigure,
    /// The window shows no frame now (it is hidden, or the wait timed out).
    Skip,
    /// Something failed, which is logged.
    Failed,
}

fn acquired(status: WGPUSurfaceGetCurrentTextureStatus) -> Acquired {
    match status {
        WGPUSurfaceGetCurrentTextureStatus_SuccessOptimal => Acquired::Texture,
        WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal => Acquired::Suboptimal,
        WGPUSurfaceGetCurrentTextureStatus_Outdated | WGPUSurfaceGetCurrentTextureStatus_Lost => Acquired::Reconfigure,
        WGPUSurfaceGetCurrentTextureStatus_Timeout | WGPUSurfaceGetCurrentTextureStatus_Occluded => Acquired::Skip,
        _ => Acquired::Failed,
    }
}

/// The surface of a window with the configuration of its swap chain and the engine for its format.
struct SwapChain {
    surface: WGPUSurface,
    configuration: Option<Configuration>,
    size: (u32, u32),
    engine: Option<(WGPUTextureFormat, Rc<ProGpuEngine>)>,
    reconfigure: bool,
}

impl Drop for SwapChain {
    fn drop(&mut self) {
        if thread_is_ending() {
            return;
        }
        // SAFETY: the surface is owned here; the connection it was made with outlives it.
        unsafe { wgpuSurfaceRelease(self.surface) };
    }
}

/// A texture of the swap chain, for one frame.
struct WindowFrame {
    texture: WGPUTexture,
    view: WGPUTextureView,
    engine: Rc<ProGpuEngine>,
}

/// What a target and the frames it has begun share.
struct WindowState {
    gpu: Rc<Gpu>,
    display: Rc<WindowDisplay>,
    swap_chain: RefCell<Option<SwapChain>>,
    disposed: Cell<bool>,
}

impl WindowState {
    fn create_surface(&self, handle: &dyn INativePlatformHandleSurface) -> Option<WGPUSurface> {
        let source = self.display.source(handle);
        let descriptor = WGPUSurfaceDescriptor {
            nextInChain: &source.chain as *const WGPUChainedStruct as *mut WGPUChainedStruct,
            label: WGPUStringView { data: ptr::null(), length: 0 },
        };
        // SAFETY: the handle is the window of a top-level of the platform, which outlives its
        // render target (the platform disposes the target before it destroys the window), and
        // the target releases the surface when it is disposed; the connection is held by the
        // target; the descriptor and its chain are borrowed for the call.
        let surface = unsafe { wgpuInstanceCreateSurface(self.gpu.device().instance(), &descriptor) };
        if surface.is_null() {
            notes::failure("the WebGPU implementation made no surface for the window");
            return None;
        }
        Some(surface)
    }

    /// Configures the swap chain for `width` by `height` pixels.
    fn configure(&self, swap_chain: &mut SwapChain, width: u32, height: u32) -> bool {
        swap_chain.configuration = None;
        // SAFETY: an all-zero value is the empty capabilities the call fills in.
        let mut capabilities: WGPUSurfaceCapabilities = unsafe { std::mem::zeroed() };
        // SAFETY: the surface and the adapter are alive; the capabilities are written by the call
        // and their members freed below.
        let status = unsafe { wgpuSurfaceGetCapabilities(swap_chain.surface, self.gpu.device().adapter(), &mut capabilities) };
        if status != WGPUStatus_Success {
            notes::failure("the surface of the window reports no capabilities");
            return false;
        }
        // SAFETY: the call succeeded, so each array holds its count of values until it is freed.
        let chosen = unsafe {
            choose_configuration(
                raw_slice(capabilities.formats, capabilities.formatCount),
                raw_slice(capabilities.presentModes, capabilities.presentModeCount),
                raw_slice(capabilities.alphaModes, capabilities.alphaModeCount),
            )
        };
        // SAFETY: the members were allocated by the call above.
        unsafe { wgpuSurfaceCapabilitiesFreeMembers(capabilities) };
        let Some(configuration) = chosen else {
            notes::failure("the surface of the window has no format of four 8 bit channels");
            return false;
        };

        let format = if configuration.format == WGPUTextureFormat_BGRA8Unorm { TargetFormat::Bgra8 } else { TargetFormat::Rgba8 };
        if swap_chain.engine.as_ref().is_none_or(|(engine_format, _)| *engine_format != configuration.format) {
            let Some(engine) = self.gpu.engine(format) else { return false };
            swap_chain.engine = Some((configuration.format, engine));
        }

        let config = WGPUSurfaceConfiguration {
            nextInChain: ptr::null_mut(),
            device: self.gpu.device().device(),
            format: configuration.format,
            usage: WGPUTextureUsage_RenderAttachment,
            width,
            height,
            viewFormatCount: 0,
            viewFormats: ptr::null(),
            alphaMode: configuration.alpha_mode,
            presentMode: configuration.present_mode,
        };
        // SAFETY: the surface and the device are alive; the configuration is borrowed. A failure
        // leaves the surface unconfigured, which the next texture asked of it reports.
        unsafe { wgpuSurfaceConfigure(swap_chain.surface, &config) };
        swap_chain.configuration = Some(configuration);
        swap_chain.size = (width, height);
        true
    }

    /// The texture of the swap chain the frame is drawn into, at `width` by `height`, or `None`
    /// when the window shows no frame now.
    fn acquire(&self, handle: &dyn INativePlatformHandleSurface, width: u32, height: u32) -> Option<WindowFrame> {
        let mut slot = self.swap_chain.borrow_mut();
        if slot.is_none() {
            let surface = self.create_surface(handle)?;
            *slot = Some(SwapChain { surface, configuration: None, size: (0, 0), engine: None, reconfigure: false });
        }
        let swap_chain = slot.as_mut()?;
        let stale = swap_chain.configuration.is_none() || swap_chain.size != (width, height) || swap_chain.reconfigure;
        if stale {
            swap_chain.reconfigure = false;
            if !self.configure(swap_chain, width, height) {
                return None;
            }
        }
        for attempt in 0..2 {
            // SAFETY: an all-zero value is the empty result the call fills in.
            let mut result: WGPUSurfaceTexture = unsafe { std::mem::zeroed() };
            // SAFETY: the surface is alive and configured; the result is written by the call.
            unsafe { wgpuSurfaceGetCurrentTexture(swap_chain.surface, &mut result) };
            let action = if result.texture.is_null() && matches!(acquired(result.status), Acquired::Texture | Acquired::Suboptimal) {
                Acquired::Failed
            } else {
                acquired(result.status)
            };
            // A texture that comes with a status that is not drawn to is given back.
            if !result.texture.is_null() && !matches!(action, Acquired::Texture | Acquired::Suboptimal) {
                // SAFETY: the reference of the result is owned here.
                unsafe { wgpuTextureRelease(result.texture) };
            }
            match action {
                Acquired::Texture | Acquired::Suboptimal => {
                    swap_chain.reconfigure = action == Acquired::Suboptimal;
                    let engine = swap_chain.engine.as_ref()?.1.clone();
                    // SAFETY: the texture is the current one of the swap chain.
                    let view = unsafe { wgpuTextureCreateView(result.texture, ptr::null()) };
                    return Some(WindowFrame { texture: result.texture, view, engine });
                }
                Acquired::Reconfigure if attempt == 0 => {
                    if !self.configure(swap_chain, width, height) {
                        return None;
                    }
                }
                Acquired::Failed => {
                    // A configuration that failed reports here: the next frame configures again.
                    swap_chain.configuration = None;
                    notes::failure("the swap chain of the window gave no texture");
                    return None;
                }
                _ => return None,
            }
        }
        None
    }

    /// Draws what was recorded into the swap chain and presents it, unless the window has another
    /// size by now.
    fn present(&self, handle: &dyn INativePlatformHandleSurface, size: PixelSize, recording: &mut crate::gpu::Recording) {
        if self.disposed.get() || !scene_fits_window(size, handle.size()) {
            return;
        }
        let (width, height) = (size.width.max(1) as u32, size.height.max(1) as u32);
        let Some(frame) = self.acquire(handle, width, height) else { return };
        let drawn = recording.render(&frame.engine, frame.view, width, height, false);
        if let Err(error) = &drawn {
            notes::failure(&format!("a frame of a window was not drawn: {error}"));
        }
        // SAFETY: the view is owned here and the engine is done with it.
        unsafe { wgpuTextureViewRelease(frame.view) };
        if drawn.is_ok() {
            if let Some(swap_chain) = self.swap_chain.borrow().as_ref() {
                // SAFETY: the texture is the current one of this surface and was rendered to.
                if unsafe { wgpuSurfacePresent(swap_chain.surface) } != WGPUStatus_Success {
                    notes::failure("the frame of a window was not presented");
                }
            }
        }
        // SAFETY: the reference of the texture is owned here; the swap chain keeps the texture.
        unsafe { wgpuTextureRelease(frame.texture) };
    }
}

/// A slice of what a C call returned.
///
/// # Safety
/// With a count that is not zero, `data` points at that many values, alive for `'a`.
unsafe fn raw_slice<'a, T>(data: *const T, count: usize) -> &'a [T] {
    if count == 0 || data.is_null() {
        &[]
    } else {
        // SAFETY: the contract of this function.
        unsafe { std::slice::from_raw_parts(data, count) }
    }
}

/// The render target of a window of the Linux or Windows platform, drawn through a surface of the
/// WebGPU implementation.
pub(crate) struct WindowRenderTarget {
    state: Rc<WindowState>,
    platform_surface: Arc<dyn IPlatformRenderSurface>,
    scene: Rc<RefCell<Option<SceneBuilder>>>,
}

impl WindowRenderTarget {
    /// The target of the first surface that is a window, if one is.
    pub(crate) fn try_new(
        gpu: &Rc<Gpu>,
        display: &Rc<WindowDisplay>,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Self> {
        let surface = surfaces.iter().find(|surface| native_handle(&***surface).is_some())?;
        Some(Self {
            state: Rc::new(WindowState {
                gpu: gpu.clone(),
                display: display.clone(),
                swap_chain: RefCell::new(None),
                disposed: Cell::new(false),
            }),
            platform_surface: surface.clone(),
            scene: Rc::new(RefCell::new(None)),
        })
    }
}

impl IRenderTarget for WindowRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The compositor draws straight into the frame, without a layer of its own.
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
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        if self.state.disposed.get() {
            return Err(RenderTargetCorruptedError::new_with_message("The render target of the window was disposed.").into());
        }
        let Some(handle) = native_handle(&*self.platform_surface) else {
            return Err(RenderTargetNotReadyError::new_with_message("The surface of the window has no handle.").into());
        };
        let size = handle.size();
        if size.width <= 0 || size.height <= 0 || !scene_fits_window(scene_info.size, size) {
            return Err(RenderTargetNotReadyError::new().into());
        }

        let scene = self.scene.borrow_mut().take().unwrap_or_default();
        let (state, slot) = (self.state.clone(), self.scene.clone());
        let context = ProGpuDrawingContext::with_gpu(
            scene,
            Some(self.state.gpu.clone()),
            Some(Box::new(move |mut recording| {
                state.present(&*handle, size, &mut recording);
                crate::perf::window_frame();
                *slot.borrow_mut() = Some(recording.scene);
            })),
        )
        .with_target_size(size.width as u32, size.height as u32);
        Ok((Box::new(context), RenderTargetDrawingContextProperties { previous_frame_is_retained: false }))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        if self.state.disposed.get() {
            PlatformRenderTargetState::DISPOSED
        } else {
            PlatformRenderTargetState::READY
        }
    }

    fn dispose(&self) {
        self.state.disposed.set(true);
        *self.scene.borrow_mut() = None;
        *self.state.swap_chain.borrow_mut() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_fits_a_window_of_its_size_up_to_a_pixel_of_rounding() {
        let window = PixelSize::new(1400, 1000);
        assert!(scene_fits_window(PixelSize::new(1400, 1000), window));
        assert!(scene_fits_window(PixelSize::new(1399, 1001), window));
        assert!(!scene_fits_window(PixelSize::new(1026, 671), window));
        assert!(!scene_fits_window(PixelSize::new(1400, 1002), window));
    }

    #[test]
    fn a_swap_chain_prefers_bgra_without_waiting_and_opaque() {
        let chosen = choose_configuration(
            &[WGPUTextureFormat_RGBA8UnormSrgb, WGPUTextureFormat_RGBA8Unorm, WGPUTextureFormat_BGRA8Unorm],
            &[WGPUPresentMode_Fifo, WGPUPresentMode_Mailbox],
            &[WGPUCompositeAlphaMode_Premultiplied, WGPUCompositeAlphaMode_Opaque],
        );
        assert_eq!(
            Some(Configuration {
                format: WGPUTextureFormat_BGRA8Unorm,
                present_mode: WGPUPresentMode_Mailbox,
                alpha_mode: WGPUCompositeAlphaMode_Opaque,
            }),
            chosen
        );
    }

    #[test]
    fn a_swap_chain_takes_what_a_surface_has() {
        let chosen = choose_configuration(
            &[WGPUTextureFormat_RGBA8Unorm],
            &[WGPUPresentMode_Fifo],
            &[WGPUCompositeAlphaMode_Premultiplied],
        );
        assert_eq!(
            Some(Configuration {
                format: WGPUTextureFormat_RGBA8Unorm,
                present_mode: WGPUPresentMode_Fifo,
                alpha_mode: WGPUCompositeAlphaMode_Premultiplied,
            }),
            chosen
        );
        assert_eq!(None, choose_configuration(&[WGPUTextureFormat_RGBA8UnormSrgb], &[WGPUPresentMode_Fifo], &[]));
    }

    #[test]
    fn the_status_of_a_texture_says_what_to_do() {
        assert_eq!(Acquired::Texture, acquired(WGPUSurfaceGetCurrentTextureStatus_SuccessOptimal));
        assert_eq!(Acquired::Suboptimal, acquired(WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal));
        assert_eq!(Acquired::Reconfigure, acquired(WGPUSurfaceGetCurrentTextureStatus_Outdated));
        assert_eq!(Acquired::Reconfigure, acquired(WGPUSurfaceGetCurrentTextureStatus_Lost));
        assert_eq!(Acquired::Skip, acquired(WGPUSurfaceGetCurrentTextureStatus_Timeout));
        assert_eq!(Acquired::Skip, acquired(WGPUSurfaceGetCurrentTextureStatus_Occluded));
        assert_eq!(Acquired::Failed, acquired(WGPUSurfaceGetCurrentTextureStatus_Error));
    }
}
