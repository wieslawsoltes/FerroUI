//! The options of the Wayland backend (the port of
//! `WaylandPlatformOptions.cs`).

use ferroui_opengl::{GlProfileType, GlVersion};
use std::any::Any;
use std::rc::Rc;

/// Presentation backend selected for a Wayland connection.
#[derive(Clone, Default)]
pub enum WaylandRenderingMode {
    /// Try EGL using the configured profiles, falling back to shared memory.
    #[default]
    Auto,
    /// Use the platform's Vulkan image presenter with VK_KHR_wayland_surface.
    Vulkan(ferroui_vulkan::VulkanOptions),
    /// Present software-rendered frames using wl_shm.
    Software,
}

impl std::fmt::Debug for WaylandRenderingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Auto => "Auto", Self::Vulkan(_) => "Vulkan", Self::Software => "Software" })
    }
}

/// Platform-specific options for the Wayland windowing backend, registered
/// with the application builder before `use_wayland`.
#[derive(Clone)]
pub struct WaylandPlatformOptions {
    pub rendering_mode: WaylandRenderingMode,
    /// The default AppId for Wayland windows (`xdg_toplevel.set_app_id`).
    /// If `None`, no application identifier is sent. Defaults to the file name of the executable
    /// (the reference: the name of the entry assembly).
    pub app_id: Option<String>,

    /// The name of the Wayland display to connect to (e.g. `wayland-0`). When `None`,
    /// the `WAYLAND_DISPLAY` environment variable is used. Ignored when [`display_fd`](Self::display_fd) is set.
    pub wl_display_name: Option<String>,

    /// An already-opened file descriptor for the Wayland display socket.
    /// When set, [`wl_display_name`](Self::wl_display_name) is ignored and the connection is made over
    /// the descriptor instead of a socket found by name.
    /// Reconnects are automatically disabled in this mode because the fd
    /// is consumed by the connection and cannot be reused.
    pub display_fd: Option<i32>,

    /// Whether to automatically attempt to reconnect to the compositor if the connection is lost.
    /// Defaults to enabled. Reconnects are always disabled when [`display_fd`](Self::display_fd) is set, because
    /// the file descriptor is consumed by the connection and cannot be reused.
    pub enable_reconnects: Option<bool>,

    /// Suppresses server-side decoration negotiation
    /// (`zxdg_decoration_manager_v1`): toplevels behave as if the
    /// compositor never advertised SSD support. Used primarily for
    /// testing the CSD path on compositors that would otherwise enforce
    /// server-side decorations (KWin, etc.). Equivalent in intent to
    /// `X11PlatformOptions::force_drawn_decorations`.
    ///
    /// Experimental, used mostly for testing.
    pub force_drawn_decorations: bool,

    /// The OpenGL/OpenGL ES versions to try, in priority order, when creating the GL context.
    /// The first profile the driver supports is used.
    pub gl_profiles: Vec<GlVersion>,

    /// Whether to use a dmabuf-based swapchain for GPU rendering. When `None`, the backend
    /// decides based on compositor and driver capabilities.
    pub use_dmabuf_swapchain: Option<bool>,

    /// If this option is set to true, a GMainLoop and GSource based dispatcher implementation will be used for the
    /// UI thread instead of the default managed one.
    /// Use this if you need to use GLib-based libraries on the main thread.
    pub use_g_lib_main_loop: bool,

    /// If the framework is in control of a run loop, we propagate exceptions by stopping the run loop frame
    /// and rethrowing an exception. However, if there is no run loop frame of the framework,
    /// there is no way to report such exceptions, since allowing those to escape the native call boundary
    /// will likely brick GLib machinery since it's not aware of them.
    /// This property allows to inspect such exceptions before they will be ignored.
    /// Only used when [`use_g_lib_main_loop`](Self::use_g_lib_main_loop) is enabled.
    pub external_g_lib_main_loop_exception_logger: Option<Rc<dyn Fn(&(dyn Any + Send))>>,
}

impl WaylandPlatformOptions {
    pub fn new() -> Self {
        Self {
            rendering_mode: WaylandRenderingMode::Auto,
            app_id: Self::entry_app_id(),
            wl_display_name: None,
            display_fd: None,
            enable_reconnects: None,
            force_drawn_decorations: false,
            gl_profiles: vec![
                GlVersion::new(GlProfileType::OpenGL, 4, 0),
                GlVersion::new(GlProfileType::OpenGL, 3, 2),
                GlVersion::new(GlProfileType::OpenGL, 3, 0),
                GlVersion::new(GlProfileType::OpenGLES, 3, 2),
                GlVersion::new(GlProfileType::OpenGLES, 3, 0),
                GlVersion::new(GlProfileType::OpenGLES, 2, 0),
            ],
            use_dmabuf_swapchain: None,
            use_g_lib_main_loop: false,
            external_g_lib_main_loop_exception_logger: None,
        }
    }

