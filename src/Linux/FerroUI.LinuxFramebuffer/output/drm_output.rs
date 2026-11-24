//! A KMS output: a connector of a card driven by a CRTC, drawn with OpenGL
//! through EGL on a GBM surface, a frame presented with a page flip (the
//! port of `Output/DrmOutput.cs`).
//!
//! Where the work happens (docs/porting/linux-framebuffer-platform.md,
//! section 6). The reference creates everything in its constructor on the
//! UI thread, sets the mode there and hands the one context to the
//! renderer as a shared context. An EGL display and its context are
//! objects of one thread in the port (`Rc`), and the compositor renders on
//! its render thread. So the constructor (on the thread that creates the
//! output) does what is plain handles: the card, the connector, the mode,
//! the CRTC, the GBM device and surface. The EGL half (the display on the
//! GBM device, the window surface, the context, the first frame, the mode
//! set with it and the initial buffer swapping) is done, in the reference's order, on the thread that first
//! asks the platform graphics for the shared context: the render thread.
//! The platform graphics still use one shared context, as the reference.

use super::drm_bindings::{DrmConnector, DrmModeInfo, DrmResources};
use super::drm::DrmModeConnection;
use crate::DrmOutputOptions;
use ferroui_base::platform::SurfaceOrientation;
use ferroui_base::PixelSize;
use std::io;

#[cfg(target_os = "linux")]
pub use imp::DrmOutput;

/// The connected connector the options select: of their connector type
/// and type identifier when they name them (the part of the constructor
/// over a card that chooses the connector).
pub(crate) fn select_connector<'a>(
    connectors: &'a [DrmConnector],
    options: Option<&DrmOutputOptions>,
) -> Option<&'a DrmConnector> {
    connectors
        .iter()
        .filter(|c| options.and_then(|options| options.connector_type).is_none_or(|t| c.connector_type() == t))
        .filter(|c| options.and_then(|options| options.connector_type_id).is_none_or(|id| c.connector_type_id() == id))
        .find(|x| x.connection() == DrmModeConnection::DRM_MODE_CONNECTED)
}

/// The mode of the options' video mode when the connector has it, else the
/// preferred mode, else the largest (the part of the constructor over a
/// card that chooses the mode). Among equals the first in the order of the
/// connector, as the stable ordering of the reference.
pub(crate) fn select_mode<'a>(connector: &'a DrmConnector, options: Option<&DrmOutputOptions>) -> Option<&'a DrmModeInfo> {
    let mut mode = None;

    if let Some(video_mode) = options.and_then(|options| options.video_mode) {
        mode = connector
            .modes()
            .iter()
            .find(|x| x.resolution().width == video_mode.width && x.resolution().height == video_mode.height);
    }

    mode.or_else(|| {
        let mut modes: Vec<&DrmModeInfo> = connector.modes().iter().collect();
        modes.sort_by(|a, b| {
            let area = |x: &DrmModeInfo| x.resolution().width as i64 * x.resolution().height as i64;
            b.is_preferred().cmp(&a.is_preferred()).then_with(|| area(b).cmp(&area(a)))
        });
        //.OrderByDescending(x => x.Resolution.Width * x.Resolution.Height)
        modes.first().copied()
    })
}

/// The CRTC for the connector (`GetCrtc` of `Init`): the one its current
/// encoder drives, else the first possible CRTC of the first of its
/// encoders that has one.
pub(crate) fn get_crtc(resources: &DrmResources, connector: &DrmConnector) -> io::Result<u32> {
    if let Some(encoder) = resources.encoders().get(&connector.encoder_id()) {
        // Not sure why that should work
        return Ok(encoder.encoder().crtc_id);
    }
    for enc_id in connector.encoder_ids() {
        if let Some(encoder) = resources.encoders().get(enc_id) {
            if let Some(crtc) = encoder.possible_crtcs().first() {
                return Ok(crtc.crtc_id);
            }
        }
    }

    Err(io::Error::other("Unable to find CRTC matching the desired mode"))
}

