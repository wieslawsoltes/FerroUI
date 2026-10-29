//! The class of the document `MaterialTheme.xaml`: the Android Material theme
//! (docs/porting/material-theme.md).
//!
//! Compiled styles with complete light/dark color schemes, configurable motion,
//! reduced-motion support, and a generated high-contrast scheme. The inherited
//! compact-density API preserves Material's minimum interactive touch targets.

use crate::register_types::register_types;
use crate::{MaterialColorScheme, MaterialMotionRole, MaterialMotionScheme};
use ferroui_base::controls::{
    IResourceNode, IResourceProvider, ResourceDictionary, ResourceHostRef, ResourceKey,
    ResourceValue, ResourcesChangedEventArgs,
};
use ferroui_base::metadata::{from_markup_value, into_markup_value, IServiceProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{IStyle, Styles, ThemeVariant};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue,
    DirectProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref,
};
use ferroui_markup_xaml::XamlLoadException;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// Material's high-contrast variant, inheriting the dark color-scheme preference.
    static HIGH_CONTRAST: ThemeVariant = ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()));
}

/// Layout density. Both modes preserve Material's minimum 48dp touch targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DensityStyle {
    #[default]
    Normal,
    Compact,
}

ferro_markup_enum!(DensityStyle { Normal, Compact }, { namespace: "FerroUI.Themes.Material" });

/// Includes the Android Material theme in an application.
#[repr(C)]
pub struct MaterialTheme {
    base: Styles,
    compact_styles: OnceCell<Ref<ResourceDictionary>>,
    density_style: Cell<DensityStyle>,
    motion_scheme: Cell<MaterialMotionScheme>,
    reduced_motion: Cell<bool>,
    pub(crate) motion_policy: crate::motion::MaterialMotionPolicy,
    pub(crate) android_color_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(MaterialTheme: Styles);
ferro_class_info!(MaterialTheme {
    new: MaterialTheme::new,
    interfaces: [
        Rc<dyn IStyle> => MaterialTheme::style_of,
        Rc<dyn IResourceProvider> => MaterialTheme::resource_provider_of,
    ],
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => MaterialTheme::with_service_provider],
        fields: [HighContrast: ThemeVariant => MaterialTheme::high_contrast],
    },
});

impl FerroObjectImpl for MaterialTheme {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::density_style_property().as_property() {
            if let Some(owner) = this.owner() {
                owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
            }
        }
        if change.property() == Self::motion_scheme_property().as_property()
            || change.property() == Self::reduced_motion_property().as_property()
        {
            this.refresh_motion_resources();
        }
    }
}

ferro_properties! {
    impl MaterialTheme {
        /// The Material spring scheme used by control transitions.
        pub fn motion_scheme_property() -> DirectProperty<MaterialTheme, MaterialMotionScheme> {
            FerroProperty::register_direct::<MaterialTheme, _>(
                "MotionScheme", |o| o.motion_scheme(), Some(|o, v| o.set_motion_scheme(v)), MaterialMotionScheme::Expressive,
            )
        }
        /// Disables nonessential motion. Platform accessibility settings are also honored.
        pub fn reduced_motion_property() -> DirectProperty<MaterialTheme, bool> {
            FerroProperty::register_direct::<MaterialTheme, _>(
                "ReducedMotion", |o| o.reduced_motion(), Some(|o, v| o.set_reduced_motion(v)), false,
            )
        }
        /// Defines the `DensityStyle` property.
        pub fn density_style_property() -> DirectProperty<MaterialTheme, DensityStyle> {
            FerroProperty::register_direct::<MaterialTheme, _>(
                "DensityStyle",
                |o| o.density_style(),
                Some(|o, v| o.set_density_style(v)),
                DensityStyle::Normal,
            )
        }
    }
}

impl MaterialTheme {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Themes.Material/MaterialTheme.xaml";

    /// Material's generated high-contrast dark scheme. Request it with
    /// `window.set_requested_theme_variant(Some(MaterialTheme::high_contrast()))`
    /// or `RequestedThemeVariant="{x:Static material:MaterialTheme.HighContrast}"`.
    /// Applications can replace its palette with [`Self::set_color_scheme`].
    pub fn high_contrast() -> ThemeVariant {
        HIGH_CONTRAST.with(Clone::clone)
    }

    /// Field initialisation; see [`Styles::construct`].
    pub fn construct() -> Self {
        Self {
            base: Styles::construct(),
            compact_styles: OnceCell::new(),
            density_style: Cell::new(DensityStyle::Normal),
            motion_scheme: Cell::new(MaterialMotionScheme::Expressive),
            reduced_motion: Cell::new(false),
            motion_policy: Default::default(),
            android_color_subscription: RefCell::new(None),
        }
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
        let compact_styles =
            from_markup_value::<Ref<ResourceDictionary>>(&Some(compact_styles.clone()))
                .unwrap_or_else(|| {
                    panic!(
                        "Unable to cast object of type '{}' to type 'ResourceDictionary'.",
                        (*compact_styles).type_name()
                    )
                });
        let _ = this.compact_styles.set(compact_styles);

        this.set_color_scheme(ThemeVariant::light(), &MaterialColorScheme::expressive_light());
        this.set_color_scheme(ThemeVariant::dark(), &MaterialColorScheme::dark());
        this.set_color_scheme(ThemeVariant::default(), &MaterialColorScheme::dark());
        let high_contrast = MaterialColorScheme::from_seed_color(
            ferroui_base::media::Color::from_rgb(0x67, 0x50, 0xa4),
            true,
            1.0,
        )
        .expect("a finite contrast value");
        this.set_color_scheme(Self::high_contrast(), &high_contrast);
        this.refresh_motion_resources();

        this
    }

