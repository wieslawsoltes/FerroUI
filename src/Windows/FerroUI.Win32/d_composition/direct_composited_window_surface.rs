//! The surface of a window in the DirectComposition mode: a virtual
//! surface of DirectComposition, a rectangle of whose texture is drawn to
//! for each frame.

use super::direct_composited_window::Transaction;
use super::{DirectCompositedWindow, DirectCompositionShared, IDCompositionSurface, IDCompositionVirtualSurface};
use crate::direct_x::{
    IDirect3D11TexturePlatformSurface, IDirect3D11TexturePlatformSurface2, IDirect3D11TextureRenderTarget,
    IDirect3D11TextureRenderTarget2, IDirect3D11TextureRenderTargetRenderSession, DXGI_ALPHA_MODE, DXGI_FORMAT,
};
use crate::i_blur_host::{BlurEffect, ICompositionEffectsSurface};
use crate::interop::unmanaged_methods::RECT;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{PixelPoint, PixelSize, RenderTargetCorruptedException};
use ferroui_microcom::{ComPtr, Guid, HResult, IUnknown, Interface};
use ferroui_opengl::egl::IEglWindowGlPlatformSurfaceInfo;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError};

/// `IID_ID3D11Texture2D`.
pub(crate) const IID_ID3D11_TEXTURE2D: Guid = crate::direct_x::ID3D11Texture2D::IID;

struct Inner {
    info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    shared: Arc<DirectCompositionShared>,
    window: Mutex<Option<Arc<DirectCompositedWindow>>>,
}

/// The surface. It is shared between the threads and holds what the
/// threads share (the window, the shared state of the mode, and the
/// composition tree once the thread that renders created it); the view the
/// thread that renders asks for is a surface of its own over the same
/// state.
pub(crate) struct DirectCompositedWindowSurface {
    inner: Arc<Inner>,
}

impl DirectCompositedWindowSurface {
    pub fn new(
        shared: Arc<DirectCompositionShared>,
        info: Arc<dyn IEglWindowGlPlatformSurfaceInfo>,
    ) -> Arc<DirectCompositedWindowSurface> {
        Arc::new(DirectCompositedWindowSurface { inner: Arc::new(Inner { info, shared, window: Mutex::new(None) }) })
    }

    fn create_target(
        &self,
        context: &Rc<dyn IPlatformGraphicsContext>,
        d3d_device: isize,
    ) -> Result<Rc<DirectCompositedWindowRenderTarget>, HResult> {
        let window = {
            let mut window = self.inner.window.lock().unwrap_or_else(PoisonError::into_inner);
            match &*window {
                Some(window) => window.clone(),
                None => {
                    let created = DirectCompositedWindow::new(self.inner.info.clone(), self.inner.shared.clone())?;
                    *window = Some(created.clone());
                    created
                }
            }
        };
        DirectCompositedWindowRenderTarget::new(context.clone(), d3d_device, self.inner.shared.clone(), window)
    }

    /// Releases the composition tree of the window.
    pub fn dispose(&self) {
        let window = self.inner.window.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(window) = window {
            window.dispose();
        }
    }
}

