//! The EGL of the system, and the platform graphics over it.
//!
//! The reference calls `EglPlatformGraphics.TryCreate()` of the OpenGL
//! project, whose display loads `libEGL.so` by name. The EGL of the port
//! takes a loader function, and its platform graphics belong to the thread
//! that created them; on Android the compositor renders on the render
//! thread. So this file has what the reference gets from the OpenGL
//! project: the loader of the library of the system, and platform graphics
//! that hold nothing of a thread. A context loads the interface and creates
//! its display object on the thread that asks for it; the display of the
//! system is one per process and counted, so every display object is the
//! same display (docs/porting/android-platform.md, section 6).

use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::reactive::IDisposable;
use ferroui_opengl::egl::{
    EglContext, EglContextOptions, EglDisplay, EglDisplayCreationOptions, EglDisplayOptions, EglInterface,
};
use ferroui_opengl::{GetProcAddress, OpenGlException};
use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;
use std::sync::OnceLock;

type EglGetProcAddress = unsafe extern "C" fn(name: *const c_char) -> *const c_void;

/// The EGL library of the system and its `eglGetProcAddress`. The library
/// is opened once and stays open.
#[derive(Clone, Copy)]
struct EglLibrary {
    library: usize,
    get_proc_address: EglGetProcAddress,
}

fn egl_library() -> Result<EglLibrary, OpenGlException> {
    static LIBRARY: OnceLock<Option<EglLibrary>> = OnceLock::new();
    LIBRARY
        .get_or_init(|| {
            // SAFETY: the names are terminated strings; the library is a library of the
            // system, opened for the life of the process.
            unsafe {
                let library = libc::dlopen(c"libEGL.so".as_ptr(), libc::RTLD_NOW);
                if library.is_null() {
                    return None;
                }
                let get_proc_address = libc::dlsym(library, c"eglGetProcAddress".as_ptr());
                if get_proc_address.is_null() {
                    return None;
                }
                Some(EglLibrary {
                    library: library as usize,
                    // An address that is not null is the function of that name, whose
                    // signature is the declared one.
                    get_proc_address: std::mem::transmute::<*mut c_void, EglGetProcAddress>(get_proc_address),
                })
            }
        })
        .ok_or_else(|| OpenGlException::new("Unable to load libEGL.so"))
}

/// The loader of the entry points of EGL and of OpenGL ES: an entry point
/// the library exports, or else what `eglGetProcAddress` answers (which on
/// Android knows the functions of OpenGL ES, core and extensions).
fn load_egl() -> Result<GetProcAddress, OpenGlException> {
    let library = egl_library()?;
    Ok(Rc::new(move |name: &str| {
        let Ok(name) = CString::new(name) else {
            return std::ptr::null();
        };
        // SAFETY: the library handle is the open library, the name is terminated, and
        // `eglGetProcAddress` takes a terminated name and returns an address or null.
        unsafe {
            let address = libc::dlsym(library.library as *mut c_void, name.as_ptr());
            if !address.is_null() {
                return address.cast_const();
            }
            (library.get_proc_address)(name.as_ptr())
        }
    }))
}

/// The platform graphics of the EGL of the system.
pub(crate) struct AndroidEglPlatformGraphics;

impl AndroidEglPlatformGraphics {
    /// The display of the system, as an object of the calling thread.
    fn create_display() -> Result<Rc<EglDisplay>, OpenGlException> {
        let get_proc_address = load_egl()?;
        // SAFETY: the loader returns, for a name, null or the address of the entry point of
        // that name in the EGL library of the system or behind its `eglGetProcAddress`;
        // the library is never closed.
        let egl = Rc::new(unsafe { EglInterface::new(&get_proc_address) });
        EglDisplay::new_with_creation_options(EglDisplayCreationOptions {
            base: EglDisplayOptions {
                egl: Some(egl),
                supports_multiple_contexts: true,
                supports_context_sharing: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    /// A context of a display, which the context disposes with itself.
    fn create_context_for_display(display: Rc<EglDisplay>) -> Result<Rc<EglContext>, OpenGlException> {
        let dispose_display = display.clone();
        let result = display.create_context(Some(EglContextOptions {
            dispose_callback: Some(Rc::new(move || dispose_display.dispose())),
            ..Default::default()
        }));
        if result.is_err() {
            display.dispose();
        }
        result
    }

    /// The platform graphics, when the display of the system can be
    /// initialized and a context of it made current; `None`, with the
    /// failure logged, when not.
    pub fn try_create() -> Option<AndroidEglPlatformGraphics> {
        let probe = || -> Result<(), OpenGlException> {
            let context = Self::create_context_for_display(Self::create_display()?)?;
            let result = match context.make_current_with_surface(None) {
                Ok(current) => {
                    current.dispose();
                    Ok(())
                }
                Err(error) => Err(OpenGlException::new(error.to_string())),
            };
            IPlatformGraphicsContext::dispose(&*context);
            result
        };

        match probe() {
            Ok(()) => Some(AndroidEglPlatformGraphics),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize EGL-based rendering: {0}", &[&e]);
                }
                None
            }
        }
    }
}

impl IPlatformGraphics for AndroidEglPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    /// # Panics
    /// Panics when the display or the context cannot be created (the
    /// exception of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match Self::create_display().and_then(Self::create_context_for_display) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    /// # Panics
    /// Always: the graphics have no shared context (the
    /// `NotSupportedException` of the reference).
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.")
    }
}
