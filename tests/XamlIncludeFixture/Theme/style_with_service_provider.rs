//! Port of `Xaml/StyleWithServiceProvider.xaml.cs` of upstream's XAML test
//! assembly: the class of the document `Xaml/StyleWithServiceProvider.xaml`.

use std::rc::Rc;

use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::{Style, StyleBaseImpl};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};

/// A style that keeps the service provider it was created with and populates
/// itself from its document through it.
#[repr(C)]
pub struct StyleWithServiceProvider {
    base: Style,
    service_provider: Option<Rc<dyn IServiceProvider>>,
}

ferro_class!(StyleWithServiceProvider: Style);
ferro_impl_classes!(StyleWithServiceProvider: FerroObjectImpl, StyleBaseImpl);
ferro_class_info!(StyleWithServiceProvider {
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => StyleWithServiceProvider::new],
        properties: [
            ServiceProvider: Option<Rc<dyn IServiceProvider>> { get: StyleWithServiceProvider::service_provider },
        ],
    },
});

impl StyleWithServiceProvider {
    /// `StyleWithServiceProvider(IServiceProvider? sp = null)`: `FerroXamlLoader.Load(sp, this)`
    /// is the populate of the compiled document, as upstream's compiler rewrites it.
    ///
    /// # Panics
    /// Panics if the document fails to load (an exception of the constructor upstream).
    pub fn new(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self { base: Style::construct(), service_provider: sp.clone() });
        if let Err(error) = crate::compiled_style_with_service_provider::populate(sp, &this) {
            panic!("{error}");
        }
        this
    }

    /// `ServiceProvider`.
    pub fn service_provider(&self) -> Option<Rc<dyn IServiceProvider>> {
        self.service_provider.clone()
    }
}
