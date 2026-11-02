//! EGL over a GBM device with buffers the backend allocates and hands to the
//! compositor as dma-bufs (the port of `WaylandEglDmaBufPlatformGraphics.cs`).
//! Opt-in through `use_dmabuf_swapchain` of the options; the default is
//! the window system integration of the driver
//! (`wayland_egl_wsi_platform_graphics.rs`).

use super::wayland_dmabuf_feedback::{DmabufFormatModifierPair, FeedbackListener, WaylandDmabufFeedback};
use super::wayland_egl_display::WaylandEglDisplay;
use super::wayland_egl_wsi_platform_graphics::load_egl_interface;
use crate::server::interop::drm_gbm_unsafe_native_methods::{DrmDevice, DrmGbmUnsafeNativeMethods};
use crate::server::interop::wayland_connection::WaylandConnection;
use crate::server::wayland_platform_graphics::{IWaylandGraphics, WaylandPlatformGraphics};
use crate::server::wayland_worker::WaylandWorkerState;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::IPlatformGraphicsContext;
use ferroui_base::reactive::IDisposable;
use ferroui_opengl::egl::egl_consts::{EGL_EXTENSIONS, EGL_PLATFORM_GBM_KHR};
use ferroui_opengl::egl::{EglDisplay, EglDisplayCreationOptions, EglDisplayOptions};
use ferroui_opengl::{GlVersion, OpenGlError};
use std::any::Any;
use std::cell::Cell;
use std::ffi::{c_void, CStr, CString};
use std::rc::Rc;
use wayland_client::Proxy;

/// The GPU backend of the dmabuf swapchain. An object of the worker thread, like the display
/// it holds.
pub struct WaylandEglDmaBufPlatformGraphics {
    display: Rc<WaylandEglDisplay>,
}

impl WaylandEglDmaBufPlatformGraphics {
    pub fn display(&self) -> &Rc<WaylandEglDisplay> {
        &self.display
    }

    fn new(display: Rc<WaylandEglDisplay>) -> Self {
        Self { display }
    }

    /// Whether the backend the platform graphics hold now is this one. On a thread other than
    /// the worker the platform graphics show no backend, and the answer is `false`.
    pub fn is_current(graphics: &WaylandPlatformGraphics) -> bool {
        graphics.current_graphics().is_some_and(|graphics| graphics.as_any().is::<Self>())
    }

