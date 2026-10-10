//! The EGL display of ANGLE on Windows: over the Direct3D 9 platform, over
//! the Direct3D 11 platform with a device ANGLE creates, and over a
//! Direct3D 11 device this backend creates on the adapter it chooses.

use super::Win32AngleEglInterface;
use crate::angle_options::{AngleOptions, PlatformApi};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_opengl::egl::egl_consts::*;
use ferroui_opengl::egl::{EglDisplay, EglDisplayOptions, EglInterface, EglSurface};
use ferroui_opengl::{GlProfileType, OpenGlException};
use std::ops::Deref;
use std::rc::Rc;

/// A display of ANGLE.
///
/// The reference derives the type from the EGL display; here it holds the
/// display and dereferences to it. The lock of the display, which the
/// reference shares with its contexts, does not exist in the port: the EGL
/// objects belong to one thread (`ferroui_opengl::egl::EglDisplay`).
pub struct AngleWin32EglDisplay {
    display: Rc<EglDisplay>,
    platform_api: PlatformApi,
    flexible_surface_supported: bool,
}

impl Deref for AngleWin32EglDisplay {
    type Target = Rc<EglDisplay>;

    fn deref(&self) -> &Rc<EglDisplay> {
        &self.display
    }
}

impl AngleWin32EglDisplay {
    pub(super) fn get_display_options(
        egl: &Rc<EglInterface>,
        device_lost_check_callback: Option<Rc<dyn Fn() -> bool>>,
        dispose_callback: Option<Rc<dyn Fn()>>,
    ) -> EglDisplayOptions {
        EglDisplayOptions {
            egl: Some(egl.clone()),
            context_loss_is_display_loss: true,
            gl_versions: FerroLocator::current().get_service::<AngleOptions>().map(|options| {
                options.gl_profiles.iter().copied().filter(|x| x.type_() == GlProfileType::OpenGLES).collect()
            }),
            device_lost_check_callback,
            dispose_callback,
            ..Default::default()
        }
    }

    /// The display of ANGLE on Direct3D 9.
    pub fn create_d3d9_display(egl: &Win32AngleEglInterface) -> Result<AngleWin32EglDisplay, OpenGlException> {
        let display = egl.egl().get_platform_display_ext(
            EGL_PLATFORM_ANGLE_ANGLE,
            0,
            Some(&[EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D9_ANGLE, EGL_NONE]),
        );

        Self::new(display, egl.egl(), Self::get_display_options(egl.egl(), None, None), PlatformApi::DirectX9)
    }

    /// The display of ANGLE on Direct3D 11, with the device ANGLE creates
    /// and shares between its displays.
    pub fn create_shared_d3d11_display(egl: &Win32AngleEglInterface) -> Result<AngleWin32EglDisplay, OpenGlException> {
        let display = egl.egl().get_platform_display_ext(
            EGL_PLATFORM_ANGLE_ANGLE,
            0,
            Some(&[EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE, EGL_NONE]),
        );

        Self::new(display, egl.egl(), Self::get_display_options(egl.egl(), None, None), PlatformApi::DirectX11)
    }

    /// Wraps a display of EGL.
    ///
    /// The reference passes a display that is zero to the constructor of
    /// the EGL display, which rejects it with an exception the callers
    /// catch; the display of the port panics for it, so the handle is
    /// checked here and the failure is the error of the EGL call that
    /// returned it.
    pub(super) fn new(
        display: isize,
        egl: &Rc<EglInterface>,
        options: EglDisplayOptions,
        platform_api: PlatformApi,
    ) -> Result<AngleWin32EglDisplay, OpenGlException> {
        if display == 0 {
            return Err(OpenGlException::get_formatted_exception_for_egl("eglGetPlatformDisplayEXT", egl));
        }
        let display = EglDisplay::new(display, options)?;
        let extensions = egl.query_string(display.handle(), EGL_EXTENSIONS);
        let flexible_surface_supported =
            extensions.is_some_and(|extensions| extensions.contains("EGL_ANGLE_flexible_surface_compatibility"));
        Ok(AngleWin32EglDisplay { display, platform_api, flexible_surface_supported })
    }

    /// The EGL display.
    pub fn display(&self) -> &Rc<EglDisplay> {
        &self.display
    }

    pub fn platform_api(&self) -> PlatformApi {
        self.platform_api
    }

