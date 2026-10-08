use crate::entry_points::{gl_entry_points, GetProcAddress};
use std::ffi::{c_char, c_void, CStr};
use std::ptr;

/// The entry points of EGL the framework itself calls.
///
/// The interface is built from a loader of entry points
/// ([`EglInterface::new`]). The constructors of the original that load the
/// EGL library by name (`EglInterface()`, `EglInterface(string library)`) are
/// not ported: a platform that has EGL resolves `eglGetProcAddress` of its
/// library and passes the loader.
pub struct EglInterface {
    egl: EglEntryPoints,
}

gl_entry_points! {
    table EglEntryPoints for EglInterface.egl(get, info: ());

    required safe pub fn get_error() -> i32 = get("eglGetError");

    required safe pub fn get_display(native_display: isize) -> isize = get("eglGetDisplay");

    optional unsafe pub fn get_platform_display_ext_native(platform: i32, native_display: isize, attrs: *const i32) -> isize
        = get("eglGetPlatformDisplayEXT");

    required unsafe pub fn initialize_native(display: isize, major: *mut i32, minor: *mut i32) -> u32
        = get("eglInitialize");

    required safe pub fn terminate(display: isize) = get("eglTerminate");

    required unsafe pub fn get_proc_address(proc: *const c_char) -> *const c_void = get("eglGetProcAddress");

    required safe pub(crate) fn bind_api_native(api: i32) -> u32 = get("eglBindAPI");

    required safe pub fn query_api() -> i32 = get("eglQueryAPI");

    required unsafe pub fn choose_config_native(
        display: isize,
        attribs: *const i32,
        configs: *mut isize,
        config_size: i32,
        num_configs: *mut i32
    ) -> u32 = get("eglChooseConfig");

    required unsafe pub fn create_context_native(display: isize, config: isize, share: isize, attrs: *const i32) -> isize
        = get("eglCreateContext");

    required safe pub(crate) fn destroy_context_native(display: isize, context: isize) -> u32 = get("eglDestroyContext");

    required unsafe pub fn create_pbuffer_surface_native(display: isize, config: isize, attrs: *const i32) -> isize
        = get("eglCreatePbufferSurface");

    required safe pub(crate) fn make_current_native(display: isize, draw: isize, read: isize, context: isize) -> u32
        = get("eglMakeCurrent");

    required safe pub fn get_current_context() -> isize = get("eglGetCurrentContext");

    required safe pub fn get_current_display() -> isize = get("eglGetCurrentDisplay");

    required safe pub fn get_current_surface(read_draw: i32) -> isize = get("eglGetCurrentSurface");

    required safe pub fn destroy_surface(display: isize, surface: isize) = get("eglDestroySurface");

    required safe pub fn swap_buffers(display: isize, surface: isize) = get("eglSwapBuffers");

    required safe pub(crate) fn swap_interval_native(display: isize, interval: i32) -> u32 = get("eglSwapInterval");

    required unsafe pub fn create_window_surface_native(display: isize, config: isize, window: isize, attrs: *const i32) -> isize
        = get("eglCreateWindowSurface");

    required safe pub fn bind_tex_image(display: isize, surface: isize, buffer: i32) -> i32 = get("eglBindTexImage");

    required unsafe pub fn get_config_attrib_native(display: isize, config: isize, attr: i32, rv: *mut i32) -> u32
        = get("eglGetConfigAttrib");

    required safe pub(crate) fn wait_gl_native() -> u32 = get("eglWaitGL");

    required safe pub(crate) fn wait_client_native() -> u32 = get("eglWaitClient");

    required safe pub(crate) fn wait_native_native(engine: i32) -> u32 = get("eglWaitNative");

    required safe pub fn query_string_native(display: isize, i: i32) -> *const c_char = get("eglQueryString");

    required unsafe pub fn create_pbuffer_from_client_buffer_ptr(
        display: isize,
        buftype: i32,
        buffer: isize,
        config: isize,
        attrib_list: *const i32
    ) -> isize = get("eglCreatePbufferFromClientBuffer");

    optional unsafe pub fn query_display_attrib_ext_native(display: isize, attr: i32, res: *mut isize) -> u32
        = get("eglQueryDisplayAttribEXT");

    optional unsafe pub fn query_device_attrib_ext_native(display: isize, attr: i32, res: *mut isize) -> u32
        = get("eglQueryDeviceAttribEXT");

    // EGL_KHR_image_base
    optional unsafe pub fn create_image_khr_native(
        display: isize,
        context: isize,
        target: i32,
        client_buffer: isize,
        attribs: *const i32
    ) -> isize = get("eglCreateImageKHR");

    optional safe pub(crate) fn destroy_image_khr_native(display: isize, image: isize) -> u32 = get("eglDestroyImageKHR");

    // EGL_EXT_image_dma_buf_import_modifiers
    optional unsafe pub fn query_dma_buf_formats_ext_native(
        display: isize,
        max_formats: i32,
        formats: *mut i32,
        num_formats: *mut i32
    ) -> u32 = get("eglQueryDmaBufFormatsEXT");

    // The flags behind `external_only` are EGL booleans (four bytes each), which is what the
    // entry point writes; the original declares the pointer as a pointer to one-byte booleans.
    optional unsafe pub fn query_dma_buf_modifiers_ext_native(
        display: isize,
        format: i32,
        max_modifiers: i32,
        modifiers: *mut u64,
        external_only: *mut u32,
        num_modifiers: *mut i32
    ) -> u32 = get("eglQueryDmaBufModifiersEXT");
}

