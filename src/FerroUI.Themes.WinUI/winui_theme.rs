//! The class of the document `WinUITheme.xaml`: the WinUI theme
//! (docs/porting/winui-theme.md, section 3).
//!
//! The class has the public surface of `FluentTheme` of `ferroui-themes-fluent` where it applies:
//! it is a styles collection constructed from its compiled document, with a density style whose
//! compact resources are looked up first. It has no colour palettes (WinUI has none); it adds the
//! high contrast theme variant of WinUI, which the theme dictionaries of its documents are keyed
//! by.

use crate::register_types::register_types;
use ferroui_base::controls::{
    IResourceNode, IResourceProvider, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceValue,
    ResourcesChangedEventArgs,
};
use ferroui_base::metadata::{from_markup_value, IServiceProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{IStyle, Styles, ThemeVariant};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
};
use ferroui_markup_xaml::XamlLoadException;
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The high contrast variant: the theme dictionaries `HighContrast` of WinUI.
    static HIGH_CONTRAST: ThemeVariant = ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()));
}

/// How much space the controls of the WinUI theme take: the normal density, or the compact one of
/// WinUI (`compact_themeresources.xaml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DensityStyle {
    #[default]
    Normal,
    Compact,
}

ferro_markup_enum!(DensityStyle { Normal, Compact }, { namespace: "FerroUI.Themes.WinUI" });

/// Includes the WinUI theme in an application.
#[repr(C)]
pub struct WinUITheme {
    base: Styles,
    compact_styles: OnceCell<Ref<ResourceDictionary>>,
    density_style: Cell<DensityStyle>,
}

ferro_class!(WinUITheme: Styles);
ferro_class_info!(WinUITheme {
    new: WinUITheme::new,
    interfaces: [
        Rc<dyn IStyle> => WinUITheme::style_of,
        Rc<dyn IResourceProvider> => WinUITheme::resource_provider_of,
    ],
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => WinUITheme::with_service_provider],
        fields: [HighContrast: ThemeVariant => WinUITheme::high_contrast],
    },
});

impl FerroObjectImpl for WinUITheme {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::density_style_property().as_property() {
            if let Some(owner) = this.owner() {
                owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
            }
        }
    }
}

ferro_properties! {
    impl WinUITheme {
        /// Defines the `DensityStyle` property.
        pub fn density_style_property() -> DirectProperty<WinUITheme, DensityStyle> {
            FerroProperty::register_direct::<WinUITheme, _>(
                "DensityStyle",
                |o| o.density_style(),
                Some(|o, v| o.set_density_style(v)),
                DensityStyle::Normal,
            )
        }
    }
}

impl WinUITheme {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Themes.WinUI/WinUITheme.xaml";

    /// The high contrast theme variant of the theme: the `HighContrast` theme dictionaries of
    /// WinUI. Request it like the light and dark variants
    /// (`window.set_requested_theme_variant(Some(WinUITheme::high_contrast()))`, or
    /// `RequestedThemeVariant="{x:Static winui:WinUITheme.HighContrast}"` in markup). It inherits
    /// the dark variant, which the platform is told of (the colours of its dictionaries are the
    /// ones of the "High Contrast Black" theme of Windows).
    pub fn high_contrast() -> ThemeVariant {
        HIGH_CONTRAST.with(Clone::clone)
    }

    /// Field initialisation; see [`Styles::construct`].
    pub fn construct() -> Self {
        Self { base: Styles::construct(), compact_styles: OnceCell::new(), density_style: Cell::new(DensityStyle::Normal) }
    }

    /// Creates the theme.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load.
    pub fn new() -> Ref<Self> {
        Self::with_service_provider(None)
    }

    /// Creates the theme; `sp` is the parent's service provider.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load, or if the loaded
    /// document lacks the compact styles.
    pub fn with_service_provider(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Err(error) = Self::load(sp, &this) {
            panic!("{error}");
        }

        let compact_styles = this.get_and_remove("CompactStyles");
        let compact_styles = from_markup_value::<Ref<ResourceDictionary>>(&Some(compact_styles.clone()))
            .unwrap_or_else(|| {
                panic!("Unable to cast object of type '{}' to type 'ResourceDictionary'.", (*compact_styles).type_name())
            });
        let _ = this.compact_styles.set(compact_styles);

        this
    }

    fn get_and_remove(&self, key: &str) -> BoxedValue {
        let resources = self.resources();
        let key = ResourceKey::from(key);
        let value = resources
            .try_get_value(&key)
            .flatten()
            .unwrap_or_else(|| panic!("Key {} was not found in the resources", key.as_str().unwrap_or_default()));
        resources.remove(&key);
        value
    }

    /// The density style of the theme (normal, compact).
    pub fn density_style(&self) -> DensityStyle {
        self.density_style.get()
    }

    pub fn set_density_style(&self, value: DensityStyle) {
        self.set_and_raise_cell(Self::density_style_property(), &self.density_style, value);
    }

    /// Tries to find a resource within the theme: the compact resources are
    /// checked first when the density style is compact.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        if self.density_style.get() == DensityStyle::Compact {
            if let Some(value) = self.compact_styles.get().and_then(|styles| styles.try_get_resource(key, theme)) {
                return Some(value);
            }
        }

        let styles: &Styles = self;
        styles.try_get_resource(key, theme)
    }

    /// The theme as a style, for the style collections of applications and
    /// elements (`application.styles().add(theme.as_style())`).
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        Rc::new(WinUIThemeHandle(self.to_ref()))
    }

    /// The theme as a resource provider.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        Rc::new(WinUIThemeHandle(self.to_ref()))
    }

    fn style_of(this: Ref<Self>) -> Rc<dyn IStyle> {
        Rc::new(WinUIThemeHandle(this))
    }

    fn resource_provider_of(this: Ref<Self>) -> Rc<dyn IResourceProvider> {
        Rc::new(WinUIThemeHandle(this))
    }

    /// Populates `this` from the document of the class with the compiled
    /// markup (`compiled_xaml.rs`): the document compiled with every document
    /// it includes as one group, exactly as the run-time loader loads it.
    fn load(sp: Option<Rc<dyn IServiceProvider>>, this: &Ref<Self>) -> Result<(), XamlLoadException> {
        register_types();
        crate::compiled_xaml::populate(sp, this)
    }
}

/// The style and resource provider interfaces of the theme: those of a
/// styles collection, with the resource lookup of the theme.
struct WinUIThemeHandle(Ref<WinUITheme>);

impl IResourceNode for WinUIThemeHandle {
    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }

    fn has_resources(&self) -> bool {
        self.0.has_resources()
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}

impl IResourceProvider for WinUIThemeHandle {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn owner(&self) -> Option<ResourceHostRef> {
        self.0.owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.owner_changed(move || handler())
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        self.0.add_owner(owner)
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.0.remove_owner(owner)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(&self.0)
    }
}

impl IStyle for WinUIThemeHandle {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        self.0.snapshot()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(&self.0)
    }

    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        Some(self)
    }
}
