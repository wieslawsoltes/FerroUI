//! The OpenGL of the system: the bootstrap context through which the
//! extensions of WGL are found, and the creation of contexts (the port of
//! `OpenGl/WglDisplay.cs`).

use super::wgl_consts::*;
use crate::interop::unmanaged_methods::{PixelFormatDescriptor, PixelFormatDescriptorFlags};
use ferroui_opengl::GlVersion;

/// The pixel format the bootstrap context asks the system for: a window
/// that is drawn to with OpenGL, double buffered, 32 bits of colour, 24 of
/// depth and 8 of stencil.
pub(crate) fn bootstrap_pixel_format_descriptor() -> PixelFormatDescriptor {
    PixelFormatDescriptor {
        size: std::mem::size_of::<PixelFormatDescriptor>() as u16,
        version: 1,
        flags: PixelFormatDescriptorFlags::PFD_DRAW_TO_WINDOW
            | PixelFormatDescriptorFlags::PFD_SUPPORT_OPENGL
            | PixelFormatDescriptorFlags::PFD_DOUBLEBUFFER,
        depth_bits: 24,
        stencil_bits: 8,
        color_bits: 32,
        ..Default::default()
    }
}

/// The attributes of the pixel format the contexts are created with, for
/// `wglChoosePixelFormatARB`: accelerated, double buffered, RGBA with eight
/// bits of alpha, no depth and no stencil.
pub(crate) fn pixel_format_attributes() -> [i32; 19] {
    [
        WGL_DRAW_TO_WINDOW_ARB,
        1,
        WGL_ACCELERATION_ARB,
        WGL_FULL_ACCELERATION_ARB,
        WGL_SUPPORT_OPENGL_ARB,
        1,
        WGL_DOUBLE_BUFFER_ARB,
        1,
        WGL_PIXEL_TYPE_ARB,
        WGL_TYPE_RGBA_ARB,
        WGL_COLOR_BITS_ARB,
        32,
        WGL_ALPHA_BITS_ARB,
        8,
        WGL_DEPTH_BITS_ARB,
        0,
        WGL_STENCIL_BITS_ARB,
        0,
        0, // End
    ]
}

/// The profile a context of `version` is created with: the compatibility
/// profile when the version asks for it and has profiles (3.2 and later),
/// the core profile otherwise.
pub(crate) fn profile_mask(version: GlVersion) -> i32 {
    if version.is_compatibility_profile() && (version.major() > 3 || version.major() == 3 && version.minor() >= 2) {
        WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB
    } else {
        WGL_CONTEXT_CORE_PROFILE_BIT_ARB
    }
}

/// The attributes of a context of `version`, for
/// `wglCreateContextAttribsARB`.
pub(crate) fn context_attributes(version: GlVersion) -> [i32; 8] {
    [
        // major
        WGL_CONTEXT_MAJOR_VERSION_ARB,
        version.major(),
        // minor
        WGL_CONTEXT_MINOR_VERSION_ARB,
        version.minor(),
        // core or compatibility profile
        WGL_CONTEXT_PROFILE_MASK_ARB,
        profile_mask(version),
        // debug
        // WGL_CONTEXT_FLAGS_ARB, 1,
        // end
        0,
        0,
    ]
}

#[cfg(windows)]
pub(crate) use imp::WglDisplay;

#[cfg(windows)]
mod imp {
    use super::super::wgl_context::WglContext;
    use super::super::wgl_gdi_resource_manager::WglGdiResourceManager;
    use super::super::wgl_restore_context::WglRestoreContext;
    use super::{bootstrap_pixel_format_descriptor, context_attributes, pixel_format_attributes};
    use crate::interop::unmanaged_methods::{
        choose_pixel_format, describe_pixel_format, load_library, set_pixel_format, wgl_create_context,
        wgl_get_proc_address, wgl_make_current,
    };
    use crate::interop::unmanaged_methods::PixelFormatDescriptor;
    use ferroui_base::reactive::IDisposable;
    use ferroui_opengl::{GlProfileType, GlVersion, OpenGlException};
    use std::ffi::{c_char, c_void};
    use std::rc::Rc;
    use std::sync::OnceLock;