    /// The device of Direct3D the display renders with: a pointer to its
    /// `IDirect3DDevice9` or `ID3D11Device`, which the display owns.
    pub fn get_direct3d_device(&self) -> Result<isize, OpenGlException> {
        let egl = self.display.egl_interface();
        let mut egl_device = 0;
        if !egl.is_query_display_attrib_ext_available()
            || !egl.query_display_attrib_ext(self.display.handle(), EGL_DEVICE_EXT, &mut egl_device)
        {
            return Err(OpenGlException::new("Unable to get EGL_DEVICE_EXT"));
        }
        let mut d3d_device_handle = 0;
        if !egl.is_query_device_attrib_ext_available()
            || !egl.query_device_attrib_ext(
                egl_device,
                if self.platform_api == PlatformApi::DirectX9 { EGL_D3D9_DEVICE_ANGLE } else { EGL_D3D11_DEVICE_ANGLE },
                &mut d3d_device_handle,
            )
        {
            return Err(OpenGlException::new("Unable to get EGL_D3D9_DEVICE_ANGLE"));
        }
        Ok(d3d_device_handle)
    }

    /// A surface over a texture of Direct3D 11 (a pointer to its
    /// `ID3D11Texture2D`).
    ///
    /// # Panics
    /// Panics when the display is not a display on Direct3D 11.
    pub fn wrap_direct3d11_texture(&self, handle: isize) -> Result<Rc<EglSurface>, OpenGlException> {
        if self.platform_api != PlatformApi::DirectX11 {
            self.throw_invalid_platform_api();
        }
        self.display.create_pbuffer_from_client_buffer(EGL_D3D_TEXTURE_ANGLE, handle, &[EGL_NONE, EGL_NONE])
    }

    /// A surface over a rectangle of a texture of Direct3D 11.
    ///
    /// # Panics
    /// Panics when the display is not a display on Direct3D 11.
    pub fn wrap_direct3d11_texture_with_offset(
        &self,
        handle: isize,
        offset_x: i32,
        offset_y: i32,
        width: i32,
        height: i32,
    ) -> Result<Rc<EglSurface>, OpenGlException> {
        if self.platform_api != PlatformApi::DirectX11 {
            self.throw_invalid_platform_api();
        }
        let attrs = [
            EGL_WIDTH,
            width,
            EGL_HEIGHT,
            height,
            EGL_TEXTURE_OFFSET_X_ANGLE,
            offset_x,
            EGL_TEXTURE_OFFSET_Y_ANGLE,
            offset_y,
            if self.flexible_surface_supported { EGL_FLEXIBLE_SURFACE_COMPATIBILITY_SUPPORTED_ANGLE } else { EGL_NONE },
            EGL_TRUE,
            EGL_NONE,
        ];
        self.display.create_pbuffer_from_client_buffer(EGL_D3D_TEXTURE_ANGLE, handle, &attrs)
    }

