//! A context of WGL (the port of `OpenGl/WglContext.cs`).

use super::wgl_display::WglDisplay;
use super::wgl_gdi_resource_manager::WglGdiResourceManager;
use super::wgl_restore_context::WglRestoreContext;
use crate::interop::unmanaged_methods::{
    get_proc_address, set_pixel_format, wgl_delete_context, wgl_get_current_context, wgl_get_current_dc,
    wgl_get_proc_address,
};
use crate::interop::unmanaged_methods::PixelFormatDescriptor;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext, PlatformGraphicsContextLostException};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_opengl::{GetProcAddress, GlInterface, GlVersion, IGlContext, OpenGlException};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell};
use std::ffi::{c_void, CString};
use std::rc::{Rc, Weak};

/// A context of WGL, with the offscreen window and the device context it
/// was created for.
// The reference guards the context with a monitor that `MakeCurrent`
// enters and the returned object leaves (`Lock`). A context here is an
// object of the thread that created it (`Rc`), so there is nothing to
// guard, as for the contexts of EGL and of GLX.
pub struct WglContext {
    this: Weak<WglContext>,
    shared_with: Option<Rc<WglContext>>,
    context: Cell<isize>,
    hwnd: isize,
    dc: isize,
    pixel_format: i32,
    format_descriptor: PixelFormatDescriptor,
    version: GlVersion,
    /// Set by the constructor, once the context is current for the first time.
    gl_interface: OnceCell<Rc<GlInterface>>,
    stencil_size: i32,
    is_lost: Cell<bool>,
}

impl WglContext {
    pub(crate) fn new(
        shared_with: Option<Rc<WglContext>>,
        version: GlVersion,
        context: isize,
        hwnd: isize,
        dc: isize,
        pixel_format: i32,
        format_descriptor: PixelFormatDescriptor,
    ) -> Result<Rc<WglContext>, OpenGlException> {
        let this = Rc::new_cyclic(|this| WglContext {
            this: this.clone(),
            shared_with,
            context: Cell::new(context),
            hwnd,
            dc,
            pixel_format,
            format_descriptor,
            version,
            gl_interface: OnceCell::new(),
            stencil_size: i32::from(format_descriptor.stencil_bits),
            is_lost: Cell::new(false),
        });

        let current = this.try_make_current()?;
        let get_proc: GetProcAddress = Rc::new(|proc: &str| -> *const c_void {
            let Ok(proc) = CString::new(proc) else {
                return std::ptr::null();
            };
            let ext = wgl_get_proc_address(&proc);
            if !ext.is_null() {
                return ext;
            }
            get_proc_address(WglDisplay::open_gl32_handle(), &proc).map_or(std::ptr::null(), |entry| entry as *const c_void)
        });
        // SAFETY: the loader answers null or the address of the named entry
        // point, of the driver of the context that was made current above
        // or of the OpenGL library of the system.
        let gl_interface = unsafe { GlInterface::new(version, get_proc) };
        let _ = this.gl_interface.set(Rc::new(gl_interface));
        current.dispose();

        Ok(this)
    }

    /// The handle of the context (`Handle`).
    pub fn handle(&self) -> isize {
        self.context.get()
    }

    /// The shared handle of the context.
    ///
    /// # Panics
    /// Panics when the context is being dropped.
    pub(crate) fn this(&self) -> Rc<WglContext> {
        self.this.upgrade().expect("the context is alive")
    }

    fn is_current(&self) -> bool {
        wgl_get_current_context() == self.context.get() && wgl_get_current_dc() == self.dc
    }

    /// `MakeCurrent()` with its failure as a result.
    ///
    /// # Panics
    /// Panics when the context is lost (the
    /// `PlatformGraphicsContextLostException` of the reference).
    pub fn try_make_current(&self) -> Result<Rc<dyn IDisposable>, OpenGlException> {
        if self.is_lost.get() {
            panic!("{}", PlatformGraphicsContextLostException);
        }
        if self.is_current() {
            return Ok(Disposable::empty());
        }
        Ok(Rc::new(WglRestoreContext::new(self.dc, self.context.get())?))
    }