    type WglChoosePixelFormatArb = unsafe extern "system" fn(
        hdc: *mut c_void,
        attrib_i_list: *const i32,
        attrib_f_list: *const f32,
        max_formats: u32,
        formats: *mut i32,
        num_formats: *mut u32,
    ) -> i32;
    type WglCreateContextAttribsArb =
        unsafe extern "system" fn(hdc: *mut c_void, share_context: *mut c_void, attrib_list: *const i32) -> *mut c_void;
    type DebugCallback = unsafe extern "system" fn(
        source: i32,
        type_: i32,
        id: i32,
        severity: i32,
        len: i32,
        message: *const c_char,
        user_param: *const c_void,
    );
    type GlDebugMessageCallback = unsafe extern "system" fn(callback: DebugCallback, user_param: *const c_void);

    /// What `InitializeCore` leaves behind.
    struct State {
        bootstrap_context: isize,
        bootstrap_dc: isize,
        default_pfd: PixelFormatDescriptor,
        default_pixel_format: i32,
        wgl_create_context_attribs_arb: WglCreateContextAttribsArb,
        gl_debug_message_callback: Option<GlDebugMessageCallback>,
    }

    /// The outcome of the one initialization of the process.
    enum Initialized {
        Ready(State),
        /// The system creates no context for the bootstrap window (the
        /// reference: `Initialize` answers false).
        NoBootstrapContext,
        /// An extension the contexts are created through is missing (the
        /// reference throws from the conversion of a null entry point, and
        /// tries again at the next call; here the outcome is kept, so that
        /// one bootstrap window is made).
        Failed(String),
    }

    static INITIALIZED: OnceLock<Initialized> = OnceLock::new();
    static OPEN_GL32_HANDLE: OnceLock<isize> = OnceLock::new();

    /// The debug output of a context goes to the standard error stream.
    unsafe extern "system" fn debug_callback(
        _source: i32,
        _type: i32,
        _id: i32,
        _severity: i32,
        len: i32,
        message: *const c_char,
        _user_param: *const c_void,
    ) {
        if message.is_null() || len <= 0 {
            return;
        }
        // SAFETY: the driver passes the message with its length in bytes,
        // valid during the call.
        let bytes = unsafe { std::slice::from_raw_parts(message.cast::<u8>(), len as usize) };
        eprintln!("{}", String::from_utf8_lossy(bytes));
    }

    pub(crate) struct WglDisplay;

    impl WglDisplay {
        /// The handle of the OpenGL library of the system (`OpenGl32Handle`).
        pub fn open_gl32_handle() -> isize {
            *OPEN_GL32_HANDLE.get_or_init(|| load_library("opengl32"))
        }

        fn initialize() -> &'static Initialized {
            INITIALIZED.get_or_init(Self::initialize_core)
        }

