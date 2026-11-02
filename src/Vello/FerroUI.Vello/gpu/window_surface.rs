//! The windows of the Linux and Windows platforms drawn by the GPU modes.
//!
//! The device is the backend's own: `wgpu` on Vulkan, or on OpenGL through
//! EGL where Vulkan has no adapter that is hardware (Linux), or on
//! Direct3D 12 (Windows), whatever graphics the platform chose for the
//! compositor (GLX or EGL on X11, ANGLE on Direct3D 11 or WGL on Windows,
//! Vulkan on either). The window is drawn through a surface of `wgpu` made
//! from the handle of the window, which the platform hands out as a render
//! surface (`INativePlatformHandleSurface`: an `XID` on X11, an `HWND` on
//! Windows); the surface configures the swap chain and presents on its own.
//! On X11 the surface is made with a connection to the X server of the
//! backend's own: an `XID` names the window on every connection.
//!
//! A machine whose only adapter is a software one (lavapipe, WARP) draws in
//! the CPU mode, which is faster there: the context of the platform then
//! gets no GPU of the backend and the window its framebuffer, unless the
//! environment variable `FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER` is `1`.
//!
//! The frame of the window is kept from frame to frame as it is on macOS
//! ([`WindowFrames`]): the compositor draws what changed into it, and the
//! texture of the swap chain gets a copy before it is presented.
//!
//! This file has `unsafe` for the raw handles of the window and the display
//! a surface is made from.

use crate::gpu::vello_wgpu_device::{block_on, device_descriptor};
use crate::gpu::window_frame::WindowFrames;
use crate::gpu::{IVelloGpu, VelloWgpuDevice, VelloWgpuDeviceError};
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IPlatformGraphicsContext, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetError, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::{PixelSize, RenderTargetNotReadyError};
use ferroui_controls::platform::INativePlatformHandleSurface;
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use wgpu::rwh;

/// The kind of window handle a surface of `wgpu` is made from on this
/// platform.
#[cfg(target_os = "linux")]
const WINDOW_HANDLE_DESCRIPTOR: &str = "XID";
#[cfg(windows)]
const WINDOW_HANDLE_DESCRIPTOR: &str = "HWND";

/// The graphics API of the platform the device of the backend is made on
/// first.
#[cfg(target_os = "linux")]
pub const PRIMARY_BACKENDS: wgpu::Backends = wgpu::Backends::VULKAN;
#[cfg(windows)]
pub const PRIMARY_BACKENDS: wgpu::Backends = wgpu::Backends::DX12;

fn unavailable(reason: impl Into<String>) -> VelloWgpuDeviceError {
    VelloWgpuDeviceError { reason: reason.into() }
}

/// Whether an adapter that is software (lavapipe, WARP) may draw the GPU
/// modes: only when the environment asks for it.
fn software_adapters_allowed() -> bool {
    std::env::var("FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER").is_ok_and(|value| value == "1")
}

/// The handle of the window a render surface is, when it is one of the
/// kind of this platform.
fn native_handle(surface: &dyn IPlatformRenderSurface) -> Option<Rc<dyn INativePlatformHandleSurface>> {
    let kind = surface.try_get_surface_kind(TypeId::of::<dyn INativePlatformHandleSurface>())?;
    let handle = kind.downcast_ref::<Rc<dyn INativePlatformHandleSurface>>()?.clone();
    (handle.handle_descriptor() == Some(WINDOW_HANDLE_DESCRIPTOR) && handle.handle() != 0).then_some(handle)
}

/// A connection to the X server of the backend's own, for the surfaces of
/// the windows it draws.
#[cfg(target_os = "linux")]
struct XConnection {
    xlib: x11_dl::xlib::Xlib,
    display: std::ptr::NonNull<x11_dl::xlib::Display>,
    screen: std::os::raw::c_int,
}

// SAFETY: the connection is used by the thread that renders only (the
// device, its surfaces and EGL on it); `wgpu` asks the instance for the
// handle to be `Send` and `Sync`.
#[cfg(target_os = "linux")]
unsafe impl Send for XConnection {}
#[cfg(target_os = "linux")]
unsafe impl Sync for XConnection {}

