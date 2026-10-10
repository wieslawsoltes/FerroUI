//! The EGL display of ANGLE on Windows: over the Direct3D 9 platform, over
//! the Direct3D 11 platform with a device ANGLE creates, and over a
//! Direct3D 11 device this backend creates on the adapter it chooses.

use super::Win32AngleEglInterface;
use crate::angle_options::{AngleOptions, PlatformApi};
use crate::win32_platform_options::GraphicsAdapterSelectionCallback;
use ferroui_base::platform::PlatformGraphicsDeviceAdapterDescription;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_opengl::egl::egl_consts::*;
use ferroui_opengl::egl::{EglDisplay, EglDisplayOptions, EglInterface, EglSurface};
use ferroui_opengl::{GlVersion, OpenGlException};
use std::ops::Deref;
use std::rc::Rc;

/// How the adapter of the Direct3D 11 device of a display is chosen.
///
/// The reference chooses on every creation of a display, with the callback
/// of the platform options. A display of the port is created on the thread
/// that renders with it, and the callback is a value of the UI thread: the
/// choice is made once, on the UI thread, when the platform graphics are
/// probed ([`Select`](Self::Select)), and the displays that are created
/// later ask for the adapter that was chosen then
/// ([`Chosen`](Self::Chosen)).
#[derive(Clone)]
pub enum D3D11Adapter {
    /// Choose now: with the callback when there is one, and away from an
    /// Adreno adapter on ARM64.
    Select(Option<GraphicsAdapterSelectionCallback>),
    /// The adapter with this identifier (`None`: the first adapter of the
    /// system, which is what the reference takes when nothing chooses).
    Chosen(Option<u64>),
}

