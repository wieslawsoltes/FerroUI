//! EGL on a connection to an X server (the port of `X11EglHelper.cs`, and
//! of the EGL part of `X11Platform.InitializeGraphics`): the configuration
//! that works with a window of the server, its visual, and the platform
//! graphics.

use crate::x11_structs::{CreateWindowArgs, SetWindowValuemask};
use crate::x11_info::X11Info;
use crate::xlib::{self, VisualInfo, XDisplay, XID};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::{IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::utilities::ThreadBound;
use ferroui_opengl::egl::egl_consts::{EGL_EXTENSIONS, EGL_NATIVE_VISUAL_ID, EGL_NONE, EGL_PLATFORM_X11_EXT};
use ferroui_opengl::egl::{EglDisplay, EglDisplayCreationOptions, EglDisplayOptions, EglInterface};
use ferroui_opengl::{GetProcAddress, GlVersion, OpenGlException};
use std::cell::RefCell;
use std::ffi::{c_void, CString};
use std::rc::Rc;

/// The name the EGL library of the system is loaded by.
const EGL_LIBRARY: &std::ffi::CStr = c"libEGL.so.1";

/// What the helper needs of a connection: the connection that renders and
/// the root window (`X11Info.DeferredDisplay`, `X11Info.RootWindow`). The
/// reference passes the whole `X11Info`, which is an object of the UI
/// thread here; the two values are what both threads may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X11EglTarget {
    pub deferred_display: XDisplay,
    pub root_window: XID,
}

impl X11EglTarget {
    pub fn new(x11: &X11Info) -> Self {
        Self { deferred_display: x11.deferred_display(), root_window: x11.root_window() }
    }
}

