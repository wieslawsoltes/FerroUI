//! Metal: the platform graphics, the device (graphics context), the render
//! surface of a top-level and its render target and drawing session.

use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::top_level_impl::SurfaceTopLevel;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::PixelSize;
use ferroui_microcom::{ComPtr, HResult};
use ferroui_skia::metal::{
    IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::{Arc, Weak};

/// The Metal platform graphics of the macOS backend.
pub struct MetalPlatformGraphics {
    display: ComPtr<IFrnMetalDisplay>,
}

// SAFETY: the platform graphics are shared by the UI thread and the thread
// that renders (`IPlatformGraphics: Send + Sync`), and each may call
// `CreateDevice` and drop its handle. The display is a process-lifetime
// singleton of the native side (`FrnMetalDisplay`, held by a `ComStaticPtr`
// in `native/FerroUI.Native/src/OSX/metal.mm`) without state: `CreateDevice`
// reads nothing of the object and makes a new device and command queue per
// call, so two threads may be inside it at once. The pointer is never
// replaced, and the reference this object holds is taken once and given
// back once, by whichever thread drops the last handle: the reference count
// of a native object is atomic (`ComObject` in
// `native/FerroUI.Native/inc/comimpl.h`). The device a call returns belongs
// to the thread that asked.
unsafe impl Send for MetalPlatformGraphics {}
// SAFETY: see `Send`.
unsafe impl Sync for MetalPlatformGraphics {}

impl MetalPlatformGraphics {
    /// Obtains the Metal display of the native side; fails when Metal is
    /// not available.
    pub fn new(factory: &IFerroNativeFactory) -> Result<MetalPlatformGraphics, HResult> {
        let display = factory.obtain_metal_display()?.ok_or(HResult::POINTER)?;
        Ok(MetalPlatformGraphics { display })
    }

    /// Creates a Metal device; fails when the system has none.
    pub fn try_create_context(&self) -> Result<Rc<MetalDevice>, HResult> {
        let native = self.display.create_device()?.ok_or(HResult::POINTER)?;
        Ok(MetalDevice::new(native))
    }
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MetalPlatformGraphics>();
};

impl IPlatformGraphics for MetalPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.try_create_context().check()
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.");
    }
}

/// A Metal device and its command queue.
pub struct MetalDevice {
    weak_self: std::rc::Weak<MetalDevice>,
    sync_root: ferroui_base::utilities::DisposableLock,
    native: RefCell<Option<ComPtr<IFrnMetalDevice>>>,
}

impl MetalDevice {
    fn new(native: ComPtr<IFrnMetalDevice>) -> Rc<MetalDevice> {
        Rc::new_cyclic(|weak_self| MetalDevice {
            weak_self: weak_self.clone(),
            sync_root: ferroui_base::utilities::DisposableLock::new(),
            native: RefCell::new(Some(native)),
        })
    }

    /// The native device.
    ///
    /// # Panics
    /// Panics when the device is disposed.
    #[track_caller]
    pub fn native(&self) -> ComPtr<IFrnMetalDevice> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: MetalDevice"),
        }
    }

    /// Calls the native device where it is held, without taking a reference
    /// of its own: for the getters, which neither take time nor release the
    /// device.
    ///
    /// # Panics
    /// Panics when the device is disposed.
    #[track_caller]
    fn with_native<R>(&self, f: impl FnOnce(&IFrnMetalDevice) -> R) -> R {
        match self.native.borrow().as_ref() {
            Some(native) => f(&**native),
            None => panic!("Cannot access a disposed object: MetalDevice"),
        }
    }
}

