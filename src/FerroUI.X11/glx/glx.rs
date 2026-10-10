//! The entry points of GLX (the port of `Glx/Glx.cs`): `libGL` is opened at
//! run time, as the other libraries of the backend, and every call is a
//! safe function here.
//!
//! The reference resolves the entry points through `glXGetProcAddress`
//! into an object per context; they are the same functions of one library
//! whoever asks, so the interface here is a handle to the one table.

use crate::xlib::{self, VisualInfo, XDisplay, XID};
use ferroui_opengl::OpenGlException;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::OnceLock;

/// A frame buffer configuration of GLX (`GLXFBConfig`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlxFbConfig(*mut c_void);

// SAFETY: a frame buffer configuration is a pointer into data the GLX
// library keeps for a connection for as long as the connection lives (the
// connections of the platform are never closed); it is never dereferenced
// here, only passed back to the library, which locks the connection.
unsafe impl Send for GlxFbConfig {}
// SAFETY: see `Send`.
unsafe impl Sync for GlxFbConfig {}

impl GlxFbConfig {
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

/// A context of GLX (`GLXContext`), as a number: null is no context.
pub type GlxContextHandle = isize;

type CreateContextAttribsArb =
    unsafe extern "C" fn(*mut x11_dl::xlib::Display, *mut c_void, *mut c_void, c_int, *const c_int) -> *mut c_void;

struct Library {
    glx: x11_dl::glx::Glx,
    create_context_attribs_arb: Option<CreateContextAttribsArb>,
}

fn library() -> Result<&'static Library, &'static str> {
    static LIBRARY: OnceLock<Result<Library, String>> = OnceLock::new();
    LIBRARY
        .get_or_init(|| {
            let glx = x11_dl::glx::Glx::open().map_err(|e| format!("Unable to load libGL: {e}"))?;
            // SAFETY: the name is a terminated string; the library returns null or the address of
            // the named entry point, whose signature is the one of the GLX_ARB_create_context
            // specification.
            let create_context_attribs_arb = unsafe {
                (glx.glXGetProcAddress)(c"glXCreateContextAttribsARB".as_ptr().cast())
                    .map(|address| std::mem::transmute::<unsafe extern "C" fn(), CreateContextAttribsArb>(address))
            };
            Ok(Library { glx, create_context_attribs_arb })
        })
        .as_ref()
        .map_err(String::as_str)
}

fn raw(display: XDisplay) -> *mut x11_dl::xlib::Display {
    display.as_ptr().cast()
}

/// The entry points of GLX.
#[derive(Clone, Copy)]
pub struct GlxInterface {
    library: &'static Library,
    create_context_attribs_arb: CreateContextAttribsArb,
}

impl GlxInterface {
    /// Loads the library and its entry points. Fails when `libGL` cannot be
    /// loaded or has no `glXCreateContextAttribsARB`, as the constructor of
    /// the reference does for an entry point it cannot resolve.
    pub fn new() -> Result<GlxInterface, OpenGlException> {
        let library = library().map_err(OpenGlException::new)?;
        let Some(create_context_attribs_arb) = library.create_context_attribs_arb else {
            return Err(OpenGlException::new("Unable to resolve the entry point glXCreateContextAttribsARB"));
        };
        Ok(GlxInterface { library, create_context_attribs_arb })
    }

    fn glx(&self) -> &'static x11_dl::glx::Glx {
        &self.library.glx
    }

    /// `glXMakeContextCurrent`.
    pub fn make_context_current(&self, display: XDisplay, draw: XID, read: XID, context: GlxContextHandle) -> bool {
        // SAFETY: plain call: the connection is open, the drawables are identifiers and the
        // context is null or one this interface created and has not destroyed; the library
        // reports what it does not accept as an error of the server or by returning false.
        unsafe { (self.glx().glXMakeContextCurrent)(raw(display), draw, read, context as *mut _) != 0 }
    }

    /// `glXGetCurrentContext`.
    pub fn get_current_context(&self) -> GlxContextHandle {
        // SAFETY: plain call without arguments.
        unsafe { (self.glx().glXGetCurrentContext)() as isize }
    }

    /// `glXGetCurrentDisplay`: the connection of the current context, when
    /// there is one.
    pub fn get_current_display(&self) -> Option<XDisplay> {
        // SAFETY: plain call without arguments; a non-null result is the connection the
        // current context was made current with, which is one of the platform's.
        unsafe { XDisplay::from_ptr((self.glx().glXGetCurrentDisplay)().cast()) }
    }