        // The reference verifies that it runs on the UI thread. Nothing of
        // what follows belongs to a thread (the window and its device
        // context are of the thread of the resource manager, and the
        // context is current only inside a call), so the port does not
        // ask: the tests of the crate run on the threads of the harness.
        fn initialize_core() -> Initialized {
            let bootstrap_window = WglGdiResourceManager::create_offscreen_window();
            let bootstrap_dc = WglGdiResourceManager::get_dc(bootstrap_window);
            let mut default_pfd = bootstrap_pixel_format_descriptor();
            let mut default_pixel_format = choose_pixel_format(bootstrap_dc, &default_pfd);
            set_pixel_format(bootstrap_dc, default_pixel_format, &default_pfd);

            let bootstrap_context = wgl_create_context(bootstrap_dc);
            if bootstrap_context == 0 {
                return Initialized::NoBootstrapContext;
            }

            wgl_make_current(bootstrap_dc, bootstrap_context);
            let create_context_attribs = wgl_get_proc_address(c"wglCreateContextAttribsARB");
            let choose_pixel_format_arb = wgl_get_proc_address(c"wglChoosePixelFormatARB");
            let debug_message_callback = wgl_get_proc_address(c"glDebugMessageCallback");

            if create_context_attribs.is_null() || choose_pixel_format_arb.is_null() {
                wgl_make_current(0, 0);
                let missing =
                    if create_context_attribs.is_null() { "wglCreateContextAttribsARB" } else { "wglChoosePixelFormatARB" };
                return Initialized::Failed(format!(
                    "the OpenGL implementation of the system has no {missing} (the generic implementation of the system is OpenGL 1.1)"
                ));
            }

            // SAFETY: the three addresses are the entry points the driver of
            // the current context has under these names, whose signatures
            // are the ones of the extension specifications
            // (WGL_ARB_create_context, WGL_ARB_pixel_format, KHR_debug).
            let (wgl_create_context_attribs_arb, wgl_choose_pixel_format_arb, gl_debug_message_callback) = unsafe {
                (
                    std::mem::transmute::<*const c_void, WglCreateContextAttribsArb>(create_context_attribs),
                    std::mem::transmute::<*const c_void, WglChoosePixelFormatArb>(choose_pixel_format_arb),
                    (!debug_message_callback.is_null())
                        .then(|| std::mem::transmute::<*const c_void, GlDebugMessageCallback>(debug_message_callback)),
                )
            };

            let attributes = pixel_format_attributes();
            let mut formats = [0i32; 1];
            let mut num_formats = 0u32;
            // SAFETY: the attribute list ends with a zero, there is no list
            // of floating-point attributes, and the driver writes at most
            // one format and the count.
            unsafe {
                wgl_choose_pixel_format_arb(
                    bootstrap_dc as *mut c_void,
                    attributes.as_ptr(),
                    std::ptr::null(),
                    1,
                    formats.as_mut_ptr(),
                    &mut num_formats,
                );
            }
            if num_formats != 0 {
                if let Some(descriptor) = describe_pixel_format(bootstrap_dc, formats[0]) {
                    default_pfd = descriptor;
                }
                default_pixel_format = formats[0];
            }

            wgl_make_current(0, 0);
            Initialized::Ready(State {
                bootstrap_context,
                bootstrap_dc,
                default_pfd,
                default_pixel_format,
                wgl_create_context_attribs_arb,
                gl_debug_message_callback,
            })
        }

        /// A context of the first of `versions` the system creates one
        /// for, sharing its objects with `share`. `Ok(None)` when the
        /// system has no OpenGL for a window or creates no context of any
        /// of the versions; an error when the extensions are missing or a
        /// context cannot be made current (the exceptions of the
        /// reference).
        pub fn create_context(
            versions: &[GlVersion],
            share: Option<&Rc<WglContext>>,
        ) -> Result<Option<Rc<WglContext>>, OpenGlException> {
            let state = match Self::initialize() {
                Initialized::Ready(state) => state,
                Initialized::NoBootstrapContext => return Ok(None),
                Initialized::Failed(message) => return Err(OpenGlException::new(message.clone())),
            };

            let restore = WglRestoreContext::new(state.bootstrap_dc, state.bootstrap_context)?;
            let result = Self::create_context_core(state, versions, share);
            restore.dispose();
            result
        }

