//! Port of `SimpleTheme.xaml.cs`: the class of the document
//! `SimpleTheme.xaml`.

use crate::register_types::register_types;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::{styles_as_style, IStyle, Styles};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
use ferroui_markup_xaml::XamlLoadError;
use std::rc::Rc;

/// The Simple theme: the styles and resources of `SimpleTheme.xaml`.
#[repr(C)]
pub struct SimpleTheme {
    base: Styles,
}

ferro_class!(SimpleTheme: Styles);
ferro_impl_classes!(SimpleTheme: FerroObjectImpl);
// The constructors of the type information (and of a load by URI) apply the units the application
// registered, never the table of all units by name: the type information is reachable from every
// construction, and naming the table there would keep every unit (#568, step 3). Compiled markup
// constructs the class with `new()` (`T::new()` by convention), or with `with_parts` when the
// application trims its themes.
ferro_class_info!(SimpleTheme {
    new: SimpleTheme::with_registered_parts_default,
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => SimpleTheme::with_registered_parts],
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
    /// The full theme: every unit of its compiled markup. A later load of the theme by URI gets the
    /// full theme too, unless the application registered the units it keeps.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load.
    pub fn with_service_provider(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        let parts = crate::compiled_xaml::THEME_UNITS;
        rt::register_theme_parts_if_absent(crate::compiled_xaml::THEME_CLASS, parts);
        Self::with_parts(sp, parts)
    }

    /// Creates the theme from some of the units of its compiled markup (#568, step 3 of
    /// docs/porting/theme-trimming.md); `sp` is the parent's service provider.
    ///
    /// `parts` are units of `crate::compiled_xaml::THEME_UNITS`, in canonical order (ascending
    /// indices), with the ambient unit (index 0) first: the table an application's build writes
    /// when it trims its themes (`[package.metadata.ferroui] theme-trimming = true`). The theme
    /// applies those units, at the places of the full theme and in its order; a unit that `parts`
    /// leaves out adds nothing. Only the units named are linked into the application.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load.
    pub fn with_parts(sp: Option<Rc<dyn IServiceProvider>>, parts: rt::ThemeParts) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Err(error) = Self::load(sp, &this, parts) {
            panic!("{error}");
        }
        this
    }

    /// Creates the theme with the units the application registered (#568, step 3): the
    /// constructor of a load by URI and of the type information. Every unit when the build has
    /// `ferroui-base/full-themes`, else the parts the application registered
    /// (`rt::register_theme_parts`), else the full theme if one was constructed before, else the
    /// ambient unit alone.
    #[doc(hidden)]
    pub fn with_registered_parts(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        Self::with_parts(sp, crate::compiled_xaml::registered_theme_units())
    }

    /// [`with_registered_parts`](Self::with_registered_parts) without a service provider: the
    /// default constructor of the type information.
    #[doc(hidden)]
    pub fn with_registered_parts_default() -> Ref<Self> {
        Self::with_registered_parts(None)
    }

    /// The theme as a style, for the style collections of applications and
    /// elements (`application.styles().add(theme.as_style())`).
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        styles_as_style(&self.to_ref())
    }

    /// Populates `this` from the document of the class: what the XAML
    /// compiler generates for a class with compiled markup (the populate
    /// method the load call of the constructor is rewritten to). The
    /// compiled markup (`compiled_xaml.rs`) is the document compiled with
    /// every document it includes as one group, exactly as the run-time
    /// loader loads it.
    fn load(sp: Option<Rc<dyn IServiceProvider>>, this: &Ref<Self>, parts: rt::ThemeParts) -> Result<(), XamlLoadError> {
        register_types();
        crate::compiled_xaml::populate_with_parts(sp, this, parts)
    }
}
