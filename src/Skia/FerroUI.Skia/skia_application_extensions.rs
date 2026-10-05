use crate::{SkiaOptions, SkiaPlatform};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::AppBuilder;

/// Skia application extensions.
pub trait SkiaApplicationExtensions {
    /// Enable Skia renderer.
    ///
    /// The backend is initialized with the [`SkiaOptions`] registered with
    /// the builder, or the default options.
    fn use_skia(&self) -> AppBuilder;
}

impl SkiaApplicationExtensions for AppBuilder {
    fn use_skia(&self) -> AppBuilder {
        self.use_rendering_subsystem(
            || {
                let options = FerroLocator::current().get_service::<SkiaOptions>();
                SkiaPlatform::initialize_with_options(options.as_deref().copied().unwrap_or_default());
            },
            "Skia",
        )
    }
}