/// Loads the EGL library of the system and resolves its entry points by
/// name (the constructor `EglInterface()` of the reference, which the
/// OpenGL crate leaves to the platform).
pub fn load_egl_interface() -> Result<EglInterface, OpenGlException> {
    // SAFETY: the name is a terminated string; the library is never closed, so what it
    // exports stays valid.
    let library = unsafe { libc::dlopen(EGL_LIBRARY.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
    if library.is_null() {
        return Err(OpenGlException::new("Unable to load shared library 'libEGL.so.1' or one of its dependencies."));
    }
    // The handle as a number, so that the loader can be an ordinary closure.
    let library = library as usize;
    let get_proc_address: GetProcAddress = Rc::new(move |name: &str| -> *const c_void {
        let Ok(name) = CString::new(name) else {
            return std::ptr::null();
        };
        // SAFETY: the handle is the one `dlopen` returned and the library is still loaded;
        // the name is a terminated string that lives for the call.
        unsafe { libc::dlsym(library as *mut c_void, name.as_ptr()).cast_const() }
    });
    if get_proc_address("eglGetDisplay").is_null() {
        return Err(OpenGlException::new("libEGL.so.1 does not export the entry points of EGL"));
    }
    // SAFETY: the loader returns null or the address of the export of that name of the EGL
    // library, which stays loaded.
    Ok(unsafe { EglInterface::new(&get_proc_address) })
}

/// Resolves the visual that matches the EGL config's native visual id.
/// nvidia's driver requires the X11 window visual to match the config used to create the surface,
/// otherwise eglCreateWindowSurface fails. Returns `None` when the config doesn't advertise a visual id
/// (e.g. mesa, where any visual works).
pub fn get_visual_info(x11: X11EglTarget, egl: &EglInterface, display: isize, config: isize) -> Option<VisualInfo> {
    let mut visual_id = 0;
    if !egl.get_config_attrib(display, config, EGL_NATIVE_VISUAL_ID, &mut visual_id) || visual_id == 0 {
        return None;
    }
    xlib::x_get_visual_info_by_id(x11.deferred_display, visual_id as u32 as _)
}

/// The visual of the configuration of a display
/// (`GetVisualInfo(X11Info, EglDisplay)`).
pub fn get_visual_info_of_display(x11: X11EglTarget, display: &EglDisplay) -> Option<VisualInfo> {
    get_visual_info(x11, display.egl_interface(), display.handle(), display.config())
}

/// The order in which the candidates are probed: those with a visual of
/// depth 32 first, each group in the order it came in (a stable sort, as
/// `OrderByDescending` of the reference).
pub(crate) fn probe_order(depths: &[i32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..depths.len()).collect();
    order.sort_by_key(|index| depths[*index] != 32);
    order
}

/// Picks the EGL config to use out of the ones matched by eglChooseConfig. Each candidate is verified by
/// creating a throwaway window with the config's native visual and attempting to create an EGL window
/// surface on it: nvidia exposes multiple identical-looking configs where only a subset actually works,
/// so the broken ones are discarded here. A 32-bit (transparent-capable) X11 visual is preferred,
/// mirroring the GLX backend which selects a 32-bit visual; mesa lists those configs after the opaque
/// ones. We resolve the (cheap) native visuals up-front and probe in preference order, so the expensive
/// window-surface creation is attempted as few times as possible.
pub fn choose_config(x11: X11EglTarget, egl: &EglInterface, display: isize, configs: &[isize]) -> Option<isize> {
    let mut candidates: Vec<(isize, VisualInfo)> = Vec::with_capacity(configs.len());
    for config in configs {
        if let Some(visual) = get_visual_info(x11, egl, display, *config) {
            candidates.push((*config, visual));
        } else if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, "OpenGL") {
            logger.log_with_values(None, "EGL config {Config} has no native visual id", &[config]);
        }
    }

    let depths: Vec<i32> = candidates.iter().map(|(_, visual)| visual.depth).collect();
    for index in probe_order(&depths) {
        let (config, visual) = candidates[index];
        if probe_config(x11, egl, display, config, &visual) {
            return Some(config);
        }
    }

    None
}

fn probe_config(x11: X11EglTarget, egl: &EglInterface, display: isize, config: isize, vi: &VisualInfo) -> bool {
    let colormap = xlib::x_create_colormap(x11.deferred_display, x11.root_window, vi, 0);
    let mut attr = xlib::new_set_window_attributes();
    attr.colormap = colormap;
    attr.border_pixel = 0;

    let window = xlib::x_create_window(
        x11.deferred_display,
        x11.root_window,
        0,
        0,
        1,
        1,
        0,
        vi.depth,
        CreateWindowArgs::InputOutput.0,
        vi.visual,
        (SetWindowValuemask::COLOR_MAP | SetWindowValuemask::BORDER_PIXEL).bits() as u32 as _,
        &mut attr,
    );

    if window == 0 {
        xlib::x_free_colormap(x11.deferred_display, colormap);
        return false;
    }

    xlib::x_flush(x11.deferred_display);

    let surface = egl.create_window_surface(display, config, window as isize, Some(&[EGL_NONE, EGL_NONE]));
    let success = surface != 0;
    if success {
        egl.destroy_surface(display, surface);
    }

    xlib::x_destroy_window(x11.deferred_display, window);
    xlib::x_free_colormap(x11.deferred_display, colormap);

    if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, "OpenGL") {
        logger.log_with_values(
            None,
            "EGL config {Config}: visualId={VisualId} depth={Depth} usable={Usable}",
            &[&config, &vi.visual_id, &vi.depth, &success],
        );
    }

    success
}

/// Whether the client extensions of the EGL library have the X11 platform
/// (the test of `InitializeGraphics`).
pub(crate) fn has_x11_platform_extension(client_extensions: Option<&str>) -> bool {
    client_extensions.is_some_and(|client_extensions| {
        client_extensions.contains("EGL_KHR_platform_x11") || client_extensions.contains("EGL_EXT_platform_x11")
    })
}

/// Creates the display of EGL for a connection (the display factory of
/// `InitializeGraphics`).
pub fn create_display(x11: X11EglTarget, gl_profiles: &[GlVersion]) -> Result<Rc<EglDisplay>, OpenGlException> {
    let egl = Rc::new(load_egl_interface()?);
    let mut options = EglDisplayCreationOptions {
        base: EglDisplayOptions {
            supports_context_sharing: true,
            supports_multiple_contexts: true,
            gl_versions: Some(gl_profiles.to_vec()),
            egl: Some(egl.clone()),
            // nvidia exposes multiple indistinguishable configs of which only some work,
            // so we probe candidates by creating a throwaway window surface and pick a usable one.
            probe_config: Some(Rc::new(move |probe_egl: &EglInterface, display: isize, configs: &[isize]| {
                choose_config(x11, probe_egl, display, configs)
            })),
            ..Default::default()
        },
        ..Default::default()
    };

    // nvidia requires the display to be created through the X11 platform extension,
    // otherwise EGL_NATIVE_VISUAL_ID doesn't match the actual window visual.
    let client_extensions = egl.query_string(0, EGL_EXTENSIONS);
    if egl.is_get_platform_display_ext_available() && has_x11_platform_extension(client_extensions.as_deref()) {
        options.platform_type = Some(EGL_PLATFORM_X11_EXT);
        options.platform_display = x11.deferred_display.as_ptr() as isize;
    }

    EglDisplay::new_with_creation_options(options)
}