/// The pointer of an optional attribute list: null for no list.
fn attrs_ptr(attrs: Option<&[i32]>) -> *const i32 {
    attrs.map_or(ptr::null(), <[i32]>::as_ptr)
}

impl EglInterface {
    /// Resolves the entry points with `get_proc_address`.
    ///
    /// # Safety
    /// For a name, the loader must return either null or the address of the
    /// EGL entry point of that name, and the entry points must stay valid
    /// for as long as the interface is used. An attribute list passed to an
    /// entry point must be terminated with `EGL_NONE`, as EGL requires.
    ///
    /// # Panics
    /// Panics when a required entry point cannot be resolved.
    pub unsafe fn new(get_proc_address: &GetProcAddress) -> Self {
        Self { egl: EglEntryPoints::load(get_proc_address, &()) }
    }

    /// `eglGetPlatformDisplayEXT`.
    pub fn get_platform_display_ext(&self, platform: i32, native_display: isize, attrs: Option<&[i32]>) -> isize {
        // SAFETY: the attribute list is null or a terminated list (see `new`).
        unsafe { self.get_platform_display_ext_native(platform, native_display, attrs_ptr(attrs)) }
    }

    /// Whether the library has `eglGetPlatformDisplayEXT`.
    pub fn is_get_platform_display_ext_available(&self) -> bool {
        self.is_get_platform_display_ext_native_available()
    }

    /// `eglInitialize`: the version of EGL is written to `major` and `minor`.
    pub fn initialize(&self, display: isize, major: &mut i32, minor: &mut i32) -> bool {
        // SAFETY: both pointers are valid for the one value written.
        unsafe { self.initialize_native(display, major, minor) != 0 }
    }

    /// `eglBindAPI`.
    pub fn bind_api(&self, api: i32) -> bool {
        self.bind_api_native(api) != 0
    }

    /// `eglChooseConfig` for one configuration: the first match is written to
    /// `surface_config` and the number of matches to `choosen_config`.
    pub fn choose_config(
        &self,
        display: isize,
        attribs: &[i32],
        surface_config: &mut isize,
        num_configs: i32,
        choosen_config: &mut i32,
    ) -> bool {
        // SAFETY: the attribute list is terminated (see `new`); EGL writes at most
        // `num_configs` configurations, and the caller passes the number `surface_config`
        // holds, which is one.
        unsafe { self.choose_config_native(display, attribs.as_ptr(), surface_config, num_configs.min(1), choosen_config) != 0 }
    }

    /// Returns all configs matching the attribute list. Pass no `configs` to
    /// query the available config count first. Some drivers (notably nvidia)
    /// expose multiple indistinguishable configs where only a subset is
    /// actually usable, so callers need to enumerate and probe them.
    pub fn choose_configs(
        &self,
        display: isize,
        attribs: &[i32],
        configs: Option<&mut [isize]>,
        config_size: i32,
        num_configs: &mut i32,
    ) -> bool {
        let (ptr, len) = match configs {
            Some(configs) => (configs.as_mut_ptr(), configs.len() as i32),
            None => (ptr::null_mut(), 0),
        };
        // SAFETY: the attribute list is terminated (see `new`); EGL writes no more
        // configurations than the buffer holds.
        unsafe { self.choose_config_native(display, attribs.as_ptr(), ptr, config_size.min(len), num_configs) != 0 }
    }

    /// `eglCreateContext`.
    pub fn create_context(&self, display: isize, config: isize, share: isize, attrs: &[i32]) -> isize {
        // SAFETY: the attribute list is terminated (see `new`).
        unsafe { self.create_context_native(display, config, share, attrs.as_ptr()) }
    }

