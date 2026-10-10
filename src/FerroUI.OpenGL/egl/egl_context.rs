use super::egl_consts::*;
use super::egl_external_objects_feature::EglExternalObjectsFeature;
use super::{EglContextFeature, EglContextFeatureFactory, EglContextOptions, EglDisplay, EglInterface, EglSurface};
use crate::{GetProcAddress, GlInterface, GlVersion, IGlContext, IGlContextExternalObjectsFeature, OpenGlException};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext, PlatformGraphicsContextLostException};
use ferroui_base::reactive::{Disposable, IDisposable};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{c_void, CString};
use std::rc::{Rc, Weak};

/// A context of an EGL display.
// The original guards the context with a monitor (its own, or the one of the display);
// see the note of `EglDisplay`: there is nothing to guard here, `ensure_locked` returns a
// handle that does nothing.
pub struct EglContext {
    this: Weak<EglContext>,
    disp: Rc<EglDisplay>,
    egl: Rc<EglInterface>,
    shared_with: Option<Rc<EglContext>>,
    api: i32,
    is_lost: Cell<bool>,
    context: Cell<isize>,
    dispose_callback: Option<Rc<dyn Fn()>>,
    features: RefCell<HashMap<TypeId, EglContextFeature>>,
    offscreen_surface: Option<Rc<EglSurface>>,
    version: GlVersion,
    /// Set by the constructor, once the context is current for the first time.
    gl_interface: OnceCell<Rc<GlInterface>>,
    sample_count: i32,
    stencil_size: i32,
}

