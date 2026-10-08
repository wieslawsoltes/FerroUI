use super::egl_consts::*;
use super::egl_display_utils::{required_egl, EglConfigInfo, EglDisplayUtils};
use super::{EglContext, EglContextOptions, EglDisplayCreationOptions, EglDisplayOptions, EglInterface, EglSurface};
use crate::OpenGlException;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::reactive::{Disposable, IDisposable};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A display of EGL: the configuration that was chosen for it and the contexts and surfaces
/// that are created on it.
// The original guards the display with a monitor, which a deriving class can share with
// the contexts of the display (`DisplayLockIsSharedWithContexts`, `ContextSharedSyncRoot`).
// The objects of this crate belong to one thread, so there is nothing to guard: `lock`
// returns a handle that does nothing and the two members are not ported.
pub struct EglDisplay {
    this: Weak<EglDisplay>,
    egl: Rc<EglInterface>,
    display: Cell<isize>,
    options: EglDisplayOptions,
    config: EglConfigInfo,
    is_lost: Cell<bool>,
    supports_sharing: bool,
    contexts: RefCell<Vec<Rc<EglContext>>>,
}

impl EglDisplay {
    /// Creates the display the options describe (`new EglDisplay(EglDisplayCreationOptions)`).
    ///
    /// The constructor of the original without arguments, which loads the EGL library of
    /// the system by name, is not ported; see [`EglInterface`].
    pub fn new_with_creation_options(options: EglDisplayCreationOptions) -> Result<Rc<EglDisplay>, OpenGlException> {
        let display = EglDisplayUtils::create_display(&options)?;
        Self::new(display, options.base)
    }

    /// Wraps a display of EGL and chooses its configuration.
    ///
    /// # Panics
    /// Panics when `display` is zero.
    pub fn new(display: isize, options: EglDisplayOptions) -> Result<Rc<EglDisplay>, OpenGlException> {
        let egl = required_egl(&options.egl);
        let supports_sharing = options.supports_context_sharing;
        if display == 0 {
            panic!("Value does not fall within the expected range.");
        }

        let config = EglDisplayUtils::initialize_and_get_config(
            &egl,
            display,
            options.gl_versions.as_deref(),
            options.probe_config.as_ref(),
            options.allow_pbuffer_only_configs,
        )?;

        Ok(Rc::new_cyclic(|this| Self {
            this: this.clone(),
            egl,
            display: Cell::new(display),
            options,
            config,
            is_lost: Cell::new(false),
            supports_sharing,
            contexts: RefCell::new(Vec::new()),
        }))
    }

    fn this(&self) -> Rc<EglDisplay> {
        self.this.upgrade().expect("a display that is called is alive")
    }

    pub fn supports_sharing(&self) -> bool {
        self.supports_sharing
    }

    pub fn handle(&self) -> isize {
        self.display.get()
    }

    pub fn config(&self) -> isize {
        self.config.config()
    }

    pub(crate) fn single_context(&self) -> bool {
        !self.options.supports_multiple_contexts
    }

    pub fn egl_interface(&self) -> &Rc<EglInterface> {
        &self.egl
    }

    pub(crate) fn api(&self) -> i32 {
        self.config.api()
    }

    /// Creates a context of the display.
    ///
    /// # Panics
    /// Panics when the options name a context to share with and the display does not
    /// support sharing (the `NotSupportedException` of the original).
    pub fn create_context(&self, options: Option<EglContextOptions>) -> Result<Rc<EglContext>, OpenGlException> {
        if self.single_context() && !self.contexts.borrow().is_empty() {
            return Err(OpenGlException::new("This EGLDisplay can only have one active context"));
        }

        let options = options.unwrap_or_default();
        let share = options.share_with;
        if share.is_some() && !self.supports_sharing {
            panic!("Context sharing is not supported by this display");
        }

        let mut offscreen_surface = options.offscreen_surface;

        if offscreen_surface.is_none() {
            // Check if eglMakeCurrent can work with EGL_NONE as read-write surfaces
            let extensions = self.egl.query_string(self.handle(), EGL_EXTENSIONS);
            if !extensions.is_some_and(|extensions| extensions.contains("EGL_KHR_surfaceless_context")) {
                // Attempt to create a PBuffer as a surface for offscreen rendering
                // (As in the original the test combines the bits with `|`, so it never fails.)
                if (self.config.surface_type() | EGL_PBUFFER_BIT) == 0 {
                    return Err(OpenGlException::new(
                        "Platform doesn't support EGL_KHR_surfaceless_context and PBUFFER surfaces",
                    ));
                }

                let p_buffer_surface = self.egl.create_pbuffer_surface(
                    self.display.get(),
                    self.config(),
                    Some(&[EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE]),
                );
                if p_buffer_surface == 0 {
                    return Err(OpenGlException::get_formatted_exception_for_egl("eglCreatePBufferSurface", &self.egl));
                }

                offscreen_surface = Some(EglSurface::new(&self.this(), p_buffer_surface));
            }
        }

        let previous_api = self.egl.query_api();
        self.egl.bind_api(self.config.api());
        let ctx = self.egl.create_context(
            self.display.get(),
            self.config(),
            share.as_ref().map_or(0, |share| share.context()),
            self.config.attributes(),
        );
        if previous_api != EGL_NONE {
            self.egl.bind_api(previous_api);
        }
        if ctx == 0 {
            let ex = OpenGlException::get_formatted_exception_for_egl("eglCreateContext", &self.egl);
            if let Some(offscreen_surface) = &offscreen_surface {
                offscreen_surface.dispose();
            }
            return Err(ex);
        }

        let rv = EglContext::new(
            &self.this(),
            self.egl.clone(),
            share,
            ctx,
            offscreen_surface,
            self.config.version(),
            self.config.sample_count(),
            self.config.stencil_size(),
            options.dispose_callback,
            options.extra_features.unwrap_or_default(),
        )?;
        self.contexts.borrow_mut().push(rv.clone());
        Ok(rv)
    }

