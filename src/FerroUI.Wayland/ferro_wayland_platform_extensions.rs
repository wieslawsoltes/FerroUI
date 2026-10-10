//! Selecting the Wayland backend for an application (the port of the
//! extensions of the application builder of the reference).

use ferroui_controls::AppBuilder;

/// [`AppBuilder`] extensions for enabling the Wayland windowing backend.
pub trait FerroWaylandPlatformExtensions {
    /// Configures the application to use the Wayland windowing backend. Options can be supplied by
    /// registering a [`WaylandPlatformOptions`](crate::WaylandPlatformOptions) instance with the builder.
    ///
    /// The backend exists on Linux only: on another system the windowing
    /// subsystem fails when it is initialized.
    fn use_wayland(&self) -> AppBuilder;

    /// Configures the application to use the Wayland windowing backend when a usable Wayland
    /// compositor is available, falling back to the previously configured windowing backend
    /// otherwise. Call it after `use_x11` or `use_platform_detect`, e. g.
    /// `.use_platform_detect().use_wayland_with_fallback()`. Does nothing on non-Linux platforms.
    ///
    /// # Panics
    /// No windowing backend was configured prior to this call (on Linux).
    fn use_wayland_with_fallback(&self) -> AppBuilder;
}

/// The message of the `InvalidOperationException` of the reference for a missing fallback.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const NO_FALLBACK: &str = "A fallback windowing backend must be configured before calling \
                           use_wayland_with_fallback, e.g. via use_x11 or use_platform_detect.";

impl FerroWaylandPlatformExtensions for AppBuilder {
    fn use_wayland(&self) -> AppBuilder {
        self.use_standard_runtime_platform_subsystem().use_windowing_subsystem(
            || {
                #[cfg(target_os = "linux")]
                crate::wayland_platform::WaylandPlatform::initialize(current_options());
                #[cfg(not(target_os = "linux"))]
                crate::FerroWaylandException::new("The Wayland backend exists on Linux only").throw();
            },
            "Wayland",
        )
    }

    fn use_wayland_with_fallback(&self) -> AppBuilder {
        #[cfg(not(target_os = "linux"))]
        {
            self.clone()
        }

        #[cfg(target_os = "linux")]
        {
            use ferroui_base::logging::{LogArea, LogEventLevel, Logger};

            let Some(fallback) = self.windowing_subsystem_initializer() else {
                panic!("{NO_FALLBACK}");
            };

            self.use_standard_runtime_platform_subsystem().use_windowing_subsystem(
                move || {
                    if let Some(error) = crate::wayland_platform::WaylandPlatform::try_initialize(current_options()) {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::PLATFORM) {
                            logger.log_with_values(
                                None,
                                "Unable to initialize the Wayland backend, falling back: {Error}",
                                &[&error],
                            );
                        }
                        // The reference logs the reason and calls the fallback. A fallback
                        // that fails as well (no X server either) would leave its own failure
                        // as the only thing said, so the reason Wayland was refused is put
                        // beside it before the failure goes on.
                        if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fallback())) {
                            eprintln!(
                                "The fallback windowing backend failed after the Wayland backend could not be \
                                 initialized: {error}"
                            );
                            std::panic::resume_unwind(payload);
                        }
                    }
                },
                "Wayland",
            )
        }
    }
}

/// The options registered with the builder, or the default ones.
#[cfg(target_os = "linux")]
fn current_options() -> crate::WaylandPlatformOptions {
    use ferroui_base::{FerroLocator, LocatorExtensions};

    FerroLocator::current()
        .get_service::<crate::WaylandPlatformOptions>()
        .map(|options| (*options).clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use ferroui_controls::Application;

    #[test]
    fn use_wayland_names_the_windowing_subsystem_and_initializes_nothing() {
        let builder = AppBuilder::configure::<Application>().use_wayland();
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("Wayland"));
        assert_eq!(builder.runtime_platform_services_name().as_deref(), Some("StandardRuntimePlatform"));
        assert!(builder.windowing_subsystem_initializer().is_some());
        // Nothing is initialized until the application is set up: no connection is made.
        assert!(builder.instance().is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[should_panic(expected = "A fallback windowing backend must be configured")]
    fn the_fallback_has_to_be_configured_first() {
        let _ = AppBuilder::configure::<Application>().use_wayland_with_fallback();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_fallback_keeps_the_place_of_the_windowing_subsystem() {
        let builder = AppBuilder::configure::<Application>()
            .use_windowing_subsystem(|| {}, "Other")
            .use_wayland_with_fallback();
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("Wayland"));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn the_fallback_does_nothing_on_other_systems() {
        let builder = AppBuilder::configure::<Application>().use_wayland_with_fallback();
        assert_eq!(builder.windowing_subsystem_name(), None);
        let builder = AppBuilder::configure::<Application>()
            .use_windowing_subsystem(|| {}, "Other")
            .use_wayland_with_fallback();
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("Other"));
    }
}
