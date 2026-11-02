//! The platform graphics of OpenGL ES over EAGL and the context of EAGL.

use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext, PlatformGraphicsContextLostError,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::ThreadBound;
use ferroui_opengl::gl_consts::{GL_SAMPLES, GL_STENCIL_BITS};
use ferroui_opengl::{GetProcAddress, GlInterface, GlProfileType, GlVersion, IGlContext, OpenGlError};
use objc2::rc::Retained;
use objc2::AllocAnyThread;
use objc2_open_gl_es::{EAGLContext, EAGLRenderingAPI, EAGLSharegroup};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::ffi::{c_void, CStr, CString};
use std::rc::{Rc, Weak};
use std::sync::{Arc, OnceLock};

/// The library of OpenGL ES of the system.
const PATH: &CStr = c"/System/Library/Frameworks/OpenGLES.framework/OpenGLES";

/// The platform graphics of OpenGL ES over EAGL: one context, shared by
/// every surface.
///
/// The graphics are shared by threads (`IPlatformGraphics: Send + Sync`):
/// the platform creates them on the main thread, and the compositor, which
/// renders on the thread of the display link (`DisplayLinkTimer`), asks
/// them for their context there. The reference creates its context in the
/// constructor and uses it on the thread of the display link; the context
/// of the port is an object of one thread (`Rc`, with the interface of
/// OpenGL ES that is one too), so it is created on the thread that first
/// asks for it, which is the thread that renders, and stays bound to that
/// thread (DEVIATIONS.md, iOS platform). Its EAGL context is therefore
/// created, made current and released on the thread that renders only. To
/// fail where the reference fails, the constructor creates a context once
/// and releases it.
pub struct EaglPlatformGraphics {
    /// The handle of the library of OpenGL ES, which stays open for the
    /// life of the process.
    lib_gl: usize,
    /// The one context, created on the thread that first asks for it.
    context: OnceLock<ThreadBound<Rc<GlContext>>>,
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EaglPlatformGraphics>();
};

impl IPlatformGraphics for EaglPlatformGraphics {
    /// The context of the graphics ([`context`](EaglPlatformGraphics::context)).
    ///
    /// # Panics
    /// As [`context`](EaglPlatformGraphics::context).
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.context()
    }

    fn uses_shared_context(&self) -> bool {
        true
    }

    /// # Panics
    /// Always: the graphics have one context, which is shared (the
    /// `NotSupportedException` of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.")
    }
}

impl EaglPlatformGraphics {
    /// The context of the graphics: created at the first call, on the
    /// calling thread, and bound to that thread.
    ///
    /// # Panics
    /// Panics on a thread other than the one of the first call, and when
    /// the context cannot be created.
    pub fn context(&self) -> Rc<GlContext> {
        let context = self.context.get_or_init(|| match self.create_main_context() {
            Ok(context) => ThreadBound::new(context),
            Err(error) => panic!("{error}"),
        });
        context.get().clone()
    }

    /// The version of OpenGL ES of the contexts: 3.0.
    pub const fn gl_version() -> GlVersion {
        GlVersion::new(GlProfileType::OpenGLES, 3, 0)
    }

    fn new() -> Result<EaglPlatformGraphics, OpenGlError> {
        // SAFETY: the path is a terminated string; the mode is the one of
        // the reference (1, `RTLD_LAZY`). The library is never closed.
        let lib_gl = unsafe { libc::dlopen(PATH.as_ptr(), 1) };
        if lib_gl.is_null() {
            return Err(OpenGlError::new(format!("Unable to load {}", PATH.to_string_lossy())));
        }

        let graphics = EaglPlatformGraphics { lib_gl: lib_gl as usize, context: OnceLock::new() };

        // The context of the reference, created here and released: the
        // context of the graphics is created on the thread that renders.
        let context = graphics.create_main_context()?;
        IPlatformGraphicsContext::dispose(&*context);

        Ok(graphics)
    }

