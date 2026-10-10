use ferroui_controls::AppBuilder;

/// Selects the platform subsystems of a desktop application.
pub trait AppBuilderDesktopExtensions {
    /// Configures the application for the operating system it runs on:
    /// HarfBuzz for text shaping, the native windowing backend and Skia for
    /// rendering.
    ///
    /// # Panics
    /// Panics on an operating system whose windowing backend is not
    /// supported yet (everything but macOS and Windows).
    fn use_platform_detect(&self) -> AppBuilder;
}

impl AppBuilderDesktopExtensions for AppBuilder {
    fn use_platform_detect(&self) -> AppBuilder {
        // Always load HarfBuzz on desktop platforms
        load_harf_buzz(self);

        #[cfg(target_os = "macos")]
        {
            load_ferro_native(self);
            load_skia(self);
        }

        #[cfg(windows)]
        {
            load_win32(self);
            load_skia(self);
        }

        #[cfg(not(any(target_os = "macos", windows)))]
        {
            panic!(
                "use_platform_detect: the windowing backend for this operating system ({}) is not supported yet; \
                 only macOS and Windows are.",
                std::env::consts::OS
            );
        }

        #[cfg(any(target_os = "macos", windows))]
        self.clone()
    }
}

#[cfg(target_os = "macos")]
fn load_ferro_native(builder: &AppBuilder) {
    use ferroui_native::FerroNativePlatformExtensions;
    builder.use_ferro_native();
}

#[cfg(windows)]
fn load_win32(builder: &AppBuilder) {
    use ferroui_win32::Win32ApplicationExtensions;
    builder.use_win32();
}

#[cfg(any(target_os = "macos", windows))]
fn load_skia(builder: &AppBuilder) {
    use ferroui_skia::SkiaApplicationExtensions;
    builder.use_skia();
}

fn load_harf_buzz(builder: &AppBuilder) {
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    builder.use_harfbuzz();
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use ferroui_controls::Application;

    #[test]
    fn platform_detect_selects_the_mac_os_subsystems() {
        let builder = AppBuilder::configure::<Application>().use_platform_detect();

        assert_eq!(builder.text_shaping_subsystem_name().as_deref(), Some("HarfBuzz"));
        assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Skia"));
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("FerroNative"));
        assert_eq!(builder.runtime_platform_services_name().as_deref(), Some("StandardRuntimePlatform"));
        assert!(builder.text_shaping_subsystem_initializer().is_some());
        assert!(builder.rendering_subsystem_initializer().is_some());
        assert!(builder.windowing_subsystem_initializer().is_some());
        assert!(builder.runtime_platform_services_initializer().is_some());
        // Nothing is initialized until the application is set up.
        assert!(builder.instance().is_none());
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use ferroui_controls::Application;

    #[test]
    fn platform_detect_selects_the_windows_subsystems() {
        let builder = AppBuilder::configure::<Application>().use_platform_detect();

        assert_eq!(builder.text_shaping_subsystem_name().as_deref(), Some("HarfBuzz"));
        assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Skia"));
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("Win32"));
        assert_eq!(builder.runtime_platform_services_name().as_deref(), Some("StandardRuntimePlatform"));
        assert!(builder.text_shaping_subsystem_initializer().is_some());
        assert!(builder.rendering_subsystem_initializer().is_some());
        assert!(builder.windowing_subsystem_initializer().is_some());
        assert!(builder.runtime_platform_services_initializer().is_some());
        // Nothing is initialized until the application is set up.
        assert!(builder.instance().is_none());
    }
}
