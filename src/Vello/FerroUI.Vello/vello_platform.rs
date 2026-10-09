use crate::platform_render_interface::PlatformRenderInterface;
use crate::vello_options::VelloOptions;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, Vector};
use std::rc::Rc;

/// The options of the backend, for every thread.
static OPTIONS: std::sync::RwLock<Option<VelloOptions>> = std::sync::RwLock::new(None);

/// Vello platform initializer.
pub struct VelloPlatform;

impl VelloPlatform {
    /// Initializes the Vello platform with the default options.
    pub fn initialize() {
        Self::initialize_with_options(VelloOptions::default());
    }

    /// Initializes the Vello platform: registers the render interface in the
    /// service locator.
    ///
    /// No font manager is registered: the font manager of the backend is
    /// stage 5 of the design document. An application that draws text binds
    /// one of its own until then, or fails where it asks for one.
    pub fn initialize_with_options(options: VelloOptions) {
        *OPTIONS.write().unwrap_or_else(|e| e.into_inner()) = Some(options);
        let render_interface: Rc<dyn IPlatformRenderInterface> = Rc::new(PlatformRenderInterface::new(options));

        FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(render_interface);
    }

    /// The options the backend was initialized with. Read by the thread
    /// that draws, which has no service locator of its own.
    pub fn options() -> Option<VelloOptions> {
        *OPTIONS.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Default DPI.
    pub fn default_dpi() -> Vector {
        Vector::new(96.0, 96.0)
    }
}
