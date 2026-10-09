use crate::{VelloOptions, VelloPlatform};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::AppBuilder;

/// Vello application extensions.
pub trait VelloApplicationExtensions {
    /// Enable Vello renderer.
    ///
    /// The backend is initialized with the [`VelloOptions`] registered with
    /// the builder, or the default options. An application has one render
    /// backend: this one or another, the last one chosen.
    fn use_vello(&self) -> AppBuilder;
}

impl VelloApplicationExtensions for AppBuilder {
    fn use_vello(&self) -> AppBuilder {
        self.use_rendering_subsystem(
            || {
                let options = FerroLocator::current().get_service::<VelloOptions>();
                VelloPlatform::initialize_with_options(options.as_deref().copied().unwrap_or_default());
            },
            "Vello",
        )
    }
}