impl EglContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        display: &Rc<EglDisplay>,
        egl: Rc<EglInterface>,
        shared_with: Option<Rc<EglContext>>,
        ctx: isize,
        offscreen_surface: Option<Rc<EglSurface>>,
        version: GlVersion,
        sample_count: i32,
        stencil_size: i32,
        dispose_callback: Option<Rc<dyn Fn()>>,
        features: HashMap<TypeId, EglContextFeatureFactory>,
    ) -> Result<Rc<EglContext>, OpenGlException> {
        let this = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            disp: display.clone(),
            egl: egl.clone(),
            shared_with,
            api: display.api(),
            is_lost: Cell::new(false),
            context: Cell::new(ctx),
            dispose_callback,
            features: RefCell::new(HashMap::new()),
            offscreen_surface,
            version,
            gl_interface: OnceCell::new(),
            sample_count,
            stencil_size,
        });

        let current = match this.make_current_with_surface(None) {
            Ok(current) => current,
            Err(MakeCurrentError::ContextLost) => {
                return Err(OpenGlException::new(PlatformGraphicsContextLostException.to_string()));
            }
            Err(MakeCurrentError::OpenGl(error)) => return Err(error),
        };

        let get_proc_address: GetProcAddress = Rc::new(move |name: &str| -> *const c_void {
            let Ok(name) = CString::new(name) else {
                return std::ptr::null();
            };
            // SAFETY: the name is a terminated string that lives for the call, which is all
            // `eglGetProcAddress` asks for.
            unsafe { egl.get_proc_address(name.as_ptr()) }
        });
        // SAFETY: `eglGetProcAddress` returns null or the address of the named entry point of
        // the client API of the current context, and the context was made current above.
        let gl_interface = unsafe { GlInterface::new(version, get_proc_address) };
        let _ = this.gl_interface.set(Rc::new(gl_interface));

        let created: Vec<(TypeId, EglContextFeature)> =
            features.iter().map(|(key, factory)| (*key, factory(&this))).collect();
        this.features.borrow_mut().extend(created);

        let external_objects_key = TypeId::of::<dyn IGlContextExternalObjectsFeature>();
        let has_external_objects = this.features.borrow().contains_key(&external_objects_key);
        if !has_external_objects {
            if let Some(external_objects) = EglExternalObjectsFeature::try_create(&this) {
                let external_objects: Rc<dyn IGlContextExternalObjectsFeature> = external_objects;
                // Deviation (DEVIATIONS.md, OpenGL): upstream's feature is not disposable and
                // stays among the features of a disposed context. Here it is listed as
                // disposable, with nothing to dispose, so that a disposed context lets go of
                // it: the feature holds the feature of the OpenGL extensions, which holds
                // the context.
                this.features.borrow_mut().insert(
                    external_objects_key,
                    EglContextFeature { feature: Rc::new(external_objects), disposable: Some(Disposable::empty()) },
                );
            }
        }

        current.dispose();
        Ok(this)
    }

    /// The handle of the context.
    ///
    /// # Panics
    /// Panics when the context is disposed.
    pub fn context(&self) -> isize {
        if self.context.get() == 0 {
            panic!("Cannot access a disposed object. Object name: 'EglContext'.");
        }
        self.context.get()
    }

    /// The shared handle of the context: what a platform surface outside this crate makes
    /// its render target with, after it recovered the context from `IGlContext::as_any` (the
    /// cast `(EglContext)context` of the original).
    ///
    /// # Panics
    /// Panics when the context is being dropped.
    pub fn this(&self) -> Rc<EglContext> {
        self.this.upgrade().expect("the context is alive")
    }

    /// Whether the context is disposed: reading its handle then fails.
    pub(crate) fn is_disposed(&self) -> bool {
        self.context.get() == 0
    }

    pub fn offscreen_surface(&self) -> Option<&Rc<EglSurface>> {
        self.offscreen_surface.as_ref()
    }

    pub fn display(&self) -> &Rc<EglDisplay> {
        &self.disp
    }

    pub fn egl_interface(&self) -> &Rc<EglInterface> {
        &self.egl
    }

    /// Makes the context current with `surface`, or with its offscreen surface when none is
    /// given. Disposing the result restores what was current before.
    pub fn make_current_with_surface(
        &self,
        surface: Option<&Rc<EglSurface>>,
    ) -> Result<Rc<dyn IDisposable>, MakeCurrentError> {
        if IPlatformGraphicsContext::is_lost(self) {
            return Err(MakeCurrentError::ContextLost);
        }

        let old = RestoreContext::new(&self.egl, self.disp.handle());
        let surf = surface.or(self.offscreen_surface.as_ref());
        let surface_handle = surf.map_or(0, |surf| surf.dangerous_get_handle());
        self.egl.bind_api(self.api);
        self.egl.make_current(self.disp.handle(), 0, 0, 0);
        if !self.egl.make_current(self.disp.handle(), surface_handle, surface_handle, self.context()) {
            let error = self.egl.get_error();
            if error == EGL_CONTEXT_LOST {
                self.notify_context_lost();
                return Err(MakeCurrentError::ContextLost);
            }

            return Err(MakeCurrentError::OpenGl(OpenGlException::get_formatted_egl_exception("eglMakeCurrent", error)));
        }

        Ok(Rc::new(old))
    }

    pub fn notify_context_lost(&self) {
        self.is_lost.set(true);
        self.disp.on_context_lost(self);
    }

    /// Enters the lock of the context; see the note of the type.
    pub fn ensure_locked(&self) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }

    pub fn is_current(&self) -> bool {
        self.context.get() != 0
            && self.egl.get_current_display() == self.disp.handle()
            && self.egl.get_current_context() == self.context.get()
            && self.egl.query_api() == self.api
    }

    fn is_same(a: &EglContext, b: &EglContext) -> bool {
        std::ptr::eq(a, b)
    }
}

/// Why a context could not be made current.
#[derive(Clone, Debug)]
pub enum MakeCurrentError {
    /// The context is lost (the `PlatformGraphicsContextLostException` of the original).
    ContextLost,
    /// `eglMakeCurrent` failed.
    OpenGl(OpenGlException),
}