    /// The graphics of a connection, when the swapchain can be set up; `None` when not, with
    /// the reason logged. The globals of the connection are those of `state`.
    pub fn try_create(
        connection: &mut WaylandConnection,
        state: &mut WaylandWorkerState,
        gl_profiles: &[GlVersion],
    ) -> Option<Rc<Self>> {
        match Self::create(connection, state, gl_profiles) {
            Ok(graphics) => graphics,
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log_with_values(None, "Unable to initialize Wayland EGL rendering: {0}", &[&error]);
                }
                None
            }
        }
    }

    fn warn(message: &str) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "OpenGL") {
            logger.log(None, message);
        }
    }

    // libEGL.so.1 only exports EGL 1.5 core entry points. Extension entry
    // points like eglGetPlatformDisplayEXT / eglQueryDmaBufFormatsEXT are
    // NOT exported as symbols and can only be resolved via eglGetProcAddress
    // (`load_egl_interface` of the WSI backend, shared).

    fn create(
        connection: &mut WaylandConnection,
        state: &mut WaylandWorkerState,
        gl_profiles: &[GlVersion],
    ) -> Result<Option<Rc<Self>>, OpenGlError> {
        let Some(dmabuf) = state.globals.as_ref().and_then(|globals| globals.linux_dmabuf.clone()) else {
            Self::warn("zwp_linux_dmabuf_v1 not available");
            return Ok(None);
        };

        // Request default feedback from the compositor
        let feedback = dmabuf.get_default_feedback(connection.queue_handle(), WaylandDmabufFeedback::new().listener());
        let roundtrip = connection.roundtrip(state);
        let (is_complete, main_device, feedback_formats) = match feedback.data::<FeedbackListener>() {
            Some(listener) => listener.with_feedback(|feedback| {
                (feedback.is_complete(), feedback.main_device(), Self::feedback_formats(feedback))
            }),
            None => (false, 0, Vec::new()),
        };
        // The feedback is read once; the reference leaves the object to the collector.
        feedback.destroy();
        roundtrip.map_err(|error| OpenGlError::new(error.to_string()))?;

        if !is_complete {
            Self::warn("Dmabuf feedback not received from compositor");
            return Ok(None);
        }

        let native = DrmGbmUnsafeNativeMethods::get().map_err(|error| OpenGlError::new(error.to_string()))?;

        // Find the DRM render node matching the compositor's main device
        let Some(render_node_path) = Self::find_render_node(native, main_device) else {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "OpenGL") {
                logger.log_with_values(None, "No DRM render node found for device {0}", &[&main_device]);
            }
            return Ok(None);
        };

        // Open the DRM render node and create GBM device
        let drm_fd = DrmGbmUnsafeNativeMethods::open(&render_node_path, DrmGbmUnsafeNativeMethods::O_RDWR);
        if drm_fd < 0 {
            return Err(OpenGlError::new(format!(
                "Failed to open DRM render node: {}",
                render_node_path.to_string_lossy()
            )));
        }

        let mut gbm_device: *mut c_void = std::ptr::null_mut();
        let mut display: Option<Rc<WaylandEglDisplay>> = None;
        let created = (|| -> Result<Rc<Self>, OpenGlError> {
            // SAFETY: the descriptor is open and outlives the device (both are released by the
            // dispose callback, the device first).
            gbm_device = unsafe { native.gbm_create_device(drm_fd) };
            if gbm_device.is_null() {
                return Err(OpenGlError::new("Failed to create GBM device"));
            }

            // Capture for the dispose callback, which runs once.
            let captured_drm_fd = drm_fd;
            let captured_gbm = gbm_device as usize;
            let disposed = Cell::new(false);

            let egl = load_egl_interface()?;
            let options = EglDisplayCreationOptions {
                base: EglDisplayOptions {
                    egl: Some(Rc::new(egl)),
                    supports_multiple_contexts: true,
                    supports_context_sharing: true,
                    dispose_callback: Some(Rc::new(move || {
                        if disposed.replace(true) {
                            return;
                        }
                        // SAFETY: the device and the descriptor of this display, released once.
                        unsafe {
                            native.gbm_device_destroy(captured_gbm as *mut c_void);
                            DrmGbmUnsafeNativeMethods::close(captured_drm_fd);
                        }
                    })),
                    gl_versions: Some(gl_profiles.to_vec()),
                    ..Default::default()
                },
                platform_type: Some(EGL_PLATFORM_GBM_KHR),
                platform_display: gbm_device as isize,
                ..Default::default()
            };

            let created_display = WaylandEglDisplay::new(options, gbm_device, drm_fd, dmabuf.clone())?;
            display = Some(created_display.clone());
            let egl_display = created_display.display();

            let extensions = egl_display.egl_interface().query_string(egl_display.handle(), EGL_EXTENSIONS).unwrap_or_default();
            let extensions: Vec<&str> = extensions.split(' ').collect();
            let check_extension = |ext: &str| -> Result<(), OpenGlError> {
                if !extensions.contains(&ext) {
                    return Err(OpenGlError::new(format!("Required EGL extension {ext} is not supported")));
                }
                Ok(())
            };

            check_extension("EGL_KHR_image_base")?;
            check_extension("EGL_EXT_image_dma_buf_import")?;

            let mut supported_formats = Self::negotiate_formats(egl_display, &feedback_formats);
            if supported_formats.is_empty() {
                supported_formats.push(DmabufFormatModifierPair::new(
                    DrmGbmUnsafeNativeMethods::DRM_FORMAT_ARGB8888,
                    DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID,
                ));
            }
            created_display.supported_formats_mut().extend(supported_formats);

            Ok(Rc::new(Self::new(created_display)))
        })();

        match created {
            Ok(graphics) => Ok(Some(graphics)),
            Err(error) => {
                match display {
                    // Disposing the display runs the dispose callback, which handles GBM + DRM fd
                    Some(display) => display.display().dispose(),
                    None => {
                        // SAFETY: the device (when made) and the descriptor were made above and
                        // nothing else releases them.
                        unsafe {
                            if !gbm_device.is_null() {
                                native.gbm_device_destroy(gbm_device);
                            }
                            DrmGbmUnsafeNativeMethods::close(drm_fd);
                        }
                    }
                }
                Err(error)
            }
        }
    }

    /// Collect all format/modifier pairs from feedback tranches, each once, in the order they
    /// first appear.
    fn feedback_formats(feedback: &WaylandDmabufFeedback) -> Vec<DmabufFormatModifierPair> {
        let mut formats: Vec<DmabufFormatModifierPair> = Vec::new();
        for tranche in feedback.tranches() {
            for format in &tranche.formats {
                if !formats.contains(format) {
                    formats.push(*format);
                }
            }
        }
        formats
    }

    fn negotiate_formats(display: &EglDisplay, feedback_formats: &[DmabufFormatModifierPair]) -> Vec<DmabufFormatModifierPair> {
        let egl = display.egl_interface();
        Self::select_formats(feedback_formats, egl.is_query_dma_buf_modifiers_ext_available(), |format| {
            // Query how many modifiers EGL supports for this format
            let mut count = 0;
            // SAFETY: null buffers with a length of zero ask for the count.
            let counted = unsafe {
                egl.query_dma_buf_modifiers_ext(
                    display.handle(),
                    format as i32,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut count,
                )
            };
            if !counted || count == 0 {
                return None;
            }

            // Retrieve the actual modifier list
            let mut egl_modifiers = vec![0u64; count as usize];
            let mut written = 0;
            // SAFETY: the buffer has `count` values; the flags of external use are not asked for.
            let queried = unsafe {
                egl.query_dma_buf_modifiers_ext(
                    display.handle(),
                    format as i32,
                    count,
                    egl_modifiers.as_mut_ptr(),
                    std::ptr::null_mut(),
                    &mut written,
                )
            };
            if !queried {
                return None;
            }
            Some(egl_modifiers)
        })
    }

    /// The pairs of the feedback that EGL can import: every pair of a format whose modifier
    /// EGL lists, or that has the implicit modifier; without the query of modifiers, the
    /// pairs with the implicit modifier only. `egl_modifiers` lists the modifiers of a
    /// format, `None` when EGL has none for it.
    fn select_formats(
        feedback_formats: &[DmabufFormatModifierPair],
        is_query_dma_buf_modifiers_available: bool,
        mut egl_modifiers: impl FnMut(u32) -> Option<Vec<u64>>,
    ) -> Vec<DmabufFormatModifierPair> {
        let mut negotiated = Vec::new();

        if feedback_formats.is_empty() {
            return negotiated;
        }

        let mut unique_formats: Vec<u32> = Vec::new();
        for pair in feedback_formats {
            if !unique_formats.contains(&pair.format) {
                unique_formats.push(pair.format);
            }
        }

        for format in unique_formats {
            if !is_query_dma_buf_modifiers_available {
                // Modifier query unavailable — only accept implicit modifier
                for pair in feedback_formats {
                    if pair.format == format && pair.modifier == DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID {
                        negotiated.push(*pair);
                    }
                }
                continue;
            }

            let Some(egl_mod_set) = egl_modifiers(format) else {
                continue;
            };

            // Keep formats whose modifier is supported by EGL or is the implicit modifier
            for pair in feedback_formats {
                if pair.format != format {
                    continue;
                }
                if egl_mod_set.contains(&pair.modifier) || pair.modifier == DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID {
                    negotiated.push(*pair);
                }
            }
        }

        negotiated
    }

    fn find_render_node(native: &DrmGbmUnsafeNativeMethods, main_device: u64) -> Option<CString> {
        let mut device: *mut DrmDevice = std::ptr::null_mut();
        if native.drm_get_device_from_dev_id(main_device, 0, &mut device) != 0 || device.is_null() {
            return None;
        }

        // SAFETY: the device is the record the library allocated for the call above, valid
        // until it is freed below; the node of a type it has is a terminated string.
        let path = unsafe {
            let record = &*device;
            if record.available_nodes & (1 << DrmGbmUnsafeNativeMethods::DRM_NODE_RENDER) == 0 {
                None
            } else {
                let node = *record.nodes.add(DrmGbmUnsafeNativeMethods::DRM_NODE_RENDER as usize);
                (!node.is_null()).then(|| CStr::from_ptr(node).to_owned())
            }
        };
        // SAFETY: the record of the call above, freed once.
        unsafe { native.drm_free_device(&mut device) };
        path
    }
}