    /// `eglDestroyContext`.
    pub fn destroy_context(&self, display: isize, context: isize) -> bool {
        self.destroy_context_native(display, context) != 0
    }

    /// `eglCreatePbufferSurface`.
    pub fn create_pbuffer_surface(&self, display: isize, config: isize, attrs: Option<&[i32]>) -> isize {
        // SAFETY: the attribute list is null or a terminated list (see `new`).
        unsafe { self.create_pbuffer_surface_native(display, config, attrs_ptr(attrs)) }
    }

    /// `eglMakeCurrent`.
    pub fn make_current(&self, display: isize, draw: isize, read: isize, context: isize) -> bool {
        self.make_current_native(display, draw, read, context) != 0
    }

    /// `eglSwapInterval`.
    pub fn swap_interval(&self, display: isize, interval: i32) -> bool {
        self.swap_interval_native(display, interval) != 0
    }

    /// `eglCreateWindowSurface`.
    pub fn create_window_surface(&self, display: isize, config: isize, window: isize, attrs: Option<&[i32]>) -> isize {
        // SAFETY: the attribute list is null or a terminated list (see `new`).
        unsafe { self.create_window_surface_native(display, config, window, attrs_ptr(attrs)) }
    }

    /// `eglGetConfigAttrib`: the value of the attribute is written to `rv`.
    pub fn get_config_attrib(&self, display: isize, config: isize, attr: i32, rv: &mut i32) -> bool {
        // SAFETY: the pointer is valid for the one value written.
        unsafe { self.get_config_attrib_native(display, config, attr, rv) != 0 }
    }

    /// `eglWaitGL`.
    pub fn wait_gl(&self) -> bool {
        self.wait_gl_native() != 0
    }

    /// `eglWaitClient`.
    pub fn wait_client(&self) -> bool {
        self.wait_client_native() != 0
    }

    /// `eglWaitNative`.
    pub fn wait_native(&self, engine: i32) -> bool {
        self.wait_native_native(engine) != 0
    }

    /// `eglQueryString`, `None` when the display has no such string.
    pub fn query_string(&self, display: isize, i: i32) -> Option<String> {
        let rv = self.query_string_native(display, i);
        if rv.is_null() {
            return None;
        }
        // SAFETY: a string EGL returns is terminated and stays valid while the display is.
        Some(unsafe { CStr::from_ptr(rv) }.to_string_lossy().into_owned())
    }

    /// `eglCreatePbufferFromClientBuffer`.
    pub fn create_pbuffer_from_client_buffer(
        &self,
        display: isize,
        buftype: i32,
        buffer: isize,
        config: isize,
        attrib_list: Option<&[i32]>,
    ) -> isize {
        // SAFETY: the attribute list is null or a terminated list (see `new`).
        unsafe { self.create_pbuffer_from_client_buffer_ptr(display, buftype, buffer, config, attrs_ptr(attrib_list)) }
    }

    /// `eglQueryDisplayAttribEXT`: the value of the attribute is written to `res`.
    pub fn query_display_attrib_ext(&self, display: isize, attr: i32, res: &mut isize) -> bool {
        // SAFETY: the pointer is valid for the one value written.
        unsafe { self.query_display_attrib_ext_native(display, attr, res) != 0 }
    }

    /// Whether the library has `eglQueryDisplayAttribEXT`.
    pub fn is_query_display_attrib_ext_available(&self) -> bool {
        self.is_query_display_attrib_ext_native_available()
    }

    /// `eglQueryDeviceAttribEXT`: the value of the attribute is written to `res`.
    pub fn query_device_attrib_ext(&self, display: isize, attr: i32, res: &mut isize) -> bool {
        // SAFETY: the pointer is valid for the one value written.
        unsafe { self.query_device_attrib_ext_native(display, attr, res) != 0 }
    }

    /// Whether the library has `eglQueryDeviceAttribEXT`.
    pub fn is_query_device_attrib_ext_available(&self) -> bool {
        self.is_query_device_attrib_ext_native_available()
    }

    /// `eglCreateImageKHR`.
    pub fn create_image_khr(&self, display: isize, context: isize, target: i32, client_buffer: isize, attribs: &[i32]) -> isize {
        // SAFETY: the attribute list is terminated (see `new`).
        unsafe { self.create_image_khr_native(display, context, target, client_buffer, attribs.as_ptr()) }
    }

    /// Whether the library has `eglCreateImageKHR`.
    pub fn is_create_image_khr_available(&self) -> bool {
        self.is_create_image_khr_native_available()
    }