    pub fn create_window_surface(&self, window: isize) -> Result<Rc<EglSurface>, OpenGlException> {
        if window == 0 {
            return Err(OpenGlException::new(format!("Window {window} is invalid.")));
        }

        let lock = self.lock();
        let s = self.egl.create_window_surface(self.handle(), self.config(), window, Some(&[EGL_NONE, EGL_NONE]));
        let result = if s == 0 {
            Err(OpenGlException::get_formatted_exception_for_egl("eglCreateWindowSurface", &self.egl))
        } else {
            Ok(EglSurface::new(&self.this(), s))
        };
        lock.dispose();
        result
    }

    /// Creates a surface from a buffer of another API. `attribs` is terminated with
    /// `EGL_NONE`. (The two overloads of the original, with an array and with a pointer,
    /// are this one function.)
    pub fn create_pbuffer_from_client_buffer(
        &self,
        buffer_type: i32,
        handle: isize,
        attribs: &[i32],
    ) -> Result<Rc<EglSurface>, OpenGlException> {
        let lock = self.lock();
        let s = self.egl.create_pbuffer_from_client_buffer(self.handle(), buffer_type, handle, self.config(), Some(attribs));

        let result = if s == 0 {
            Err(OpenGlException::get_formatted_exception_for_egl("eglCreatePbufferFromClientBuffer", &self.egl))
        } else {
            Ok(EglSurface::new(&self.this(), s))
        };
        lock.dispose();
        result
    }

    pub(crate) fn on_context_lost(&self, _context: &EglContext) {
        if self.options.context_loss_is_display_loss {
            self.is_lost.set(true);
        }
    }

    pub(crate) fn on_context_disposed(&self, context: &EglContext) {
        self.contexts.borrow_mut().retain(|other| !std::ptr::eq(&**other, context));
    }

    pub fn is_lost(&self) -> bool {
        if self.is_lost.get() || self.display.get() == 0 {
            return true;
        }
        if self.options.device_lost_check_callback.as_ref().is_some_and(|callback| callback()) {
            self.is_lost.set(true);
            return true;
        }
        false
    }