impl std::fmt::Display for MakeCurrentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContextLost => write!(f, "{}", PlatformGraphicsContextLostException),
            Self::OpenGl(error) => write!(f, "{error}"),
        }
    }
}

impl Error for MakeCurrentError {}

/// What was current before a context was made current, restored when it is disposed.
struct RestoreContext {
    egl: Rc<EglInterface>,
    display: isize,
    context: isize,
    read: isize,
    draw: isize,
    api: i32,
}

impl RestoreContext {
    fn new(egl: &Rc<EglInterface>, def_display: isize) -> Self {
        let api = egl.query_api();
        let display = egl.get_current_display();
        if display == 0 {
            // Fallback display, it's invalid to call eglMakeCurrent without a display
            Self { egl: egl.clone(), display: def_display, context: 0, read: 0, draw: 0, api }
        } else {
            Self {
                egl: egl.clone(),
                display,
                context: egl.get_current_context(),
                read: egl.get_current_surface(EGL_READ),
                draw: egl.get_current_surface(EGL_DRAW),
                api,
            }
        }
    }
}

impl IDisposable for RestoreContext {
    fn dispose(&self) {
        self.egl.bind_api(self.api);
        self.egl.make_current(self.display, self.draw, self.read, self.context);
    }
}

impl IOptionalFeatureProvider for EglContext {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if let Some(feature) = self.features.borrow().get(&feature_type) {
            return Some(feature.feature.clone());
        }
        // The context as an OpenGL context: what the original tests the object for.
        if feature_type == TypeId::of::<dyn IGlContext>() {
            let this: Rc<dyn IGlContext> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for EglContext {
    // As in the original, asking a disposed context whose display is not lost reads its
    // handle, which fails.
    fn is_lost(&self) -> bool {
        self.is_lost.get() || self.disp.is_lost() || self.context() == 0
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        if self.is_current() {
            return Disposable::empty();
        }
        IGlContext::make_current(self)
    }

    fn dispose(&self) {
        if self.context.get() == 0 {
            return;
        }

        let disposable: Vec<(TypeId, Rc<dyn IDisposable>)> = self
            .features
            .borrow()
            .iter()
            .filter_map(|(key, feature)| feature.disposable.clone().map(|disposable| (*key, disposable)))
            .collect();
        for (key, d) in disposable {
            d.dispose();
            self.features.borrow_mut().remove(&key);
        }

        self.egl.destroy_context(self.disp.handle(), self.context());
        if let Some(offscreen_surface) = &self.offscreen_surface {
            offscreen_surface.dispose();
        }
        self.context.set(0);
        self.disp.on_context_disposed(self);
        if let Some(dispose_callback) = &self.dispose_callback {
            dispose_callback();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlContext for EglContext {
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
    /// Panics when the context is lost or cannot be made current (the exceptions of the
    /// original); [`EglContext::make_current_with_surface`] returns them.
    fn make_current(&self) -> Rc<dyn IDisposable> {
        match self.make_current_with_surface(None) {
            Ok(current) => current,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Panics when `context` is not an EGL context (the failing cast of the original).
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool {
        let Some(c) = context.as_any().downcast_ref::<EglContext>() else {
            panic!("Unable to cast the context to type 'EglContext'.");
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
        self.disp.supports_sharing()
    }

    /// Creates a context that shares its objects with this one. As in the original the
    /// preferred versions are not used: the context has the version of the display. A
    /// failure is logged and gives `None` (the original throws).
    fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        let share_with = match &self.shared_with {
            Some(shared_with) => shared_with.clone(),
            None => self.this.upgrade()?,
        };
        match self.disp.create_context(Some(EglContextOptions { share_with: Some(share_with), ..Default::default() })) {
            Ok(context) => Some(context),
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    let source: &dyn Any = &"EglContext";
                    logger.log_with_values(Some(source), "Unable to create a shared context: {0}", &[&error]);
                }
                None
            }
        }
    }
}
