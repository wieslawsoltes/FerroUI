use crate::HarfBuzzTextShaper;
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::FerroLocator;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

/// HarfBuzz text shaping subsystem initializer.
pub struct HarfBuzzPlatform;

impl HarfBuzzPlatform {
    /// Registers the HarfBuzz text shaper in the service locator.
    pub fn initialize() {
        let text_shaper: Rc<dyn ITextShaperImpl> = Rc::new(HarfBuzzTextShaper::new());

        FerroLocator::current_mutable().bind::<dyn ITextShaperImpl>().to_constant(text_shaper);
    }
}

/// HarfBuzz application extensions.
pub trait HarfBuzzApplicationExtensions {
    /// Enable the HarfBuzz text shaper.
    fn use_harfbuzz(&self) -> AppBuilder;
}

impl HarfBuzzApplicationExtensions for AppBuilder {
    fn use_harfbuzz(&self) -> AppBuilder {
        self.use_text_shaping_subsystem(HarfBuzzPlatform::initialize, "HarfBuzz")
    }
}