    /// `eglDestroyImageKHR`.
    pub fn destroy_image_khr(&self, display: isize, image: isize) -> bool {
        self.destroy_image_khr_native(display, image) != 0
    }

    /// Whether the library has `eglDestroyImageKHR`.
    pub fn is_destroy_image_khr_available(&self) -> bool {
        self.is_destroy_image_khr_native_available()
    }

    /// `eglQueryDmaBufFormatsEXT`: the number of formats is written to
    /// `num_formats`.
    ///
    /// # Safety
    /// `formats` must be null or valid for `max_formats` values.
    pub unsafe fn query_dma_buf_formats_ext(
        &self,
        display: isize,
        max_formats: i32,
        formats: *mut i32,
        num_formats: &mut i32,
    ) -> bool {
        // SAFETY: the caller upholds the requirement on `formats`.
        unsafe { self.query_dma_buf_formats_ext_native(display, max_formats, formats, num_formats) != 0 }
    }

    /// Whether the library has `eglQueryDmaBufFormatsEXT`.
    pub fn is_query_dma_buf_formats_ext_available(&self) -> bool {
        self.is_query_dma_buf_formats_ext_native_available()
    }

    /// `eglQueryDmaBufModifiersEXT`: the number of modifiers is written to
    /// `num_modifiers`.
    ///
    /// # Safety
    /// `modifiers` and `external_only` must each be null or valid for
    /// `max_modifiers` values.
    pub unsafe fn query_dma_buf_modifiers_ext(
        &self,
        display: isize,
        format: i32,
        max_modifiers: i32,
        modifiers: *mut u64,
        external_only: *mut u32,
        num_modifiers: &mut i32,
    ) -> bool {
        // SAFETY: the caller upholds the requirement on the buffers.
        unsafe {
            self.query_dma_buf_modifiers_ext_native(display, format, max_modifiers, modifiers, external_only, num_modifiers)
                != 0
        }
    }

    /// Whether the library has `eglQueryDmaBufModifiersEXT`.
    pub fn is_query_dma_buf_modifiers_ext_available(&self) -> bool {
        self.is_query_dma_buf_modifiers_ext_native_available()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of the interface.
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    unsafe extern "system" fn get_error() -> i32 {
        crate::egl::egl_consts::EGL_BAD_ALLOC
    }

    unsafe extern "system" fn bind_api(api: i32) -> u32 {
        (api == 7) as u32
    }

    unsafe extern "system" fn initialize(_display: isize, major: *mut i32, minor: *mut i32) -> u32 {
        // SAFETY: the interface passes valid pointers.
        unsafe {
            *major = 1;
            *minor = 5;
        }
        1
    }

    unsafe extern "system" fn other() {}

    /// A loader that resolves three entry points to functions of this module and every other
    /// required one to a function that is never called; the optional ones are missing.
    fn loader(asked: Rc<RefCell<Vec<String>>>) -> GetProcAddress {
        Rc::new(move |name: &str| {
            asked.borrow_mut().push(name.to_string());
            match name {
                "eglGetError" => get_error as *const c_void,
                "eglBindAPI" => bind_api as *const c_void,
                "eglInitialize" => initialize as *const c_void,
                name if name.ends_with("EXT") || name.ends_with("KHR") => ptr::null(),
                _ => other as *const c_void,
            }
        })
    }

    #[test]
    fn the_entry_points_are_resolved_by_their_names_and_called() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        // SAFETY: the loader returns functions of the declared signatures for the entry
        // points this test calls.
        let egl = unsafe { EglInterface::new(&loader(asked.clone())) };

        assert!(asked.borrow().iter().any(|name| name == "eglChooseConfig"));
        assert_eq!(crate::egl::egl_consts::EGL_BAD_ALLOC, egl.get_error());
        assert!(egl.bind_api(7));
        assert!(!egl.bind_api(8));

        let (mut major, mut minor) = (0, 0);
        assert!(egl.initialize(0, &mut major, &mut minor));
        assert_eq!((1, 5), (major, minor));
    }

    #[test]
    fn missing_optional_entry_points_are_reported() {
        let asked = Rc::new(RefCell::new(Vec::new()));
        // SAFETY: as above.
        let egl = unsafe { EglInterface::new(&loader(asked)) };

        assert!(!egl.is_get_platform_display_ext_available());
        assert!(!egl.is_create_image_khr_available());
        assert!(!egl.is_destroy_image_khr_available());
        assert!(!egl.is_query_dma_buf_formats_ext_available());
    }
}
