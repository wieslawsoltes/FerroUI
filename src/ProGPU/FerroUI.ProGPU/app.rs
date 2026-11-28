//! The entry of the backend in the application builder.

use crate::font_manager::ProGpuFontManager;
use crate::platform::ProGpuPlatformRenderInterface;
use crate::text_shaper::ProGpuTextShaper;
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::FerroLocator;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

/// Registers the render interface and the font manager of the backend in the service locator.
pub fn initialize() {
    let render_interface: Rc<dyn IPlatformRenderInterface> = Rc::new(ProGpuPlatformRenderInterface::new());
    let font_manager: Rc<dyn IFontManagerImpl> = Rc::new(ProGpuFontManager::new());
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(render_interface)
        .bind::<dyn IFontManagerImpl>()
        .to_constant(font_manager);
}

/// Registers the text shaper of the backend in the service locator.
pub fn initialize_text_shaping() {
    let text_shaper: Rc<dyn ITextShaperImpl> = Rc::new(ProGpuTextShaper::new());
    FerroLocator::current_mutable().bind::<dyn ITextShaperImpl>().to_constant(text_shaper);
}

/// The ProGPU backend in the application builder.
pub trait ProGpuApplicationExtensions {
    /// Draws the application with the ProGPU backend and lays its text out with the text stack
    /// of the engine: the catalog of the system fonts, the faces and the OpenType shaper. An
    /// application has one render backend and one text shaper: these or others, the last ones
    /// chosen (a text shaper named after this call replaces the one of the engine).
    fn use_progpu(&self) -> AppBuilder;
}

impl ProGpuApplicationExtensions for AppBuilder {
    fn use_progpu(&self) -> AppBuilder {
        self.use_rendering_subsystem(initialize, "ProGPU").use_text_shaping_subsystem(initialize_text_shaping, "ProGPU")
    }
}