    #[cold]
    fn throw_invalid_platform_api(&self) -> ! {
        panic!("Current platform API is {:?}", self.platform_api)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from upstream: the reference has no tests of the display. The EGL
    // these tests run against is a set of functions of this module.
    use super::*;
    use ferroui_base::reactive::IDisposable;
    use ferroui_opengl::GetProcAddress;
    use std::cell::{Cell, RefCell};
    use std::ffi::{c_char, c_void};

    thread_local! {
        /// The platform displays that were asked for: the platform, the
        /// native display and the attributes.
        pub(crate) static PLATFORM_DISPLAYS: RefCell<Vec<(i32, isize, Vec<i32>)>> = const { RefCell::new(Vec::new()) };
        /// The client buffers surfaces were created for: the type, the
        /// buffer and the attributes.
        static CLIENT_BUFFERS: RefCell<Vec<(i32, isize, Vec<i32>)>> = const { RefCell::new(Vec::new()) };
        /// The display handle `eglGetPlatformDisplayEXT` answers with.
        pub(crate) static NEXT_DISPLAY: Cell<isize> = const { Cell::new(41) };
        /// Whether the display names the flexible surface extension.
        static FLEXIBLE: Cell<bool> = const { Cell::new(false) };
        pub(crate) static TERMINATED: Cell<i32> = const { Cell::new(0) };
    }

    /// The attributes of a list terminated with `EGL_NONE`.
    ///
    /// # Safety
    /// `attrs` is null or points at such a list.
    unsafe fn read_attrs(attrs: *const i32) -> Vec<i32> {
        let mut list = Vec::new();
        if attrs.is_null() {
            return list;
        }
        let mut i = 0;
        loop {
            // SAFETY: the list is terminated, and `i` has not passed its end.
            let value = unsafe { *attrs.add(i) };
            list.push(value);
            if value == EGL_NONE && (i % 2 == 0) {
                return list;
            }
            i += 1;
        }
    }

    unsafe extern "system" fn get_error() -> i32 {
        EGL_BAD_DISPLAY
    }

    unsafe extern "system" fn get_platform_display(platform: i32, native_display: isize, attrs: *const i32) -> isize {
        // SAFETY: the display passes null or a terminated list.
        let attrs = unsafe { read_attrs(attrs) };
        PLATFORM_DISPLAYS.with(|displays| displays.borrow_mut().push((platform, native_display, attrs)));
        NEXT_DISPLAY.with(Cell::get)
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
        // SAFETY: the interface passes a valid count pointer and a buffer of
        // `config_size` configurations or null.
        unsafe {
            *num_configs = 1;
            if !configs.is_null() && config_size >= 1 {
                *configs = 11;
            }
        }
        1
    }

    unsafe extern "system" fn get_config_attrib(_display: isize, _config: isize, _attr: i32, rv: *mut i32) -> u32 {
        // SAFETY: the interface passes a valid pointer.
        unsafe {
            *rv = 8;
        }
        1
    }

    unsafe extern "system" fn query_string(_display: isize, name: i32) -> *const c_char {
        if name == EGL_EXTENSIONS && FLEXIBLE.with(Cell::get) {
            c"EGL_ANGLE_d3d_texture_client_buffer EGL_ANGLE_flexible_surface_compatibility".as_ptr()
        } else {
            c"EGL_ANGLE_d3d_texture_client_buffer".as_ptr()
        }
    }

    unsafe extern "system" fn terminate(_display: isize) {
        TERMINATED.with(|count| count.set(count.get() + 1));
    }

    unsafe extern "system" fn create_pbuffer_from_client_buffer(
        _display: isize,
        buftype: i32,
        buffer: isize,
        _config: isize,
        attrs: *const i32,
    ) -> isize {
        // SAFETY: the display passes a terminated list.
        let attrs = unsafe { read_attrs(attrs) };
        CLIENT_BUFFERS.with(|buffers| buffers.borrow_mut().push((buftype, buffer, attrs)));
        91
    }

    unsafe extern "system" fn query_display_attrib(_display: isize, attr: i32, res: *mut isize) -> u32 {
        if attr != EGL_DEVICE_EXT {
            return 0;
        }
        // SAFETY: the interface passes a valid pointer.
        unsafe {
            *res = 500;
        }
        1
    }

    unsafe extern "system" fn query_device_attrib(device: isize, attr: i32, res: *mut isize) -> u32 {
        if device != 500 {
            return 0;
        }
        // SAFETY: the interface passes a valid pointer.
        unsafe {
            *res = if attr == EGL_D3D11_DEVICE_ANGLE { 1100 } else { 900 };
        }
        1
    }

    unsafe extern "system" fn destroy_surface(_display: isize, _surface: isize) {}

    unsafe extern "system" fn other() {}

    /// A loader of an EGL that initialises, has one configuration, and
    /// records what it is asked. The entry points of the device creation
    /// come from `device_creation`.
    pub(crate) fn loader(device_creation: Option<GetProcAddress>) -> GetProcAddress {
        Rc::new(move |name: &str| match name {
            "eglGetError" => get_error as *const c_void,
            "eglGetPlatformDisplayEXT" => get_platform_display as *const c_void,
            "eglInitialize" => initialize as *const c_void,
            "eglBindAPI" => bind_api as *const c_void,
            "eglChooseConfig" => choose_config as *const c_void,
            "eglGetConfigAttrib" => get_config_attrib as *const c_void,
            "eglQueryString" => query_string as *const c_void,
            "eglTerminate" => terminate as *const c_void,
            "eglCreatePbufferFromClientBuffer" => create_pbuffer_from_client_buffer as *const c_void,
            "eglQueryDisplayAttribEXT" => query_display_attrib as *const c_void,
            "eglQueryDeviceAttribEXT" => query_device_attrib as *const c_void,
            "eglDestroySurface" => destroy_surface as *const c_void,
            name if name.ends_with("ANGLE") => device_creation.as_ref().map_or(std::ptr::null(), |get| get(name)),
            name if name.ends_with("EXT") || name.ends_with("KHR") => std::ptr::null(),
            _ => other as *const c_void,
        })
    }

    pub(crate) fn egl() -> Win32AngleEglInterface {
        // SAFETY: the loader returns functions of the declared signatures
        // for the entry points the tests call.
        unsafe { Win32AngleEglInterface::from_loader(&loader(None)) }
    }

    fn reset() {
        PLATFORM_DISPLAYS.with(|displays| displays.borrow_mut().clear());
        CLIENT_BUFFERS.with(|buffers| buffers.borrow_mut().clear());
        NEXT_DISPLAY.with(|next| next.set(41));
        FLEXIBLE.with(|flexible| flexible.set(false));
    }

    #[test]
    fn the_displays_of_the_two_platforms_are_asked_for_with_their_types() {
        reset();
        let egl = egl();

        let d3d9 = AngleWin32EglDisplay::create_d3d9_display(&egl).expect("a display");
        let d3d11 = AngleWin32EglDisplay::create_shared_d3d11_display(&egl).expect("a display");

        assert_eq!(PlatformApi::DirectX9, d3d9.platform_api());
        assert_eq!(PlatformApi::DirectX11, d3d11.platform_api());
        assert_eq!(41, d3d11.handle());
        assert_eq!(
            vec![
                (EGL_PLATFORM_ANGLE_ANGLE, 0, vec![EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D9_ANGLE, EGL_NONE]),
                (EGL_PLATFORM_ANGLE_ANGLE, 0, vec![EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE, EGL_NONE]),
            ],
            PLATFORM_DISPLAYS.with(|displays| displays.borrow().clone())
        );
    }

    #[test]
    fn a_display_that_egl_does_not_give_is_an_error_with_the_code_of_egl() {
        reset();
        NEXT_DISPLAY.with(|next| next.set(0));

        let error = AngleWin32EglDisplay::create_shared_d3d11_display(&egl()).err().expect("an error");

        assert_eq!(Some(EGL_BAD_DISPLAY), error.error_code());
        assert!(error.message().starts_with("eglGetPlatformDisplayEXT failed"));
    }

    #[test]
    fn the_direct3d_device_is_read_through_the_device_of_the_display() {
        reset();
        let egl = egl();

        let d3d11 = AngleWin32EglDisplay::create_shared_d3d11_display(&egl).expect("a display");
        let d3d9 = AngleWin32EglDisplay::create_d3d9_display(&egl).expect("a display");

        assert_eq!(Ok(1100), d3d11.get_direct3d_device());
        assert_eq!(Ok(900), d3d9.get_direct3d_device());
    }

    #[test]
    fn a_texture_is_wrapped_whole_or_as_a_rectangle() {
        reset();
        let display = AngleWin32EglDisplay::create_shared_d3d11_display(&egl()).expect("a display");

        let whole = display.wrap_direct3d11_texture(77).expect("a surface");
        let part = display.wrap_direct3d11_texture_with_offset(78, 3, 4, 100, 50).expect("a surface");

        assert_eq!(91, whole.dangerous_get_handle());
        assert_eq!(91, part.dangerous_get_handle());
        assert_eq!(
            vec![
                (EGL_D3D_TEXTURE_ANGLE, 77, vec![EGL_NONE]),
                (
                    EGL_D3D_TEXTURE_ANGLE,
                    78,
                    vec![
                        EGL_WIDTH,
                        100,
                        EGL_HEIGHT,
                        50,
                        EGL_TEXTURE_OFFSET_X_ANGLE,
                        3,
                        EGL_TEXTURE_OFFSET_Y_ANGLE,
                        4,
                        EGL_NONE
                    ]
                ),
            ],
            CLIENT_BUFFERS.with(|buffers| buffers.borrow().clone())
        );
    }

    #[test]
    fn the_flexible_surface_attribute_is_passed_when_the_display_names_the_extension() {
        reset();
        FLEXIBLE.with(|flexible| flexible.set(true));
        let display = AngleWin32EglDisplay::create_shared_d3d11_display(&egl()).expect("a display");

        display.wrap_direct3d11_texture_with_offset(78, 0, 0, 10, 10).expect("a surface");

        let attrs = CLIENT_BUFFERS.with(|buffers| buffers.borrow().last().expect("a surface was created").2.clone());
        assert_eq!(&[EGL_FLEXIBLE_SURFACE_COMPATIBILITY_SUPPORTED_ANGLE, EGL_TRUE, EGL_NONE], &attrs[attrs.len() - 3..]);
    }

    #[test]
    #[should_panic(expected = "Current platform API is DirectX9")]
    fn a_texture_of_direct3d_11_is_not_wrapped_on_direct3d_9() {
        reset();
        let display = AngleWin32EglDisplay::create_d3d9_display(&egl()).expect("a display");

        let _ = display.wrap_direct3d11_texture(77);
    }

    #[test]
    fn disposing_the_display_terminates_it() {
        reset();
        let display = AngleWin32EglDisplay::create_shared_d3d11_display(&egl()).expect("a display");
        let terminated = TERMINATED.with(Cell::get);

        display.dispose();

        assert_eq!(terminated + 1, TERMINATED.with(Cell::get));
        assert!(display.is_lost());
    }
}
