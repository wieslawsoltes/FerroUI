//! The entry of the backend in the application builder.

use crate::platform::ProGpuPlatformRenderInterface;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::FerroLocator;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

/// Registers the render interface of the backend in the service locator.
pub fn initialize() {
    let render_interface: Rc<dyn IPlatformRenderInterface> = Rc::new(ProGpuPlatformRenderInterface::new());
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(render_interface);
}

/// The ProGPU backend in the application builder.
pub trait ProGpuApplicationExtensions {
    /// Draws the application with the ProGPU backend. An application has one render backend:
    /// this one or another, the last one chosen.
    ///
    /// The backend has no text services yet (step P4 of docs/porting/progpu-backend.md): it
    /// registers no font manager and no text shaper, and draws glyph runs as nothing. An
    /// application that lays out text registers those services itself until then.
    fn use_progpu(&self) -> AppBuilder;
}

impl ProGpuApplicationExtensions for AppBuilder {
    fn use_progpu(&self) -> AppBuilder {
        self.use_rendering_subsystem(initialize, "ProGPU")
    }
}