#[cfg(target_os = "linux")]
impl std::fmt::Debug for XConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XConnection").field("display", &self.display).finish()
    }
}

/// The display of the backend's connection as `wgpu` takes it: the instance
/// of OpenGL through EGL is made for it.
#[cfg(target_os = "linux")]
#[derive(Debug)]
struct XDisplayHandle(Arc<XConnection>);

#[cfg(target_os = "linux")]
impl rwh::HasDisplayHandle for XDisplayHandle {
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, rwh::HandleError> {
        let handle = rwh::XlibDisplayHandle::new(Some(self.0.display.cast()), self.0.screen);
        // SAFETY: the display stays open while the handle lives: it holds
        // the connection.
        Ok(unsafe { rwh::DisplayHandle::borrow_raw(rwh::RawDisplayHandle::Xlib(handle)) })
    }
}

#[cfg(target_os = "linux")]
impl XConnection {
    /// Opens a connection to the display of the environment (`DISPLAY`),
    /// the one the X11 platform connects to.
    fn open() -> Result<Self, String> {
        let xlib = x11_dl::xlib::Xlib::open().map_err(|error| format!("libX11 was not loaded ({error})"))?;
        // SAFETY: a null name opens the display of the environment; the
        // connection is closed when this value is dropped, after every
        // surface made with it (the render targets hold the GPU that holds
        // it).
        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };
        let display = std::ptr::NonNull::new(display).ok_or("the X server of the environment refused a connection")?;
        // SAFETY: the display was just opened.
        let screen = unsafe { (xlib.XDefaultScreen)(display.as_ptr()) };
        Ok(Self { xlib, display, screen })
    }
}

#[cfg(target_os = "linux")]
impl Drop for XConnection {
    fn drop(&mut self) {
        // SAFETY: the display was opened by `open` and is closed once.
        unsafe { (self.xlib.XCloseDisplay)(self.display.as_ptr()) };
    }
}

/// The GPU of the Vello backend for the windows of the Linux and Windows
/// platforms: a device of its own and surfaces of `wgpu` over the handles
/// of the windows (see the module).
pub struct VelloSurfaceGpu {
    context: Rc<dyn IPlatformGraphicsContext>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<VelloWgpuDevice>,
    /// Swap chains of render targets that were disposed, to be dropped
    /// when the next frame of a window starts (see
    /// [`VelloSurfaceRenderTarget::dispose`]).
    retired: RefCell<Vec<WindowSwapChain>>,
    #[cfg(target_os = "linux")]
    x_connection: Arc<XConnection>,
}