impl IPlatformRenderSurface for DirectCompositedWindowSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        let view = || Rc::new(DirectCompositedWindowSurface { inner: self.inner.clone() });
        if kind == TypeId::of::<dyn IDirect3D11TexturePlatformSurface2>() {
            let this: Rc<dyn IDirect3D11TexturePlatformSurface2> = view();
            return Some(Rc::new(this));
        }
        if kind == TypeId::of::<dyn IDirect3D11TexturePlatformSurface>() {
            let this: Rc<dyn IDirect3D11TexturePlatformSurface> = view();
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDirect3D11TexturePlatformSurface for DirectCompositedWindowSurface {
    fn create_render_target(
        &self,
        context: &Rc<dyn IPlatformGraphicsContext>,
        d3d_device: isize,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget>, HResult> {
        Ok(self.create_target(context, d3d_device)?)
    }
}

impl IDirect3D11TexturePlatformSurface2 for DirectCompositedWindowSurface {
    fn create_render_target(
        &self,
        context: &Rc<dyn IPlatformGraphicsContext>,
        d3d_device: isize,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTarget2>, HResult> {
        Ok(self.create_target(context, d3d_device)?)
    }
}

impl ICompositionEffectsSurface for DirectCompositedWindowSurface {
    // TODO: we can implement BlurEffect.GaussianBlur in with IDCompositionDevice3.CreateGaussianBlurEffect.
    fn is_blur_supported(&self, effect: BlurEffect) -> bool {
        effect == BlurEffect::None
    }
}

/// Says which step of beginning a frame failed, with the result code of the
/// system, the size of the scene and of the surface, and the window.
fn log_frame_failure(stage: &str, error: HResult, scene: PixelSize, surface: PixelSize, handle: isize) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
        logger.log(
            None,
            &format!(
                "DirectComposition: a frame could not be begun: {stage} failed with {:#010X}; the scene is {} by {} pixels, \
                 the surface {} by {}, the window {handle:#x}",
                error.0, scene.width, scene.height, surface.width, surface.height
            ),
        );
    }
}

pub(crate) struct DirectCompositedWindowRenderTarget {
    context: Rc<dyn IPlatformGraphicsContext>,
    shared: Arc<DirectCompositionShared>,
    window: Arc<DirectCompositedWindow>,
    surface: RefCell<Option<ComPtr<IDCompositionVirtualSurface>>>,
    lost: Cell<bool>,
    size: Cell<PixelSize>,
    d3d_device: RefCell<Option<ComPtr<IUnknown>>>,
    is_surface_support_transparency: Cell<bool>,
}

impl DirectCompositedWindowRenderTarget {
    fn new(
        context: Rc<dyn IPlatformGraphicsContext>,
        d3d_device: isize,
        shared: Arc<DirectCompositionShared>,
        window: Arc<DirectCompositedWindow>,
    ) -> Result<Rc<DirectCompositedWindowRenderTarget>, HResult> {
        // SAFETY: the caller of the surface contract passes a pointer to
        // the device of Direct3D 11 of its display, which lives through the
        // call; the render target takes a reference of its own.
        let d3d_device = unsafe { ComPtr::<IUnknown>::from_raw_add_ref(d3d_device as *mut IUnknown) }.ok_or(HResult::POINTER)?;
        Ok(Rc::new(DirectCompositedWindowRenderTarget {
            context,
            shared,
            window,
            surface: RefCell::new(None),
            lost: Cell::new(false),
            size: Cell::new(PixelSize::default()),
            d3d_device: RefCell::new(Some(d3d_device)),
            is_surface_support_transparency: Cell::new(false),
        }))
    }

    fn create_surface(&self, scene_info: &RenderTargetSceneInfo) -> Result<ComPtr<IDCompositionVirtualSurface>, HResult> {
        let d3d_device = self.d3d_device.borrow();
        let d3d_device = d3d_device.as_ref().ok_or(HResult::OBJECTDISPOSED)?;
        let surface_factory = self.shared.device().create_surface_factory(Some(&**d3d_device))?.ok_or(HResult::POINTER)?;
        let is_transparency = scene_info.transparency_level != CompositionTransparencyLevel::None;
        let surface_size = scene_info.size;
        let alpha_mode =
            if is_transparency { DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_PREMULTIPLIED } else { DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_IGNORE };
        let surface = surface_factory
            .create_virtual_surface(
                surface_size.width as u32,
                surface_size.height as u32,
                DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM,
                alpha_mode,
            )?
            .ok_or(HResult::POINTER)?;
        self.is_surface_support_transparency.set(is_transparency);
        self.size.set(surface_size);
        Ok(surface)
    }

    fn current_state(&self) -> PlatformRenderTargetState {
        if self.context.is_lost() || self.lost.get() {
            PlatformRenderTargetState::CORRUPTED
        } else {
            PlatformRenderTargetState::READY
        }
    }

    fn begin_draw_scene(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException> {
        if self.current_state().is_corrupted {
            return Err(RenderTargetCorruptedException::new());
        }
        // Committed, and the lock left, when it is dropped: at the end of
        // the session, or here when the frame cannot be begun.
        let transaction = self.window.begin_transaction();

        // A failure here is an exception that is not the corrupted render
        // target in the reference; the contract of the port has the one
        // error, with the failure of the system as its cause.
        let failed = |stage: &'static str| {
            let (size, surface_size, handle) = (scene_info.size, self.size.get(), self.window.window_info().handle());
            move |error: HResult| {
                log_frame_failure(stage, error, size, surface_size, handle);
                RenderTargetCorruptedException::new_with_inner_exception(Rc::new(error))
            }
        };

        let is_transparency = scene_info.transparency_level != CompositionTransparencyLevel::None;
        let existing = self.surface.borrow().clone();
        let surface = match existing {
            Some(surface) if is_transparency == self.is_surface_support_transparency.get() => surface,
            _ => {
                *self.surface.borrow_mut() = None;
                let surface = self.create_surface(scene_info).map_err(failed("creating the virtual surface"))?;
                *self.surface.borrow_mut() = Some(surface.clone());
                surface
            }
        };

        let size = scene_info.size;
        let scale = scene_info.scaling;

        if self.size.get() != size {
            surface.resize(size.width as u32, size.height as u32).map_err(failed("resizing the virtual surface"))?;
            self.size.set(size);
        }
        let surface_interop: &IDCompositionSurface = &surface;
        self.window.set_surface(surface_interop).map_err(failed("setting the content of the visual"))?;

        let mut rect = RECT { left: 0, top: 0, right: size.width, bottom: size.height };
        let mut iid = IID_ID3D11_TEXTURE2D;
        let mut texture = std::ptr::null_mut();
        // SAFETY: the rectangle, the identifier and the pointer the
        // texture is written to are values of this frame that live through
        // the call.
        let offset = match unsafe { surface.begin_draw(&mut rect, &mut iid, &mut texture) } {
            Ok(offset) => offset,
            Err(error) => {
                self.lost.set(true);
                return Err(failed("beginning the draw on the virtual surface")(error));
            }
        };

        // SAFETY: the call succeeded, so the pointer is the texture asked
        // for, with a reference the caller owns.
        let Some(texture) = (unsafe { ComPtr::<IUnknown>::from_raw(texture.cast()) }) else {
            let _ = surface.end_draw();
            return Err(failed("taking the texture of the frame")(HResult::POINTER));
        };

        Ok(Rc::new(Session {
            texture_pointer: Cell::new(texture.as_ptr() as isize),
            state: RefCell::new(Some(SessionState {
                texture,
                surface_interop: ComPtr::from_ref(surface_interop),
                transaction,
            })),
            size,
            offset: PixelPoint::new(offset.x, offset.y),
            scaling: scale,
        }))
    }

    fn dispose_target(&self) {
        *self.surface.borrow_mut() = None;
        *self.d3d_device.borrow_mut() = None;
    }
}

impl IPlatformRenderSurfaceRenderTarget for DirectCompositedWindowRenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        self.current_state()
    }
}

