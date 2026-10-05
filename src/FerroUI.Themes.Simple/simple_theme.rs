//! Port of `SimpleTheme.xaml.cs`: the class of the document
//! `SimpleTheme.xaml`.

use crate::register_types::register_types;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::{styles_as_style, IStyle, Styles};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// The Simple theme: the styles and resources of `SimpleTheme.xaml`.
#[repr(C)]
pub struct SimpleTheme {
    base: Styles,
}

ferro_class!(SimpleTheme: Styles);
ferro_impl_classes!(SimpleTheme: FerroObjectImpl);
ferro_class_info!(SimpleTheme {
    new: SimpleTheme::new,
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => SimpleTheme::with_service_provider],
    },
});

impl SimpleTheme {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Themes.Simple/SimpleTheme.xaml";

    /// Field initialisation; see [`Styles::construct`].
    pub fn construct() -> Self {
        Self { base: Styles::construct() }
    }

    /// Creates the theme.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load (an exception of
    /// the constructor in the managed original).
    pub fn new() -> Ref<Self> {
        Self::with_service_provider(None)
    }

    /// Creates the theme; `sp` is the parent's service provider.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load.
    pub fn with_service_provider(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Err(error) = Self::load(sp, &this) {
            panic!("{error}");
        }
        this
    }

    /// The theme as a style, for the style collections of applications and
    /// elements (`application.styles().add(theme.as_style())`).
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        styles_as_style(&self.to_ref())
    }

    /// Populates `this` from the document of the class: what the XAML
    /// compiler generates for a class with compiled markup (the populate
    /// method the load call of the constructor is rewritten to).
    ///
    /// Until the compiler exists the document registered for the class
    /// (`register_types()`) is loaded at run time. The loader loads it as a
    /// group with every document it includes, so the group transformers
    /// link the documents to each other exactly as the compiler links the
    /// documents of a project (merged resource includes are merged into the
    /// including dictionary, style includes become the styles of the
    /// included document).
    fn load(sp: Option<Rc<dyn IServiceProvider>>, this: &Ref<Self>) -> Result<(), XamlLoadException> {
        register_types();
        let instance: BoxedValue = Rc::new(this.clone());
        FerroRuntimeXamlLoader::load_object(&instance, sp.as_ref())
    }
}