        fn create_context_core(
            state: &State,
            versions: &[GlVersion],
            share: Option<&Rc<WglContext>>,
        ) -> Result<Option<Rc<WglContext>>, OpenGlException> {
            let window = WglGdiResourceManager::create_offscreen_window();
            let dc = WglGdiResourceManager::get_dc(window);
            set_pixel_format(dc, state.default_pixel_format, &state.default_pfd);
            for version in versions {
                if version.type_() != GlProfileType::OpenGL {
                    continue;
                }
                let attributes = context_attributes(*version);
                let share_handle = share.map_or(0, |share| share.handle());
                // SAFETY: a device context with a pixel format, a context
                // or null, and an attribute list that ends with a zero.
                let context = unsafe {
                    (state.wgl_create_context_attribs_arb)(dc as *mut c_void, share_handle as *mut c_void, attributes.as_ptr())
                } as isize;

                // The reference sets the callback whatever the creation
                // answered, which with no context made nothing current and
                // called into the driver without one.
                if context != 0 {
                    if let Some(gl_debug_message_callback) = state.gl_debug_message_callback {
                        let current = WglRestoreContext::new(dc, context)?;
                        // SAFETY: the context is current; the callback is a
                        // function of the process with the signature of
                        // KHR_debug.
                        unsafe { gl_debug_message_callback(debug_callback, std::ptr::null()) };
                        current.dispose();
                    }

                    return WglContext::new(
                        share.cloned(),
                        *version,
                        context,
                        window,
                        dc,
                        state.default_pixel_format,
                        state.default_pfd,
                    )
                    .map(Some);
                }
            }

            WglGdiResourceManager::release_dc(window, dc);
            WglGdiResourceManager::destroy_window(window);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of WGL.
    use super::*;
    use ferroui_opengl::GlProfileType;

    #[test]
    fn the_pixel_format_descriptor_has_the_layout_of_the_system() {
        assert_eq!(40, std::mem::size_of::<PixelFormatDescriptor>());
        assert_eq!(4, std::mem::align_of::<PixelFormatDescriptor>());
        assert_eq!(4, std::mem::offset_of!(PixelFormatDescriptor, flags));
        assert_eq!(9, std::mem::offset_of!(PixelFormatDescriptor, color_bits));
        assert_eq!(23, std::mem::offset_of!(PixelFormatDescriptor, depth_bits));
        assert_eq!(24, std::mem::offset_of!(PixelFormatDescriptor, stencil_bits));
        assert_eq!(28, std::mem::offset_of!(PixelFormatDescriptor, layer_mask));
    }

    #[test]
    fn the_bootstrap_context_asks_for_a_double_buffered_window_with_depth_and_stencil() {
        let pfd = bootstrap_pixel_format_descriptor();

        assert_eq!(40, pfd.size);
        assert_eq!(1, pfd.version);
        assert_eq!(0x25, pfd.flags);
        assert_eq!((32, 24, 8), (pfd.color_bits, pfd.depth_bits, pfd.stencil_bits));
        // PFD_TYPE_RGBA and PFD_MAIN_PLANE are zero.
        assert_eq!((0, 0), (pfd.pixel_type, pfd.layer_type));
    }

    #[test]
    fn the_pixel_format_of_the_contexts_is_accelerated_rgba_without_depth_and_stencil() {
        let attributes = pixel_format_attributes();

        assert_eq!(Some(&0), attributes.last());
        let pairs: Vec<(i32, i32)> = attributes[..18].chunks(2).map(|pair| (pair[0], pair[1])).collect();
        assert!(pairs.contains(&(WGL_ACCELERATION_ARB, WGL_FULL_ACCELERATION_ARB)));
        assert!(pairs.contains(&(WGL_PIXEL_TYPE_ARB, WGL_TYPE_RGBA_ARB)));
        assert!(pairs.contains(&(WGL_COLOR_BITS_ARB, 32)));
        assert!(pairs.contains(&(WGL_ALPHA_BITS_ARB, 8)));
        assert!(pairs.contains(&(WGL_DEPTH_BITS_ARB, 0)));
        assert!(pairs.contains(&(WGL_STENCIL_BITS_ARB, 0)));
        assert!(pairs.contains(&(WGL_DOUBLE_BUFFER_ARB, 1)));
    }

    #[test]
    fn a_context_has_the_core_profile_unless_its_version_asks_for_compatibility_and_has_profiles() {
        let core = GlVersion::new(GlProfileType::OpenGL, 4, 0);
        let compatibility = |major, minor| GlVersion::with_compatibility_profile(GlProfileType::OpenGL, major, minor, true);

        assert_eq!(WGL_CONTEXT_CORE_PROFILE_BIT_ARB, profile_mask(core));
        assert_eq!(WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB, profile_mask(compatibility(4, 0)));
        assert_eq!(WGL_CONTEXT_COMPATIBILITY_PROFILE_BIT_ARB, profile_mask(compatibility(3, 2)));
        // Before 3.2 there are no profiles.
        assert_eq!(WGL_CONTEXT_CORE_PROFILE_BIT_ARB, profile_mask(compatibility(3, 1)));
        assert_eq!(WGL_CONTEXT_CORE_PROFILE_BIT_ARB, profile_mask(compatibility(2, 1)));
    }

    #[test]
    fn the_attributes_of_a_context_name_its_version_and_profile() {
        assert_eq!(
            [
                WGL_CONTEXT_MAJOR_VERSION_ARB,
                3,
                WGL_CONTEXT_MINOR_VERSION_ARB,
                2,
                WGL_CONTEXT_PROFILE_MASK_ARB,
                WGL_CONTEXT_CORE_PROFILE_BIT_ARB,
                0,
                0
            ],
            context_attributes(GlVersion::new(GlProfileType::OpenGL, 3, 2))
        );
    }
}