impl VelloSurfaceGpu {
    /// Drops the swap chains of render targets that were disposed.
    fn drop_retired(&self) {
        let retired = std::mem::take(&mut *self.retired.borrow_mut());
        for swap_chain in retired {
            // A swap chain that cannot be dropped (its context cannot be
            // made current) is left to the end of the process.
            let dropped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || drop(swap_chain)));
            if dropped.is_err() {
                log_window_failure("the swap chain of a window that was closed could not be released");
            }
        }
    }

    /// Makes the device of the backend for the graphics context of a
    /// platform, which it keeps (the compositor's GPU is the context's).
    ///
    /// Fails when `wgpu` has no adapter on the graphics API of the
    /// platform, when the only one is software and that is not allowed, and
    /// when the adapter gives no device.
    /// `backends` are the graphics APIs the adapter is looked for on:
    /// [`PRIMARY_BACKENDS`], or OpenGL through EGL (Linux) where the
    /// platform has no context of OpenGL of its own to draw on
    /// (`gl_context_gpu.rs`).
    pub fn new(context: Rc<dyn IPlatformGraphicsContext>, backends: wgpu::Backends) -> Result<Rc<Self>, VelloWgpuDeviceError> {
        #[cfg(target_os = "linux")]
        let x_connection = Arc::new(XConnection::open().map_err(unavailable)?);
        #[cfg(target_os = "linux")]
        let descriptor = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(XDisplayHandle(x_connection.clone())));
        #[cfg(windows)]
        let descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends, ..descriptor });

        // The adapter: hardware first, by the order of the graphics APIs;
        // a software one only when the environment allows it.
        let adapters = block_on(instance.enumerate_adapters(backends));
        let rank = |adapter: &wgpu::Adapter| {
            let info = adapter.get_info();
            let software = info.device_type == wgpu::DeviceType::Cpu;
            let api = match info.backend {
                wgpu::Backend::Vulkan | wgpu::Backend::Dx12 => 0,
                _ => 1,
            };
            (software, api)
        };
        let names = adapters.iter().map(|adapter| adapter.get_info().name).collect::<Vec<_>>();
        let adapter = adapters
            .into_iter()
            .filter(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu || software_adapters_allowed())
            .min_by_key(rank)
            .ok_or_else(|| {
                unavailable(format!(
                    "no adapter that is hardware for {backends:?} (found: {names:?}): the CPU mode draws instead \
                     (FERROUI_VELLO_ALLOW_SOFTWARE_ADAPTER=1 allows a software one)"
                ))
            })?;
        let info = adapter.get_info();

        let (device, queue) = block_on(adapter.request_device(&device_descriptor(&adapter)))
            .map_err(|error| unavailable(format!("the adapter {} gave no device ({error})", info.name)))?;
        let device = VelloWgpuDevice::new(&adapter, device, queue);
        // Layers and bitmaps of the GPU modes are drawn on this device too.
        device.prefer();

        Ok(Rc::new(Self {
            context,
            instance,
            adapter,
            device,
            retired: RefCell::new(Vec::new()),
            #[cfg(target_os = "linux")]
            x_connection,
        }))
    }

    /// Makes a surface of `wgpu` for the window of a handle.
    fn create_surface(&self, handle: &dyn INativePlatformHandleSurface) -> Result<wgpu::Surface<'static>, String> {
        let (display, window) = self.raw_handles(handle)?;
        // SAFETY: the handle is the window of a top-level of the platform,
        // which outlives its render target (the platform disposes the
        // target before it destroys the window), and the render target
        // drops the surface when it is disposed; the display connection is
        // the backend's own and lives as long as this GPU, which the render
        // target holds.
        unsafe {
            self.instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(display),
                raw_window_handle: window,
            })
        }
        .map_err(|error| format!("wgpu made no surface for the window ({error})"))
    }

    #[cfg(target_os = "linux")]
    fn raw_handles(
        &self,
        handle: &dyn INativePlatformHandleSurface,
    ) -> Result<(rwh::RawDisplayHandle, rwh::RawWindowHandle), String> {
        let connection = &self.x_connection;
        let display = rwh::XlibDisplayHandle::new(Some(connection.display.cast()), connection.screen);
        let window = rwh::XlibWindowHandle::new(handle.handle() as std::os::raw::c_ulong);
        Ok((rwh::RawDisplayHandle::Xlib(display), rwh::RawWindowHandle::Xlib(window)))
    }

    #[cfg(windows)]
    fn raw_handles(
        &self,
        handle: &dyn INativePlatformHandleSurface,
    ) -> Result<(rwh::RawDisplayHandle, rwh::RawWindowHandle), String> {
        let hwnd = std::num::NonZeroIsize::new(handle.handle()).ok_or("the window has no handle")?;
        let window = rwh::Win32WindowHandle::new(hwnd);
        Ok((rwh::RawDisplayHandle::Windows(rwh::WindowsDisplayHandle::new()), rwh::RawWindowHandle::Win32(window)))
    }
}

impl Drop for VelloSurfaceGpu {
    fn drop(&mut self) {
        self.drop_retired();
    }
}

impl IVelloGpu for VelloSurfaceGpu {
    fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    fn platform_graphics_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.context.clone()
    }

    fn try_create_render_target(
        self: Rc<Self>,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
        rendering_modes: &[VelloRenderingMode],
    ) -> Option<Rc<dyn IRenderTarget>> {
        let surface = surfaces.iter().find(|surface| native_handle(&***surface).is_some())?;
        Some(Rc::new(VelloSurfaceRenderTarget::new(self, surface.clone(), rendering_modes.to_vec())))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        surfaces.iter().find(|surface| native_handle(&***surface).is_some()).is_some_and(|surface| surface.is_ready())
    }

    fn dispose(&self) {
        // The device ends with the last scene that holds it; what the
        // renderers keep on it goes with it.
    }
}

