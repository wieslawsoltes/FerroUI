//! The application builder extension that selects the macOS backend, and
//! the options of the backend.

#[cfg(target_os = "macos")]
use ferroui_base::{FerroLocator, LocatorExtensions};
#[cfg(target_os = "macos")]
use ferroui_controls::AppBuilder;

/// Selects the macOS backend for an application.
#[cfg(target_os = "macos")]
pub trait FerroNativePlatformExtensions {
    /// Uses the standard runtime platform and the macOS windowing
    /// subsystem.
    ///
    /// The backend is initialized with the [`FerroNativePlatformOptions`]
    /// registered with the builder, or the default options; once the
    /// application is set up its name becomes the application title.
    fn use_ferro_native(&self) -> AppBuilder;
}

#[cfg(target_os = "macos")]
impl FerroNativePlatformExtensions for AppBuilder {
    fn use_ferro_native(&self) -> AppBuilder {
        let builder = self.clone();
        self.use_standard_runtime_platform_subsystem().use_windowing_subsystem(
            move || {
                let options = FerroLocator::current().get_service::<FerroNativePlatformOptions>();
                let platform =
                    crate::FerroNativePlatform::initialize(options.as_deref().cloned().unwrap_or_default());

                builder.after_setup(move |_| {
                    platform.setup_application_name();
                    platform.setup_application_menu_exporter();
                    platform.setup_application_dock_menu_exporter();
                });
            },
            "FerroNative",
        );

        self.clone()
    }
}

/// Represents the rendering mode for platform graphics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FerroNativeRenderingMode {
    /// The toolkit would try to use native OpenGL with GPU rendering.
    OpenGl = 1,
    /// The toolkit is rendered into a framebuffer.
    Software = 2,
    /// The toolkit would try to use Metal with GPU rendering.
    Metal = 3,
}

/// OSX backend options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FerroNativePlatformOptions {
    /// Gets or sets the rendering modes with fallbacks.
    /// The first element in the list has the highest priority.
    /// The default value is: [`FerroNativeRenderingMode::Metal`],
    /// [`FerroNativeRenderingMode::OpenGl`],
    /// [`FerroNativeRenderingMode::Software`].
    ///
    /// If application should work on as wide range of devices as possible,
    /// at least add [`FerroNativeRenderingMode::Software`] as a fallback
    /// value.
    pub rendering_mode: Vec<FerroNativeRenderingMode>,

    /// Embeds popups to the window when set to true. The default value is
    /// false.
    pub overlay_popups: bool,

    /// This property should be used in case you want to build the OSX
    /// native part by yourself and make your app run with it. The default
    /// value is `None`: the library linked into the application is used.
    pub ferro_native_library_path: Option<String>,

    /// If you distribute your app in App Store - it should be with sandbox
    /// enabled. This parameter enables the bookmark APIs of storage items,
    /// as well as wrapping all storage related calls in secure context. The
    /// default value is true.
    pub app_sandbox_enabled: bool,
}

impl Default for FerroNativePlatformOptions {
    fn default() -> Self {
        Self {
            rendering_mode: vec![
                FerroNativeRenderingMode::Metal,
                FerroNativeRenderingMode::OpenGl,
                FerroNativeRenderingMode::Software,
            ],
            overlay_popups: false,
            ferro_native_library_path: None,
            app_sandbox_enabled: true,
        }
    }
}

/// OSX front-end options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacOSPlatformOptions {
    /// Determines whether to show your application in the dock when it
    /// runs. The default value is true.
    pub show_in_dock: bool,

    /// By default, items like Quit, Hide are added to the OSX Application
    /// Menu. You can prevent those items from being added with this
    /// property. The default value is false.
    pub disable_default_application_menu_items: bool,

    /// Gets or sets a value indicating whether the native macOS menu bar
    /// will be enabled for the application.
    pub disable_native_menus: bool,

    /// Gets or sets a value indicating whether the native macOS should set
    /// `[NSProcessInfo setProcessName]` in runtime.
    pub disable_set_process_name: bool,

    /// Gets or sets a value indicating whether the backend can install its
    /// own AppDelegate. Disabling this can be useful in some scenarios like
    /// when running as a plugin inside an existing macOS application.
    pub disable_ferro_app_delegate: bool,
}

impl Default for MacOSPlatformOptions {
    fn default() -> Self {
        Self {
            show_in_dock: true,
            disable_default_application_menu_items: false,
            disable_native_menus: false,
            disable_set_process_name: false,
            disable_ferro_app_delegate: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_reference_defaults() {
        let options = FerroNativePlatformOptions::default();
        assert_eq!(
            options.rendering_mode,
            [FerroNativeRenderingMode::Metal, FerroNativeRenderingMode::OpenGl, FerroNativeRenderingMode::Software]
        );
        assert!(!options.overlay_popups);
        assert_eq!(options.ferro_native_library_path, None);
        assert!(options.app_sandbox_enabled);

        let mac = MacOSPlatformOptions::default();
        assert!(mac.show_in_dock);
        assert!(!mac.disable_default_application_menu_items);
        assert!(!mac.disable_native_menus);
        assert!(!mac.disable_set_process_name);
        assert!(!mac.disable_ferro_app_delegate);

        assert_eq!(FerroNativeRenderingMode::OpenGl as i32, 1);
        assert_eq!(FerroNativeRenderingMode::Software as i32, 2);
        assert_eq!(FerroNativeRenderingMode::Metal as i32, 3);
    }
}
