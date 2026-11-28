//! The classes of the theme documents of the library. Each is a styles collection populated from the
//! units of its compiled markup, as the themes of the framework are (#568).

use std::rc::Rc;

use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::Styles;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;

/// The theme `GeneratedRust.Library.A_B.PartsTheme`: constructed from selected units with
/// [`with_parts`](Self::with_parts).
#[repr(C)]
pub struct PartsTheme {
    base: Styles,
}

ferro_class!(PartsTheme: Styles);
ferro_impl_classes!(PartsTheme: FerroObjectImpl);
ferro_class_info!(PartsTheme { new: PartsTheme::new, markup: { namespace: "GeneratedRust.Library.A_B" } });

impl PartsTheme {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: Styles::construct() }
    }

    /// The theme with every unit.
    pub fn new() -> Ref<Self> {
        Self::with_parts(None, crate::compiled_parts_theme::THEME_UNITS)
    }

    /// The theme with the units `parts`, in canonical order.
    ///
    /// # Panics
    /// Panics if the document of the theme fails to load.
    pub fn with_parts(sp: Option<Rc<dyn IServiceProvider>>, parts: rt::ThemeParts) -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        if let Err(error) = crate::compiled_parts_theme::populate_with_parts(sp, &this, parts) {
            panic!("{error}");
        }
        this
    }
}

/// The theme `GeneratedRust.Library.A.B.Partstheme`: constructed from selected units with
/// [`with_parts`](Self::with_parts).
#[repr(C)]
pub struct Partstheme {
    base: Styles,
}

ferro_class!(Partstheme: Styles);
ferro_impl_classes!(Partstheme: FerroObjectImpl);
ferro_class_info!(Partstheme { new: Partstheme::new, markup: { namespace: "GeneratedRust.Library.A.B" } });

impl Partstheme {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: Styles::construct() }
    }

    /// The theme with every unit.
    pub fn new() -> Ref<Self> {
        Self::with_parts(None, crate::compiled_parts_theme_lower::THEME_UNITS)
    }

    /// The theme with the units `parts`, in canonical order.
    ///
    /// # Panics
    /// Panics if the document of the theme fails to load.
    pub fn with_parts(sp: Option<Rc<dyn IServiceProvider>>, parts: rt::ThemeParts) -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        if let Err(error) = crate::compiled_parts_theme_lower::populate_with_parts(sp, &this, parts) {
            panic!("{error}");
        }
        this
    }
}

/// The theme `GeneratedRust.Library.RenamedTheme`: its markup is partitioned into units, and its
/// constructor from some of them is [`from_parts`](Self::from_parts), which the compiler does not
/// construct a class with.
#[repr(C)]
pub struct RenamedTheme {
    base: Styles,
}

ferro_class!(RenamedTheme: Styles);
ferro_impl_classes!(RenamedTheme: FerroObjectImpl);
ferro_class_info!(RenamedTheme { new: RenamedTheme::new });

impl RenamedTheme {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: Styles::construct() }
    }

    /// The theme with every unit.
    pub fn new() -> Ref<Self> {
        Self::from_parts(None, crate::compiled_renamed_theme::THEME_UNITS)
    }

    /// The theme with the units `parts`, in canonical order.
    ///
    /// # Panics
    /// Panics if the document of the theme fails to load.
    pub fn from_parts(sp: Option<Rc<dyn IServiceProvider>>, parts: rt::ThemeParts) -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        if let Err(error) = crate::compiled_renamed_theme::populate_with_parts(sp, &this, parts) {
            panic!("{error}");
        }
        this
    }
}