    /// `glXGetCurrentDrawable`.
    pub fn get_current_drawable(&self) -> XID {
        // SAFETY: plain call without arguments.
        unsafe { (self.glx().glXGetCurrentDrawable)() }
    }

    /// `glXGetCurrentReadDrawable`.
    pub fn get_current_read_drawable(&self) -> XID {
        // SAFETY: plain call without arguments.
        unsafe { (self.glx().glXGetCurrentReadDrawable)() }
    }

    /// `glXCreatePbuffer`. `attrib_list` ends with a zero.
    pub fn create_pbuffer(&self, display: XDisplay, config: GlxFbConfig, attrib_list: &[i32]) -> XID {
        assert_eq!(attrib_list.last(), Some(&0), "the attribute list is terminated");
        // SAFETY: the list is terminated (checked above) and lives for the call; the
        // configuration is one the library gave for this connection.
        unsafe { (self.glx().glXCreatePbuffer)(raw(display), config.0.cast(), attrib_list.as_ptr()) }
    }

    /// `glXDestroyPbuffer`.
    pub fn destroy_pbuffer(&self, display: XDisplay, pbuffer: XID) {
        // SAFETY: plain call; an identifier that is not a pixel buffer is an error of the server.
        unsafe { (self.glx().glXDestroyPbuffer)(raw(display), pbuffer) }
    }

    /// `glXCreateContextAttribsARB`: null when the context cannot be
    /// created. `attribs` ends with a zero.
    pub fn create_context_attribs_arb(
        &self,
        display: XDisplay,
        config: GlxFbConfig,
        share_list: GlxContextHandle,
        direct: bool,
        attribs: &[i32],
    ) -> GlxContextHandle {
        assert_eq!(attribs.last(), Some(&0), "the attribute list is terminated");
        // SAFETY: the list is terminated (checked above) and lives for the call; the
        // configuration is one the library gave for this connection; the share context is
        // null or a context of this interface.
        unsafe {
            (self.create_context_attribs_arb)(raw(display), config.0, share_list as *mut _, c_int::from(direct), attribs.as_ptr())
                as isize
        }
    }

    /// `glXGetProcAddress`: null for a name with a NUL in it.
    pub fn glx_get_proc_address(name: &str) -> *const c_void {
        let (Ok(library), Ok(name)) = (library(), CString::new(name)) else {
            return std::ptr::null();
        };
        // SAFETY: the name is a terminated string that lives for the call.
        unsafe {
            (library.glx.glXGetProcAddress)(name.as_ptr().cast()).map_or(std::ptr::null(), |address| address as *const c_void)
        }
    }

    /// `glXDestroyContext`.
    pub fn destroy_context(&self, display: XDisplay, context: GlxContextHandle) {
        // SAFETY: the context is one this interface created; the caller destroys it once.
        unsafe { (self.glx().glXDestroyContext)(raw(display), context as *mut _) }
    }

    /// `glXChooseFBConfig`: the configurations that match the attributes,
    /// in the order of the library. `attrib_list` ends with a zero.
    pub fn choose_fb_config(&self, display: XDisplay, screen: i32, attrib_list: &[i32]) -> Vec<GlxFbConfig> {
        assert_eq!(attrib_list.last(), Some(&0), "the attribute list is terminated");
        let mut count = 0;
        // SAFETY: the list is terminated and lives for the call; the result is null or an array
        // of `count` configurations that the caller frees with `XFree`, which is done here
        // after the configurations were copied (the configurations themselves stay valid).
        unsafe {
            let configs = (self.glx().glXChooseFBConfig)(raw(display), screen, attrib_list.as_ptr(), &mut count);
            if configs.is_null() {
                return Vec::new();
            }
            let result = std::slice::from_raw_parts(configs, count.max(0) as usize).iter().map(|c| GlxFbConfig(c.cast())).collect();
            (xlib::libraries().xlib.XFree)(configs.cast());
            result
        }
    }

    /// `ChooseFbConfig`: [`choose_fb_config`](Self::choose_fb_config) with
    /// the terminator added.
    pub fn choose_fb_config_terminated(&self, display: XDisplay, screen: i32, attribs: &[i32]) -> Vec<GlxFbConfig> {
        let mut arr = attribs.to_vec();
        arr.push(0);
        self.choose_fb_config(display, screen, &arr)
    }