impl IOptionalFeatureProvider for MetalDevice {
    /// The device announces itself as a Metal device. The external-objects
    /// features (GPU handle wrapping, IOSurface and shared event import)
    /// are absent until their contracts are ported.
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IMetalDevice>() {
            let this: Rc<dyn IMetalDevice> = self.weak_self.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for MetalDevice {
    fn is_lost(&self) -> bool {
        false
    }

    /// Takes the lock of the device: the UI thread and the render thread
    /// both use it, one at a time.
    ///
    /// The device lives in the graph of the server compositor, so every
    /// caller already holds the compositor lock: the order is always the
    /// compositor lock first, then this one. Something that used the device
    /// outside the compositor would have to keep that order.
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.sync_root.lock()
    }

    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalDevice for MetalDevice {
    fn device(&self) -> *mut c_void {
        self.with_native(|native| native.get_device())
    }

    fn command_queue(&self) -> *mut c_void {
        self.with_native(|native| native.get_queue())
    }
}

/// The Metal render surface of a top-level.
///
/// The surface is shared with the thread that renders
/// (`IPlatformRenderSurface: Send + Sync`). What it holds of the native side
/// is the native top-level, which only the UI thread may use (see
/// `SurfaceTopLevel`): the render target is created on the UI thread, as in
/// the reference, and the native side refuses anything else
/// (`TopLevelImpl::CreateMetalRenderTarget` in
/// `native/FerroUI.Native/src/OSX/TopLevelImpl.mm` returns
/// `COR_E_INVALIDOPERATION` off the main thread).
pub struct MetalPlatformSurface {
    weak_self: Weak<MetalPlatformSurface>,
    top_level: SurfaceTopLevel,
}

impl MetalPlatformSurface {
    pub(crate) fn new(top_level: ComPtr<IFrnTopLevel>) -> Arc<MetalPlatformSurface> {
        Arc::new_cyclic(|weak_self| MetalPlatformSurface {
            weak_self: weak_self.clone(),
            top_level: SurfaceTopLevel::new(top_level),
        })
    }

    /// Releases the native top-level; called by the top-level when it is
    /// disposed, on the UI thread.
    pub(crate) fn close(&self) {
        self.top_level.release();
    }
}

impl IPlatformRenderSurface for MetalPlatformSurface {
    /// Deviation (DEVIATIONS.md, Native backend): the reference throws
    /// `RenderTargetNotReadyException` from `CreateMetalRenderTarget` off the
    /// UI thread and the composition target catches it; here the surface
    /// answers that it is not ready, which the composition target treats the
    /// same way, without a panic to catch.
    fn is_ready(&self) -> bool {
        self.top_level.get().is_some()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            // The contract hands out an `Rc`, and the surface lives in an
            // `Arc`: the view forwards to the surface.
            let this: Rc<dyn IMetalPlatformSurface> = Rc::new(MetalPlatformSurfaceView(self.weak_self.upgrade()?));
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IMetalPlatformSurface for MetalPlatformSurface {
    /// # Panics
    /// Panics when called off the UI thread or when the top-level is
    /// disposed (the render target is not ready) and when `device` is not a
    /// device of this backend.
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        if !Dispatcher::ui_thread().check_access() {
            panic!("The render target is not ready.");
        }

        let Some(top_level) = self.top_level.get() else {
            panic!("The render target is not ready.");
        };

        let Some(dev) = device.as_any().downcast_ref::<MetalDevice>() else {
            panic!("The Metal device belongs to a different platform backend.");
        };
        let native_device = dev.native();
        let target = top_level.create_metal_render_target(Some(&native_device)).check();
        Rc::new(MetalRenderTarget { native: RefCell::new(target) })
    }
}

/// The Metal surface as the Metal contract hands it out
/// (`Rc<dyn IMetalPlatformSurface>`); it forwards to the shared surface.
struct MetalPlatformSurfaceView(Arc<MetalPlatformSurface>);

impl IPlatformRenderSurface for MetalPlatformSurfaceView {
    fn is_ready(&self) -> bool {
        self.0.is_ready()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.0.try_get_surface_kind(kind)
    }

    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }
}

impl IMetalPlatformSurface for MetalPlatformSurfaceView {
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        self.0.create_metal_render_target(device)
    }
}

/// The Metal render target of a top-level.
pub struct MetalRenderTarget {
    native: RefCell<Option<ComPtr<IFrnMetalRenderTarget>>>,
}

impl MetalRenderTarget {
    #[track_caller]
    fn native(&self) -> ComPtr<IFrnMetalRenderTarget> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: MetalRenderTarget"),
        }
    }
}

impl IPlatformRenderSurfaceRenderTarget for MetalRenderTarget {}

impl IMetalPlatformSurfaceRenderTarget for MetalRenderTarget {
    fn begin_rendering(&self) -> Rc<dyn IMetalPlatformSurfaceRenderingSession> {
        let session = self.native().begin_drawing().check();
        Rc::new(MetalDrawingSession { session: RefCell::new(session) })
    }

    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }
}

/// One frame drawn to the Metal render target of a top-level; releasing the
/// native session presents the frame.
pub struct MetalDrawingSession {
    session: RefCell<Option<ComPtr<IFrnMetalRenderingSession>>>,
}

impl MetalDrawingSession {
    /// Calls the native session where it is held, without taking a reference
    /// of its own: its members are getters, and releasing the session is
    /// what presents the frame.
    ///
    /// # Panics
    /// Panics when the session is disposed.
    #[track_caller]
    fn with_session<R>(&self, f: impl FnOnce(&IFrnMetalRenderingSession) -> R) -> R {
        match self.session.borrow().as_ref() {
            Some(session) => f(&**session),
            None => panic!("Cannot access a disposed object: MetalDrawingSession"),
        }
    }
}

impl IMetalPlatformSurfaceRenderingSession for MetalDrawingSession {
    fn texture(&self) -> *mut c_void {
        self.with_session(|session| session.get_texture())
    }

    fn size(&self) -> PixelSize {
        let size = self.with_session(|session| session.get_pixel_size()).check();
        PixelSize::new(size.width, size.height)
    }

    fn scaling(&self) -> f64 {
        self.with_session(|session| session.get_scaling())
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        let session = self.session.borrow_mut().take();
        drop(session);
    }
}
