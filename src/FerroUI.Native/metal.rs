//! Metal: the platform graphics, the device (graphics context), the render
//! surface of a top-level and its render target and drawing session.

use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::PixelSize;
use ferroui_microcom::{ComPtr, HResult};
use ferroui_skia::metal::{
    IMetalDevice, IMetalPlatformSurface, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::{Rc, Weak};

/// The Metal platform graphics of the macOS backend.
pub struct MetalPlatformGraphics {
    display: ComPtr<IFrnMetalDisplay>,
}

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
    weak_self: Weak<MetalDevice>,
    native: RefCell<Option<ComPtr<IFrnMetalDevice>>>,
}

impl MetalDevice {
    fn new(native: ComPtr<IFrnMetalDevice>) -> Rc<MetalDevice> {
        Rc::new_cyclic(|weak_self| MetalDevice { weak_self: weak_self.clone(), native: RefCell::new(Some(native)) })
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

    /// The device is only used on the UI thread, so there is nothing to
    /// lock or make current.
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        Disposable::empty()
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
        self.native().get_device()
    }

    fn command_queue(&self) -> *mut c_void {
        self.native().get_queue()
    }
}

/// The Metal render surface of a top-level.
pub struct MetalPlatformSurface {
    weak_self: Weak<MetalPlatformSurface>,
    top_level: ComPtr<IFrnTopLevel>,
}

impl MetalPlatformSurface {
    pub(crate) fn new(top_level: ComPtr<IFrnTopLevel>) -> Rc<MetalPlatformSurface> {
        Rc::new_cyclic(|weak_self| MetalPlatformSurface { weak_self: weak_self.clone(), top_level })
    }
}

impl IPlatformRenderSurface for MetalPlatformSurface {
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IMetalPlatformSurface>() {
            let this: Rc<dyn IMetalPlatformSurface> = self.weak_self.upgrade()?;
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
    /// Panics when called off the UI thread (the render target is not
    /// ready) and when `device` is not a device of this backend.
    fn create_metal_render_target(&self, device: Rc<dyn IMetalDevice>) -> Rc<dyn IMetalPlatformSurfaceRenderTarget> {
        if !Dispatcher::ui_thread().check_access() {
            panic!("The render target is not ready.");
        }

        let Some(dev) = device.as_any().downcast_ref::<MetalDevice>() else {
            panic!("The Metal device belongs to a different platform backend.");
        };
        let native_device = dev.native();
        let target = self.top_level.create_metal_render_target(Some(&native_device)).check();
        Rc::new(MetalRenderTarget { native: RefCell::new(target) })
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
    #[track_caller]
    fn session(&self) -> ComPtr<IFrnMetalRenderingSession> {
        match self.session.borrow().clone() {
            Some(session) => session,
            None => panic!("Cannot access a disposed object: MetalDrawingSession"),
        }
    }
}

impl IMetalPlatformSurfaceRenderingSession for MetalDrawingSession {
    fn texture(&self) -> *mut c_void {
        self.session().get_texture()
    }

    fn size(&self) -> PixelSize {
        let size = self.session().get_pixel_size().check();
        PixelSize::new(size.width, size.height)
    }

    fn scaling(&self) -> f64 {
        self.session().get_scaling()
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        let session = self.session.borrow_mut().take();
        drop(session);
    }
}