    /// The interface of OpenGL ES over the library, and a context with it.
    fn create_main_context(&self) -> Result<Rc<GlContext>, OpenGlError> {
        let lib_gl = self.lib_gl;
        let get_proc_address: GetProcAddress = Rc::new(move |proc: &str| -> *const c_void {
            let Ok(name) = CString::new(proc) else {
                return std::ptr::null();
            };
            // SAFETY: the handle is the open library, and the name is a
            // terminated string; the result is an address or null.
            unsafe { libc::dlsym(lib_gl as *mut c_void, name.as_ptr()).cast_const() }
        });
        // SAFETY: the loader returns null or the address of the entry point
        // of that name in the library of OpenGL ES of the system, whose
        // calling convention is the one of the platform. As in the
        // reference, the interface is created before its context exists:
        // with no context current, a call of OpenGL ES does nothing and
        // answers zero, so the interface knows no extensions and no version
        // strings, as the reference's does [R]. It is used afterwards only
        // by its context, on the thread where that context is current.
        let iface = unsafe { GlInterface::new(Self::gl_version(), get_proc_address) };
        GlContext::new(Rc::new(iface), None)
    }

    /// The graphics, when the library of OpenGL ES can be loaded and a
    /// context of OpenGL ES 3.0 created; `None`, with the failure logged,
    /// when not.
    pub fn try_create() -> Option<Arc<EaglPlatformGraphics>> {
        match EaglPlatformGraphics::new() {
            Ok(graphics) => Some(Arc::new(graphics)),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize EAGL-based rendering: {0}", &[&e]);
                }
                None
            }
        }
    }
}

/// Whether `context` is the current context of the calling thread.
pub(crate) fn is_current(context: &EAGLContext) -> bool {
    // SAFETY: a class method of EAGL that reads the context of the calling
    // thread.
    let current = unsafe { EAGLContext::currentContext() };
    current.is_some_and(|current| std::ptr::eq(&*current, context))
}

/// A context of OpenGL ES 3.0 of EAGL.
///
/// An object of the thread that created it: a context of EAGL may be used
/// by one thread at a time, and the port keeps each to one thread.
pub struct GlContext {
    this: Weak<GlContext>,
    context: RefCell<Option<Retained<EAGLContext>>>,
    gl_interface: Rc<GlInterface>,
}

impl GlContext {
    /// The context of EAGL; none once the context is disposed.
    pub fn context(&self) -> Option<Retained<EAGLContext>> {
        self.context.borrow().clone()
    }

    /// Creates a context of OpenGL ES 3.0 with the interface, in the share
    /// group when one is given.
    ///
    /// # Errors
    /// When EAGL cannot create the context (the failing initializer of the
    /// reference).
    pub fn new(
        gl_interface: Rc<GlInterface>,
        sharegroup: Option<&EAGLSharegroup>,
    ) -> Result<Rc<GlContext>, OpenGlError> {
        let api = EAGLRenderingAPI::OpenGLES3;
        // SAFETY: the initializers of a newly allocated context, with a
        // constant of the API and a share group of EAGL.
        let (context, initializer) = unsafe {
            match sharegroup {
                None => (EAGLContext::initWithAPI(EAGLContext::alloc(), api), "initWithAPI:"),
                Some(sharegroup) => {
                    (EAGLContext::initWithAPI_sharegroup(EAGLContext::alloc(), api, sharegroup), "initWithAPI:sharegroup:")
                }
            }
        };
        let Some(context) = context else {
            return Err(OpenGlError::new(format!(
                "Could not initialize an instance of the type 'OpenGLES.EAGLContext': the native '{initializer}' method returned nil."
            )));
        };

        Ok(Rc::new_cyclic(|this| GlContext {
            this: this.clone(),
            context: RefCell::new(Some(context)),
            gl_interface,
        }))
    }

