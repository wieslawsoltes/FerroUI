//! A context of GLX (the port of `Glx/GlxContext.cs`).

use super::glx::{GlxContextHandle, GlxInterface};
use super::glx_display::GlxDisplay;
use crate::xlib::{XDisplay, XID};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_opengl::features::ExternalObjectsOpenGlExtensionFeature;
use ferroui_opengl::{
    GetProcAddress, GlInterface, GlVersion, IGlContext, IGlContextExternalObjectsFeature, OpenGlException,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::ffi::c_void;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// A context of GLX.
// The reference guards the context with a monitor that `MakeCurrent`
// enters and the returned object leaves. A context here is an object of
// the thread that created it (`Rc`), so there is nothing to guard, as for
// the contexts of EGL.
pub struct GlxContext {
    this: Weak<GlxContext>,
    handle: Cell<GlxContextHandle>,
    glx: GlxInterface,
    shared_with: Option<Rc<GlxContext>>,
    deferred_display: XDisplay,
    default_xid: XID,
    owns_p_buffer: bool,
    external_objects: RefCell<Option<Rc<dyn IGlContextExternalObjectsFeature>>>,
    display: Arc<GlxDisplay>,
    version: GlVersion,
    /// Set by the constructor, once the context is current for the first time.
    gl_interface: OnceCell<Rc<GlInterface>>,
    sample_count: i32,
    stencil_size: i32,
}

impl GlxContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        glx: GlxInterface,
        handle: GlxContextHandle,
        display: &Arc<GlxDisplay>,
        shared_with: Option<Rc<GlxContext>>,
        version: GlVersion,
        sample_count: i32,
        stencil_size: i32,
        default_xid: XID,
        owns_p_buffer: bool,
    ) -> Result<Rc<GlxContext>, OpenGlException> {
        let this = Rc::new_cyclic(|this| GlxContext {
            this: this.clone(),
            handle: Cell::new(handle),
            glx,
            shared_with,
            deferred_display: display.deferred_display(),
            default_xid,
            owns_p_buffer,
            external_objects: RefCell::new(None),
            display: display.clone(),
            version,
            gl_interface: OnceCell::new(),
            sample_count,
            stencil_size,
        });

        let current = this.make_current_with(default_xid)?;
        let get_proc_address: GetProcAddress =
            Rc::new(|name: &str| -> *const c_void { GlxInterface::safe_get_proc_address(name) });
        // SAFETY: `glXGetProcAddress` returns null or the address of the named entry point of
        // the OpenGL library, and the context was made current above.
        let gl_interface = unsafe { GlInterface::new(version, get_proc_address) };
        let _ = this.gl_interface.set(Rc::new(gl_interface));
        let as_gl_context: Rc<dyn IGlContext> = this.clone();
        if let Some(external_objects) = ExternalObjectsOpenGlExtensionFeature::try_create(&as_gl_context) {
            *this.external_objects.borrow_mut() = Some(external_objects);
        }
        current.dispose();

        Ok(this)
    }

    /// The handle of the context (`Handle`).
    pub fn handle(&self) -> GlxContextHandle {
        self.handle.get()
    }

    pub fn glx(&self) -> &GlxInterface {
        &self.glx
    }

    pub fn display(&self) -> &Arc<GlxDisplay> {
        &self.display
    }

    /// The shared handle of the context.
    ///
    /// # Panics
    /// Panics when the context is being dropped.
    pub(crate) fn this(&self) -> Rc<GlxContext> {
        self.this.upgrade().expect("the context is alive")
    }

    /// Makes the context current with a drawable (`MakeCurrent(IntPtr xid)`).
    /// Disposing the result restores what was current before.
    pub fn make_current_with(&self, xid: XID) -> Result<Rc<dyn IDisposable>, OpenGlException> {
        let old = RestoreContext::new(self.glx, self.deferred_display);
        if !self.glx.make_context_current(self.deferred_display, xid, xid, self.handle.get()) {
            return Err(OpenGlException::new("glXMakeContextCurrent failed "));
        }

        Ok(Rc::new(old))
    }

    pub fn is_current(&self) -> bool {
        self.glx.get_current_context() == self.handle.get()
    }

    fn is_same(a: &GlxContext, b: &GlxContext) -> bool {
        std::ptr::eq(a, b)
    }
}