    fn get_and_remove(&self, key: &str) -> BoxedValue {
        let resources = self.resources();
        let key = ResourceKey::from(key);
        let value = resources.try_get_value(&key).flatten().unwrap_or_else(|| {
            panic!(
                "Key {} was not found in the resources",
                key.as_str().unwrap_or_default()
            )
        });
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

    /// Replaces every color role and its state brushes atomically for a variant.
    /// Supply a full Android dynamic-color or Material Theme Builder scheme here.
    /// Existing controls update through DynamicResource without rebuilding their templates.
    pub fn set_color_scheme(&self, variant: ThemeVariant, scheme: &MaterialColorScheme) {
        self.resources()
            .set_theme_dictionary(variant, scheme.resources());
    }

    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.motion_scheme.get()
    }

    pub fn set_motion_scheme(&self, value: MaterialMotionScheme) {
        self.set_and_raise_cell(Self::motion_scheme_property(), &self.motion_scheme, value);
    }

    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion.get()
    }

    pub fn set_reduced_motion(&self, value: bool) {
        self.set_and_raise_cell(Self::reduced_motion_property(), &self.reduced_motion, value);
    }

    fn refresh_motion_resources(&self) {
        let resources = self.resources();
        let scheme = self.motion_scheme();
        let reduced = self.reduced_motion();
        self.motion_policy.set_reduced_motion(reduced);
        for role in MaterialMotionRole::ALL {
            resources.set(
                format!("MaterialMotion{}Duration", role.resource_name()),
                into_markup_value(scheme.duration(role, reduced)),
            );
            resources.set(
                format!("MaterialMotion{}Easing", role.resource_name()),
                into_markup_value(scheme.easing_with_policy(role, &self.motion_policy)),
            );
        }
        resources.set("MaterialReducedMotion", into_markup_value(reduced));
        resources.set("MaterialMotionScheme", into_markup_value(scheme));
        // PageSlide has ordinary Rust properties, so build it here rather than
        // attaching DynamicResource to its Duration in compiled XAML. Replacing
        // these resources also updates existing NavigationPage instances.
        use ferroui_base::animation::{IPageTransition, PageSlide, SlideAxis};
        for (key, axis) in [
            ("MaterialPageTransition", SlideAxis::Horizontal),
            ("MaterialModalTransition", SlideAxis::Vertical),
        ] {
            let role = MaterialMotionRole::DefaultSpatial;
            let slide = PageSlide::with_duration(scheme.duration(role, reduced), axis);
            slide.set_slide_in_easing(scheme.easing_with_policy(role, &self.motion_policy));
            slide.set_slide_out_easing(scheme.easing_with_policy(role, &self.motion_policy));
            let transition: Rc<dyn IPageTransition> = Rc::new(slide);
            resources.set(key, into_markup_value(transition));
        }
    }

    /// Tries to find a resource within the theme: the compact resources are
    /// checked first when the density style is compact.
    pub fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        if self.density_style.get() == DensityStyle::Compact {
            if let Some(value) = self
                .compact_styles
                .get()
                .and_then(|styles| styles.try_get_resource(key, theme))
            {
                return Some(value);
            }
        }

        let styles: &Styles = self;
        styles.try_get_resource(key, theme)
    }

    /// The theme as a style, for the style collections of applications and
    /// elements (`application.styles().add(theme.as_style())`).
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        Rc::new(MaterialThemeHandle(self.to_ref()))
    }

    /// The theme as a resource provider.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        Rc::new(MaterialThemeHandle(self.to_ref()))
    }

    fn style_of(this: Ref<Self>) -> Rc<dyn IStyle> {
        Rc::new(MaterialThemeHandle(this))
    }

    fn resource_provider_of(this: Ref<Self>) -> Rc<dyn IResourceProvider> {
        Rc::new(MaterialThemeHandle(this))
    }

    /// Populates `this` from the document of the class with the compiled
    /// markup (`compiled_xaml.rs`): the document compiled with every document
    /// it includes as one group, exactly as the run-time loader loads it.
    fn load(
        sp: Option<Rc<dyn IServiceProvider>>,
        this: &Ref<Self>,
    ) -> Result<(), XamlLoadException> {
        register_types();
        crate::compiled_xaml::populate(sp, this)
    }
}

/// The style and resource provider interfaces of the theme: those of a
/// styles collection, with the resource lookup of the theme.
struct MaterialThemeHandle(Ref<MaterialTheme>);

impl IResourceNode for MaterialThemeHandle {
    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }

    fn has_resources(&self) -> bool {
        self.0.has_resources()
    }

    fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}

impl IResourceProvider for MaterialThemeHandle {
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

impl IStyle for MaterialThemeHandle {
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