    /// Enters the lock of the display; see the note of the type.
    pub fn lock(&self) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

impl IDisposable for EglDisplay {
    fn dispose(&self) {
        let contexts = self.contexts.borrow().clone();
        for ctx in contexts {
            ctx.dispose();
        }
        self.contexts.borrow_mut().clear();
        if self.display.get() != 0 {
            self.egl.terminate(self.display.get());
        }
        self.display.set(0);
        if let Some(dispose_callback) = &self.options.dispose_callback {
            dispose_callback();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of the display.
    use super::*;
    use crate::GetProcAddress;
    use std::ffi::c_void;

    thread_local! {
        static TERMINATED: Cell<i32> = const { Cell::new(0) };
        static DESTROYED_SURFACES: Cell<i32> = const { Cell::new(0) };
    }

    unsafe extern "system" fn get_error() -> i32 {
        EGL_BAD_ALLOC
    }

    unsafe extern "system" fn initialize(_display: isize, major: *mut i32, minor: *mut i32) -> u32 {
        // SAFETY: the interface passes valid pointers.
        unsafe {
            *major = 1;
            *minor = 5;
        }
        1
    }

    unsafe extern "system" fn bind_api(_api: i32) -> u32 {
        1
    }

    unsafe extern "system" fn choose_config(
        _display: isize,
        _attribs: *const i32,
        configs: *mut isize,
        config_size: i32,
        num_configs: *mut i32,
    ) -> u32 {
        // SAFETY: the interface passes a valid count pointer and a buffer of `config_size`
        // configurations or null.
        unsafe {
            *num_configs = 2;
            if !configs.is_null() && config_size >= 2 {
                *configs = 11;
                *configs.add(1) = 22;
            }
        }
        1
    }

    unsafe extern "system" fn get_config_attrib(_display: isize, _config: isize, attr: i32, rv: *mut i32) -> u32 {
        // SAFETY: the interface passes a valid pointer.
        unsafe {
            *rv = if attr == EGL_SAMPLES { 4 } else { 8 };
        }
        1
    }

    unsafe extern "system" fn terminate(_display: isize) {
        TERMINATED.with(|count| count.set(count.get() + 1));
    }

    unsafe extern "system" fn create_window_surface(
        _display: isize,
        _config: isize,
        window: isize,
        _attrs: *const i32,
    ) -> isize {
        // The window 13 stands for one a surface cannot be created for.
        if window == 13 {
            0
        } else {
            77
        }
    }

    unsafe extern "system" fn destroy_surface(_display: isize, _surface: isize) {
        DESTROYED_SURFACES.with(|count| count.set(count.get() + 1));
    }

    unsafe extern "system" fn other() {}

    fn egl() -> Rc<EglInterface> {
        let loader: GetProcAddress = Rc::new(|name: &str| match name {
            "eglGetError" => get_error as *const c_void,
            "eglInitialize" => initialize as *const c_void,
            "eglBindAPI" => bind_api as *const c_void,
            "eglChooseConfig" => choose_config as *const c_void,
            "eglGetConfigAttrib" => get_config_attrib as *const c_void,
            "eglTerminate" => terminate as *const c_void,
            "eglCreateWindowSurface" => create_window_surface as *const c_void,
            "eglDestroySurface" => destroy_surface as *const c_void,
            name if name.ends_with("EXT") || name.ends_with("KHR") => std::ptr::null(),
            _ => other as *const c_void,
        });
        // SAFETY: the loader returns functions of the declared signatures for the entry
        // points the tests call.
        Rc::new(unsafe { EglInterface::new(&loader) })
    }

    fn options() -> EglDisplayOptions {
        EglDisplayOptions { egl: Some(egl()), ..Default::default() }
    }

    #[test]
    fn the_first_matching_configuration_is_chosen_without_a_probe() {
        let display = EglDisplay::new(5, options()).expect("a display");

        assert_eq!(5, display.handle());
        assert_eq!(11, display.config());
        assert_eq!(EGL_OPENGL_ES_API, display.api());
        assert!(display.single_context());
        assert!(!display.supports_sharing());
    }

    #[test]
    fn the_probe_chooses_among_the_matching_configurations() {
        let probe: crate::egl::EglConfigProbeCallback = Rc::new(|_: &EglInterface, _: isize, configs: &[isize]| configs.last().copied());
        let display = EglDisplay::new(5, EglDisplayOptions { probe_config: Some(probe), ..options() }).expect("a display");

        assert_eq!(22, display.config());
    }

    #[test]
    fn a_display_is_lost_once_its_check_says_so_or_it_is_disposed() {
        let lost = Rc::new(Cell::new(false));
        let disposed = Rc::new(Cell::new(0));
        let (l, d) = (lost.clone(), disposed.clone());
        let display = EglDisplay::new(
            5,
            EglDisplayOptions {
                device_lost_check_callback: Some(Rc::new(move || l.get())),
                dispose_callback: Some(Rc::new(move || d.set(d.get() + 1))),
                ..options()
            },
        )
        .expect("a display");
        assert!(!display.is_lost());

        lost.set(true);
        assert!(display.is_lost());
        lost.set(false);
        assert!(display.is_lost());

        let terminated = TERMINATED.with(Cell::get);
        display.dispose();
        assert_eq!(terminated + 1, TERMINATED.with(Cell::get));
        assert_eq!(0, display.handle());
        assert_eq!(1, disposed.get());
    }

    #[test]
    fn window_surfaces_are_created_and_destroyed_once() {
        let display = EglDisplay::new(5, options()).expect("a display");

        assert_eq!("Window 0 is invalid.", display.create_window_surface(0).err().expect("an error").message());
        let failure = display.create_window_surface(13).err().expect("an error");
        assert_eq!(Some(EGL_BAD_ALLOC), failure.error_code());

        let surface = display.create_window_surface(3).expect("a surface");
        assert_eq!(77, surface.dangerous_get_handle());
        assert!(!surface.is_invalid());

        let destroyed = DESTROYED_SURFACES.with(Cell::get);
        surface.dispose();
        surface.dispose();
        drop(surface);
        assert_eq!(destroyed + 1, DESTROYED_SURFACES.with(Cell::get));
    }
}