    /// The device context of a window, with the pixel format of the
    /// context (`CreateConfiguredDeviceContext`). It is released through
    /// the resource manager.
    pub fn create_configured_device_context(&self, hwnd: isize) -> isize {
        let dc = WglGdiResourceManager::get_dc(hwnd);
        set_pixel_format(dc, self.pixel_format, &self.format_descriptor);
        dc
    }

    /// Makes the context current with a device context (`MakeCurrent(IntPtr hdc)`).
    /// Disposing the result restores what was current before.
    pub fn make_current_with(&self, hdc: isize) -> Result<Rc<dyn IDisposable>, OpenGlException> {
        Ok(Rc::new(WglRestoreContext::new(hdc, self.context.get())?))
    }

    fn is_same(a: &WglContext, b: &WglContext) -> bool {
        std::ptr::eq(a, b)
    }
}

impl IOptionalFeatureProvider for WglContext {
    /// The reference has no feature. The context as an OpenGL context is
    /// what the reference tests the object for.
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IGlContext>() {
            let this: Rc<dyn IGlContext> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for WglContext {
    fn is_lost(&self) -> bool {
        self.is_lost.get()
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        IGlContext::make_current(self)
    }

    fn dispose(&self) {
        // The reference deletes the context whenever it is asked to; a
        // second call would release a window that is gone, so the handle
        // is forgotten with the first.
        let context = self.context.replace(0);
        if context == 0 {
            return;
        }
        wgl_delete_context(context);
        WglGdiResourceManager::release_dc(self.hwnd, self.dc);
        WglGdiResourceManager::destroy_window(self.hwnd);
        self.is_lost.set(true);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlContext for WglContext {
    fn version(&self) -> GlVersion {
        self.version
    }

    fn gl_interface(&self) -> Rc<GlInterface> {
        self.gl_interface.get().expect("the interface is set by the constructor").clone()
    }

    fn sample_count(&self) -> i32 {
        0
    }

    fn stencil_size(&self) -> i32 {
        self.stencil_size
    }

    /// # Panics
    /// Panics when the context is lost or cannot be made current (the
    /// exceptions of the reference); [`WglContext::try_make_current`]
    /// returns the second.
    fn make_current(&self) -> Rc<dyn IDisposable> {
        match self.try_make_current() {
            Ok(current) => current,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Panics when `context` is not a context of WGL (the failing cast of
    /// the reference).
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool {
        let Some(c) = context.as_any().downcast_ref::<WglContext>() else {
            panic!("Unable to cast the context to type 'WglContext'.");
        };
        Self::is_same(c, self)
            || c.shared_with.as_ref().is_some_and(|shared| Self::is_same(shared, self))
            || self.shared_with.as_ref().is_some_and(|shared| Self::is_same(shared, c))
            || match (&self.shared_with, &c.shared_with) {
                (Some(a), Some(b)) => Self::is_same(a, b),
                _ => false,
            }
    }

    fn can_create_shared_context(&self) -> bool {
        true
    }

    /// A context of the first of the preferred versions, then of the
    /// version of this context, that shares its objects with this one (or
    /// with the context this one shares with). A failure is logged and
    /// gives `None` (the reference throws), as for the contexts of EGL.
    fn create_shared_context(&self, preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        let mut versions: Vec<GlVersion> = preferred_versions.map(<[GlVersion]>::to_vec).unwrap_or_default();
        versions.push(self.version);
        let share = match &self.shared_with {
            Some(shared_with) => shared_with.clone(),
            None => self.this.upgrade()?,
        };
        match WglDisplay::create_context(&versions, Some(&share)) {
            Ok(context) => context.map(|context| context as Rc<dyn IGlContext>),
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    let source: &dyn Any = &"WglContext";
                    logger.log_with_values(Some(source), "Unable to create a shared context: {0}", &[&error]);
                }
                None
            }
        }
    }
}