/// The surface of `wgpu` of a window with the configuration of its swap
/// chain.
struct WindowSwapChain {
    surface: wgpu::Surface<'static>,
    config: Option<wgpu::SurfaceConfiguration>,
}

/// The render target of a window of the Linux or Windows platform drawn
/// through a surface of `wgpu`.
///
/// It keeps the frame of the window ([`WindowFrames`]) and says that it
/// retains it and can be rendered to directly, as the render target of a
/// Metal surface does: the compositor draws what changed straight into the
/// frame, and the texture of the swap chain gets a copy of it before it is
/// presented.
pub struct VelloSurfaceRenderTarget {
    gpu: Rc<VelloSurfaceGpu>,
    platform_surface: Arc<dyn IPlatformRenderSurface>,
    swap_chain: RefCell<Option<WindowSwapChain>>,
    frames: WindowFrames,
    /// Whether the swap chain is to be configured again before the next
    /// frame: the last texture was suboptimal.
    reconfigure: Cell<bool>,
    disposed: Cell<bool>,
}

impl VelloSurfaceRenderTarget {
    fn new(
        gpu: Rc<VelloSurfaceGpu>,
        platform_surface: Arc<dyn IPlatformRenderSurface>,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        let frames = WindowFrames::new(gpu.device.clone(), rendering_modes);
        Self {
            gpu,
            platform_surface,
            swap_chain: RefCell::new(None),
            frames,
            reconfigure: Cell::new(false),
            disposed: Cell::new(false),
        }
    }

    /// The texture of the swap chain the next frame is presented in, at
    /// `size`, or `None` when the window shows no frame now (it is hidden,
    /// or the surface could not be made or configured: the frame is still
    /// drawn into the frame of the window and shown with the next one).
    fn acquire(&self, handle: &dyn INativePlatformHandleSurface, size: PixelSize) -> Option<wgpu::SurfaceTexture> {
        let mut swap_chain = self.swap_chain.borrow_mut();
        if swap_chain.is_none() {
            match self.gpu.create_surface(handle) {
                Ok(surface) => *swap_chain = Some(WindowSwapChain { surface, config: None }),
                Err(error) => {
                    log_window_failure(&error);
                    return None;
                }
            }
        }
        let swap_chain = swap_chain.as_mut()?;

        let (width, height) = (size.width.max(1) as u32, size.height.max(1) as u32);
        let fits = swap_chain.config.as_ref().is_some_and(|config| config.width == width && config.height == height);
        if !fits || self.reconfigure.replace(false) {
            swap_chain.config = self.configure(&swap_chain.surface, width, height);
        }
        swap_chain.config.as_ref()?;

        for attempt in 0..2 {
            match swap_chain.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(texture) => return Some(texture),
                wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                    self.reconfigure.set(true);
                    return Some(texture);
                }
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost if attempt == 0 => {
                    swap_chain.config = self.configure(&swap_chain.surface, width, height);
                    swap_chain.config.as_ref()?;
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    log_window_failure("the texture of the swap chain was refused by the validation of wgpu");
                    return None;
                }
                _ => return None,
            }
        }
        None
    }

    /// Configures the swap chain of a surface for `width` by `height`
    /// pixels: a format of four 8 bit channels that is not sRGB (the
    /// renderers write the encoded colors), opaque unless the surface has
    /// no opaque mode, without waiting for the display when it can.
    fn configure(&self, surface: &wgpu::Surface<'static>, width: u32, height: u32) -> Option<wgpu::SurfaceConfiguration> {
        let capabilities = surface.get_capabilities(&self.gpu.adapter);
        let format = [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba8Unorm]
            .into_iter()
            .find(|format| capabilities.formats.contains(format));
        let Some(format) = format else {
            log_window_failure(&format!("the surface has no format of four 8 bit channels: {:?}", capabilities.formats));
            return None;
        };
        let mut config = surface.get_default_config(&self.gpu.adapter, width, height)?;
        config.format = format;
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        config.present_mode = if capabilities.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };
        if capabilities.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        }
        config.view_formats = Vec::new();
        surface.configure(self.gpu.device.device(), &config);
        Some(config)
    }
}