/// The index of the adapter to create the device on, among the adapters of
/// the system, and whether the choice moved away from an Adreno adapter.
///
/// `None` when nothing redefines the default adapter: no callback, and not
/// ARM64.
pub(crate) fn choose_adapter_index(
    adapters: &[PlatformGraphicsDeviceAdapterDescription],
    selection_callback: Option<&GraphicsAdapterSelectionCallback>,
    apply_arm_adreno_blacklist: bool,
) -> Result<(usize, bool), OpenGlException> {
    if adapters.is_empty() {
        return Err(OpenGlException::new("No adapters found"));
    }

    // The Adreno blacklist now only moves the *default* selection away
    // from Adreno GPUs - it no longer hides adapters from the selection callback, so
    // an application can deliberately opt back into hardware acceleration.
    let mut chosen_adapter_index = 0usize;
    let mut moved_from_adreno = false;
    if let Some(selection_callback) = selection_callback {
        let index = selection_callback(adapters);
        // The reference indexes its list with the answer and fails with the
        // exception of the index.
        chosen_adapter_index = usize::try_from(index)
            .ok()
            .filter(|index| *index < adapters.len())
            .ok_or_else(|| OpenGlException::new(format!("The graphics adapter selection callback chose the adapter {index} of {}", adapters.len())))?;
    } else if apply_arm_adreno_blacklist && adapters.len() > 1 {
        let first_non_adreno =
            adapters.iter().position(|a| !a.description.as_deref().is_some_and(|description| description.contains("adreno")));
        if let Some(first_non_adreno) = first_non_adreno.filter(|index| *index > 0) {
            chosen_adapter_index = first_non_adreno;
            moved_from_adreno = true;
        }
    }

    Ok((chosen_adapter_index, moved_from_adreno))
}

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
    /// The versions of OpenGL ES the options of ANGLE ask for: the options
    /// registered with the services of the calling thread, `None` without
    /// them (the display then asks for its default versions).
    pub fn gl_versions_of_options() -> Option<Vec<GlVersion>> {
        FerroLocator::current().get_service::<AngleOptions>().map(|options| options.open_gl_es_profiles())
    }

    /// The reference reads the versions from the services where the options
    /// are built; here the caller passes them, because a display is also
    /// created on the render thread, whose services are not the ones of the
    /// UI thread.
    pub(super) fn get_display_options(
        egl: &Rc<EglInterface>,
        gl_versions: Option<Vec<GlVersion>>,
        device_lost_check_callback: Option<Rc<dyn Fn() -> bool>>,
        dispose_callback: Option<Rc<dyn Fn()>>,
    ) -> EglDisplayOptions {
        EglDisplayOptions {
            egl: Some(egl.clone()),
            context_loss_is_display_loss: true,
            gl_versions,
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

        Self::new(display, egl.egl(), Self::get_display_options(egl.egl(), Self::gl_versions_of_options(), None, None), PlatformApi::DirectX9)
    }

    /// The display of ANGLE on Direct3D 11, with the device ANGLE creates
    /// and shares between its displays.
    pub fn create_shared_d3d11_display(egl: &Win32AngleEglInterface) -> Result<AngleWin32EglDisplay, OpenGlException> {
        let display = egl.egl().get_platform_display_ext(
            EGL_PLATFORM_ANGLE_ANGLE,
            0,
            Some(&[EGL_PLATFORM_ANGLE_TYPE_ANGLE, EGL_PLATFORM_ANGLE_TYPE_D3D11_ANGLE, EGL_NONE]),
        );

        Self::new(display, egl.egl(), Self::get_display_options(egl.egl(), Self::gl_versions_of_options(), None, None), PlatformApi::DirectX11)
    }

    /// The display of ANGLE on a Direct3D 11 device this backend creates on
    /// the adapter that is chosen, and the identifier of that adapter
    /// (`None` when the first adapter of the system was taken without a
    /// choice).
    #[cfg(windows)]
    pub fn create_d3d11_display(
        egl: &Rc<Win32AngleEglInterface>,
        gl_versions: Option<Vec<GlVersion>>,
        adapter: &D3D11Adapter,
    ) -> Result<(AngleWin32EglDisplay, Option<u64>), OpenGlException> {
        use crate::direct_x::{DirectXUnmanagedMethods, IDXGIAdapter1, D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL};
        use ferroui_base::logging::{LogEventLevel, Logger};
        use ferroui_microcom::ComPtr;
        use std::cell::{Cell, RefCell};

        let feature_levels = [
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_11_1,
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_11_0,
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_10_1,
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_10_0,
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_9_3,
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_9_2,
            D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_9_1,
        ];

        let factory = DirectXUnmanagedMethods::create_dxgi_factory1()
            .map_err(|error| OpenGlException::new(format!("CreateDXGIFactory1 failed: {error}")))?;
        let mut chosen_adapter: Option<ComPtr<IDXGIAdapter1>> = None;
        let mut chosen_luid = None;
        if let Some(factory) = factory {
            // The adapter at an index: the reference of the factory, owned.
            let adapter_at = |index: u32| -> Option<ComPtr<IDXGIAdapter1>> {
                let mut p_adapter = std::ptr::null_mut();
                // SAFETY: the pointer is valid for the one interface pointer
                // the factory writes, which is an adapter of the first
                // version (the method is `EnumAdapters1`).
                if unsafe { factory.enum_adapters1(index, &mut p_adapter) } != 0 {
                    return None;
                }
                // SAFETY: the call succeeded: the pointer is null or a
                // reference to an adapter this call owns.
                unsafe { ComPtr::from_raw(p_adapter.cast()) }
            };
            let enumerate = || -> Result<Vec<(ComPtr<IDXGIAdapter1>, PlatformGraphicsDeviceAdapterDescription, u64)>, OpenGlException> {
                let mut adapters = Vec::new();
                let mut adapter_index = 0u32;
                while let Some(adapter) = adapter_at(adapter_index) {
                    let desc = adapter
                        .get_desc1()
                        .map_err(|error| OpenGlException::new(format!("IDXGIAdapter1::GetDesc1 failed: {error}")))?;
                    let length = desc.description.iter().position(|c| *c == 0).unwrap_or(desc.description.len());
                    let name = String::from_utf16_lossy(&desc.description[..length]).to_lowercase();
                    let luid = desc.adapter_luid.to_le_bytes().to_vec();
                    adapters.push((
                        adapter,
                        PlatformGraphicsDeviceAdapterDescription {
                            description: Some(name),
                            device_luid: Some(luid),
                            ..Default::default()
                        },
                        desc.adapter_luid,
                    ));
                    adapter_index += 1;
                }
                Ok(adapters)
            };

            match adapter {
                D3D11Adapter::Select(selection_callback) => {
                    let apply_arm_adreno_blacklist = cfg!(target_arch = "aarch64");

                    // As for now, we only need to redefine default adapter only on ARM64 just in case of Adreno GPU.
                    let redefine_default_adapter = selection_callback.is_some() || apply_arm_adreno_blacklist;

                    if redefine_default_adapter {
                        let mut adapters = enumerate()?;
                        let descriptions: Vec<_> = adapters.iter().map(|a| a.1.clone()).collect();
                        let (chosen_adapter_index, moved_from_adreno) =
                            choose_adapter_index(&descriptions, selection_callback.as_ref(), apply_arm_adreno_blacklist)?;
                        if moved_from_adreno {
                            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "OpenGL") {
                                let fallback = descriptions[chosen_adapter_index].description.clone().unwrap_or_default();
                                logger.log_with_values(
                                    None,
                                    "ARM64 Adreno GPU detected; the Adreno rendering blocklist is forcing a \
                                     fallback to '{FallbackAdapter}' (typically a software renderer). Set \
                                     Win32PlatformOptions.graphics_adapter_selection_callback to choose an adapter \
                                     explicitly, or switch Win32PlatformOptions.rendering_mode to Vulkan or Wgl.",
                                    &[&fallback],
                                );
                            }
                        }

                        let (adapter, _, luid) = adapters.swap_remove(chosen_adapter_index);
                        chosen_adapter = Some(adapter);
                        chosen_luid = Some(luid);
                    } else {
                        chosen_adapter = Some(adapter_at(0).ok_or_else(|| OpenGlException::new("No adapters found"))?);
                    }
                }
                D3D11Adapter::Chosen(Some(luid)) => {
                    // The adapter that was chosen; the first one when it is
                    // gone (an adapter that was removed).
                    let mut adapters = enumerate()?;
                    if adapters.is_empty() {
                        return Err(OpenGlException::new("No adapters found"));
                    }
                    let index = adapters.iter().position(|a| a.2 == *luid).unwrap_or(0);
                    let (adapter, _, luid) = adapters.swap_remove(index);
                    chosen_adapter = Some(adapter);
                    chosen_luid = Some(luid);
                }
                D3D11Adapter::Chosen(None) => {
                    chosen_adapter = Some(adapter_at(0).ok_or_else(|| OpenGlException::new("No adapters found"))?);
                }
            }
        }

        let (d3d_device, _) = DirectXUnmanagedMethods::d3d11_create_device(
            chosen_adapter.as_deref(),
            D3D_DRIVER_TYPE::D3D_DRIVER_TYPE_UNKNOWN,
            0,
            &feature_levels,
            7,
        )
        .map_err(|error| OpenGlException::new(format!("D3D11CreateDevice failed: {error}")))?;
        drop(chosen_adapter);

        let Some(d3d_device) = d3d_device else {
            return Err(OpenGlException::new("Unable to create D3D11 Device"));
        };
        let p_d3d_device = d3d_device.as_ptr() as isize;

        // What `Cleanup` of the reference releases: the device of EGL and
        // the reference to the device of Direct3D.
        let angle_device = Rc::new(Cell::new(0isize));
        let device_slot = Rc::new(RefCell::new(Some(d3d_device.clone())));
        let cleanup: Rc<dyn Fn()> = {
            let (egl, angle_device, device_slot) = (egl.clone(), angle_device.clone(), device_slot.clone());
            Rc::new(move || {
                let device = angle_device.replace(0);
                if device != 0 {
                    egl.release_device_angle(device);
                }
                let d3d_device = device_slot.borrow_mut().take();
                drop(d3d_device);
            })
        };

        let mut display = 0;
        let result = (|| {
            angle_device.set(egl.create_device_angle(EGL_D3D11_DEVICE_ANGLE, p_d3d_device, None));
            if angle_device.get() == 0 {
                return Err(OpenGlException::get_formatted_exception_for_egl("eglCreateDeviceANGLE", egl.egl()));
            }

            display = egl.egl().get_platform_display_ext(EGL_PLATFORM_DEVICE_EXT, angle_device.get(), None);
            if display == 0 {
                return Err(OpenGlException::get_formatted_exception_for_egl("eglGetPlatformDisplayEXT", egl.egl()));
            }

            let device_lost_check: Rc<dyn Fn() -> bool> = Rc::new(move || d3d_device.get_device_removed_reason() != 0);
            Self::new(
                display,
                egl.egl(),
                Self::get_display_options(egl.egl(), gl_versions, Some(device_lost_check), Some(cleanup.clone())),
                PlatformApi::DirectX11,
            )
        })();

        match result {
            Ok(rv) => Ok((rv, chosen_luid)),
            Err(error) => {
                if display != 0 {
                    egl.egl().terminate(display);
                }
                cleanup();
                Err(error)
            }
        }
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

    unsafe extern "system" fn query_api() -> i32 {
        EGL_OPENGL_ES_API
    }

    unsafe extern "system" fn create_pbuffer_surface(_display: isize, _config: isize, _attrs: *const i32) -> isize {
        55
    }

    /// This EGL creates no context.
    unsafe extern "system" fn create_context(_display: isize, _config: isize, _share: isize, _attrs: *const i32) -> isize {
        0
    }

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
            "eglQueryAPI" => query_api as *const c_void,
            "eglCreatePbufferSurface" => create_pbuffer_surface as *const c_void,
            "eglCreateContext" => create_context as *const c_void,
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

    fn adapter(name: &str) -> PlatformGraphicsDeviceAdapterDescription {
        PlatformGraphicsDeviceAdapterDescription { description: Some(name.to_string()), ..Default::default() }
    }

    #[test]
    fn the_first_adapter_is_the_default_choice() {
        let adapters = [adapter("nvidia geforce"), adapter("microsoft basic render driver")];

        assert_eq!(Ok((0, false)), choose_adapter_index(&adapters, None, false));
        assert_eq!(Ok((0, false)), choose_adapter_index(&adapters, None, true));
        assert_eq!(
            Err(OpenGlException::new("No adapters found")),
            choose_adapter_index(&[], None, true)
        );
    }

    #[test]
    fn the_default_choice_moves_away_from_an_adreno_adapter_on_arm64() {
        let adapters = [adapter("qualcomm(r) adreno(tm) 690 gpu"), adapter("microsoft basic render driver")];

        assert_eq!(Ok((1, true)), choose_adapter_index(&adapters, None, true));
        // Not on other architectures, not when it is the only adapter, and
        // not when every adapter is one.
        assert_eq!(Ok((0, false)), choose_adapter_index(&adapters, None, false));
        assert_eq!(Ok((0, false)), choose_adapter_index(&adapters[..1], None, true));
        assert_eq!(Ok((0, false)), choose_adapter_index(&[adapters[0].clone(), adapters[0].clone()], None, true));
    }

    #[test]
    fn the_callback_chooses_among_all_adapters_and_may_choose_adreno() {
        let adapters = [adapter("qualcomm(r) adreno(tm) 690 gpu"), adapter("microsoft basic render driver")];
        let seen = Rc::new(Cell::new(0));
        let s = seen.clone();
        let adreno: GraphicsAdapterSelectionCallback = Rc::new(move |adapters| {
            s.set(adapters.len());
            0
        });
        let second: GraphicsAdapterSelectionCallback = Rc::new(|_| 1);
        let outside: GraphicsAdapterSelectionCallback = Rc::new(|_| 2);
        let negative: GraphicsAdapterSelectionCallback = Rc::new(|_| -1);

        assert_eq!(Ok((0, false)), choose_adapter_index(&adapters, Some(&adreno), true));
        assert_eq!(2, seen.get());
        assert_eq!(Ok((1, false)), choose_adapter_index(&adapters, Some(&second), true));
        assert!(choose_adapter_index(&adapters, Some(&outside), true).is_err());
        assert!(choose_adapter_index(&adapters, Some(&negative), false).is_err());
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