impl IWaylandGraphics for WaylandEglDmaBufPlatformGraphics {
    /// # Panics
    /// Panics when the context cannot be created (the exception of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.display.display().create_context(None) {
            Ok(context) => context,
            Err(error) => panic!("{error}"),
        }
    }

    fn dispose(&self) {
        self.display.display().dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    const ARGB: u32 = DrmGbmUnsafeNativeMethods::DRM_FORMAT_ARGB8888;
    const XRGB: u32 = DrmGbmUnsafeNativeMethods::DRM_FORMAT_XRGB8888;
    const INVALID: u64 = DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID;

    fn pair(format: u32, modifier: u64) -> DmabufFormatModifierPair {
        DmabufFormatModifierPair::new(format, modifier)
    }

    #[test]
    fn the_pairs_egl_lists_and_the_implicit_modifier_are_kept() {
        let feedback = [pair(ARGB, 0), pair(ARGB, 5), pair(ARGB, INVALID), pair(XRGB, 0), pair(7, 0)];
        let negotiated = WaylandEglDmaBufPlatformGraphics::select_formats(&feedback, true, |format| match format {
            ARGB => Some(vec![0, 1]),
            XRGB => Some(vec![9]),
            _ => None,
        });
        assert_eq!(negotiated, vec![pair(ARGB, 0), pair(ARGB, INVALID)]);
    }

    #[test]
    fn without_the_query_of_modifiers_only_the_implicit_modifier_is_kept() {
        let feedback = [pair(ARGB, 0), pair(ARGB, INVALID), pair(XRGB, INVALID)];
        let negotiated =
            WaylandEglDmaBufPlatformGraphics::select_formats(&feedback, false, |_| panic!("EGL is not asked"));
        assert_eq!(negotiated, vec![pair(ARGB, INVALID), pair(XRGB, INVALID)]);
    }

    #[test]
    fn no_feedback_negotiates_nothing() {
        assert!(WaylandEglDmaBufPlatformGraphics::select_formats(&[], true, |_| Some(vec![0])).is_empty());
    }
}