/// Whether a scene of `scene` pixels is the one for a window of `window` pixels: the same size, or
/// one pixel off in either direction, the rounding between the logical size of the scene and the
/// pixels of the window.
fn scene_fits_window(scene: PixelSize, window: PixelSize) -> bool {
    (scene.width - window.width).abs() <= 1 && (scene.height - window.height).abs() <= 1
}

fn log_window_failure(error: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        logger.log_with_values(None, "The Vello backend could not present a frame of a window: {Error}", &[&error]);
    }
}

impl IRenderTarget for VelloSurfaceRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The frame of the window is kept by the render target (a texture of
        // the swap chain holds nothing of the frame before), and the
        // compositor draws into it directly.
        RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
    }

    /// Not ready while the window has a size other than the one the scene was laid out for
    /// (beyond a pixel of rounding): a frame of the window's size would show the scene of the
    /// other size with a strip of the frame before it, and a frame of the scene's size would be
    /// stretched to the window by the swap chain (`DXGI_SCALING_STRETCH` of `wgpu` on DX12). The
    /// compositor skips the frame and asks again with the next tick; the platform renders the
    /// scene of the new size itself when it resizes (issue 476, cooperative resize).
    fn try_create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        if !self.disposed.get() {
            if let Some(handle) = native_handle(&*self.platform_surface) {
                if !scene_fits_window(scene_info.size, handle.size()) {
                    return Err(RenderTargetNotReadyError::new().into());
                }
            }
        }
        Ok(self.create_drawing_context(scene_info))
    }

    fn create_drawing_context(
        &self,
        _scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        if self.disposed.get() {
            panic!("VelloSurfaceRenderTarget has been disposed");
        }
        let frame_start = crate::perf::frame_start();
        self.gpu.drop_retired();

        let handle = native_handle(&*self.platform_surface)
            .unwrap_or_else(|| panic!("The surface of the window no longer has a handle"));
        let size = handle.size();
        let pixel_size = PixelSize::new(size.width.max(1), size.height.max(1));
        let texture = self.acquire(&*handle, pixel_size);

        let frame = self.frames.frame_for(pixel_size, VelloPlatform::default_dpi() * handle.scaling());
        let properties = RenderTargetDrawingContextProperties { previous_frame_is_retained: frame.has_content() };

        let (device, copied) = (self.gpu.device.clone(), frame.clone());
        let (width, height) = (pixel_size.width as u32, pixel_size.height as u32);
        let disposables: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || {
            if let Some(texture) = texture {
                copied.copy_to(&device, &texture.texture);
                let _perf = crate::perf::scope(crate::perf::Phase::Present, 0);
                device.queue().present(texture);
            }
            crate::perf::frame_end(frame_start, "window", width, height);
        })];

        (Box::new(frame.create_drawing_context_with(disposables)), properties)
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        if self.disposed.get() {
            return PlatformRenderTargetState::DISPOSED;
        }
        if self.gpu.device.is_lost() {
            return PlatformRenderTargetState::CORRUPTED;
        }
        PlatformRenderTargetState::READY
    }

    fn dispose(&self) {
        self.disposed.set(true);
        self.frames.dispose();
        // The compositor disposes a render target with the graphics context
        // of the platform current (`ensure_current`): with a context of GLX
        // current on the thread, a device of `wgpu` on OpenGL through EGL
        // cannot make its own context current, which dropping the swap
        // chain does. It is dropped when the next frame of a window starts,
        // or with the GPU.
        if let Some(swap_chain) = self.swap_chain.borrow_mut().take() {
            self.gpu.retired.borrow_mut().push(swap_chain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scene_fits_window;
    use ferroui_base::PixelSize;

    #[test]
    fn a_scene_fits_a_window_of_its_size_up_to_a_pixel_of_rounding() {
        let window = PixelSize::new(1400, 1000);
        assert!(scene_fits_window(PixelSize::new(1400, 1000), window));
        assert!(scene_fits_window(PixelSize::new(1399, 1001), window));
        // A scene laid out before the window grew or shrank is not drawn into it.
        assert!(!scene_fits_window(PixelSize::new(1026, 671), window));
        assert!(!scene_fits_window(PixelSize::new(1400, 1002), window));
        assert!(!scene_fits_window(PixelSize::new(1600, 1000), window));
    }
}
