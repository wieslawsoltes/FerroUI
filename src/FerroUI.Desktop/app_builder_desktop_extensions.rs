use ferroui_controls::AppBuilder;

/// Selects the platform subsystems of a desktop application.
pub trait AppBuilderDesktopExtensions {
    /// Configures the application for the operating system it runs on:
    /// HarfBuzz for text shaping, the native windowing backend and Skia for
    /// rendering.
    ///
    /// # Panics
    /// Panics on an operating system whose windowing backend is not
    /// supported yet (everything but macOS and Linux).
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

        #[cfg(target_os = "linux")]
        {
            load_x11(self);
            load_skia(self);
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            panic!(
                "use_platform_detect: the windowing backend for this operating system ({}) is not supported yet; \
                 only macOS and Linux are.",
                std::env::consts::OS
            );
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        self.clone()
    }
}

#[cfg(target_os = "macos")]
fn load_ferro_native(builder: &AppBuilder) {
    use ferroui_native::FerroNativePlatformExtensions;
    builder.use_ferro_native();
}

#[cfg(target_os = "linux")]
fn load_x11(builder: &AppBuilder) {
    use ferroui_x11::FerroX11PlatformExtensions;
    builder.use_x11();
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
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

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use super::*;
    use ferroui_controls::Application;

    // Not from the reference, whose platform detection has no test.
    #[test]
    fn platform_detect_selects_the_linux_subsystems() {
        let builder = AppBuilder::configure::<Application>().use_platform_detect();

        assert_eq!(builder.text_shaping_subsystem_name().as_deref(), Some("HarfBuzz"));
        assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Skia"));
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("X11"));
        assert_eq!(builder.runtime_platform_services_name().as_deref(), Some("StandardRuntimePlatform"));
        // Nothing is initialized until the application is set up: no connection to a server is made.
        assert!(builder.instance().is_none());
    }
}
