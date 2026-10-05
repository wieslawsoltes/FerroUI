//! Port of `FluentTheme.xaml.cs`: the class of the document
//! `FluentTheme.xaml`.

use crate::register_types::register_types;
use crate::ColorPaletteResourcesCollection;
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

/// How much space the controls of the Fluent theme take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DensityStyle {
    #[default]
    Normal,
    Compact,
}

ferro_markup_enum!(DensityStyle { Normal, Compact }, { namespace: "FerroUI.Themes.Fluent" });

/// Includes the fluent theme in an application.
#[repr(C)]
pub struct FluentTheme {
    base: Styles,
    compact_styles: OnceCell<Ref<ResourceDictionary>>,
    density_style: Cell<DensityStyle>,
    palettes: OnceCell<Ref<ColorPaletteResourcesCollection>>,
}

ferro_class!(FluentTheme: Styles);
ferro_class_info!(FluentTheme {
    new: FluentTheme::new,
    interfaces: [
        Rc<dyn IStyle> => FluentTheme::style_of,
        Rc<dyn IResourceProvider> => FluentTheme::resource_provider_of,
    ],
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => FluentTheme::with_service_provider],
        properties: [
            Palettes: Ref<ColorPaletteResourcesCollection> { get: FluentTheme::palettes },
        ],
    },
});

impl FerroObjectImpl for FluentTheme {
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
    impl FluentTheme {
        /// Defines the `DensityStyle` property.
        pub fn density_style_property() -> DirectProperty<FluentTheme, DensityStyle> {
            FerroProperty::register_direct::<FluentTheme, _>(
                "DensityStyle",
                |o| o.density_style(),
                Some(|o, v| o.set_density_style(v)),
                DensityStyle::Normal,
            )
        }
    }
}

impl FluentTheme {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Themes.Fluent/FluentTheme.xaml";

    /// Field initialisation; see [`Styles::construct`].
    pub fn construct() -> Self {
        Self {
            base: Styles::construct(),
            compact_styles: OnceCell::new(),
            density_style: Cell::new(DensityStyle::Normal),
            palettes: OnceCell::new(),
        }
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
    /// Panics if the documents of the theme fail to load, or if the loaded
    /// document lacks the compact styles or the palette collection.
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

        let palettes = this
            .resources()
            .merged_dictionaries()
            .snapshot()
            .iter()
            .find_map(|provider| {
                provider.as_object().and_then(|object| object.downcast_ref::<ColorPaletteResourcesCollection>()).map(|c| c.to_ref())
            })
            .unwrap_or_else(|| panic!("FluentTheme was initialized with missing ColorPaletteResourcesCollection."));
        let _ = this.palettes.set(palettes);

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

    /// The density style of the fluent theme (normal, compact).
    pub fn density_style(&self) -> DensityStyle {
        self.density_style.get()
    }

    pub fn set_density_style(&self, value: DensityStyle) {
        self.set_and_raise_cell(Self::density_style_property(), &self.density_style, value);
    }

    /// The colour palettes of the theme, by theme variant.
    pub fn palettes(&self) -> Ref<ColorPaletteResourcesCollection> {
        self.palettes.get().expect("the palettes are set by the constructor").clone()
    }

    /// Tries to find a resource within the theme: the compact styles are
    /// checked first when the density style is compact.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        // DensityStyle dictionary should be checked first
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
        Rc::new(FluentThemeHandle(self.to_ref()))
    }

    /// The theme as a resource provider.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        Rc::new(FluentThemeHandle(self.to_ref()))
    }

    fn style_of(this: Ref<Self>) -> Rc<dyn IStyle> {
        Rc::new(FluentThemeHandle(this))
    }

    fn resource_provider_of(this: Ref<Self>) -> Rc<dyn IResourceProvider> {
        Rc::new(FluentThemeHandle(this))
    }

    /// Populates `this` from the document of the class: what the XAML
    /// compiler generates for a class with compiled markup (the populate
    /// method the load call of the constructor is rewritten to). The
    /// compiled markup (`compiled_xaml.rs`) is the document compiled with
    /// every document it includes as one group, exactly as the run-time
    /// loader loads it.
    fn load(sp: Option<Rc<dyn IServiceProvider>>, this: &Ref<Self>) -> Result<(), XamlLoadException> {
        register_types();
        crate::compiled_xaml::populate(sp, this)
    }
}

/// The style and resource provider interfaces of the theme: those of a
/// styles collection, with the resource lookup of the theme (the explicit
/// `IResourceNode.TryGetResource` of the managed class).
struct FluentThemeHandle(Ref<FluentTheme>);

impl IResourceNode for FluentThemeHandle {
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

impl IResourceProvider for FluentThemeHandle {
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

impl IStyle for FluentThemeHandle {
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