/// What was current before a context was made current, restored when it is
/// disposed.
struct RestoreContext {
    glx: GlxInterface,
    default_display: XDisplay,
    display: Option<XDisplay>,
    context: GlxContextHandle,
    read: XID,
    draw: XID,
}

impl RestoreContext {
    fn new(glx: GlxInterface, default_display: XDisplay) -> Self {
        Self {
            glx,
            default_display,
            display: glx.get_current_display(),
            context: glx.get_current_context(),
            read: glx.get_current_read_drawable(),
            draw: glx.get_current_drawable(),
        }
    }
}

impl IDisposable for RestoreContext {
    fn dispose(&self) {
        let disp = self.display.unwrap_or(self.default_display);
        self.glx.make_context_current(disp, self.draw, self.read, self.context);
    }
}

impl IOptionalFeatureProvider for GlxContext {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IGlContextExternalObjectsFeature>() {
            let external_objects = self.external_objects.borrow().clone()?;
            return Some(Rc::new(external_objects));
        }
        // The context as an OpenGL context: what the reference tests the object for.
        if feature_type == TypeId::of::<dyn IGlContext>() {
            let this: Rc<dyn IGlContext> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for GlxContext {
    fn is_lost(&self) -> bool {
        false
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        if self.is_current() {
            return Disposable::empty();
        }
        IGlContext::make_current(self)
    }

    fn dispose(&self) {
        // The reference destroys the context whenever it is asked to; a second call would
        // destroy a context that is gone, so the handle is forgotten with the first.
        let handle = self.handle.replace(0);
        if handle == 0 {
            return;
        }
        // The feature holds the context: let go of it, or the two keep each other alive.
        let external_objects = self.external_objects.borrow_mut().take();
        drop(external_objects);

        self.glx.destroy_context(self.deferred_display, handle);
        if self.owns_p_buffer {
            self.glx.destroy_pbuffer(self.deferred_display, self.default_xid);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlContext for GlxContext {
    fn version(&self) -> GlVersion {
        self.version
    }

    fn gl_interface(&self) -> Rc<GlInterface> {
        self.gl_interface.get().expect("the interface is set by the constructor").clone()
    }

    fn sample_count(&self) -> i32 {
        self.sample_count
    }

    fn stencil_size(&self) -> i32 {
        self.stencil_size
    }

    /// # Panics
    /// Panics when the context cannot be made current (the exception of the
    /// reference); [`GlxContext::make_current_with`] returns it.
    fn make_current(&self) -> Rc<dyn IDisposable> {
        match self.make_current_with(self.default_xid) {
            Ok(current) => current,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Panics when `context` is not a context of GLX (the failing cast of
    /// the reference).
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool {
        let Some(c) = context.as_any().downcast_ref::<GlxContext>() else {
            panic!("Unable to cast the context to type 'GlxContext'.");
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

    /// Creates a context that shares its objects with this one. As in the
    /// reference the preferred versions are not used. A failure is logged
    /// and gives `None` (the reference throws), as for the contexts of EGL.
    fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        let share = match &self.shared_with {
            Some(shared_with) => shared_with.clone(),
            None => self.this.upgrade()?,
        };
        match self.display.create_context_shared_with(&share) {
            Ok(context) => Some(context),
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    let source: &dyn Any = &"GlxContext";
                    logger.log_with_values(Some(source), "Unable to create a shared context: {0}", &[&error]);
                }
                None
            }
        }
    }
}
