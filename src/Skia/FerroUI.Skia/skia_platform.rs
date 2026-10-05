use crate::platform_render_interface::PlatformRenderInterface;
use crate::skia_options::SkiaOptions;
use crate::font_manager_impl::FontManagerImpl;
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface};
use ferroui_base::{FerroLocator, Vector};
use std::rc::Rc;

/// Skia platform initializer.
pub struct SkiaPlatform;

impl SkiaPlatform {
    /// Initializes the Skia platform with the default options.
    pub fn initialize() {
        Self::initialize_with_options(SkiaOptions::default());
    }

    /// Initializes the Skia platform: registers the render interface in the
    /// service locator.
    pub fn initialize_with_options(options: SkiaOptions) {
        let render_interface: Rc<dyn IPlatformRenderInterface> =
            Rc::new(PlatformRenderInterface::new(options.max_gpu_resource_size_bytes, options.use_stencil_buffers));

        let font_manager: Rc<dyn IFontManagerImpl> = Rc::new(FontManagerImpl::new());

        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(render_interface)
            .bind::<dyn IFontManagerImpl>()
            .to_constant(font_manager);
    }

    /// Default DPI.
    pub fn default_dpi() -> Vector {
        Vector::new(96.0, 96.0)
    }
}