    /// `glXGetVisualFromFBConfig`: the visual of a configuration, when it
    /// has one.
    pub fn get_visual_from_fb_config(&self, display: XDisplay, config: GlxFbConfig) -> Option<VisualInfo> {
        // SAFETY: the configuration is one the library gave for this connection; the result is
        // null or one structure the caller frees with `XFree`, which is done here after it was
        // copied (the visual it names belongs to the connection).
        unsafe {
            let info = (self.glx().glXGetVisualFromFBConfig)(raw(display), config.0.cast());
            if info.is_null() {
                return None;
            }
            let result = VisualInfo { visual: (*info).visual, visual_id: (*info).visualid, depth: (*info).depth };
            (xlib::libraries().xlib.XFree)(info.cast());
            Some(result)
        }
    }

    /// `glXGetFBConfigAttrib`: the value of the attribute, `None` when the
    /// call fails (the reference compares its result with zero, success).
    pub fn get_fb_config_attrib(&self, display: XDisplay, config: GlxFbConfig, attribute: i32) -> Option<i32> {
        let mut value = 0;
        // SAFETY: the out pointer is a valid place for the value.
        let status = unsafe { (self.glx().glXGetFBConfigAttrib)(raw(display), config.0.cast(), attribute, &mut value) };
        (status == 0).then_some(value)
    }

    /// `glXSwapBuffers`.
    pub fn swap_buffers(&self, display: XDisplay, drawable: XID) {
        // SAFETY: plain call; a drawable the library does not know is an error of the server.
        unsafe { (self.glx().glXSwapBuffers)(raw(display), drawable) }
    }

    /// `glXWaitX`.
    pub fn wait_x(&self) {
        // SAFETY: plain call without arguments.
        unsafe { (self.glx().glXWaitX)() }
    }

    /// `glXWaitGL`.
    pub fn wait_gl(&self) {
        // SAFETY: plain call without arguments.
        unsafe { (self.glx().glXWaitGL)() }
    }

    /// `glXQueryExtensionsString`.
    pub fn query_extensions_string(&self, display: XDisplay, screen: i32) -> Option<String> {
        // SAFETY: plain call; the result is null or a terminated string of the library that
        // stays valid while the connection is open.
        unsafe {
            let extensions: *const c_char = (self.glx().glXQueryExtensionsString)(raw(display), screen);
            (!extensions.is_null()).then(|| CStr::from_ptr(extensions).to_string_lossy().into_owned())
        }
    }

    // Ignores egl functions.
    // On some Linux systems, glXGetProcAddress will return valid pointers for even EGL functions.
    // This makes Skia try to load some data from EGL,
    // which can then cause segmentation faults because they return garbage.
    pub fn safe_get_proc_address(proc: &str) -> *const c_void {
        if proc.starts_with("egl") {
            return std::ptr::null();
        }

        Self::glx_get_proc_address(proc)
    }

    /// The extensions of the display (`GetExtensions`), which the
    /// reference asks of screen 0.
    pub fn get_extensions(&self, display: XDisplay) -> Vec<String> {
        split_extensions(self.query_extensions_string(display, 0).as_deref())
    }
}

/// Splits an extension string at commas and spaces.
pub(crate) fn split_extensions(extensions: Option<&str>) -> Vec<String> {
    let Some(s) = extensions.filter(|s| !s.is_empty()) else {
        return Vec::new();
    };

    s.split([',', ' ']).filter(|x| !x.is_empty()).map(|x| x.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn an_extension_string_is_split_at_spaces_and_commas() {
        assert_eq!(split_extensions(None), Vec::<String>::new());
        assert_eq!(split_extensions(Some("")), Vec::<String>::new());
        assert_eq!(
            split_extensions(Some("GLX_ARB_create_context GLX_EXT_create_context_es2_profile,GLX_SGI_swap_control  ")),
            vec!["GLX_ARB_create_context", "GLX_EXT_create_context_es2_profile", "GLX_SGI_swap_control"]
        );
    }

    #[test]
    fn entry_points_of_egl_are_never_resolved() {
        // Whatever the library says (and whether or not there is one on this system).
        assert!(GlxInterface::safe_get_proc_address("eglGetCurrentDisplay").is_null());
        assert!(GlxInterface::safe_get_proc_address("eglQueryString").is_null());
    }
}