impl IDirect3D11TextureRenderTarget for DirectCompositedWindowRenderTarget {
    fn begin_draw(&self) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException> {
        let info = self.window.window_info();
        let fallback_scene_info = RenderTargetSceneInfo::new(info.size(), info.scaling(), CompositionTransparencyLevel::None);
        self.begin_draw_scene(&fallback_scene_info)
    }

    fn dispose(&self) {
        self.dispose_target();
    }
}

impl IDirect3D11TextureRenderTarget2 for DirectCompositedWindowRenderTarget {
    fn begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IDirect3D11TextureRenderTargetRenderSession>, RenderTargetCorruptedException> {
        self.begin_draw_scene(scene_info)
    }

    fn dispose(&self) {
        self.dispose_target();
    }
}

struct SessionState {
    texture: ComPtr<IUnknown>,
    surface_interop: ComPtr<IDCompositionSurface>,
    transaction: Transaction,
}

struct Session {
    /// `None` once disposed.
    state: RefCell<Option<SessionState>>,
    texture_pointer: Cell<isize>,
    size: PixelSize,
    offset: PixelPoint,
    scaling: f64,
}

impl IDirect3D11TextureRenderTargetRenderSession for Session {
    fn d3d11_texture2d(&self) -> isize {
        self.texture_pointer.get()
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn offset(&self) -> PixelPoint {
        self.offset
    }

    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn dispose(&self) {
        let Some(SessionState { texture, surface_interop, transaction }) = self.state.borrow_mut().take() else {
            return;
        };
        self.texture_pointer.set(0);
        drop(texture);
        // A failure to end the frame throws out of the disposal in the
        // reference, after the transaction was disposed; the contract has
        // no error here, and a device that is lost is found by the next
        // frame.
        let _ = surface_interop.end_draw();
        drop(surface_interop);
        drop(transaction);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // A session that is dropped without being disposed still ends its
        // frame and leaves the lock.
        IDirect3D11TextureRenderTargetRenderSession::dispose(self);
    }
}