    /// The context that is current while the result lives: the context of
    /// EAGL of the calling thread, made current again when the result is
    /// disposed.
    ///
    /// # Panics
    /// Panics with the message of a lost graphics context once the context
    /// is disposed, and when EAGL does not make it current.
    fn make_current_context(&self) -> Rc<dyn IDisposable> {
        let Some(context) = self.context() else {
            panic!("{}", PlatformGraphicsContextLostError);
        };
        // SAFETY: class methods of EAGL that read and set the context of
        // the calling thread. A context is current on one thread at a
        // time: the context of the platform graphics is bound to the
        // thread that renders, and any other context of the port to the
        // thread that created it.
        let old = unsafe { EAGLContext::currentContext() };
        // SAFETY: as above.
        if !unsafe { EAGLContext::setCurrentContext(Some(&context)) } {
            panic!("{}", OpenGlError::new("Unable to make context current"));
        }
        Rc::new(ResetContext::new(old))
    }
}

/// Makes the context that was current before current again.
struct ResetContext {
    old: RefCell<Option<Retained<EAGLContext>>>,
    disposed: Cell<bool>,
}

impl ResetContext {
    fn new(old: Option<Retained<EAGLContext>>) -> Self {
        Self { old: RefCell::new(old), disposed: Cell::new(false) }
    }
}

impl IDisposable for ResetContext {
    fn dispose(&self) {
        if self.disposed.get() {
            return;
        }
        self.disposed.set(true);
        let old = self.old.borrow().clone();
        // SAFETY: a class method of EAGL that sets the context of the
        // calling thread, to the context that was current on it.
        unsafe { EAGLContext::setCurrentContext(old.as_deref()) };
        *self.old.borrow_mut() = None;
    }
}

impl IOptionalFeatureProvider for GlContext {
    /// The reference's context has no optional features. The context
    /// announces itself as an OpenGL context here, which is how a render
    /// backend of the port finds the OpenGL contract of a graphics context
    /// (`ferroui-opengl`).
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IGlContext>() {
            let this: Rc<dyn IGlContext> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for GlContext {
    fn is_lost(&self) -> bool {
        self.context.borrow().is_none()
    }

    /// # Panics
    /// Panics with the message of a lost graphics context once the context
    /// is disposed, and when EAGL does not make it current.
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        let Some(context) = self.context() else {
            panic!("{}", PlatformGraphicsContextLostError);
        };
        if is_current(&context) {
            return Disposable::empty();
        }
        self.make_current_context()
    }

    fn dispose(&self) {
        let context = self.context.borrow_mut().take();
        drop(context);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlContext for GlContext {
    fn version(&self) -> GlVersion {
        EaglPlatformGraphics::gl_version()
    }

    fn gl_interface(&self) -> Rc<GlInterface> {
        self.gl_interface.clone()
    }

    fn sample_count(&self) -> i32 {
        self.gl_interface.get_integerv(GL_SAMPLES)
    }

    fn stencil_size(&self) -> i32 {
        self.gl_interface.get_integerv(GL_STENCIL_BITS)
    }

    /// # Panics
    /// Panics with the message of a lost graphics context once the context
    /// is disposed, and when EAGL does not make it current.
    fn make_current(&self) -> Rc<dyn IDisposable> {
        self.make_current_context()
    }

    /// Whether the other context is a context of EAGL in the same share
    /// group.
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool {
        let Some(other) = context.as_any().downcast_ref::<GlContext>() else {
            return false;
        };
        let sharegroup = |context: &GlContext| {
            // SAFETY: the share group of a context of EAGL, which every
            // context has.
            context.context().map(|context| Retained::as_ptr(&unsafe { context.sharegroup() }))
        };
        sharegroup(other) == sharegroup(self)
    }

    fn can_create_shared_context(&self) -> bool {
        true
    }

    /// A context in the share group of this one; the preferred versions are
    /// not used, as in the reference.
    ///
    /// # Panics
    /// Panics with the message of a lost graphics context once the context
    /// is disposed, and when EAGL cannot create the context (the exceptions
    /// of the reference).
    fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        let Some(context) = self.context() else {
            panic!("{}", PlatformGraphicsContextLostError);
        };
        // SAFETY: the share group of a context of EAGL, which every context
        // has.
        let sharegroup = unsafe { context.sharegroup() };
        match GlContext::new(self.gl_interface.clone(), Some(&sharegroup)) {
            Ok(context) => Some(context),
            Err(error) => panic!("{error}"),
        }
    }
}