    pub(crate) fn force_drawn_decorations_internal(&self) -> bool {
        self.force_drawn_decorations
    }

    /// The default of [`app_id`](Self::app_id): the file name of the executable, where the
    /// reference takes the name of the entry assembly (and ignores a failure to get it).
    fn entry_app_id() -> Option<String> {
        let executable = std::env::current_exe().ok()?;
        Some(executable.file_stem()?.to_string_lossy().into_owned())
    }

    /// What the worker thread needs of the options, as values.
    pub fn for_worker(&self) -> WaylandWorkerOptions {
        WaylandWorkerOptions {
            rendering_mode: self.rendering_mode.clone(),
            wl_display_name: self.wl_display_name.clone(),
            display_fd: self.display_fd,
            enable_reconnects: self.enable_reconnects,
            force_drawn_decorations: self.force_drawn_decorations_internal(),
            gl_profiles: self.gl_profiles.clone(),
            use_dmabuf_swapchain: self.use_dmabuf_swapchain,
            app_id: self.app_id.clone(),
        }
    }
}

impl Default for WaylandPlatformOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// The options the worker thread reads. The options object itself holds a callback of the UI
/// thread, so the worker gets the values it needs.
#[derive(Clone, Debug)]
pub struct WaylandWorkerOptions {
    pub rendering_mode: WaylandRenderingMode,
    pub wl_display_name: Option<String>,
    pub display_fd: Option<i32>,
    pub enable_reconnects: Option<bool>,
    pub force_drawn_decorations: bool,
    pub gl_profiles: Vec<GlVersion>,
    pub use_dmabuf_swapchain: Option<bool>,
    pub app_id: Option<String>,
}

impl WaylandWorkerOptions {
    /// Whether a lost connection ends the worker instead of starting the reconnect probes.
    pub fn reconnects_disabled(&self) -> bool {
        self.enable_reconnects == Some(false) || self.display_fd.is_some()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_default_options_are_those_of_the_reference() {
        let options = WaylandPlatformOptions::default();
        assert_eq!(options.wl_display_name, None);
        assert_eq!(options.display_fd, None);
        assert_eq!(options.enable_reconnects, None);
        assert!(!options.force_drawn_decorations);
        assert_eq!(options.gl_profiles.len(), 6);
        assert_eq!(options.gl_profiles[0], GlVersion::new(GlProfileType::OpenGL, 4, 0));
        assert_eq!(options.gl_profiles[5], GlVersion::new(GlProfileType::OpenGLES, 2, 0));
        assert_eq!(options.use_dmabuf_swapchain, None);
        assert!(!options.use_g_lib_main_loop);
        assert!(options.external_g_lib_main_loop_exception_logger.is_none());
    }

    #[test]
    fn reconnects_are_off_when_asked_and_for_a_descriptor() {
        let mut options = WaylandPlatformOptions::default();
        assert!(!options.for_worker().reconnects_disabled());
        options.enable_reconnects = Some(true);
        assert!(!options.for_worker().reconnects_disabled());
        options.enable_reconnects = Some(false);
        assert!(options.for_worker().reconnects_disabled());
        options.enable_reconnects = None;
        options.display_fd = Some(7);
        assert!(options.for_worker().reconnects_disabled());
    }

    #[test]
    fn the_application_identifier_defaults_to_the_name_of_the_executable() {
        let mut options = WaylandPlatformOptions::default();
        let default = options.app_id.clone().expect("a test has an executable");
        assert!(!default.is_empty());
        assert!(!default.contains('/'));
        assert_eq!(options.for_worker().app_id.as_deref(), Some(default.as_str()));
        options.app_id = None;
        assert_eq!(options.for_worker().app_id, None);
    }

    // WaylandAppIdTests.cs
    #[test]
    fn wayland_platform_options_can_set_app_id() {
        let options = WaylandPlatformOptions { app_id: Some("org.ferroui.test".to_string()), ..WaylandPlatformOptions::new() };

        assert_eq!(Some("org.ferroui.test"), options.app_id.as_deref());
    }
}