/// The size of the scene for an orientation: width and height exchanged
/// for a quarter turn.
pub(crate) fn rotated_size(orientation: SurfaceOrientation, resolution: PixelSize) -> PixelSize {
    if orientation == SurfaceOrientation::Rotation90 || orientation == SurfaceOrientation::Rotation270 {
        PixelSize::new(resolution.height, resolution.width)
    } else {
        resolution
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::super::drm::lib_drm::{
        drmModeModeInfo, gbm_color_formats, DrmEventContext, DrmModePageFlip, GbmBoFlags, LibDrm,
    };
    use super::super::drm_bindings::{DrmCard, DrmConnector, DrmModeInfo, DrmResources};
    use super::super::{IGlOutputBackend, IOutputBackend};
    use super::{get_crtc, rotated_size, select_connector, select_mode};
    use crate::DrmOutputOptions;
    use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
    use ferroui_base::platform::{
        IPartialPresentationFeature, IPlatformGraphics, IPlatformGraphicsContext, ISurfaceOrientation,
        PlatformRenderTargetState, RenderTargetProperties, RenderTargetSceneInfo, SurfaceOrientation,
    };
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::utilities::ThreadBound;
    use ferroui_base::PixelSize;
    use ferroui_opengl::egl::egl_consts::EGL_NONE;
    use ferroui_opengl::egl::{
        EglContext, EglDisplay, EglDisplayCreationOptions, EglDisplayOptions, EglInterface, EglPartialPresentation,
        EglPartialSession, EglSurface,
    };
    use ferroui_opengl::gl_consts::*;
    use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
    use ferroui_opengl::{GetProcAddress, IGlContext, OpenGlError};
    use std::any::{Any, TypeId};
    use std::cell::Cell;
    use std::ffi::{c_char, c_int, c_uint, c_void, CString};
    use std::io;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

    /// `EGL_PLATFORM_GBM_KHR`.
    const EGL_PLATFORM_GBM_KHR: i32 = 0x31D7;

    /// A pointer of `libgbm` (a device, a surface), shared by the thread
    /// that creates the output and the thread that renders it.
    #[derive(Clone, Copy, Debug)]
    struct GbmHandle(*mut c_void);

    // SAFETY: GBM objects are not tied to a thread; the device and the surface are created by
    // the constructor and then used by the thread that renders, one thread at a time.
    unsafe impl Send for GbmHandle {}
    // SAFETY: see `Send`; nothing calls into the objects from two threads at once.
    unsafe impl Sync for GbmHandle {}

    type EglGetProcAddress = unsafe extern "C" fn(name: *const c_char) -> *const c_void;

    /// `libEGL.so.1` and its `eglGetProcAddress`, opened once for the
    /// process.
    #[derive(Clone, Copy)]
    struct EglLibrary {
        library: usize,
        get_proc_address: EglGetProcAddress,
    }

    fn egl_library() -> io::Result<EglLibrary> {
        static LIBRARY: OnceLock<Option<EglLibrary>> = OnceLock::new();
        LIBRARY
            .get_or_init(|| {
                // SAFETY: the names are terminated strings; the library stays open for the life
                // of the process.
                unsafe {
                    let library = libc::dlopen(c"libEGL.so.1".as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
                    if library.is_null() {
                        return None;
                    }
                    let get_proc_address = libc::dlsym(library, c"eglGetProcAddress".as_ptr());
                    if get_proc_address.is_null() {
                        return None;
                    }
                    Some(EglLibrary {
                        library: library as usize,
                        // The address is the function of that name, of the declared signature.
                        get_proc_address: std::mem::transmute::<*mut c_void, EglGetProcAddress>(get_proc_address),
                    })
                }
            })
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Unable to load shared library 'libEGL.so.1' or one of its dependencies.")
            })
    }

    /// `eglGetProcAddress` of `libEGL.so.1`, the loader of the EGL
    /// interface. An entry point the library exports is taken from it
    /// first, so that the core functions do not depend on the library
    /// answering for them through `eglGetProcAddress`.
    fn egl_get_proc_address() -> io::Result<GetProcAddress> {
        let library = egl_library()?;
        Ok(Rc::new(move |name: &str| -> *const c_void {
            let Ok(name) = CString::new(name) else {
                return std::ptr::null();
            };
            // SAFETY: the library is open and the name terminated; `eglGetProcAddress` takes a
            // terminated name and returns an address or null.
            unsafe {
                let address = libc::dlsym(library.library as *mut c_void, name.as_ptr());
                if !address.is_null() {
                    return address.cast_const();
                }
                (library.get_proc_address)(name.as_ptr())
            }
        }))
    }

    /// What `gbm_bo_set_user_data` keeps with a buffer object: the DRM
    /// framebuffer made of it, and the output whose card removes it when
    /// the buffer object is destroyed.
    struct FbUserData {
        output: Weak<Inner>,
        fb_id: u32,
    }

    /// `FbDestroyCallback`: removes the framebuffer of a buffer object
    /// that is being destroyed.
    unsafe extern "C" fn fb_destroy_callback(_bo: *mut c_void, user_data: *mut c_void) {
        // SAFETY: the user data is the box `get_fb_id_for_bo` leaked for this buffer object, and
        // the library calls this once, when the object is destroyed.
        let data = unsafe { Box::from_raw(user_data.cast::<FbUserData>()) };
        if let Some(output) = data.output.upgrade() {
            // SAFETY: a call with the card's descriptor (or -1 once it is closed) and a number.
            unsafe {
                output.lib.drm_mode_rm_fb(output.card.fd(), data.fb_id);
            }
        }
    }

    /// The handler of the page flip event: the frame is on the screen.
    unsafe extern "C" fn flip_cb(_fd: c_int, _sequence: c_uint, _tv_sec: c_uint, _tv_usec: c_uint, user_data: *mut c_void) {
        // SAFETY: the user data of a flip is the flag of its output, allocated for the life of the
        // process.
        unsafe { (*user_data.cast::<Cell<bool>>()).set(false) };
    }

    /// What the output and its platform graphics share.
    struct Inner {
        this: Weak<Inner>,
        output_options: Mutex<DrmOutputOptions>,
        card: DrmCard,
        lib: &'static LibDrm,
        mode: drmModeModeInfo,
        connector_id: u32,
        crtc_id: u32,
        gbm_device: GbmHandle,
        gbm_target_surface: GbmHandle,
        /// The EGL half, created by the thread that renders.
        egl: OnceLock<ThreadBound<Rc<EglState>>>,
    }

    /// The objects of the thread that renders.
    // The display and the buffer of the quad are kept as the reference keeps them, though
    // nothing reads them again (the context holds the display as well).
    #[allow(dead_code)]
    struct EglState {
        egl_display: Rc<EglDisplay>,
        egl_surface: Rc<EglSurface>,
        deferred_context: Rc<EglContext>,
        current_bo: Cell<*mut c_void>,
        /// The buffer of the flip that is queued, shown after the next vblank.
        pending_bo: Cell<*mut c_void>,
        /// Whether a flip of this output is queued. Allocated for the life of the process: a page
        /// flip event can arrive through another output that shares the descriptor of the card.
        flip_pending: &'static Cell<bool>,
    }

    impl Inner {
        /// Waits for the flip that is queued, if any, and releases the buffer it replaced.
        fn wait_for_pending_flip(&self, egl: &EglState) {
            if egl.pending_bo.get().is_null() {
                return;
            }

            let fd = self.card.fd();
            let mut ctx = DrmEventContext { version: 4, page_flip_handler: Some(flip_cb), ..Default::default() };
            while egl.flip_pending.get() {
                let mut pfd = libc::pollfd { events: libc::POLLIN, fd, revents: 0 };
                // SAFETY: one descriptor, in a structure alive for the call; the context and its
                // handler are alive for the call.
                unsafe {
                    libc::poll(&mut pfd, 1, -1);
                    self.lib.drm_handle_event(fd, &mut ctx);
                }
            }

            // SAFETY: the buffer that was shown until the flip, locked before and released once.
            unsafe { self.lib.gbm_surface_release_buffer(self.gbm_target_surface.0, egl.current_bo.get()) };
            egl.current_bo.set(egl.pending_bo.replace(std::ptr::null_mut()));
        }

        fn options(&self) -> std::sync::MutexGuard<'_, DrmOutputOptions> {
            self.output_options.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn orientation(&self) -> SurfaceOrientation {
            self.options().orientation
        }

        fn scaling(&self) -> f64 {
            self.options().scaling
        }

        fn resolution(&self) -> PixelSize {
            self.mode.resolution()
        }

        /// `GetFbIdForBo`: the DRM framebuffer of a buffer object, made
        /// once and kept with it.
        fn get_fb_id_for_bo(&self, bo: *mut c_void) -> io::Result<u32> {
            if bo.is_null() {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "bo is 0"));
            }
            let gbm = self.lib;
            let drm = self.lib;
            // SAFETY: `bo` is a buffer object of the output's surface, locked by the caller; the
            // user data is null or the box this function attached.
            unsafe {
                let data = gbm.gbm_bo_get_user_data(bo);
                if !data.is_null() {
                    return Ok((*data.cast::<FbUserData>()).fb_id);
                }

                let w = gbm.gbm_bo_get_width(bo);
                let h = gbm.gbm_bo_get_height(bo);
                let stride = gbm.gbm_bo_get_stride(bo);
                let handle = gbm.gbm_bo_get_handle(bo).u32;
                let format = gbm.gbm_bo_get_format(bo);

                // prepare for the new ioctl call
                let handles = [handle, 0, 0, 0];
                let pitches = [stride, 0, 0, 0];
                let offsets = [0u32; 4];

                let mut fb_handle = 0u32;
                let mut ret = drm.drm_mode_add_fb2(
                    self.card.fd(),
                    w,
                    h,
                    format,
                    &handles,
                    &pitches,
                    &offsets,
                    &mut fb_handle,
                    0,
                );

                if ret != 0 {
                    // legacy fallback
                    ret = drm.drm_mode_add_fb(self.card.fd(), w, h, 24, 32, stride, handle, &mut fb_handle);

                    if ret != 0 {
                        return Err(io::Error::other(format!("drmModeAddFb failed {ret}")));
                    }
                }

                let data = Box::into_raw(Box::new(FbUserData { output: self.this.clone(), fb_id: fb_handle }));
                gbm.gbm_bo_set_user_data(bo, data.cast(), Some(fb_destroy_callback));

                Ok(fb_handle)
            }
        }

        /// The objects of the calling thread; created on the first call
        /// (the EGL part of `Init`).
        ///
        /// # Panics
        /// When the EGL part cannot be created (the exception of the
        /// reference's constructor), and on a thread other than the one
        /// that created it.
        fn egl_state(&self) -> Rc<EglState> {
            if let Some(state) = self.egl.get() {
                return state.get().clone();
            }
            let state = match self.init_egl() {
                Ok(state) => state,
                Err(error) => panic!("{error}"),
            };
            if self.egl.set(ThreadBound::new(state.clone())).is_err() {
                panic!("The EGL objects of the DRM output were created by two threads");
            }
            state
        }

        /// The part of `Init` that creates the EGL objects, sets the mode
        /// with the first frame and swaps the buffers twice, in the
        /// reference's order.
        fn init_egl(&self) -> Result<Rc<EglState>, OpenGlError> {
            let get_proc_address = egl_get_proc_address().map_err(|error| OpenGlError::new(error.to_string()))?;
            // SAFETY: the loader returns null or the address of the entry point of that name of
            // the EGL library, which stays loaded.
            let egl = Rc::new(unsafe { EglInterface::try_new(&get_proc_address) }?);
            let egl_display = EglDisplay::new_with_creation_options(EglDisplayCreationOptions {
                base: EglDisplayOptions {
                    egl: Some(egl),
                    supports_multiple_contexts: true,
                    supports_context_sharing: true,
                    ..Default::default()
                },
                platform_type: Some(EGL_PLATFORM_GBM_KHR),
                platform_display: self.gbm_device.0 as isize,
                ..Default::default()
            })?;

            let surface = egl_display.egl_interface().create_window_surface(
                egl_display.handle(),
                egl_display.config(),
                self.gbm_target_surface.0 as isize,
                Some(&[EGL_NONE, EGL_NONE]),
            );
            if surface == 0 {
                return Err(OpenGlError::get_formatted_exception_for_egl(
                    "eglCreateWindowSurface",
                    egl_display.egl_interface(),
                ));
            }

            let egl_surface = EglSurface::new(&egl_display, surface);

            let deferred_context = egl_display.create_context(None)?;

            let options = self.options().clone();
            let color = options.initial_buffer_swapping_color;
            let initial_buffer_swapping_color_r = color.r as f32 / 255.0;
            let initial_buffer_swapping_color_g = color.g as f32 / 255.0;
            let initial_buffer_swapping_color_b = color.b as f32 / 255.0;
            let initial_buffer_swapping_color_a = color.a as f32 / 255.0;
            let make_current = || {
                deferred_context
                    .make_current_with_surface(Some(&egl_surface))
                    .map_err(|error| OpenGlError::new(error.to_string()))
            };
            {
                let current = make_current()?;
                let gl = deferred_context.gl_interface();
                gl.clear_color(
                    initial_buffer_swapping_color_r,
                    initial_buffer_swapping_color_g,
                    initial_buffer_swapping_color_b,
                    initial_buffer_swapping_color_a,
                );
                gl.clear(GL_COLOR_BUFFER_BIT | GL_STENCIL_BUFFER_BIT);
                egl_surface.swap_buffers();
                current.dispose();
            }

            // SAFETY: the surface of the output, whose buffers were just swapped.
            let bo = unsafe { self.lib.gbm_surface_lock_front_buffer(self.gbm_target_surface.0) };
            let fb_id = self.get_fb_id_for_bo(bo).map_err(|error| OpenGlError::new(error.to_string()))?;
            let mut connector_id = self.connector_id;
            let mut mode = self.mode;

            // SAFETY: the card's descriptor, one connector and the mode, all alive for the call.
            let res =
                unsafe { self.lib.drm_mode_set_crtc(self.card.fd(), self.crtc_id, fb_id, 0, 0, &mut connector_id, 1, &mut mode) };
            if res != 0 {
                return Err(OpenGlError::new(format!(
                    "drmModeSetCrtc failed: {} ({res})",
                    io::Error::last_os_error()
                )));
            }

            let current_bo = Cell::new(bo);

            let state = Rc::new(EglState {
                egl_display,
                egl_surface,
                deferred_context,
                current_bo,
                pending_bo: Cell::new(std::ptr::null_mut()),
                flip_pending: Box::leak(Box::new(Cell::new(false))),
            });

            if options.enable_initial_buffer_swapping {
                //Go through two cycles of buffer swapping (there are render artifacts otherwise)
                let scene_info =
                    RenderTargetSceneInfo::new(self.pixel_size(), 1.0, CompositionTransparencyLevel::None);
                for _ in 0..2 {
                    // The swaps of the start present whole frames.
                    let target = RenderTarget {
                        parent: self.this(),
                        egl: state.clone(),
                        partial_presentation: None,
                        corrupted: Rc::new(Cell::new(false)),
                    };
                    let session = target.begin_draw(&scene_info);
                    let gl = state.deferred_context.gl_interface();
                    gl.clear_color(
                        initial_buffer_swapping_color_r,
                        initial_buffer_swapping_color_g,
                        initial_buffer_swapping_color_b,
                        initial_buffer_swapping_color_a,
                    );
                    gl.clear(GL_COLOR_BUFFER_BIT | GL_STENCIL_BUFFER_BIT);
                    session.dispose();
                }
            }

            Ok(state)
        }

        fn this(&self) -> Arc<Inner> {
            self.this.upgrade().expect("an output that is called is alive")
        }

        /// The size of the scene: the resolution of the screen, its sides exchanged for a
        /// quarter turn. The compositor turns the scene into the framebuffer
        /// (`RenderTarget::properties`).
        fn pixel_size(&self) -> PixelSize {
            rotated_size(self.orientation(), self.resolution())
        }
    }

    /// The platform graphics of the output: the one context of the output,
    /// shared with the renderer.
    ///
    /// The reference constructs it with the context. Here the context is
    /// an object of the thread that renders, created when that thread
    /// first asks for it.
    struct SharedContextGraphics {
        inner: Arc<Inner>,
    }

    impl IPlatformGraphics for SharedContextGraphics {
        fn uses_shared_context(&self) -> bool {
            true
        }

        /// # Panics
        /// Always, as the reference (`NotSupportedException`).
        fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            panic!("Specified method is not supported.")
        }

        /// # Panics
        /// When the EGL objects of the output cannot be created, and on a
        /// thread other than the one that first asked.
        fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            self.inner.egl_state().deferred_context.clone()
        }
    }

    /// A KMS output: a connector of a card, drawn with OpenGL.
    pub struct DrmOutput {
        inner: Arc<Inner>,
        platform_graphics: Arc<SharedContextGraphics>,
    }

    impl DrmOutput {
        /// The output of a connector and a mode of the card that are
        /// already chosen, with options.
        pub fn new_with_mode_and_options(
            card: DrmCard,
            resources: &DrmResources,
            connector: &DrmConnector,
            mode_info: &DrmModeInfo,
            options: Option<DrmOutputOptions>,
        ) -> io::Result<Arc<Self>> {
            Self::init(card, resources, connector, mode_info, options.unwrap_or_default())
        }

        /// Opens the card `path` (or the first usable card) and drives its
        /// first connected connector (of the options' connector type and
        /// type identifier) in the options' video mode or its preferred
        /// mode.
        pub fn new(path: Option<&str>, connectors_force_probe: bool, options: Option<DrmOutputOptions>) -> io::Result<Arc<Self>> {
            Self::from_card(DrmCard::new(path)?, connectors_force_probe, options)
        }

        /// As [`new`](Self::new), on a card that is open.
        pub fn from_card(card: DrmCard, connectors_force_probe: bool, options: Option<DrmOutputOptions>) -> io::Result<Arc<Self>> {
            let resources = card.get_resources(connectors_force_probe)?;

            let Some(connector) = select_connector(resources.connectors(), options.as_ref()) else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "Unable to find connected DRM connector"));
            };

            let Some(mode) = select_mode(connector, options.as_ref()) else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "Unable to find a usable DRM mode"));
            };
            Self::init(card, &resources, connector, mode, options.unwrap_or_default())
        }

        /// The output of a connector and a mode of the card that are
        /// already chosen, with the default options.
        pub fn new_with_mode(
            card: DrmCard,
            resources: &DrmResources,
            connector: &DrmConnector,
            mode_info: &DrmModeInfo,
        ) -> io::Result<Arc<Self>> {
            Self::init(card, resources, connector, mode_info, DrmOutputOptions::default())
        }

        /// `Init`: the part on the thread that creates the output (the
        /// CRTC, the GBM device and surface); the EGL part follows on the
        /// thread that renders (`Inner::init_egl`).
        fn init(
            card: DrmCard,
            resources: &DrmResources,
            connector: &DrmConnector,
            mode_info: &DrmModeInfo,
            options: DrmOutputOptions,
        ) -> io::Result<Arc<Self>> {
            let lib = LibDrm::get()?;
            // The EGL library is opened here, so that a machine without it
            // fails when the output is created, as in the reference.
            egl_library()?;

            let crtc_id = get_crtc(resources, connector)?;
            // SAFETY: a call with the card's descriptor; the result is a device or null.
            let device = unsafe { lib.gbm_create_device(card.fd()) };
            if device.is_null() {
                return Err(io::Error::other("Unable to create GBM device"));
            }
            let resolution = mode_info.resolution();
            // SAFETY: the device just created, a size and the flags of a scan-out buffer.
            let gbm_target_surface = unsafe {
                lib.gbm_surface_create(
                    device,
                    resolution.width as u32,
                    resolution.height as u32,
                    gbm_color_formats::GBM_FORMAT_XRGB8888,
                    GbmBoFlags::GBM_BO_USE_SCANOUT | GbmBoFlags::GBM_BO_USE_RENDERING,
                )
            };
            if gbm_target_surface.is_null() {
                return Err(io::Error::other("Unable to create GBM surface"));
            }

            let inner = Arc::new_cyclic(|this| Inner {
                this: this.clone(),
                output_options: Mutex::new(options),
                card,
                lib,
                mode: mode_info.mode,
                connector_id: connector.id(),
                crtc_id,
                gbm_device: GbmHandle(device),
                gbm_target_surface: GbmHandle(gbm_target_surface),
                egl: OnceLock::new(),
            });
            let platform_graphics = Arc::new(SharedContextGraphics { inner: inner.clone() });
            Ok(Arc::new(Self { inner, platform_graphics }))
        }

        pub fn set_orientation(&self, value: SurfaceOrientation) {
            self.inner.options().orientation = value;
        }

        /// The card the output drives. An addition, for reading back what
        /// the card shows (`drm_scanout`).
        pub fn card(&self) -> &DrmCard {
            &self.inner.card
        }

        /// The CRTC that shows the output. An addition, as
        /// [`card`](Self::card).
        pub fn crtc_id(&self) -> u32 {
            self.inner.crtc_id
        }

        /// The render target over the output's context
        /// (`CreateGlRenderTarget()`).
        ///
        /// # Panics
        /// As [`IPlatformGraphics::get_shared_context`] of the output's
        /// platform graphics: the call creates the EGL objects when this
        /// is the first.
        pub fn create_gl_render_target_of_primary_context(&self) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
            let egl = self.inner.egl_state();
            // Partial presentation (upstream 070c3be990): per render target, so a target that
            // failed and is made again starts from a whole repaint.
            let use_partial_presentation = self.inner.options().use_partial_presentation.unwrap_or(false)
                && egl.egl_display.partial_presentation_support().buffer_age;
            let corrupted = Rc::new(Cell::new(false));
            let partial_presentation = use_partial_presentation.then(|| {
                let display = &egl.egl_display;
                EglPartialPresentation::new(
                    display.egl_interface().clone(),
                    display.handle(),
                    display.partial_presentation_support(),
                    corrupted.clone(),
                )
            });
            Rc::new(RenderTarget { parent: self.inner.clone(), egl, partial_presentation, corrupted })
        }

        /// # Panics
        /// Always, as the reference (`NotImplementedException`).
        pub fn create_context(&self) -> Rc<dyn IGlContext> {
            panic!("The method or operation is not implemented.")
        }
    }

    impl IPlatformRenderSurface for DrmOutput {
        fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
            if kind == TypeId::of::<dyn IGlPlatformSurface>() {
                // The view is a handle of the calling thread over the same output.
                let this: Rc<dyn IGlPlatformSurface> =
                    Rc::new(Self { inner: self.inner.clone(), platform_graphics: self.platform_graphics.clone() });
                return Some(Rc::new(this));
            }
            None
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl IGlPlatformSurface for DrmOutput {
        /// # Panics
        /// When `context` is not the context of the output (the exception
        /// of the reference).
        fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
            let egl = self.inner.egl_state();
            let is_primary = context
                .as_any()
                .downcast_ref::<EglContext>()
                .is_some_and(|context| std::ptr::eq(context, Rc::as_ptr(&egl.deferred_context)));
            if !is_primary {
                panic!("This platform backend can only create render targets for its primary context");
            }
            self.create_gl_render_target_of_primary_context()
        }
    }

    impl IOutputBackend for DrmOutput {
        fn pixel_size(&self) -> PixelSize {
            self.inner.pixel_size()
        }

        fn scaling(&self) -> f64 {
            self.inner.scaling()
        }

        fn set_scaling(&self, value: f64) {
            self.inner.options().scaling = value;
        }

        fn as_render_surface(self: Arc<Self>) -> Arc<dyn IPlatformRenderSurface> {
            self
        }

        fn as_gl_output_backend(&self) -> Option<&dyn IGlOutputBackend> {
            Some(self)
        }

        fn as_surface_orientation(&self) -> Option<&dyn ISurfaceOrientation> {
            Some(self)
        }
    }

    impl IGlOutputBackend for DrmOutput {
        fn platform_graphics(&self) -> Arc<dyn IPlatformGraphics> {
            self.platform_graphics.clone()
        }
    }

    impl ISurfaceOrientation for DrmOutput {
        fn orientation(&self) -> SurfaceOrientation {
            self.inner.orientation()
        }
    }

    struct RenderTarget {
        parent: Arc<Inner>,
        egl: Rc<EglState>,
        partial_presentation: Option<Rc<EglPartialPresentation>>,
        /// Set when a declared region or a swap of a partial frame failed.
        corrupted: Rc<Cell<bool>>,
    }

    impl IPlatformRenderSurfaceRenderTarget for RenderTarget {
        fn state(&self) -> PlatformRenderTargetState {
            if self.corrupted.get() {
                PlatformRenderTargetState::CORRUPTED
            } else {
                PlatformRenderTargetState::READY
            }
        }
    }

    impl IGlPlatformSurfaceRenderTarget for RenderTarget {
        /// # Panics
        /// When the context cannot be made current with the surface (the
        /// exception of the reference).
        fn begin_draw(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
            let clear_context = match self.egl.deferred_context.make_current_with_surface(Some(&self.egl.egl_surface)) {
                Ok(clear_context) => clear_context,
                Err(error) => panic!("{error}"),
            };
            let gl = self.egl.deferred_context.gl_interface();

            // The framebuffer of the screen: a rotation is drawn by the compositor (upstream
            // 9b3e9c881b), which turns its layer into it.
            gl.bind_framebuffer(GL_FRAMEBUFFER, 0);
            let resolution = self.parent.resolution();
            gl.viewport(0, 0, resolution.width, resolution.height);

            // The rectangles of partial presentation are in the pixels of the framebuffer, the
            // compositor rotates them with the scene.
            let partial = self.partial_presentation.as_ref().and_then(|partial| {
                match partial.begin_draw(self.egl.egl_surface.dangerous_get_handle(), resolution, false) {
                    Ok(session) => Some(session),
                    Err(error) => {
                        partial.invalidate();
                        println!("Partial presentation is not used for the frame: {}", error.message());
                        None
                    }
                }
            });

            Rc::new(RenderSession {
                parent: self.parent.clone(),
                egl: self.egl.clone(),
                clear_context,
                disposed: Cell::new(false),
                partial,
            })
        }

        /// The rotation of the screen: the compositor draws the scene into a layer and turns
        /// it into the framebuffer, overlays included (upstream 9b3e9c881b replaced the
        /// rotation framebuffer and shaders of the output with it).
        fn properties(&self) -> RenderTargetProperties {
            // Direct rendering needs a stencil buffer (the clip of several rectangles) and a
            // driver that gives back buffers with contents.
            let supports_partial_presentation = self.partial_presentation.is_some();
            RenderTargetProperties {
                supports_partial_presentation,
                is_suitable_for_direct_rendering: supports_partial_presentation
                    && self.egl.deferred_context.stencil_size() >= 8
                    && !self.partial_presentation.as_ref().is_some_and(|partial| partial.prefer_retained_layer()),
                required_rotation_for_orientation: self.parent.orientation(),
                ..RenderTargetProperties::default()
            }
        }

        fn dispose(&self) {
            // We are wrapping GBM buffer chain associated with CRTC, and don't free it on a whim
            if let Some(partial) = &self.partial_presentation {
                partial.invalidate();
            }
        }
    }

    struct RenderSession {
        parent: Arc<Inner>,
        egl: Rc<EglState>,
        clear_context: Rc<dyn IDisposable>,
        disposed: Cell<bool>,
        partial: Option<Rc<EglPartialSession>>,
    }

    impl RenderSession {
        /// Swaps the buffers of the frame; `false` when it was not presented (a partial frame
        /// that did not succeed, or whose swap failed), so nothing is flipped.
        fn swap(&self) -> bool {
            match &self.partial {
                Some(partial) if partial.is_taken() => {
                    if !partial.can_present() {
                        partial.cancel();
                        return false;
                    }
                    match partial.present() {
                        Ok(presented) => presented,
                        Err(error) => {
                            println!("{}", error.message());
                            false
                        }
                    }
                }
                Some(untaken) => {
                    untaken.present_whole(|| self.egl.egl_surface.swap_buffers());
                    true
                }
                None => {
                    self.egl.egl_surface.swap_buffers();
                    true
                }
            }
        }
    }

    impl IGlPlatformSurfaceRenderingSession for RenderSession {
        fn context(&self) -> Rc<dyn IGlContext> {
            self.egl.deferred_context.clone()
        }

        /// The framebuffer of the screen, in its own orientation.
        fn size(&self) -> PixelSize {
            self.parent.resolution()
        }

        fn scaling(&self) -> f64 {
            self.parent.scaling()
        }

        fn is_y_flipped(&self) -> bool {
            false
        }

        fn partial_presentation(&self) -> Option<Rc<dyn IPartialPresentationFeature>> {
            let partial = self.partial.as_ref()?;
            partial.take();
            Some(partial.clone())
        }

        /// # Panics
        /// When the DRM framebuffer of the new front buffer cannot be made
        /// (the exception of the reference).
        fn dispose(&self) {
            // A frame is presented once: a second call would flip again.
            if self.disposed.replace(true) {
                return;
            }
            let parent = &self.parent;
            let egl = &self.egl;
            let gl = egl.deferred_context.gl_interface();

            gl.flush();
            if !self.swap() {
                self.clear_context.dispose();
                return;
            }

            let lib = parent.lib;
            let surface = parent.gbm_target_surface.0;
            // SAFETY: the surface of the output, whose buffers were just swapped.
            let next_bo = unsafe { lib.gbm_surface_lock_front_buffer(surface) };
            if next_bo.is_null() {
                // Not sure what else can be done
                println!("gbm_surface_lock_front_buffer failed");
            } else {
                let fb = match parent.get_fb_id_for_bo(next_bo) {
                    Ok(fb) => fb,
                    Err(error) => panic!("{error}"),
                };
                // Only the flip queued before is waited for, so the next frame can be rendered
                // while this one waits for vblank.
                parent.wait_for_pending_flip(&egl);

                let fd = parent.card.fd();
                egl.flip_pending.set(true);
                // SAFETY: the card's descriptor and numbers; the user data is the flag of this
                // output, which lives as long as the process.
                let ret = unsafe {
                    lib.drm_mode_page_flip(
                        fd,
                        parent.crtc_id,
                        fb,
                        DrmModePageFlip::EVENT,
                        (egl.flip_pending as *const Cell<bool>).cast_mut().cast(),
                    )
                };

                if ret == 0 {
                    egl.pending_bo.set(next_bo);

                    // No buffer is left to render the next frame into: wait for the flip now.
                    // SAFETY: the surface of the output.
                    if unsafe { lib.gbm_surface_has_free_buffers(surface) } == 0 {
                        parent.wait_for_pending_flip(&egl);
                    }
                } else {
                    egl.flip_pending.set(false);
                    println!("drmModePageFlip failed {ret}");
                    // SAFETY: the buffer object locked above, released once.
                    unsafe { lib.gbm_surface_release_buffer(surface, next_bo) };
                }
            }
            self.clear_context.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The
    // selection of the connector, the mode and the CRTC, from values.
    use super::super::drm::lib_drm::{drmModeCrtc, drmModeEncoder};
    use super::super::drm_bindings::tests::{connector, mode};
    use super::super::drm_bindings::DrmEncoder;
    use super::*;
    use crate::DrmConnectorType;
    use std::collections::HashMap;

    const CONNECTED: DrmModeConnection = DrmModeConnection::DRM_MODE_CONNECTED;
    const DISCONNECTED: DrmModeConnection = DrmModeConnection::DRM_MODE_DISCONNECTED;

    fn connectors() -> Vec<DrmConnector> {
        vec![
            connector(30, 11, 1, DISCONNECTED, 0, &[50], &[mode(1920, 1080, true)]),
            connector(31, 11, 2, CONNECTED, 0, &[50], &[mode(1280, 720, true), mode(1920, 1080, false)]),
            connector(32, 10, 1, CONNECTED, 51, &[51], &[mode(800, 600, false), mode(1024, 768, false), mode(640, 480, false)]),
        ]
    }

    #[test]
    fn the_first_connected_connector_is_chosen_unless_the_options_name_a_type() {
        let connectors = connectors();
        assert_eq!(select_connector(&connectors, None).map(DrmConnector::id), Some(31));

        let display_port = DrmOutputOptions { connector_type: Some(DrmConnectorType::DISPLAY_PORT), ..Default::default() };
        assert_eq!(select_connector(&connectors, Some(&display_port)).map(DrmConnector::id), Some(32));

        let second_hdmi = DrmOutputOptions {
            connector_type: Some(DrmConnectorType::HDMI_A),
            connector_type_id: Some(2),
            ..Default::default()
        };
        assert_eq!(select_connector(&connectors, Some(&second_hdmi)).map(DrmConnector::id), Some(31));

        // The first HDMI connector is not connected.
        let first_hdmi = DrmOutputOptions { connector_type_id: Some(1), connector_type: Some(DrmConnectorType::HDMI_A), ..Default::default() };
        assert!(select_connector(&connectors, Some(&first_hdmi)).is_none());
        let vga = DrmOutputOptions { connector_type: Some(DrmConnectorType::VGA), ..Default::default() };
        assert!(select_connector(&connectors, Some(&vga)).is_none());
    }

    #[test]
    fn the_preferred_mode_wins_then_the_largest_then_the_first() {
        let connectors = connectors();
        // Preferred over larger.
        assert_eq!(select_mode(&connectors[1], None).map(DrmModeInfo::resolution), Some(PixelSize::new(1280, 720)));
        // Without a preferred mode, the largest.
        assert_eq!(select_mode(&connectors[2], None).map(DrmModeInfo::resolution), Some(PixelSize::new(1024, 768)));

        // Of two modes with the same size the first.
        let mut first = mode(1024, 768, false);
        first.vrefresh = 60;
        let mut second = mode(1024, 768, false);
        second.vrefresh = 75;
        let same = connector(40, 15, 1, CONNECTED, 0, &[], &[mode(800, 600, false), first, second]);
        assert_eq!(select_mode(&same, None).map(|m| m.mode.vrefresh), Some(60));

        let empty = connector(41, 15, 1, CONNECTED, 0, &[], &[]);
        assert!(select_mode(&empty, None).is_none());
    }

    #[test]
    fn a_video_mode_of_the_options_is_used_when_the_connector_has_it() {
        let connectors = connectors();
        let full_hd = DrmOutputOptions { video_mode: Some(PixelSize::new(1920, 1080)), ..Default::default() };
        assert_eq!(select_mode(&connectors[1], Some(&full_hd)).map(DrmModeInfo::resolution), Some(PixelSize::new(1920, 1080)));
        // A mode the connector lacks falls back to the preferred one.
        let four_k = DrmOutputOptions { video_mode: Some(PixelSize::new(3840, 2160)), ..Default::default() };
        assert_eq!(select_mode(&connectors[1], Some(&four_k)).map(DrmModeInfo::resolution), Some(PixelSize::new(1280, 720)));
    }

    #[test]
    fn the_crtc_is_the_current_encoders_or_the_first_possible_one() {
        let crtcs: Vec<drmModeCrtc> = (0..2).map(|c| drmModeCrtc { crtc_id: 60 + c, ..Default::default() }).collect();
        let mut encoders = HashMap::new();
        encoders.insert(
            50,
            DrmEncoder::new(drmModeEncoder { encoder_id: 50, crtc_id: 0, possible_crtcs: 0b10, ..Default::default() }, &crtcs),
        );
        encoders.insert(
            51,
            DrmEncoder::new(drmModeEncoder { encoder_id: 51, crtc_id: 60, possible_crtcs: 0b01, ..Default::default() }, &crtcs),
        );
        let resources = DrmResources::from_parts(connectors(), encoders);

        // A connector with a current encoder: the CRTC of that encoder.
        assert_eq!(get_crtc(&resources, &resources.connectors()[2]).unwrap(), 60);
        // Without one: the first possible CRTC of its encoders.
        assert_eq!(get_crtc(&resources, &resources.connectors()[1]).unwrap(), 61);

        let lonely = connector(42, 15, 1, CONNECTED, 0, &[99], &[]);
        assert_eq!(
            get_crtc(&resources, &lonely).unwrap_err().to_string(),
            "Unable to find CRTC matching the desired mode"
        );
    }

    #[test]
    fn a_quarter_turn_exchanges_the_sides_of_the_scene() {
        let size = PixelSize::new(1024, 768);
        assert_eq!(rotated_size(SurfaceOrientation::Rotation0, size), size);
        assert_eq!(rotated_size(SurfaceOrientation::Rotation180, size), size);
        assert_eq!(rotated_size(SurfaceOrientation::Rotation90, size), PixelSize::new(768, 1024));
        assert_eq!(rotated_size(SurfaceOrientation::Rotation270, size), PixelSize::new(768, 1024));
    }
}