thread_local! {
    /// The display of the thread that renders, when that is not the thread
    /// the platform graphics were created on.
    static THREAD_DISPLAY: RefCell<Option<Rc<EglDisplay>>> = const { RefCell::new(None) };
}

/// The platform graphics of EGL on a connection to an X server.
///
/// The reference registers the `EglPlatformGraphics` of the OpenGL project
/// with its one display. A display here is an object of one thread, and
/// the platform graphics are shared with the thread that renders, which
/// creates the contexts. So the graphics keep the display they were probed
/// with for the thread that created them, and give any other thread a
/// display of its own over the same connection: EGL has one display per
/// native connection, so both are the same display of the library, with
/// the same configuration. What the window needs of the configuration, its
/// visual, is read once and kept as a value.
pub struct X11EglPlatformGraphics {
    x11: X11EglTarget,
    gl_profiles: Vec<GlVersion>,
    visual: Option<VisualInfo>,
    display: ThreadBound<Rc<EglDisplay>>,
}

// SAFETY: the display is kept to its thread by `ThreadBound`; the rest are
// values and one pointer, the visual, which belongs to the connection (never
// closed) and is only handed to Xlib, which reads its identifier.
unsafe impl Send for X11EglPlatformGraphics {}
// SAFETY: see `Send`.
unsafe impl Sync for X11EglPlatformGraphics {}

impl X11EglPlatformGraphics {
    /// The platform graphics, when the display can be created; `None`,
    /// with the failure logged, when not (`EglPlatformGraphics.TryCreate`).
    pub fn try_create(x11: &X11Info, gl_profiles: &[GlVersion]) -> Option<X11EglPlatformGraphics> {
        let target = X11EglTarget::new(x11);
        match create_display(target, gl_profiles) {
            Ok(display) => Some(X11EglPlatformGraphics {
                x11: target,
                gl_profiles: gl_profiles.to_vec(),
                visual: get_visual_info_of_display(target, &display),
                display: ThreadBound::new(display),
            }),
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize EGL-based rendering: {0}", &[&e]);
                }
                None
            }
        }
    }

    /// The visual the windows are created with
    /// (`X11EglHelper.GetVisualInfo(_x11, egl.Display)`): `None` when the
    /// configuration names none.
    pub fn visual_info(&self) -> Option<VisualInfo> {
        self.visual
    }

    /// The display of the calling thread.
    fn display(&self) -> Result<Rc<EglDisplay>, OpenGlException> {
        if self.display.is_on_thread() {
            return Ok(self.display.get().clone());
        }
        THREAD_DISPLAY.with(|slot| {
            if let Some(display) = slot.borrow().as_ref() {
                return Ok(display.clone());
            }
            let display = create_display(self.x11, &self.gl_profiles)?;
            *slot.borrow_mut() = Some(display.clone());
            Ok(display)
        })
    }
}

impl IPlatformGraphics for X11EglPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    /// # Panics
    /// Panics when the display of the thread or the context cannot be
    /// created (the exception of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.display().and_then(|display| display.create_context(None)) {
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

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn configurations_with_a_visual_of_depth_32_are_probed_first_in_their_order() {
        assert_eq!(probe_order(&[]), Vec::<usize>::new());
        assert_eq!(probe_order(&[24, 24, 32, 24, 32]), [2, 4, 0, 1, 3]);
        assert_eq!(probe_order(&[32, 24]), [0, 1]);
        assert_eq!(probe_order(&[24, 30]), [0, 1]);
    }

    #[test]
    fn the_platform_of_the_display_needs_one_of_the_two_extensions() {
        assert!(!has_x11_platform_extension(None));
        assert!(!has_x11_platform_extension(Some("EGL_EXT_client_extensions EGL_KHR_platform_wayland")));
        assert!(has_x11_platform_extension(Some("EGL_EXT_platform_base EGL_KHR_platform_x11 EGL_MESA_platform_gbm")));
        assert!(has_x11_platform_extension(Some("EGL_EXT_platform_x11")));
    }
}
