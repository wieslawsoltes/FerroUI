//! EGL through the window system integration of the driver (the port of
//! `WaylandEglWsiPlatformGraphics.cs`).

use super::wayland_framebuffer::log_render_error;
use crate::server::interop::wayland_connection::WaylandConnection;
use crate::server::wayland_platform_graphics::IWaylandGraphics;
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::reactive::IDisposable;
use ferroui_opengl::egl::{EglDisplay, EglDisplayCreationOptions, EglDisplayOptions, EglInterface};
use ferroui_opengl::{GetProcAddress, GlVersion, OpenGlException};
use std::any::Any;
use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;

/// The name the EGL library of the system is loaded by.
const EGL_LIBRARY: &std::ffi::CStr = c"libEGL.so.1";

const EGL_PLATFORM_WAYLAND_KHR: i32 = 0x31D8;

/// "Classic" Wayland EGL backend: uses `EGL_PLATFORM_WAYLAND_KHR` +
/// `wl_egl_window` + `eglSwapBuffers`. The driver is in charge of buffer
/// allocation, format/modifier negotiation, presentation and the implicit
/// `wl_surface.commit`. Required for drivers without `linux-dmabuf`
/// allocation support (NVIDIA proprietary without GBM, llvmpipe via
/// `wl_shm`, etc.).
///
/// An object of the worker thread, like the display it holds.
pub struct WaylandEglWsiPlatformGraphics {
    display: Rc<EglDisplay>,
}

/// Loads the EGL library and resolves its entry points.
///
/// libEGL.so.1 only exports EGL 1.5 core entry points. Extension entry
/// points like eglGetPlatformDisplayEXT are NOT exported as symbols and
/// can only be resolved via eglGetProcAddress; without this, the interface
/// would fail to bind it and EGL_PLATFORM_WAYLAND_KHR display creation
/// wouldn't work. The reference resolves everything through
/// `eglGetProcAddress`; a name it does not know is looked up among the
/// exported symbols here, which covers a library older than EGL 1.5, whose
/// `eglGetProcAddress` answers for extensions only.
fn load_egl_interface() -> Result<EglInterface, OpenGlException> {
    // SAFETY: the name is a terminated string; the library is never closed, so what it
    // exports stays valid.
    let library = unsafe { libc::dlopen(EGL_LIBRARY.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
    if library.is_null() {
        return Err(OpenGlException::new("Unable to load shared library 'libEGL.so.1' or one of its dependencies."));
    }
    // SAFETY: the handle is the one `dlopen` returned; the name is a terminated string.
    let egl_get_proc_address = unsafe { libc::dlsym(library, c"eglGetProcAddress".as_ptr()) };
    if egl_get_proc_address.is_null() {
        return Err(OpenGlException::new("libEGL.so.1 does not export the entry points of EGL"));
    }
    // SAFETY: the symbol is the function of that name of the EGL library, whose C
    // declaration this type is.
    let egl_get_proc_address: unsafe extern "C" fn(*const c_char) -> *const c_void =
        unsafe { std::mem::transmute(egl_get_proc_address) };
    // The handle as a number, so that the loader can be an ordinary closure.
    let library = library as usize;
    let get_proc_address: GetProcAddress = Rc::new(move |name: &str| -> *const c_void {
        let Ok(name) = CString::new(name) else {
            return std::ptr::null();
        };
        // SAFETY: the function takes a terminated string that lives for the call.
        let address = unsafe { egl_get_proc_address(name.as_ptr()) };
        if !address.is_null() {
            return address;
        }
        // SAFETY: the handle is the one `dlopen` returned and the library is still loaded.
        unsafe { libc::dlsym(library as *mut c_void, name.as_ptr()).cast_const() }
    });
    // SAFETY: the loader returns null or the address of the entry point of that name of
    // the EGL library, which stays loaded.
    Ok(unsafe { EglInterface::new(&get_proc_address) })
}

impl WaylandEglWsiPlatformGraphics {
    pub fn display(&self) -> &Rc<EglDisplay> {
        &self.display
    }

    /// The graphics of a connection, when a display of EGL can be created for it; `None`,
    /// with the failure logged, when not.
    pub fn try_create(connection: &WaylandConnection, gl_profiles: &[GlVersion]) -> Option<Rc<Self>> {
        let created = load_egl_interface().and_then(|egl| {
            let options = EglDisplayCreationOptions {
                base: EglDisplayOptions {
                    egl: Some(Rc::new(egl)),
                    supports_multiple_contexts: true,
                    supports_context_sharing: true,
                    gl_versions: Some(gl_profiles.to_vec()),
                    ..Default::default()
                },
                platform_type: Some(EGL_PLATFORM_WAYLAND_KHR),
                platform_display: connection.display_ptr() as isize,
                ..Default::default()
            };
            EglDisplay::new_with_creation_options(options)
        });
        match created {
            Ok(display) => Some(Rc::new(Self { display })),
            Err(error) => {
                log_render_error("Unable to initialize Wayland WSI EGL rendering: {0}", &error);
                None
            }
        }
    }
}

impl IWaylandGraphics for WaylandEglWsiPlatformGraphics {
    /// # Panics
    /// Panics when the context cannot be created (the exception of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.display.create_context(None) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    fn dispose(&self) {
        self.display.dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
